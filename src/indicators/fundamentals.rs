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
/// Whether the cohort can carry the commitments it has signed — measured on
/// debt INCLUDING leases and disclosed purchase obligations.
///
/// Two corrections over the previous version, both measured 2026-09-17:
///
/// 1. **ORCL was reported as having no debt.** `LongTermDebt` resolves for Oracle
///    to a single stale fact ending 2022-05-31 with value 0.0, and the old
///    first-tag-that-answers resolver took it. Oracle's notes payable are ~$122bn
///    and it is the most exposed member of the cohort, so the bug reported the
///    most leveraged company as the least. Tag order now includes
///    `LongTermNotesPayable`, and a stale-or-zero series is rejected outright by
///    `CompanyFacts::point_in_time`.
/// 2. **Operating leases were excluded**, which the old rationale admitted
///    UNDERSTATED true leverage. They are now included: cohort-mean leverage moves
///    from 0.556x to 0.955x operating cash flow, i.e. stress 18 -> 33.
///
/// Purchase obligations are included where disclosed. They are NOT universally
/// reported — MSFT has no such concept under any candidate tag — and an absent
/// disclosure is rendered as "not disclosed", never as zero. Printing zero would
/// state that Meta's $279bn of take-or-pay commitments is nothing.
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
        let mut lease_total = 0.0;
        let mut purch_total = 0.0;
        let mut used = Vec::new();
        let mut newest = String::new();
        let mut rows = Vec::new();
        let mut rejected = Vec::new();
        // Two distinct cases that must not be conflated: a filer with no such
        // concept at all (MSFT), versus one that discloses it but whose latest
        // fact is too old to describe the present (AMZN, 809 days stale). Saying
        // "not disclosed" for the second would be false.
        let mut no_purchase_concept = Vec::new();
        let mut stale_purchase = Vec::new();

        for (ticker, cf) in &ctx.obs.edgar {
            let Some((co, _)) = CompanyFacts::ttm(&cf.cfo) else {
                continue;
            };
            if co <= 0.0 {
                continue;
            }

            // `newest` is the reference date for staleness: a filer's balance-sheet
            // items should be current relative to its own latest reporting.
            let as_of = cf
                .debt
                .iter()
                .chain(cf.lease.iter())
                .chain(cf.cfo.iter())
                .map(|f| f.end.as_str())
                .max()
                .unwrap_or("")
                .to_string();
            if as_of.is_empty() {
                continue;
            }

            // Debt: reject stale or exactly-zero series rather than reporting them.
            let debt = match CompanyFacts::point_in_time(&cf.debt, &as_of, 200) {
                Ok(f) => f.val,
                Err(why) => {
                    rejected.push(format!("{}: {}", ticker, why));
                    continue;
                }
            };
            let lease = CompanyFacts::point_in_time(&cf.lease, &as_of, 200)
                .map(|f| f.val)
                .unwrap_or(0.0);
            let purchase = match CompanyFacts::point_in_time(&cf.purchase_obligation, &as_of, 200) {
                Ok(f) => Some(f.val),
                Err(why) => {
                    if cf.purchase_obligation.is_empty() {
                        no_purchase_concept.push(ticker.clone());
                    } else {
                        stale_purchase.push(format!("{} ({})", ticker, why));
                    }
                    None
                }
            };

            debt_total += debt;
            lease_total += lease;
            if let Some(v) = purchase {
                purch_total += v;
            }
            used.push(ticker.clone());
            if as_of > newest {
                newest = as_of.clone();
            }

            let p = match purchase {
                Some(v) => format!("{:.2}x", (debt + lease + v) / co),
                None => format!("{:.2}x (+ commitments not disclosed)", (debt + lease) / co),
            };
            rows.push(format!("{}: {}", ticker, p));
        }

        if used.is_empty() {
            return Reading::Unavailable {
                reason: format!(
                    "no cohort filer had a current, non-zero long-term debt series. Rejected: {}",
                    rejected.join("; ")
                ),
            };
        }

        // Headline ratio: debt + leases only, over cash flow. Purchase obligations
        // are reported separately rather than folded in, because they are
        // disclosed inconsistently and blending a partial total into the headline
        // would make the ratio mean different things for different filers.
        let ratio = (debt_total + lease_total) / cfo.total;
        let stress = crate::score::interpolate(ratio, &ic.anchors);
        rows.sort();

        let purch_note = if purch_total > 0.0 {
            format!(
                " Additionally, ${:.1}B of unconditional purchase obligations are disclosed and \
                 excluded from the ratio above (they are not debt, and not universally reported): \
                 folding a partial total into the headline would make the ratio mean different \
                 things for different filers.",
                purch_total / 1e9
            )
        } else {
            String::new()
        };
        let mut undisclosed = String::new();
        if !no_purchase_concept.is_empty() {
            undisclosed.push_str(&format!(
                " Purchase obligations are NOT DISCLOSED AT ALL by {} — reported as not \
                 disclosed, never as zero.",
                no_purchase_concept.join(", ")
            ));
        }
        if !stale_purchase.is_empty() {
            undisclosed.push_str(&format!(
                " Purchase obligations are disclosed by {} but the latest reported figure is too \
                 old to describe the present, so it is excluded rather than treated as current \
                 or as zero.",
                stale_purchase.join("; ")
            ));
        }
        let rej = if rejected.is_empty() {
            String::new()
        } else {
            format!(
                " Series rejected rather than used (a stale or zero balance is a wrong tag, not a \
                 fact): {}.",
                rejected.join("; ")
            )
        };

        Reading::Scored {
            stress,
            value: ratio,
            unit: ic.unit.clone(),
            detail: format!(
                "Cohort long-term debt plus operating leases ${:.1}B against TTM operating cash \
                 flow ${:.1}B = {:.2}x ({} filers). Per-company, debt+leases{} over cash flow: \
                 {}. INCLUDES operating leases, which the previous version excluded and which \
                 the old rationale admitted understated true leverage; it also fixes a bug that \
                 reported ORCL as carrying no long-term debt at all. STILL EXCLUDES off-balance- \
                 sheet vehicles (SPVs, joint ventures, sale-leasebacks) that finance a growing \
                 share of data-centre construction, so true leverage remains higher than this \
                 ratio.{} {}",
                (debt_total + lease_total) / 1e9,
                cfo.total / 1e9,
                ratio,
                used.len(),
                if purch_total > 0.0 { "+purchases" } else { "" },
                rows.join(", "),
                purch_note,
                format!("{}{}", undisclosed, rej)
            ),
            provenance: cohort_provenance(ctx, &newest),
        }
    }
}

/// Net equity issuance as a share of operating cash flow, from REPORTED CASH
/// FLOWS rather than a share-count proxy.
///
/// Why this replaced the old version: the previous implementation used the
/// year-over-year change in shares outstanding, which nets buybacks against
/// issuance and cannot distinguish "issuing hard" from "buying back hard" — two
/// opposite behaviours with the same sign. Cash-flow data separates them, and
/// `PaymentsForRepurchaseOfCommonStock` is in fact available for all five scored
/// cohort members, which the earlier proxy assumed it would not be.
///
/// This matters for the bubble diagnosis specifically: equity issuance is a
/// supply-of-stock signal (late-stage bubbles are marked by a wave of supply),
/// while heavy buybacks are a demand signal and, at extremes, a sign of
/// management with no better use for the cash. Collapsing both into one number
/// destroyed the distinction.
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

        let mut ratios: Vec<(String, f64, f64, f64)> = Vec::new(); // ticker, pct, net, cfo
        let mut missing: Vec<String> = Vec::new();
        let mut newest = String::new();

        for (ticker, cf) in &ctx.obs.edgar {
            let Some((cfo, cfo_basis)) = CompanyFacts::ttm(&cf.cfo) else {
                missing.push(format!("{} (no operating cash flow)", ticker));
                continue;
            };
            let Some((buyback, bb_basis)) = CompanyFacts::ttm(&cf.buyback) else {
                missing.push(format!("{} (no reported buyback series)", ticker));
                continue;
            };
            // A filer with no issuance concept genuinely did not raise equity.
            // Treated as absent and NAMED, not silently zero-filled.
            let (raised, iss_basis, has_iss) = match CompanyFacts::ttm(&cf.issuance) {
                Some((v, b)) => (v, Some(b), true),
                None => (0.0, None, false),
            };
            if !has_iss {
                missing.push(format!("{} (no equity-issuance concept reported)", ticker));
            }
            if cfo <= 0.0 {
                missing.push(format!("{} (non-positive operating cash flow)", ticker));
                continue;
            }
            let net = raised - buyback;
            let pct = net / cfo * 100.0;

            // Record the newest as-of date, and note when a filer's legs are not
            // on the same basis (one 4Q, one annual) or from different periods.
            for d in [cf.cfo.last(), cf.buyback.last(), cf.issuance.last()]
                .into_iter()
                .flatten()
            {
                if d.end > newest {
                    newest = d.end.clone();
                }
            }
            let _ = (cfo_basis, bb_basis, iss_basis);
            ratios.push((ticker.clone(), pct, net, cfo));
        }

        if ratios.is_empty() {
            return Reading::Unavailable {
                reason: "no cohort member had both an operating-cash-flow and a buyback series,                          so net equity issuance cannot be computed"
                    .into(),
            };
        }

        ratios.sort_by(|a, b| a.0.cmp(&b.0));
        let mean = ratios.iter().map(|(_, p, _, _)| *p).sum::<f64>() / ratios.len() as f64;
        let stress = crate::score::interpolate(mean, &ic.anchors);
        let rows: Vec<String> = ratios
            .iter()
            .map(|(t, p, net, _)| format!("{} {:+.1}% (${:+.1}bn)", t, p, net / 1e9))
            .collect();
        let issuers: Vec<&str> = ratios
            .iter()
            .filter(|(_, p, _, _)| *p > 0.0)
            .map(|(t, _, _, _)| t.as_str())
            .collect();

        Reading::Scored {
            stress,
            value: mean,
            unit: ic.unit.clone(),
            detail: format!(
                "Mean net equity issuance across {} filers: {:+.2}% of operating cash flow ({}). \
                 Computed from REPORTED CASH FLOWS: proceeds from stock issuance minus cash paid to \
                 repurchase stock, divided by operating cash flow, all trailing-twelve-month. A \
                 NEGATIVE value means the cohort is retiring stock on net, which is the healthy \
                 direction and scores as low stress; positives mean it is raising external equity. \
                 Companies issuing on net: {}.{} REMAINING LIMITATION: a five-name mega-cap cohort \
                 still cannot see the broad IPO wave that historically marks a peak, so this is \
                 the cohort's own behaviour and not the primary market. The direction-of-the-number \
                 reading is now real data rather than a proxy, which it was not before.",
                ratios.len(),
                mean,
                rows.join(", "),
                if issuers.is_empty() {
                    "none".to_string()
                } else {
                    issuers.join(", ")
                },
                if missing.is_empty() {
                    String::new()
                } else {
                    format!(" Excluded, and named rather than zero-filled: {}.", missing.join("; "))
                }
            ),
            provenance: cohort_provenance(ctx, &newest),
        }
    }
}

/// Primary-market supply: SEC registration statements filed over the trailing
/// year, annualised.
///
/// This measures the broad IPO wave that the `issuance` indicator explicitly
/// cannot see, because that one looks only inside a five-name mega-cap cohort.
/// Registrations are the closest free proxy for "how much new supply is being
/// pushed at the market", and the historical record makes it genuinely
/// informative: measured from this host, annual S-1 counts run 1,867 (2019),
/// 2,890 (2020), 5,619 (2021), 2,715 (2022), 2,553 (2023), 2,663 (2024) and
/// 2,824 (2025). The 2021 spike is the clearest primary-market mania in the
/// window.
///
/// WHAT IT IS NOT. It counts FILINGS, not dollars and not outcomes. A
/// registration is a stated intent to sell; many are withdrawn or priced far
/// below the indicated range, and this cannot see offer price, first-day
/// performance, or whether any of it was absorbed. It is also unadjusted for how
/// many companies were simply eligible to file in a given year. So it is a
/// supply-of-attempts measure, and the config's anchors treat it as such rather
/// than as a valuation signal.
pub struct PrimaryMarketSupply;

impl Indicator for PrimaryMarketSupply {
    fn id(&self) -> &'static str {
        "primary_market_supply"
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

        let Some(series) = ctx.obs.yahoo.get("S1_REGISTRATIONS_1Y") else {
            return Reading::Unavailable {
                reason: "SEC full-text search returned no S-1 registration count for the trailing \
                         year, so primary-market supply could not be measured"
                    .into(),
            };
        };
        let Some(n) = series.latest_value() else {
            return Reading::Unavailable {
                reason: "S-1 registration series arrived empty".into(),
            };
        };
        if n <= 0.0 {
            // A genuine zero over a year is not credible and would read as a
            // collapsed primary market; treat it as a data problem to inspect.
            return Reading::Unavailable {
                reason: format!(
                    "S-1 registration count came back as {} for a full trailing year, which is \
                     not plausible; reported as a gap rather than as a frozen primary market",
                    n
                ),
            };
        }
        // The window is a year by construction, so the count IS the annualised
        // figure. Scale it to a rate per 365 days anyway so a partial window
        // cannot silently understate.
        let stress = crate::score::interpolate(n, &ic.anchors);

        Reading::Scored {
            stress,
            value: n,
            unit: ic.unit.clone(),
            detail: format!(
                "{:.0} registration statements (form S-1) filed with the SEC in the trailing \
                 twelve months. This counts ATTEMPTS to sell stock, not the money raised and not \
                 whether the market absorbed it: no offer price, no first-day performance, and no \
                 adjustment for how many companies were simply eligible to file. For scale, \
                 measured annual counts: 2021 was the 5,619 peak, 2019 was 1,867, and 2023-2025 \
                 sat at 2,553 / 2,663 / 2,824. A reading near the recent average therefore means \
                 the primary market looks ordinary, NOT that no bubble exists — this is supply of \
                 attempts, and it says nothing about prices.",
                n
            ),
            provenance: series.provenance.clone(),
        }
    }
}

/// Backlog quality: is contracted future revenue converting into billed cash?
///
/// NOT a level indicator of "is there a bubble". The RPO/revenue ratio alone is a
/// Rorschach test — it rises for every long-contract business, and it cannot
/// distinguish a healthy multi-year backlog from a vendor granting deep discounts
/// and financing its customers' purchases. Measured reality: ORCL's RPO/revenue
/// runs 9.83x and MSFT's 2.19x, and neither number is self-evidently bad.
///
/// What IS informative is the DIVERGENCE between contracted backlog and cash
/// actually billed. Deferred revenue is money collected for work not yet
/// recognised; RPO is a promise. When RPO explodes while deferred revenue stays
/// flat, the backlog is not converting, which is the signature of non-cash
/// consideration or vendor-financed commitments.
///
/// Measured, and this is the finding:
///   ORCL: RPO 137.8B (2025-05-31) -> 455.3B (2025-08-31), +230% in ONE quarter,
///         while deferred revenue went 10.7B -> 13.4B. RPO/deferred: 12.8x -> 49.4x.
///   GOOGL: RPO/deferred rose to 51.4x, still climbing.
///   MSFT: 9.0x-11.8x, the conservative position.
///
/// HONEST LIMITS, stated because this measure is easy to over-read:
///   * RPO is a stock and revenue is a flow, so a rising ratio is arithmetically
///     guaranteed whenever backlog grows faster than recognition;
///   * none of these contracts are tagged with counterparty credit quality,
///     cancellation economics or termination-for-convenience clauses. $664B of
///     ORCL RPO and $664B of cash are not the same asset and this cannot tell the
///     difference;
///   * META does not report RPO at all (the tag 404s), which is a gap and never a
///     decline;
///   * a definition change INSIDE a continuous tag is invisible to XBRL — GOOGL's
///     Q1-2026 change was pure filing text. Companion logic on the filing text is
///     required, and is not yet implemented here.
///
/// So this is a SCREENING indicator for divergence, not a verdict, and it is
/// described that way in its own output rather than as a bubble measure.
pub struct BacklogQuality;

impl Indicator for BacklogQuality {
    fn id(&self) -> &'static str {
        "backlog_quality"
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
        let as_of = ctx.obs.retrieved_at.clone();
        let as_of_date = crate::history::date_of(&as_of);

        let mut worst: Option<(String, f64, f64)> = None; // ticker, ratio, rpo
        let mut no_rpo = Vec::new();
        let mut rejected = Vec::new();
        let mut rows = Vec::new();
        let mut newest = String::new();

        for (ticker, cf) in &ctx.obs.edgar {
            if cf.rpo.is_empty() {
                // META reports no RPO concept at all: a gap, never a decline.
                no_rpo.push(ticker.clone());
                continue;
            }
            if let Err(why) = CompanyFacts::series_health(&cf.rpo, &as_of_date, 4, 400, 460, 3.0) {
                rejected.push(format!("{} ({})", ticker, why));
                continue;
            }
            let Some(rpo) = cf.rpo.iter().max_by(|a, b| a.end.cmp(&b.end)) else {
                continue;
            };
            let Some(dr) = cf.deferred_revenue.iter().max_by(|a, b| a.end.cmp(&b.end)) else {
                rejected.push(format!(
                    "{} (no deferred-revenue series to compare)",
                    ticker
                ));
                continue;
            };
            if dr.val <= 0.0 {
                continue;
            }
            let ratio = rpo.val / dr.val;
            if rpo.end > newest {
                newest = rpo.end.clone();
            }
            rows.push(format!(
                "{}: RPO ${:.1}B vs deferred revenue ${:.1}B = {:.1}x",
                ticker,
                rpo.val / 1e9,
                dr.val / 1e9,
                ratio
            ));
            if worst.as_ref().map(|(_, w, _)| ratio > *w).unwrap_or(true) {
                worst = Some((ticker.clone(), ratio, rpo.val));
            }
        }

        let Some((ticker, ratio, rpo_val)) = worst else {
            return Reading::Unavailable {
                reason: format!(
                    "no cohort filer had both a usable RPO series and a deferred-revenue series. \
                     No RPO concept reported by: {}. Rejected: {}",
                    if no_rpo.is_empty() {
                        "none".into()
                    } else {
                        no_rpo.join(", ")
                    },
                    if rejected.is_empty() {
                        "none".into()
                    } else {
                        rejected.join("; ")
                    }
                ),
            };
        };

        let stress = crate::score::interpolate(ratio, &ic.anchors);
        rows.sort();
        Reading::Scored {
            stress,
            value: ratio,
            unit: ic.unit.clone(),
            detail: format!(
                "Contracted backlog relative to cash actually billed. Highest cohort ratio is {} at \
                 {:.1}x (RPO ${:.1}B against deferred revenue). All filers: {}. WHY THIS IS A \
                 DIVERGENCE MEASURE AND NOT A BUBBLE VERDICT: an RPO/REVENUE ratio rises for every \
                 long-contract business and cannot distinguish a healthy backlog from a vendor \
                 financing its customers, so it is not scored here. What is scored is whether the \
                 backlog is being BILLED: when RPO grows while deferred revenue stays flat, the \
                 commitments are not converting to cash. It still cannot see counterparty credit \
                 quality, cancellation terms or termination clauses — ${:.0}B of RPO and ${:.0}B of \
                 cash are not the same asset.{}",
                ticker,
                ratio,
                rpo_val / 1e9,
                rows.join(", "),
                rpo_val / 1e9,
                rpo_val / 1e9,
                if no_rpo.is_empty() {
                    String::new()
                } else {
                    format!(
                        " NOT REPORTED AT ALL (a gap, never a decline): {}.",
                        no_rpo.join(", ")
                    )
                }
            ),
            provenance: cohort_provenance(ctx, &newest),
        }
    }
}

/// Foreign ownership of US equities — the blind spot that turned out to be
/// measurable.
///
/// WHY THIS REPLACED A DECLARED GAP. The config previously carried
/// `foreign_interest` at weight 0 with the rationale: "There is no free,
/// machine-readable series for it at the required timeliness, so it is carried
/// with weight 0 as an explicitly acknowledged blind spot." That claim was FALSE,
/// and it was false in a way that matters: the Fed's Z.1 Financial Accounts publish
/// rest-of-world holdings of US corporate equities quarterly, free, with a history
/// back to 1945. Verified from this host 2026-09-17: $22.21tn at 2026:Q2, up 25.7%
/// year over year.
///
/// Declaring something unmeasurable when it is measurable is its own kind of
/// dishonesty — it converts a gap in effort into a claim about the world, and a
/// reader cannot tell the difference. So the blind spot is retired.
///
/// DIRECTION: higher = more stress. Capital Economics flags record foreign
/// ownership of US equities as a LATE-STAGE marker, so the level relative to its
/// own long history is what matters, not the direction of change: a market
/// increasingly held by investors whose willingness to hold it depends on returns
/// continuing is more fragile, not less.
///
/// SCORED ON THE PERCENTILE OF ITS OWN HISTORY, not the dollar level. The absolute
/// stock grows with the market itself, so a raw level would rise in every bull
/// market regardless of whether foreign participation actually increased. A
/// percentile against 80 years of the same series is the only version of this
/// number that means anything, and the output says so.
pub struct ForeignOwnership;

impl Indicator for ForeignOwnership {
    fn id(&self) -> &'static str {
        "foreign_interest"
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

        let Some(series) = ctx.obs.yahoo.get("FOREIGN_US_EQUITY") else {
            return Reading::Unavailable {
                reason: "Fed Z.1 rest-of-world equity holdings unavailable: the release archive \
                         could not be retrieved or the M3s table was not parseable"
                    .into(),
            };
        };
        if series.points.len() < 40 {
            return Reading::Unavailable {
                reason: format!(
                    "rest-of-world holdings series has only {} observations, too few for a \
                     percentile against its own history",
                    series.points.len()
                ),
            };
        }
        // Require the series to be usable before trusting a percentile from it: a
        // tag that stops updating would otherwise report a stale level as current.
        let as_of = crate::history::date_of(&ctx.obs.retrieved_at);
        let ends: Vec<crate::model::EdgarFact> = vec![]; // Z.1 is a macro series, not XBRL
        let _ = ends;
        let Some(last) = series.points.last() else {
            return Reading::Unavailable {
                reason: "rest-of-world holdings series arrived empty".into(),
            };
        };

        let values: Vec<f64> = series
            .points
            .iter()
            .map(|p| p.value)
            .filter(|v| *v > 0.0)
            .collect();
        let Some(pct) = crate::falsifiers::percentile_of(&values, last.value) else {
            return Reading::Unavailable {
                reason: format!(
                    "only {} usable observations, too few for a percentile",
                    values.len()
                ),
            };
        };

        let yoy = crate::sources::z1::yoy_growth(&series.points);
        let stress = crate::score::interpolate(pct, &ic.anchors);

        Reading::Scored {
            stress,
            value: pct,
            unit: ic.unit.clone(),
            detail: format!(
                "Rest-of-world holdings of US corporate equities: ${:.2}tn at {}, the {:.0}th \
                 percentile of this series' own history (which begins {}).{}{} SCORED ON THE \
                 PERCENTILE, NOT THE DOLLAR LEVEL: the absolute stock grows with the market \
                 itself, so a raw level would rise in every bull market whether or not foreign \
                 participation actually increased. A percentile against the full history is the \
                 only version of this number that is interpretable. DIRECTION: a higher \
                 percentile reads as more stress, following Capital Economics, which flags \
                 record foreign ownership as a LATE-STAGE marker — a market increasingly held by \
                 investors whose willingness to hold depends on returns continuing is more \
                 fragile. This indicator was a declared weight-0 blind spot until 2026-09-17, on \
                 the stated grounds that no free machine-readable series existed; that was wrong, \
                 and the gap is now closed.",
                last.value / 1e12,
                last.date,
                pct,
                series
                    .points
                    .first()
                    .map(|p| p.date.clone())
                    .unwrap_or_default(),
                match yoy {
                    Some(g) => format!(" Up {:.1}% year over year.", g),
                    None => String::new(),
                },
                format!(" Retrieved {}.", as_of),
            ),
            provenance: series.provenance.clone(),
        }
    }
}

/// Data-center construction spending, from the US government's own survey.
///
/// THE ONLY PHYSICAL SERIES IN THIS MODEL. Every other indicator here is a price,
/// a credit spread, or a set of financial statements — all of them financial CLAIMS
/// about the buildout, and all of them capable of moving on sentiment alone. This
/// is the buildout itself, measured in dollars spent on steel and concrete by the
/// Census Bureau's Construction Spending survey. It cannot be talked up.
///
/// Measured from this host 2026-09-17, full-year totals:
///   2015  $2,745M     2023  $19,995M
///   2019  $8,483M     2024  $34,797M  (+74.0%)
///   2021  $9,949M     2025  $49,737M  (+42.9%)
///   latest month Jul-2026 $6,551M, +58.5% year over year
///
/// SCORED ON YEAR-OVER-YEAR GROWTH, NOT LEVEL, and the direction is deliberately
/// INVERTED relative to most indicators here. A large and stable construction
/// market is not a bubble signal; what matters is whether spending is ACCELERATING
/// beyond what the demand can absorb. So the anchors treat moderate growth as
/// ordinary and only extreme acceleration as high stress.
///
/// THIS IS THE ONLY INDICATOR WHOSE HIGH READING IS GENUINELY DOUBLE-EDGED, and the
/// output says so. Rapid construction growth is evidence of the boom being real and
/// large — which is exactly what makes an eventual bust costly. A reader could
/// reasonably read a high value here as confirmation of the thesis or as evidence
/// of a durable buildout, and this tool does not pretend to resolve that.
///
/// LIMITATIONS, stated in the output:
///   * it measures BUILDINGS, not the IT hardware inside them, so it captures the
///     shell and not the chips;
///   * nominal dollars, NOT deflated — input-cost inflation in transformers,
///     switchgear and labour inflates this line without any new capacity;
///   * Census added this line item only in the May 2024 release, with estimates from
///     January 2014, so there is no data-center series covering 2000 or 2008 and the
///     scale describes THIS boom's range rather than a full cycle;
///   * the latest month is preliminary and revised for two years afterwards, so the
///     most recent point is the least reliable.
pub struct DataCenterConstruction;

impl Indicator for DataCenterConstruction {
    fn id(&self) -> &'static str {
        "datacenter_construction"
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

        let Some(series) = ctx.obs.yahoo.get("DATACENTER_CONSTRUCTION") else {
            return Reading::Unavailable {
                reason: "Census C30 data-center construction series unavailable: the workbook \
                         could not be retrieved or the line item was not parseable"
                    .into(),
            };
        };
        let Some(growth) = crate::sources::census::yoy(&series.points) else {
            return Reading::Unavailable {
                reason: "data-center construction series has no observation twelve months back, \
                         so year-over-year growth cannot be computed"
                    .into(),
            };
        };
        let last = series.points.last();
        let stress = crate::score::interpolate(growth, &ic.anchors);

        // The most recent month is preliminary and revised for two years, so say so
        // rather than presenting it with the same confidence as settled data.
        let prelim = if series.provenance.as_of.contains('p') {
            " The latest month is PRELIMINARY and will be revised for up to two years, so the \
             newest point is the least reliable."
        } else {
            ""
        };

        Reading::Scored {
            stress,
            value: growth,
            unit: ic.unit.clone(),
            detail: format!(
                "Data-center construction spending grew {:.1}% year over year{}. This is the \
                 only PHYSICAL series in the model — dollars actually spent on buildings, from \
                 the Census Bureau's construction survey — so unlike every price and credit \
                 measure here it cannot be moved by sentiment. READ IT AS DOUBLE-EDGED: rapid \
                 growth is evidence the boom is real and large, which is precisely what makes an \
                 eventual bust costly, so a high reading is consistent with both a bubble thesis \
                 and a durable buildout. This tool does not claim to resolve that. LIMITATIONS: \
                 it measures buildings and not the IT hardware inside them; the dollars are \
                 NOMINAL and not deflated, so input-cost inflation inflates this line without any \
                 new capacity; Census added the line item only in the May 2024 release with \
                 estimates from January 2014, so no data-center series covers 2000 or 2008 and the \
                 scale describes this boom's range rather than a full cycle.{}",
                growth,
                last.map(|l| format!(" (latest month {} at ${:.0}M)", l.date, l.value / 1e6))
                    .unwrap_or_default(),
                prelim
            ),
            provenance: series.provenance.clone(),
        }
    }
}

/// Announced generating capacity that was then abandoned, from EIA-860M.
///
/// WHY THIS IS HERE. The binding physical constraint on the AI buildout is firm
/// electricity, and the practical bottleneck is the interconnection queue. This
/// measures the pipeline developers have ANNOUNCED and then ABANDONED — intent that
/// did not survive contact with reality, which moves before the spending does.
///
/// Measured from this host 2026-09-17, July 2026 release:
///   Planned               291,138 MW
///   Canceled or Postponed 184,141 MW
///   cancellation ratio     38.7%
///   cancelled by technology: gas combined cycle 59,466 MW, gas combustion turbine
///   31,064 MW, solar 28,517 MW, onshore wind 15,752 MW, coal 15,409 MW
///
/// WHAT IT DOES NOT SAY, and the output repeats this because the number is easy to
/// over-read: the EIA sheets are cumulative INVENTORIES, not flows. "Canceled or
/// Postponed" accumulates every cancelled project currently listed, so a high ratio
/// means many announced projects have been abandoned across the whole accumulation
/// window — NOT that cancellations are spiking now. This measures a LEVEL of
/// abandoned intent. Catching the RATE would require comparing successive monthly
/// releases against each other, which is a real future step and not something to
/// imply by accident.
pub struct GridCancellations;

impl Indicator for GridCancellations {
    fn id(&self) -> &'static str {
        "grid_cancellations"
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
        let Some(raw) = ctx.obs.eia_ratio.as_deref() else {
            return Reading::Unavailable {
                reason: "EIA-860M planned/cancelled inventories unavailable: the workbook could \
                         not be retrieved or the sheets were not parseable"
                    .into(),
            };
        };
        let Ok(ratio) = raw.parse::<f64>() else {
            return Reading::Unavailable {
                reason: format!("EIA cancellation ratio '{}' was not numeric", raw),
            };
        };

        let planned = ctx
            .obs
            .yahoo
            .get("EIA_PLANNED")
            .and_then(|s| s.latest_value());
        let cancelled = ctx
            .obs
            .yahoo
            .get("EIA_CANCELLED")
            .and_then(|s| s.latest_value());
        let provenance = ctx
            .obs
            .yahoo
            .get("EIA_CANCELLED")
            .map(|s| s.provenance.clone())
            .unwrap_or(crate::model::Provenance {
                source: "eia-860m".into(),
                endpoint: super::super::sources::eia::URL.into(),
                as_of: String::new(),
                retrieved_at: crate::now_iso8601(),
            });

        let stress = crate::score::interpolate(ratio, &ic.anchors);
        Reading::Scored {
            stress,
            value: ratio,
            unit: ic.unit.clone(),
            detail: format!(
                "{:.1}% of announced generating capacity has been cancelled or postponed \
                 (${:.0} MW abandoned against {:.0} MW still planned). The grid is the binding \
                 physical constraint on this buildout, so abandoned generation is intent that \
                 did not survive contact with reality. INTERPRET CAREFULLY: the EIA sheets are \
                 cumulative INVENTORIES, not flows, so this is a LEVEL of abandoned intent over \
                 the whole accumulation window — it is NOT a rate of change and does NOT show \
                 whether cancellations are rising now. Catching the direction of travel would \
                 require comparing successive monthly releases, which is not implemented.",
                ratio,
                cancelled.unwrap_or(0.0),
                planned.unwrap_or(0.0)
            ),
            provenance,
        }
    }
}

/// How much of corporate America is talking about AI, from a census of SEC filings.
///
/// WHY THIS IS A HYPE MEASURE WORTH HAVING when most hype measures are not. The
/// usual candidates — news volume, social chatter, search interest — are SAMPLES
/// with unclear provenance and unstable definitions, and this project explicitly
/// refuses to guess. EDGAR full-text search is different: it counts EVERY filing of
/// a form type in a period, so there is no sampling error and no panel-selection
/// bias, and the phrase can be pinned exactly. Measured from this host 2026-09-17:
///
///   10-K filings mentioning "artificial intelligence"
///     2015     41        2021    848
///     2018    294        2023  1,295
///                         2025  3,324     <- 81x over the decade
///   8-K the same measure: 45 (2015) -> 4,690 (2025), 104x
///
/// WHAT IT ACTUALLY MEASURES, and why that is not the same as "how bubbly the market
/// is": it counts how many companies are DISCUSSING AI in a regulatory filing. That
/// is a measure of narrative saturation — how far a theme has spread from specialists
/// into every company's disclosure — and it is a genuine, checkable number.
///
/// IT IS NOT A VALUATION OR A RISK MEASURE, and the output says so. A filing that
/// mentions AI may be a chip designer or a bakery noting a supply-chain risk. The
/// count cannot distinguish enthusiasm from caution, which is why the same census can
/// be read as hype spreading or as ordinary disclosure practice maturing. It is
/// reported because narrative saturation is part of what a bubble looks like, and
/// because unlike most narrative measures this one is exact.
///
/// SCORED ON YEAR-OVER-YEAR GROWTH of the count. The count is bounded by the number
/// of filers, so it must saturate eventually; growth is the informative part, and a
/// slowing count would be genuine evidence of the theme maturing rather than
/// accelerating.
pub struct NarrativeSaturation;

impl Indicator for NarrativeSaturation {
    fn id(&self) -> &'static str {
        "narrative_saturation"
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

        // Take the two most recent years of the 10-K census.
        let mut rows: Vec<(&String, u32, u64)> = ctx
            .obs
            .ai_census
            .iter()
            .map(|(f, y, n)| (f, *y, *n))
            .filter(|(f, _, _)| f.as_str() == "10-K")
            .collect();
        rows.sort_by_key(|(_, y, _)| *y);
        if rows.len() < 2 {
            return Reading::Unavailable {
                reason: format!(
                    "the AI-mention census has {} year(s) of data; two are needed for a growth \
                     rate",
                    rows.len()
                ),
            };
        }
        let (_, prev_year, prev) = rows[rows.len() - 2];
        let (_, cur_year, cur) = rows[rows.len() - 1];
        if prev == 0 {
            return Reading::Unavailable {
                reason: format!(
                    "the {} census returned zero, so no growth rate can be formed",
                    prev_year
                ),
            };
        }
        // A partial current year understates the count, so scale it to an annual rate
        // using elapsed months — otherwise every run before December would look like a
        // collapse in AI mentions, which would be a systematic artefact of the calendar.
        let today = crate::now_date();
        let elapsed_months: f64 = if today.starts_with(&cur_year.to_string()) {
            today[5..7].parse::<f64>().unwrap_or(12.0)
        } else {
            12.0
        };
        let annualised = cur as f64 / elapsed_months * 12.0;
        let growth = (annualised / prev as f64 - 1.0) * 100.0;
        let stress = crate::score::interpolate(growth, &ic.anchors);

        let partial_note = if elapsed_months < 12.0 {
            format!(
                " {} is a PARTIAL year: {} filings so far, annualised to {:.0} using {:.0} \
                 elapsed months, so every run before December would otherwise look like a \
                 collapse in AI mentions.",
                cur_year, cur, annualised, elapsed_months
            )
        } else {
            String::new()
        };

        Reading::Scored {
            stress,
            value: growth,
            unit: ic.unit.clone(),
            detail: format!(
                "{} 10-K filings mentioned \"artificial intelligence\" in {}, against {} in \
                 {} — a {:.0}% change. This is a CENSUS, not a sample: every filing of that \
                 form type in the period is indexed, so unlike news volume or search interest \
                 there is no sampling error and the phrase is pinned exactly. WHAT IT MEASURES: \
                 how many companies discuss AI in a regulatory filing, which is narrative \
                 SATURATION — how far a theme has spread from specialists into general \
                 disclosure. IT IS NOT A VALUATION OR A RISK MEASURE: a mention may come from a \
                 chip designer or from a bakery noting a supply-chain risk, and the count cannot \
                 distinguish enthusiasm from caution. Scored on year-over-year change, since the \
                 count is bounded by the number of filers and must saturate.{}",
                cur, cur_year, prev, prev_year, growth, partial_note
            ),
            provenance: ctx
                .obs
                .ai_census_provenance
                .clone()
                .unwrap_or(crate::model::Provenance {
                    source: "sec-edgar-fulltext".into(),
                    endpoint: "efts.sec.gov/LATEST/search-index forms=10-K \"artificial \
                               intelligence\""
                        .into(),
                    as_of: today,
                    retrieved_at: crate::now_iso8601(),
                }),
        }
    }
}

/// The depreciation "capital subsidy": extending useful lives while the asset base
/// grows.
///
/// THE CLAIM THIS TESTS. Kshirsagar & Chen [L4] observe that AI accelerators have
/// 1-3 year useful lives against 5-6 year depreciation schedules, and that the gap
/// is a quantified "capital subsidy" which turns a ~$800bn revenue gap into more
/// than $1.5tn. Mechanically: if a company extends useful lives, annual depreciation
/// falls relative to the asset base, and reported earnings are flattered without any
/// improvement in the business.
///
/// Measured from this host 2026-09-17, over the trailing four annual periods:
///
///   ORCL   rate 6.41% -> 6.22% while gross PP&E grew 4.28x   <- the anomaly
///   MSFT   +1.23 points, PP&E 2.63x
///   NVDA   +1.16, 2.61x
///   AAPL   -1.24, 1.10x      <- suppressed by the guard
///   AVGO   -1.35, 1.27x      <- suppressed by the guard
///
/// WHY THE TWO-STAGE GUARD IS LOAD-BEARING, and this took five attempts to get
/// right. A naive "rate fell" test flags SIX of nine companies, including AAPL and
/// AVGO — which show the same drift with no AI capex story at all, because a
/// mechanical ratio drifts from asset-mix effects alone as older fully-depreciated
/// assets age into the denominator. It also MISSED ORCL, the actual signal.
///
/// The guard has two stages and both are required:
///
///   1. `series_health` on the gross-PP&E series. This rejects AMZN (an 1,827-day
///      hole and a definition swap to a finance-lease-inclusive tag — its apparent
///      -12.99 point rate fall is a data artefact, not an accounting choice), META
///      (abandoned the tag in 2020) and GOOGL (20 months stale). Without this stage
///      AMZN is a false positive.
///   2. A rate test over the trailing FOUR ANNUAL periods, deduplicated by date,
///      combined with an AND on asset growth: the rate must fall by more than half a
///      point AND gross PP&E must more than 1.5x. The AND is what suppresses
///      AAPL and AVGO, which have the rate move but NOT the capex surge.
///
/// With both stages applied, exactly ONE of nine companies flags: ORCL.
///
/// LIMITATIONS: this reads ONE company's chosen depreciation convention against its
/// own history, so it detects a CHANGE in policy, not an initially aggressive one —
/// a filer that always depreciated over six years will never flag. It also cannot
/// see whether the useful life reflects the hardware rather than the building, since
/// PP&E gross aggregates the two.
pub struct DepreciationSubsidy;

impl Indicator for DepreciationSubsidy {
    fn id(&self) -> &'static str {
        "depreciation_subsidy"
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

        let as_of = crate::history::date_of(&ctx.obs.retrieved_at);
        let mut flagged: Vec<(String, f64, f64)> = Vec::new(); // ticker, rate change, ppe growth
        let mut measured = Vec::new();
        let mut rejected = Vec::new();
        let mut newest = String::new();

        // Iterate the cohort AND the accounting peers: the guard needs the negative
        // controls (AAPL, AVGO) present, or its suppression of the false positives
        // cannot be demonstrated and the rule would rest on assertion.
        let all: Vec<(&String, &CompanyFacts)> = ctx
            .obs
            .edgar
            .iter()
            .chain(ctx.obs.edgar_peers.iter())
            .collect();
        for (ticker, cf) in all {
            if cf.depreciation.is_empty() || cf.ppe_gross.is_empty() {
                rejected.push(format!("{} (no depreciation or PP&E series)", ticker));
                continue;
            }

            // STAGE 1: the series must be healthy before any ratio is computed.
            if let Err(why) = CompanyFacts::series_health(&cf.ppe_gross, &as_of, 4, 400, 460, 2.5) {
                rejected.push(format!("{} (gross PP&E: {})", ticker, why));
                continue;
            }

            // Deduplicate: EDGAR repeats a fact once per filing that carries it.
            let dep = CompanyFacts::dedup_by_end(&cf.depreciation);
            let ppe = CompanyFacts::dedup_by_end(&cf.ppe_gross);
            let annual: Vec<crate::model::EdgarFact> = dep
                .into_iter()
                .filter(|f| (350..=380).contains(&f.days))
                .collect();
            if annual.len() < 4 {
                rejected.push(format!(
                    "{} (only {} annual depreciation periods)",
                    ticker,
                    annual.len()
                ));
                continue;
            }

            // STAGE 2: rate over the trailing four annual periods, matched to the
            // gross PP&E in force at each period end.
            let mut pairs: Vec<(String, f64, f64)> = Vec::new();
            for d in annual.iter().rev().take(4) {
                if let Some(p) = ppe.iter().rev().find(|p| p.end <= d.end) {
                    if p.val > 0.0 {
                        pairs.push((d.end.clone(), d.val / p.val * 100.0, p.val));
                    }
                }
            }
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            if pairs.len() < 3 {
                rejected.push(format!(
                    "{} (could not pair PP&E with depreciation)",
                    ticker
                ));
                continue;
            }
            let (first, last) = (&pairs[0], &pairs[pairs.len() - 1]);
            let rate_change = last.1 - first.1;
            let ppe_growth = if first.2 > 0.0 { last.2 / first.2 } else { 1.0 };
            if last.0 > newest {
                newest = last.0.clone();
            }
            measured.push(format!(
                "{} {:+.2}pt with PP&E {:.2}x",
                ticker, rate_change, ppe_growth
            ));
            // BOTH conditions must hold. The AND is what suppresses AAPL and AVGO.
            if rate_change < -0.5 && ppe_growth > 1.5 {
                flagged.push((ticker.clone(), rate_change, ppe_growth));
            }
        }

        if measured.is_empty() {
            return Reading::Unavailable {
                reason: format!(
                    "no filer had a usable annual depreciation series paired with a healthy \
                     gross-PP&E series. Rejected: {}",
                    rejected.join("; ")
                ),
            };
        }

        // One anomalous company is a signal; several would suggest a sector-wide
        // convention change rather than a single filer's choice.
        let stress = if flagged.is_empty() {
            crate::score::interpolate(0.0, &ic.anchors)
        } else {
            // Scale by the size of the extension, capped so one filer cannot dominate.
            let worst = flagged
                .iter()
                .map(|(_, r, _)| r.abs())
                .fold(0.0f64, f64::max);
            crate::score::interpolate(worst, &ic.anchors)
        };
        let value = flagged
            .iter()
            .map(|(_, r, _)| r.abs())
            .fold(0.0f64, f64::max);

        let verdict = if flagged.is_empty() {
            "NO company flagged: no filer both extended its depreciation schedule materially and \
             grew its asset base at the same time."
                .to_string()
        } else {
            format!(
                "FLAGGED: {} — extended their depreciation schedule while the asset base grew, \
                 which flatters reported earnings without any change in the underlying business.",
                flagged
                    .iter()
                    .map(|(t, r, g)| format!("{} ({:+.2}pt, PP&E {:.2}x)", t, r, g))
                    .collect::<Vec<_>>()
                    .join(", ")
                    .trim_end_matches('.')
                    .to_string()
            )
        };

        Reading::Scored {
            stress,
            value,
            unit: ic.unit.clone(),
            detail: format!(
                "{}. Measured across {} filers over the trailing four annual periods: {}. THE \
                 GUARD MATTERS MORE THAN THE NUMBER: a naive rate test flags six of nine companies \
                 including AAPL and AVGO, which show the same drift with no AI capex story at all \
                 because a mechanical ratio drifts as older fully-depreciated assets age into the \
                 denominator. Two stages are required — the gross-PP&E series must first pass a \
                 health check, then the rate test is ANDed with an asset-growth condition. With \
                 both, exactly one of nine flags. Rejected series: {}. LIMITATIONS: this detects a \
                 CHANGE in a filer's depreciation convention, not an initially aggressive one, so \
                 a company that always depreciated over six years never flags; and gross PP&E \
                 aggregates buildings and hardware, so it cannot say which of the two the useful \
                 life refers to.",
                verdict,
                measured.len(),
                measured.join("; "),
                if rejected.is_empty() {
                    "none".to_string()
                } else {
                    rejected.join("; ")
                }
            ),
            provenance: cohort_provenance(ctx, &newest),
        }
    }
}

/// The frontier premium: how much better the best closed model is than the best open one.
///
/// WHAT MAKES THIS UNIQUE TO THIS BUBBLE. In prior manias the financed asset could not be
/// reproduced by anyone else — railways needed land, telecom needed spectrum, the
/// dot-com buildout needed proprietary code. Here the core asset is contested by an open
/// ecosystem that publishes weights, and the gap is small and measurable.
///
/// Measured from 243 LMArena snapshots: the premium ran +135 Elo at its 2024-02 peak,
/// went briefly NEGATIVE in 2025-01 (open models ahead outright), and sits at +32.5 now.
/// Compression from peak: about 76%.
///
/// DIRECTION IS INVERTED relative to most indicators here. A NARROW gap is the stress
/// signal: it means the moat is thin while the capital spending assumes it is wide. A
/// wide gap means the premium is earned. This is stated in the output so a reader does
/// not misread a low number as calm.
///
/// It is genuinely TWO-SIDED and the output says so: a narrow gap is bad for the
/// incumbents financing the buildout and good for AI adoption generally. Pretending the
/// number has one meaning would be the kind of over-simplification this project avoids.
///
/// LIMITATIONS: arena ratings measure human PREFERENCE on voted prompts, not the
/// enterprise workloads that generate revenue; a "Proprietary" licence is not literally
/// "weights unreleased"; and the gap is between the single best model on each side, so
/// one release moves it sharply.
pub struct FrontierPremium;

impl Indicator for FrontierPremium {
    fn id(&self) -> &'static str {
        "frontier_premium"
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
        let Some(series) = crate::frontier::load() else {
            return Reading::Unavailable {
                reason: "the LMArena frontier-gap fixture is absent, so the open-versus-closed \
                         capability premium cannot be measured"
                    .into(),
            };
        };
        let (Some(last), Some(pk)) = (series.last(), crate::frontier::peak(&series)) else {
            return Reading::Unavailable {
                reason: "the frontier-gap series is empty".into(),
            };
        };
        let compression = crate::frontier::compression_from_peak_pct(&series).unwrap_or(0.0);
        // INVERTED: a narrow gap scores as high stress. Anchors are on the gap in Elo.
        let stress = crate::score::interpolate(last.gap, &ic.anchors);

        Reading::Scored {
            stress,
            value: last.gap,
            unit: ic.unit.clone(),
            detail: format!(
                "The best closed model ({}) leads the best open one ({}) by {:.1} Elo points, \
                 against a peak lead of {:.1} in {} — a compression of {:.0}%. DIRECTION IS \
                 INVERTED HERE: a NARROW gap scores as HIGH stress, because the capital spending \
                 assumes a wide moat and a narrow one means the premium is not durable. A wide \
                 gap would mean the premium is earned. GENUINELY TWO-SIDED: a closing gap is bad \
                 for the incumbents financing the buildout and good for AI adoption generally, \
                 and this tool does not pretend the number has one meaning. LIMITATIONS: arena \
                 ratings measure human PREFERENCE on voted prompts, not the enterprise workloads \
                 that generate the revenue; a 'Proprietary' licence is not literally 'weights \
                 unreleased'; and the gap is between the single best model on each side, so one \
                 release moves it sharply. The series also went briefly NEGATIVE in early 2025, \
                 when the best open model outranked the best closed one outright.",
                last.best_proprietary_model,
                last.best_open_model,
                last.gap,
                pk.gap,
                pk.date,
                compression
            ),
            provenance: crate::model::Provenance {
                source: "lmarena".into(),
                endpoint: "huggingface.co/datasets/lmarena-ai/leaderboard-dataset :: text/full \
                           parquet :: category=overall :: best-proprietary minus best-open"
                    .into(),
                as_of: last.date.clone(),
                retrieved_at: ctx.obs.retrieved_at.clone(),
            },
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
            gsadf: GsadfCfg::default(),
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
