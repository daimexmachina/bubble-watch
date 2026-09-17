//! Fundamental indicators, built from SEC XBRL filings.
//!
//! These carry the heaviest weights in the model because they are the only
//! inputs that are *audited accounting figures* rather than market prices. That
//! makes them the least subjective things we can measure — and the reason the
//! capex-versus-cash-flow ratio is weighted above any price-based signal.
//!
//! Every one of them has a known, stated limitation (leases off balance sheet,
//! revenue mostly not AI revenue, shares net of buybacks). Each `detail` string
//! names its own limitation inline so no reader can mistake the proxy for the
//! ideal.

use super::{Ctx, Indicator};
use crate::model::{CompanyFacts, Provenance, Reading, TtmBasis};

fn cohort_provenance(ctx: &Ctx, as_of: &str) -> Provenance {
    Provenance {
        source: "sec-edgar".into(),
        endpoint: format!(
            "data.sec.gov/api/xbrl/companyconcept — {} filers: {}",
            ctx.obs.edgar.len(),
            ctx.obs.edgar.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
        as_of: as_of.to_string(),
        retrieved_at: ctx.obs.retrieved_at.clone(),
    }
}

fn basis_note(b: TtmBasis) -> &'static str {
    match b {
        TtmBasis::FourQuarters => "TTM from four quarters",
        TtmBasis::AnnualFallback => {
            "TTM from most recent annual filing (quarterly data insufficient)"
        }
    }
}
/// Sum a per-company TTM quantity across the cohort, tracking the newest period
/// end date and which basis each company's number came from.
struct CohortSum {
    total: f64,
    used: Vec<String>,
    newest_end: String,
    annual_fallbacks: Vec<String>,
}

fn cohort_ttm<F>(ctx: &Ctx, pick: F) -> CohortSum
where
    F: Fn(&CompanyFacts) -> &Vec<crate::model::EdgarFact>,
{
    let mut out = CohortSum {
        total: 0.0,
        used: Vec::new(),
        newest_end: String::new(),
        annual_fallbacks: Vec::new(),
    };
    for (ticker, cf) in &ctx.obs.edgar {
        if let Some((v, basis)) = CompanyFacts::ttm(pick(cf)) {
            out.total += v;
            out.used.push(ticker.clone());
            if basis == TtmBasis::AnnualFallback {
                out.annual_fallbacks.push(ticker.clone());
            }
            if let Some(f) = pick(cf)
                .iter()
                .filter(|f| (80..=100).contains(&f.days) || (350..=380).contains(&f.days))
                .map(|f| f.end.clone())
                .max()
            {
                if f > out.newest_end {
                    out.newest_end = f;
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------

pub struct CapexVsCashflow;

impl Indicator for CapexVsCashflow {
    fn id(&self) -> &'static str {
        "capex_vs_cashflow"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };

        let capex = cohort_ttm(ctx, |c| &c.capex);
        let cfo = cohort_ttm(ctx, |c| &c.cfo);

        if capex.used.is_empty() || cfo.used.is_empty() {
            return Reading::Unavailable {
                reason: "no cohort filings retrieved from SEC EDGAR".into(),
            };
        }
        if cfo.total <= 0.0 {
            return Reading::Unavailable {
                reason: "cohort operating cash flow is non-positive; ratio undefined".into(),
            };
        }

        let ratio = capex.total / cfo.total;
        let stress = crate::score::interpolate(ratio, &ic.anchors);

        // Per-company breakdown, so the aggregate can be inspected rather than
        // trusted. This is the number a reader should sanity-check first.
        let mut rows = Vec::new();
        for ticker in &capex.used {
            if let Some(cf) = ctx.obs.edgar.get(ticker) {
                if let (Some((cx, _)), Some((co, _))) =
                    (CompanyFacts::ttm(&cf.capex), CompanyFacts::ttm(&cf.cfo))
                {
                    if co > 0.0 {
                        rows.push(format!("{}: {:.2}x", ticker, cx / co));
                    }
                }
            }
        }
        rows.sort();

        let mut notes = Vec::new();
        if !capex.annual_fallbacks.is_empty() {
            notes.push(format!(
                "annual-basis fallback for {}",
                capex.annual_fallbacks.join(", ")
            ));
        }

        Reading::Scored {
            stress,
            value: ratio,
            unit: ic.unit.clone(),
            detail: format!(
                "Cohort TTM capex ${:.1}B / TTM operating cash flow ${:.1}B = {:.2}x across {} \
                 filers ({}). Per-company: {}. Approaching or exceeding 1.0x means the buildout \
                 is no longer self-funding and must be financed externally — the regime that \
                 turns an earnings disappointment into a refinancing problem. Reported debt and \
                 cash flow exclude the large off-balance-sheet lease and purchase commitments, \
                 so this UNDERSTATES the true commitment burden.",
                capex.total / 1e9,
                cfo.total / 1e9,
                ratio,
                capex.used.len(),
                if notes.is_empty() {
                    basis_note(TtmBasis::FourQuarters).to_string()
                } else {
                    format!(
                        "{} — {}",
                        basis_note(TtmBasis::AnnualFallback),
                        notes.join("; ")
                    )
                },
                rows.join(", ")
            ),
            provenance: cohort_provenance(ctx, &capex.newest_end),
        }
    }
}

pub struct FundingGap;

impl Indicator for FundingGap {
    fn id(&self) -> &'static str {
        "funding_gap"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };
        let capex = cohort_ttm(ctx, |c| &c.capex);
        let rev = cohort_ttm(ctx, |c| &c.revenue);

        if capex.used.is_empty() || rev.used.is_empty() || rev.total <= 0.0 {
            return Reading::Unavailable {
                reason: "cohort capex or revenue unavailable from SEC EDGAR".into(),
            };
        }
        let ratio = capex.total / rev.total;
        let stress = crate::score::interpolate(ratio, &ic.anchors);

        // Share of filers whose capex exceeds a third of revenue — a simple,
        // inspectable concentration check to accompany the aggregate.
        let mut heavy = Vec::new();
        for ticker in &rev.used {
            if let Some(cf) = ctx.obs.edgar.get(ticker) {
                if let (Some((cx, _)), Some((rv, _))) =
                    (CompanyFacts::ttm(&cf.capex), CompanyFacts::ttm(&cf.revenue))
                {
                    if rv > 0.0 && cx / rv > 0.33 {
                        heavy.push(format!("{} {:.0}%", ticker, cx / rv * 100.0));
                    }
                }
            }
        }
        heavy.sort();

        Reading::Scored {
            stress,
            value: ratio,
            unit: ic.unit.clone(),
            detail: format!(
                "Cohort TTM capex ${:.1}B against TTM revenue ${:.1}B = {:.1}% of revenue. \
                 LIMITATION, stated plainly: hyperscaler revenue is mostly non-AI cloud and \
                 advertising, so this is NOT the widely-quoted ~$600B AI capex-versus-AI-revenue \
                 gap, which requires segment data unavailable in XBRL. Read it as the direction \
                 and scale of the buildout relative to the businesses funding it. Filers above \
                 one third of revenue: {}.",
                capex.total / 1e9,
                rev.total / 1e9,
                ratio * 100.0,
                if heavy.is_empty() {
                    "none".to_string()
                } else {
                    heavy.join(", ")
                }
            ),
            provenance: cohort_provenance(ctx, &capex.newest_end),
        }
    }
}

pub struct Leverage;

impl Indicator for Leverage {
    fn id(&self) -> &'static str {
        "leverage"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };
        let cfo = cohort_ttm(ctx, |c| &c.cfo);
        if cfo.used.is_empty() || cfo.total <= 0.0 {
            return Reading::Unavailable {
                reason: "cohort operating cash flow unavailable from SEC EDGAR".into(),
            };
        }

        let mut debt_total = 0.0;
        let mut used = Vec::new();
        let mut newest = String::new();
        let mut rows = Vec::new();
        for (ticker, cf) in &ctx.obs.edgar {
            let Some(d) = CompanyFacts::latest(&cf.debt) else {
                continue;
            };
            let Some((co, _)) = CompanyFacts::ttm(&cf.cfo) else {
                continue;
            };
            if co <= 0.0 {
                continue;
            }
            debt_total += d.val;
            used.push(ticker.clone());
            if d.end > newest {
                newest = d.end.clone();
            }
            rows.push(format!("{}: {:.2}x", ticker, d.val / co));
        }

        if used.is_empty() {
            return Reading::Unavailable {
                reason: "no cohort long-term debt facts retrieved from SEC EDGAR".into(),
            };
        }

        let ratio = debt_total / cfo.total;
        let stress = crate::score::interpolate(ratio, &ic.anchors);
        rows.sort();

        Reading::Scored {
            stress,
            value: ratio,
            unit: ic.unit.clone(),
            detail: format!(
                "Cohort reported long-term debt ${:.1}B against TTM operating cash flow ${:.1}B \
                 = {:.2}x ({} filers). Per-company: {}. The anchors are set so that current \
                 levels read as MODERATE stress, consistent with Capital Economics' view that \
                 leverage is not yet alarming relative to valuations. IMPORTANT LIMITATION: this \
                 uses the reported balance-sheet debt line only, so it excludes the very large \
                 lease and multi-year purchase commitments that sit off balance sheet — true \
                 leverage is materially higher than this ratio suggests.",
                debt_total / 1e9,
                cfo.total / 1e9,
                ratio,
                used.len(),
                rows.join(", ")
            ),
            provenance: cohort_provenance(ctx, &newest),
        }
    }
}

pub struct Issuance;

impl Indicator for Issuance {
    fn id(&self) -> &'static str {
        "issuance"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };

        let mut growth: Vec<(String, f64)> = Vec::new();
        let mut newest = String::new();
        let mut no_share_data: Vec<String> = Vec::new();

        for (ticker, cf) in &ctx.obs.edgar {
            let facts = &cf.shares;
            let Some(latest) = CompanyFacts::latest(facts) else {
                // Not every filer publishes a share-count concept under any of
                // the tags we know. META is the concrete example: all three
                // candidate tags 404 for it. Say so rather than silently
                // shrinking the sample.
                no_share_data.push(ticker.clone());
                continue;
            };
            // Find the observation closest to one year before the latest.
            // `days_between(a, b)` returns b - a, so for an earlier fact this is
            // the number of days it sits BEFORE the latest observation. Compare
            // it against a POSITIVE target, not a negative one.
            let target = 365i64;
            let mut prior: Option<&crate::model::EdgarFact> = None;
            let mut best_gap = i64::MAX;
            for f in facts {
                let ago = crate::sources::edgar::days_between(&f.end, &latest.end);
                if ago <= 0 {
                    continue; // the latest itself, or a later duplicate
                }
                let gap = (ago - target).abs();
                if gap < best_gap {
                    best_gap = gap;
                    prior = Some(f);
                }
            }
            let Some(prior) = prior else {
                no_share_data.push(ticker.clone());
                continue;
            };
            if prior.val <= 0.0 {
                no_share_data.push(ticker.clone());
                continue;
            }
            // Refuse a "year" that is not actually about a year.
            if best_gap > 120 {
                no_share_data.push(ticker.clone());
                continue;
            }
            growth.push((ticker.clone(), (latest.val / prior.val - 1.0) * 100.0));
            if latest.end > newest {
                newest = latest.end.clone();
            }
        }

        if growth.is_empty() {
            return Reading::Unavailable {
                reason: "no cohort share-count history with a usable year-earlier comparison"
                    .into(),
            };
        }

        growth.sort_by(|a, b| a.0.cmp(&b.0));
        let mean = growth.iter().map(|(_, g)| *g).sum::<f64>() / growth.len() as f64;
        let stress = crate::score::interpolate(mean, &ic.anchors);
        let rows: Vec<String> = growth
            .iter()
            .map(|(t, g)| format!("{} {:+.2}%", t, g))
            .collect();

        Reading::Scored {
            stress,
            value: mean,
            unit: ic.unit.clone(),
            detail: format!(
                "Mean trailing share-count change across {} filers: {:+.2}% ({}).{} \
                 PROXY, and a weak one: buybacks net against issuance, and a five-name mega-cap \
                 cohort cannot see the broad IPO/secondary wave that historically marks a peak, \
                 so this UNDERSTATES a genuine supply surge. Included because issuance is one of \
                 the more reliable pre-peak signals in the literature; treat it as a lower bound.",
                growth.len(),
                mean,
                rows.join(", "),
                if no_share_data.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Filers with no usable share-count series (excluded, not zero-filled): {}.",
                        no_share_data.join(", ")
                    )
                }
            ),
            provenance: cohort_provenance(ctx, &newest),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::model::{EdgarFact, Observations};

    fn q(tag: &str, start: &str, end: &str, val: f64) -> EdgarFact {
        EdgarFact {
            tag: tag.into(),
            start: start.into(),
            end: end.into(),
            val,
            form: "10-Q".into(),
            filed: end.into(),
            days: crate::sources::edgar::days_between(start, end),
        }
    }

    #[test]
    fn ttm_sums_four_non_overlapping_quarters() {
        let facts = vec![
            q("x", "2025-07-01", "2025-09-30", 10.0),
            q("x", "2025-10-01", "2025-12-31", 10.0),
            q("x", "2026-01-01", "2026-03-31", 10.0),
            q("x", "2026-04-01", "2026-06-30", 10.0),
        ];
        let (v, basis) = CompanyFacts::ttm(&facts).unwrap();
        assert!((v - 40.0).abs() < 1e-9);
        assert_eq!(basis, TtmBasis::FourQuarters);
    }

    #[test]
    fn ttm_does_not_double_count_overlapping_periods() {
        // A cumulative YTD fact (Jan-Jun) overlaps two quarters and must not be
        // summed together with them.
        let facts = vec![
            q("x", "2026-01-01", "2026-03-31", 10.0),
            q("x", "2026-04-01", "2026-06-30", 10.0),
            q("x", "2025-07-01", "2025-09-30", 10.0),
            q("x", "2025-10-01", "2025-12-31", 10.0),
        ];
        let (v, _) = CompanyFacts::ttm(&facts).unwrap();
        assert!((v - 40.0).abs() < 1e-9, "got {}", v);
    }

    #[test]
    fn ttm_falls_back_to_annual_and_says_so() {
        let facts = vec![q("x", "2025-07-01", "2026-06-30", 100.0)];
        let (v, basis) = CompanyFacts::ttm(&facts).unwrap();
        assert!((v - 100.0).abs() < 1e-9);
        assert_eq!(
            basis,
            TtmBasis::AnnualFallback,
            "must disclose the weaker basis"
        );
    }

    #[test]
    fn ttm_is_none_with_no_facts() {
        assert!(CompanyFacts::ttm(&[]).is_none());
    }

    #[test]
    fn latest_picks_newest_by_end_date() {
        let facts = vec![
            q("x", "2025-01-01", "2025-03-31", 5.0),
            q("x", "2026-01-01", "2026-03-31", 9.0),
        ];
        assert!((CompanyFacts::latest(&facts).unwrap().val - 9.0).abs() < 1e-9);
    }

    #[test]
    fn capex_ratio_is_unavailable_without_filings() {
        let cfg = Config {
            meta: Meta {
                schema_version: "1".into(),
            },
            phase: PhaseCfg {
                early_max: 35.0,
                mid_max: 55.0,
                late_max: 75.0,
            },
            coverage_floor: CoverageFloor {
                low_below: 0.6,
                high_above: 0.85,
            },
            trend: TrendCfg::defaults(),
            analog: AnalogCfg {
                method: "historical_analog".into(),
                analogs: vec![],
                derivation: String::new(),
                caveat: String::new(),
            },
            analog_band: vec![],
            indicator: vec![IndicatorCfg {
                id: "capex_vs_cashflow".into(),
                label: "X".into(),
                weight: 14.0,
                unit: "ratio".into(),
                source: "edgar".into(),
                rationale: String::new(),
                anchors: vec![[0.0, 0.0], [2.0, 100.0]],
                fred_series: None,
            }],
        };
        let obs = Observations::default();
        let ctx = Ctx {
            obs: &obs,
            cfg: &cfg,
        };
        let r = CapexVsCashflow.evaluate(&ctx);
        assert!(!r.is_available(), "must not invent a ratio with no filings");
        match r {
            Reading::Unavailable { reason } => assert!(reason.contains("cohort")),
            _ => panic!("expected Unavailable"),
        }
    }
}
