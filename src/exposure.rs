//! Per-company exposure: who is most exposed, and who might be tested first.
//!
//! CONTEXT, NOT A SCORE. Nothing here enters the composite. The composite answers
//! "how bubble-like is the configuration"; this answers "who is carrying the
//! risk", which is a different question at a different level of analysis. Folding
//! a company-level ranking into a market-level score would confuse the two.
//!
//! TWO QUESTIONS, TWO DIFFERENT ANSWERS, and conflating them is the easy mistake:
//!
//!   * **Who is most exposed?** A LEVEL question. Answered by debt plus leases
//!     plus commitments relative to what the business generates.
//!   * **Who is tested first?** A TIMING question. Answered by the maturity
//!     schedule and the unconditional commitments — what falls due earliest.
//!
//! The measured answer surprised the intuition, which is why both are reported.
//! Near-term maturities are small for every filer in this cohort ($2.8bn-$9.2bn
//! against operating cash flows of $45bn-$165bn), so a classic maturity wall is
//! NOT the mechanism in this cycle. The binding constraint is unconditional
//! take-or-pay commitments, which must be paid whether or not the demand appears.
//!
//! EVERY ABSENT VALUE IS NAMED, NEVER ZEROED. MSFT reports no purchase-obligation
//! concept at all, so showing it as 0.00x would state that its commitments are
//! nothing. AMZN discloses one but its latest figure is years stale. Those are two
//! different facts and are reported differently.

use crate::model::{CompanyFacts, Observations};

/// One company's exposure, with every component optional so an undisclosed figure
/// is visibly missing rather than silently zero.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompanyExposure {
    pub ticker: String,
    /// Total long-term debt divided by TTM operating cash flow.
    pub debt_to_cfo: Option<f64>,
    /// Debt falling due within one year, over cash flow. The timing question.
    pub near_term_to_cfo: Option<f64>,
    /// Operating leases over cash flow.
    pub lease_to_cfo: Option<f64>,
    /// Unconditional purchase obligations over cash flow. `None` when the filer
    /// does not disclose them or the disclosure is stale — never zero.
    pub commitments_to_cfo: Option<f64>,
    /// RPO over TTM revenue. `None` when the filer does not report RPO.
    pub rpo_to_revenue: Option<f64>,
    /// Plain-language reason a value is missing, when it is.
    pub notes: Vec<String>,
}

impl CompanyExposure {
    /// A deliberately simple rank key: how much of a company's cash generation is
    /// already spoken for, weighting what must be paid soonest most heavily.
    ///
    /// It is NOT a probability and NOT a score out of 100 — it exists only to
    /// order the table, and the ordering is all it claims. `None` for any missing
    /// component is treated as contributing nothing to the key, and the company is
    /// flagged in `notes` so the reader knows the key rests on partial data.
    pub fn rank_key(&self) -> f64 {
        let near = self.near_term_to_cfo.unwrap_or(0.0);
        let debt = self.debt_to_cfo.unwrap_or(0.0);
        let lease = self.lease_to_cfo.unwrap_or(0.0);
        let commit = self.commitments_to_cfo.unwrap_or(0.0);
        near * 3.0 + debt + lease + commit
    }

    /// True when at least one component is undisclosed, so the rank key is
    /// computed on partial information.
    pub fn is_partial(&self) -> bool {
        self.commitments_to_cfo.is_none()
    }
}

/// True whole days between two dates, treating an unusable pair as zero so a
/// malformed date degrades rather than panics.
fn age_days(end: &str, as_of: &str) -> i64 {
    crate::sources::edgar::days_between(end, as_of)
}

/// Build the exposure table for every cohort filer with usable data.
///
/// PURE — takes the fetched observations and the as-of date, does no IO.
pub fn build(obs: &Observations, as_of: &str) -> Vec<CompanyExposure> {
    let mut out = Vec::new();

    for (ticker, cf) in &obs.edgar {
        let mut notes = Vec::new();

        let cfo = CompanyFacts::ttm(&cf.cfo)
            .map(|(v, _)| v)
            .filter(|v| *v > 0.0);
        let Some(cfo) = cfo else {
            notes.push("no usable operating cash flow, so no ratio can be formed".to_string());
            out.push(CompanyExposure {
                ticker: ticker.clone(),
                debt_to_cfo: None,
                near_term_to_cfo: None,
                lease_to_cfo: None,
                commitments_to_cfo: None,
                rpo_to_revenue: None,
                notes,
            });
            continue;
        };

        // Debt: same staleness/zero guard the leverage indicator uses, so the two
        // can never disagree about what a filer's debt is.
        let debt = CompanyFacts::point_in_time(&cf.debt, as_of, 200)
            .map(|f| f.val)
            .map_err(|e| notes.push(format!("long-term debt unavailable ({})", e)))
            .ok();

        let lease = CompanyFacts::point_in_time(&cf.lease, as_of, 200)
            .map(|f| f.val)
            .map_err(|e| notes.push(format!("operating leases unavailable ({})", e)))
            .ok();

        // Commitments: distinguish "never disclosed" from "disclosed but stale".
        let commitments = if cf.purchase_obligation.is_empty() {
            notes.push(
                "purchase obligations are not disclosed at all by this filer, so they are shown \
                 as absent rather than as zero"
                    .to_string(),
            );
            None
        } else {
            match CompanyFacts::point_in_time(&cf.purchase_obligation, as_of, 200) {
                Ok(f) => Some(f.val),
                Err(e) => {
                    // Quantify the staleness so the reader can judge it.
                    let latest = cf
                        .purchase_obligation
                        .iter()
                        .max_by(|a, b| a.end.cmp(&b.end));
                    let why = latest
                        .map(|l| {
                            format!(
                                "latest figure is from {} ({} days old)",
                                l.end,
                                age_days(&l.end, as_of)
                            )
                        })
                        .unwrap_or_else(|| e.clone());
                    notes.push(format!(
                        "purchase obligations ARE disclosed by this filer but are excluded as \
                         not current: {}. Not treated as zero.",
                        why
                    ));
                    None
                }
            }
        };

        let near = CompanyFacts::point_in_time(&cf.debt_due[0], as_of, 200)
            .map(|f| f.val)
            .map_err(|e| notes.push(format!("near-term maturities unavailable ({})", e)))
            .ok();

        let revenue = CompanyFacts::ttm(&cf.revenue)
            .map(|(v, _)| v)
            .filter(|v| *v > 0.0);
        let rpo = if cf.rpo.is_empty() {
            notes.push("does not report a remaining-performance-obligation concept".to_string());
            None
        } else {
            CompanyFacts::point_in_time(&cf.rpo, as_of, 200)
                .ok()
                .map(|f| f.val)
        };
        let rpo_to_revenue = match (rpo, revenue) {
            (Some(r), Some(rev)) => Some(r / rev),
            _ => None,
        };

        out.push(CompanyExposure {
            ticker: ticker.clone(),
            debt_to_cfo: debt.map(|d| d / cfo),
            near_term_to_cfo: near.map(|d| d / cfo),
            lease_to_cfo: lease.map(|l| l / cfo),
            commitments_to_cfo: commitments.map(|c| c / cfo),
            rpo_to_revenue,
            notes,
        });
    }

    // Most exposed first.
    out.sort_by(|a, b| {
        b.rank_key()
            .partial_cmp(&a.rank_key())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.ticker.cmp(&b.ticker))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EdgarFact;
    use std::collections::BTreeMap;

    fn f(end: &str, val: f64, days: i64) -> EdgarFact {
        EdgarFact {
            tag: "t".into(),
            start: "2025-01-01".into(),
            end: end.into(),
            val,
            form: "10-Q".into(),
            filed: end.into(),
            days,
        }
    }

    fn co_with(
        cfo: f64,
        debt: Option<f64>,
        lease: Option<f64>,
        purch: Vec<EdgarFact>,
        rpo: Option<f64>,
    ) -> CompanyFacts {
        let mut cf = CompanyFacts {
            cik: "0".into(),
            name: "t".into(),
            ..Default::default()
        };
        // four quarters of cash flow summing to `cfo`
        cf.cfo = (0..4)
            .map(|i| f(&format!("2026-0{}-30", 3 + i), cfo / 4.0, 90))
            .collect();
        if let Some(d) = debt {
            cf.debt = vec![f("2026-06-30", d, 0)];
        }
        if let Some(l) = lease {
            cf.lease = vec![f("2026-06-30", l, 0)];
        }
        cf.purchase_obligation = purch;
        if let Some(r) = rpo {
            cf.rpo = vec![f("2026-06-30", r, 0)];
        }
        cf.revenue = (0..4)
            .map(|i| f(&format!("2026-0{}-30", 3 + i), 50.0, 90))
            .collect();
        cf
    }

    fn obs_with(pairs: Vec<(&str, CompanyFacts)>) -> Observations {
        let mut o = Observations::default();
        let mut m: BTreeMap<String, CompanyFacts> = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v);
        }
        o.edgar = m;
        o
    }

    #[test]
    fn an_undisclosed_commitment_is_absent_not_zero() {
        // The MSFT case. Rendering 0.00x would state its commitments are nothing.
        let o = obs_with(vec![(
            "MSFT",
            co_with(100.0, Some(30.0), Some(20.0), vec![], None),
        )]);
        let e = &build(&o, "2026-09-17")[0];
        assert!(
            e.commitments_to_cfo.is_none(),
            "an undisclosed obligation must be absent, not 0.0"
        );
        assert!(e.is_partial());
        assert!(
            e.notes.iter().any(|n| n.contains("not disclosed at all")),
            "must say why: {:?}",
            e.notes
        );
    }

    #[test]
    fn a_stale_disclosure_is_reported_differently_from_no_disclosure() {
        // The AMZN case: the concept exists, the latest figure is old. Saying
        // "not disclosed" would be false.
        let stale = vec![f("2024-06-30", 32.4e9, 0)];
        let o = obs_with(vec![(
            "AMZN",
            co_with(100.0, Some(30.0), None, stale, None),
        )]);
        let e = &build(&o, "2026-09-17")[0];
        assert!(e.commitments_to_cfo.is_none());
        let joined = e.notes.join(" ");
        assert!(
            joined.contains("ARE disclosed") && joined.contains("not current"),
            "must distinguish stale from absent: {}",
            joined
        );
        assert!(
            !joined.contains("not disclosed at all"),
            "a stale disclosure must not be called undisclosed: {}",
            joined
        );
    }

    #[test]
    fn a_current_commitment_is_included() {
        let rows = vec![f("2026-08-31", 34.1e9, 0)];
        let o = obs_with(vec![("ORCL", co_with(100.0, Some(30.0), None, rows, None))]);
        let e = &build(&o, "2026-09-17")[0];
        assert!(e.commitments_to_cfo.is_some());
        assert!(!e.is_partial());
    }

    #[test]
    fn a_stale_debt_series_is_rejected_not_used() {
        // The ORCL debt bug, at the exposure layer this time.
        let mut cf = co_with(100.0, None, None, vec![], None);
        cf.debt = vec![f("2022-05-31", 0.0, 0)];
        let o = obs_with(vec![("ORCL", cf)]);
        let e = &build(&o, "2026-09-17")[0];
        assert!(
            e.debt_to_cfo.is_none(),
            "a dead series must not become a ratio"
        );
        assert!(e.notes.iter().any(|n| n.contains("debt unavailable")));
    }

    #[test]
    fn missing_rpo_is_named() {
        let o = obs_with(vec![(
            "META",
            co_with(100.0, Some(30.0), None, vec![], None),
        )]);
        let e = &build(&o, "2026-09-17")[0];
        assert!(e.rpo_to_revenue.is_none());
        assert!(e
            .notes
            .iter()
            .any(|n| n.contains("remaining-performance-obligation")));
    }

    #[test]
    fn ranking_is_most_exposed_first() {
        // ORCL-like: heavy debt. MSFT-like: light and undisclosed.
        let heavy = co_with(
            45.0,
            Some(113.0),
            Some(34.0),
            vec![f("2026-06-30", 34.0, 0)],
            None,
        );
        let light = co_with(165.0, Some(31.0), Some(22.0), vec![], None);
        let o = obs_with(vec![("ORCL", heavy), ("MSFT", light)]);
        let rows = build(&o, "2026-09-17");
        assert_eq!(
            rows[0].ticker, "ORCL",
            "the more exposed company must rank first"
        );
        assert_eq!(rows[1].ticker, "MSFT");
        assert!(rows[0].rank_key() > rows[1].rank_key());
    }

    #[test]
    fn a_company_with_no_cash_flow_is_kept_with_a_reason_not_dropped() {
        let mut cf = co_with(100.0, Some(30.0), None, vec![], None);
        cf.cfo.clear();
        let o = obs_with(vec![("X", cf)]);
        let rows = build(&o, "2026-09-17");
        assert_eq!(rows.len(), 1, "the company must still appear");
        assert!(rows[0]
            .notes
            .iter()
            .any(|n| n.contains("no usable operating cash flow")));
    }

    #[test]
    fn near_term_maturities_are_captured_for_the_timing_question() {
        let mut cf = co_with(100.0, Some(50.0), None, vec![], None);
        cf.debt_due[0] = vec![f("2026-06-30", 7.21e9, 0)];
        let o = obs_with(vec![("ORCL", cf)]);
        let e = &build(&o, "2026-09-17")[0];
        let nt = e.near_term_to_cfo.expect("near-term maturities present");
        assert!((nt - 7.21e9 / 100.0).abs() < 1e-9, "got {}", nt);
    }

    #[test]
    fn the_table_is_deterministic() {
        let o = obs_with(vec![
            ("ORCL", co_with(45.0, Some(113.0), None, vec![], None)),
            ("MSFT", co_with(165.0, Some(31.0), None, vec![], None)),
        ]);
        assert_eq!(build(&o, "2026-09-17"), build(&o, "2026-09-17"));
    }
}
