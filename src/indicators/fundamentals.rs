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
