//! Falsification: measurements that could show the bubble thesis is WRONG.
//!
//! WHY THIS MODULE EXISTS. Every other indicator in this tool was built by asking
//! "is there evidence of a bubble?" and finding it. That method has a structural
//! bias: the composite can only drift upward as inputs are added, because a
//! researcher looking for confirming evidence tends to find it. During one session
//! of development the composite moved 30.6 -> 35.3, and a large part of that came
//! from ADDING indicators that happened to score high.
//!
//! A model that cannot say "I was wrong" is not an instrument, it is a position.
//! This module asks the opposite question for each candidate: **what measurement
//! would show the thesis is mistaken, and what does that measurement currently
//! say?**
//!
//! THIS IS NOT ANTI-BUBBLE STRESS AND IS DELIBERATELY NOT IN THE COMPOSITE.
//! Folding "evidence against" into a "bubble stress" number would be a category
//! error: a reading of 30 would then mean either "mild bubble" or "strong
//! counter-evidence", which are opposite statements. Falsifiers are reported
//! beside the composite, like the explosiveness test, and never averaged into it.
//!
//! THREE QUESTIONS, each with a testable answer from data the tool already
//! fetches:
//!
//!   1. Is the revenue actually arriving? A backlog that converts to billed cash
//!      is a real business; one that does not is a promise.
//!   2. Are credit markets pricing any stress? Sustained tightness is
//!      counter-evidence to IMMINENT deterioration — though emphatically not to
//!      the existence of a bubble, since complacency is consistent with it.
//!   3. Is the buildout self-correcting? Capex discipline returning would show the
//!      boom decelerating on its own.
//!
//! Each carries a verdict of `CounterEvidence`, `ConsistentWithBubble`, or
//! `Uninformative`. `Uninformative` is a real answer and is used whenever the data
//! cannot support a direction, rather than forcing one.

use crate::model::{CompanyFacts, Observations};

/// Which way a falsification test currently reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The measurement came out the way the bubble thesis predicts.
    ConsistentWithBubble,
    /// The measurement came out AGAINST the thesis. This is the whole point of the
    /// module: these are the readings that should make a reader doubt it.
    CounterEvidence,
    /// The data cannot support a direction. Stated rather than forced.
    Uninformative,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::ConsistentWithBubble => "consistent with the bubble thesis",
            Verdict::CounterEvidence => "COUNTER-EVIDENCE to the bubble thesis",
            Verdict::Uninformative => "uninformative",
        }
    }
}

/// One falsification test and what it currently says.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Falsifier {
    pub id: String,
    /// The claim this test exists to disprove.
    pub question: String,
    /// What was actually measured.
    pub reading: String,
    pub verdict: Verdict,
    /// The supporting numbers, and what the test cannot see.
    pub detail: String,
}

/// Percentile of `value` within `series`, as 0-100.
///
/// Pure, and used to say where a current reading sits against its own history —
/// which is the only way a bare level becomes interpretable.
pub fn percentile_of(series: &[f64], value: f64) -> Option<f64> {
    if series.len() < 30 {
        return None;
    }
    let below = series.iter().filter(|v| **v < value).count();
    Some(below as f64 / series.len() as f64 * 100.0)
}

/// Test 1: is the contracted backlog converting into billed cash?
///
/// Deferred revenue is money collected; RPO is a promise. A *falling* ratio means
/// the backlog is being recognised as revenue and paid for, which is genuine
/// counter-evidence. A *rising* ratio means commitments are outpacing billing,
/// which is consistent with the thesis.
///
/// Uses the cohort's largest filer by RPO for a single clear reading, and reports
/// how many filers even disclosed both concepts, because META tags no RPO at all.
fn f1_revenue_arriving(obs: &Observations) -> Falsifier {
    let id = "revenue_arriving".to_string();
    let question =
        "Is the contracted backlog turning into revenue actually billed and paid?".to_string();

    let mut best: Option<(String, f64, f64, usize)> = None; // ticker, ratio_now, ratio_yr_ago, points
    let mut disclosed = 0usize;

    for (ticker, cf) in &obs.edgar {
        if cf.rpo.is_empty() || cf.deferred_revenue.is_empty() {
            continue;
        }
        disclosed += 1;
        // Pair each RPO observation with the deferred revenue at the SAME date, so
        // the ratio compares like with like rather than mixing periods.
        let dr: std::collections::BTreeMap<&str, f64> = cf
            .deferred_revenue
            .iter()
            .map(|f| (f.end.as_str(), f.val))
            .collect();
        let mut ratios: Vec<(&str, f64)> = cf
            .rpo
            .iter()
            .filter_map(|f| {
                dr.get(f.end.as_str())
                    .filter(|d| **d > 0.0)
                    .map(|d| (f.end.as_str(), f.val / *d))
            })
            .collect();
        ratios.sort_by(|a, b| a.0.cmp(b.0));
        if ratios.len() < 5 {
            continue;
        }
        let now = ratios[ratios.len() - 1].1;
        let prior = ratios[ratios.len() - 5].1;
        // Track the largest backlog, which is the reading that matters most.
        let magnitude = cf.rpo.iter().map(|f| f.val).fold(0.0, f64::max);
        if best.as_ref().map(|(_, _, _, _)| true).unwrap_or(true) {
            let better = best.as_ref().map(|(_, _, _, _)| false).unwrap_or(true);
            if better || magnitude > 0.0 {
                if best.is_none() {
                    best = Some((ticker.clone(), now, prior, ratios.len()));
                } else {
                    // replace only if this filer's backlog is larger
                    let cur_max = best.as_ref().unwrap();
                    let cur_max_val = obs
                        .edgar
                        .get(&cur_max.0)
                        .map(|c| c.rpo.iter().map(|f| f.val).fold(0.0, f64::max))
                        .unwrap_or(0.0);
                    if magnitude > cur_max_val {
                        best = Some((ticker.clone(), now, prior, ratios.len()));
                    }
                }
            }
        }
    }

    let Some((ticker, now, prior, pts)) = best else {
        return Falsifier {
            id,
            question,
            reading: "no cohort filer disclosed both a backlog and a deferred-revenue series"
                .into(),
            verdict: Verdict::Uninformative,
            detail: "Without both concepts the backlog's conversion to billed cash cannot be \
                     assessed at all. Reported as uninformative rather than as either verdict."
                .into(),
        };
    };

    let change_pct = if prior > 0.0 {
        (now / prior - 1.0) * 100.0
    } else {
        0.0
    };
    // A materially rising ratio means commitments are outpacing billing.
    let verdict = if change_pct < -10.0 {
        Verdict::CounterEvidence
    } else if change_pct > 10.0 {
        Verdict::ConsistentWithBubble
    } else {
        Verdict::Uninformative
    };

    Falsifier {
        id,
        question,
        reading: format!(
            "{}: backlog-to-billed-cash ratio {:.1}x, {:+.0}% over four quarters ({} quarters of paired data; {} cohort filers disclose both concepts)",
            ticker, now, change_pct, pts, disclosed
        ),
        verdict,
        detail: "A FALLING ratio would mean the backlog is being recognised and paid for, which \
                 is real counter-evidence: it would show the contracts are genuine revenue rather \
                 than a promise. A RISING ratio means commitments are outpacing cash collection. \
                 This cannot see counterparty credit quality, cancellation terms or \
                 termination-for-convenience clauses, so a low ratio would not prove the \
                 counterparties are sound."
            .into(),
    }
}

/// Test 2: are credit markets pricing any stress?
///
/// Sustained TIGHT spreads are counter-evidence to *imminent* deterioration: the
/// market most exposed to a default wave is not pricing one. They are NOT
/// counter-evidence to the bubble thesis itself — complacency is entirely
/// consistent with a bubble — and the detail says so, because a reassuring reading
/// here is the easiest thing in this report to over-read.
fn f2_credit_stress(obs: &Observations) -> Falsifier {
    let id = "credit_pricing_stress".to_string();
    let question = "Are credit markets pricing the risk of near-term deterioration?".to_string();

    // Prefer the long-history series: a percentile against 2.5 years is not
    // meaningful, and this is exactly why BAA10Y was added.
    // Use the SERIES ID, never the endpoint. The FRED endpoint carries an api_key
    // query parameter, and a report should not echo a URL shape that contains a
    // credential slot, even with the value masked.
    let (series_id, series) = match obs
        .fred
        .get("BAA10Y")
        .map(|s| ("BAA10Y", s))
        .or_else(|| obs.fred.get("BAMLH0A0HYM2").map(|s| ("BAMLH0A0HYM2", s)))
    {
        Some(v) => v,
        None => {
            return Falsifier {
                id,
                question,
                reading: "no credit-spread series was retrieved".into(),
                verdict: Verdict::Uninformative,
                detail: "FRED is an optional source and becomes a reported gap when unavailable."
                    .into(),
            }
        }
    };
    let values: Vec<f64> = series.points.iter().map(|p| p.value).collect();
    let Some(current) = series.points.last() else {
        return Falsifier {
            id,
            question,
            reading: "credit-spread series was empty".into(),
            verdict: Verdict::Uninformative,
            detail: String::new(),
        };
    };
    let Some(pct) = percentile_of(&values, current.value) else {
        return Falsifier {
            id,
            question,
            reading: format!(
                "only {} observations of series {}, too few for a percentile against history",
                values.len(),
                series_id
            ),
            verdict: Verdict::Uninformative,
            detail: "A percentile computed on a handful of years would not be interpretable."
                .into(),
        };
    };

    // Below the 25th percentile of its own history is a material tightening.
    let verdict = if pct <= 25.0 {
        Verdict::CounterEvidence
    } else if pct >= 60.0 {
        Verdict::ConsistentWithBubble
    } else {
        Verdict::Uninformative
    };

    let span_years = series
        .points
        .first()
        .and_then(|p| p.date.get(0..4))
        .unwrap_or("?");

    Falsifier {
        id,
        question,
        reading: format!(
            "credit spread {} at {:.2}%, the {:.0}th percentile of its own history since {}. Spreads are {}",
            series_id,
            current.value,
            pct,
            span_years,
            if pct <= 25.0 {
                "near the TIGHTEST levels on record"
            } else if pct >= 60.0 {
                "wide relative to their own history"
            } else {
                "mid-range"
            }
        ),
        verdict,
        detail: "READ THIS CAREFULLY. Tight spreads are counter-evidence to IMMINENT \
                 deterioration — the market most exposed to a default wave is not pricing one — \
                 but they are NOT counter-evidence to a bubble EXISTING. Complacency, and lenders \
                 competing for volume on thin compensation, are both entirely consistent with a \
                 bubble. So a reassuring reading here means 'not yet', never 'not a bubble'. The \
                 public spread also cannot see private credit, which the private-credit indicator \
                 covers separately."
            .into(),
    }
}

/// Test 3: is the buildout self-correcting?
///
/// Capex divided by revenue, trended. A FALLING ratio means capex is growing slower
/// than the business, i.e. discipline returning — genuine counter-evidence. A
/// RISING ratio means the buildout is still outrunning monetisation, which is
/// consistent with the thesis.
fn f3_self_correcting(obs: &Observations) -> Falsifier {
    let id = "buildout_self_correcting".to_string();
    let question =
        "Is capital spending growing slower than the revenue it must eventually be paid from?"
            .to_string();

    // Cohort aggregate capex / revenue, trailing four quarters, now vs a year ago.
    let agg = |offset_q: usize| -> Option<(f64, f64)> {
        let mut capex = 0.0;
        let mut rev = 0.0;
        let mut n = 0;
        for cf in obs.edgar.values() {
            let c = CompanyFacts::ttm_at_offset(&cf.capex, offset_q);
            let r = CompanyFacts::ttm_at_offset(&cf.revenue, offset_q);
            if let (Some((c, _)), Some((r, _))) = (c, r) {
                if r > 0.0 {
                    capex += c;
                    rev += r;
                    n += 1;
                }
            }
        }
        if n == 0 || rev <= 0.0 {
            None
        } else {
            Some((capex / rev, n as f64))
        }
    };

    let (Some((now, filers)), Some((prior, _))) = (agg(0), agg(4)) else {
        return Falsifier {
            id,
            question,
            reading: "not enough quarterly history to compare capex-to-revenue against a year ago"
                .into(),
            verdict: Verdict::Uninformative,
            detail: "Requires four quarters this year and four a year earlier for the same filers."
                .into(),
        };
    };

    let change = now - prior;
    let change_pct = if prior > 0.0 {
        change / prior * 100.0
    } else {
        0.0
    };
    let verdict = if change_pct < -5.0 {
        Verdict::CounterEvidence
    } else if change_pct > 5.0 {
        Verdict::ConsistentWithBubble
    } else {
        Verdict::Uninformative
    };

    Falsifier {
        id,
        question,
        reading: format!(
            "cohort capex/revenue {:.3} now vs {:.3} a year ago ({:+.1}%), across {} filers",
            now, prior, change_pct, filers
        ),
        verdict,
        detail: "A FALLING ratio would show capital spending growing more slowly than the \
                 business it is meant to serve, which is what a self-correcting boom looks like \
                 and would be genuine counter-evidence. A RISING ratio means the buildout is \
                 still outrunning monetisation. Caveat: hyperscaler revenue is mostly non-AI \
                 cloud and advertising, so this overstates the true AI capex-to-AI-revenue gap."
            .into(),
    }
}

/// Run every falsification test.
pub fn build(obs: &Observations) -> Vec<Falsifier> {
    vec![
        f1_revenue_arriving(obs),
        f2_credit_stress(obs),
        f3_self_correcting(obs),
        f4_export_controls_binding(obs),
    ]
}

/// How many tests came out against the thesis. The headline number for this
/// module, and the one a reader should look for before accepting a high composite.
pub fn counter_evidence_count(f: &[Falsifier]) -> usize {
    f.iter()
        .filter(|x| x.verdict == Verdict::CounterEvidence)
        .count()
}

/// F4 — Are export controls actually binding on non-US AI revenue?
///
/// ## Why a QUIET period is the counter-evidence
///
/// The natural framing is that US export controls cap where AI hardware and
/// services can be sold, which would constrain revenue. If that were the binding
/// constraint, the rule-making pace should be high and tightening. Measured over
/// the twelve months to 2026-09-20: **two** BIS rules matched "advanced
/// computing", and the most recent (2026-07-14) **eases** access for the United
/// Arab Emirates.
///
/// So a LOW count reads as counter-evidence to the constraint narrative, and a
/// HIGH count with recent tightening would read as consistent with it. The
/// direction is the opposite of every other falsifier here, which is exactly why
/// it is worth having.
///
/// ## Thresholds are a judgement, and are stated
///
/// There is no natural law about rule counts. The bands below are chosen from the
/// observed pace (2/year) with generous margins, and they are deliberately coarse:
/// this test exists to catch a REGIME change in policy activity, not to grade
/// month-to-month noise.
pub fn f4_export_controls_binding(obs: &Observations) -> Falsifier {
    const ID: &str = "export_controls";
    const QUESTION: &str = "Are US export controls tightening enough to bind non-US AI revenue?";

    let Some(count) = obs.bis_year_count else {
        return Falsifier {
            id: ID.into(),
            question: QUESTION.into(),
            reading: "not measured".into(),
            verdict: Verdict::Uninformative,
            detail: "The Federal Register API did not return a rule count, so the pace of export-control activity is unknown. This is a reported gap, not a finding of calm."
                .into(),
        };
    };
    let latest = obs
        .bis_latest_date
        .as_deref()
        .unwrap_or("no dated rule in the window");

    // Coarse regimes. 2/year was the measured pace; a shift to 6+ would be a real
    // change in policy tempo.
    let (verdict, reading) = if count >= 6 {
        (
            Verdict::ConsistentWithBubble,
            format!("{count} BIS rules in twelve months ({latest}) — an active rule-making period"),
        )
    } else if count <= 2 {
        (
            Verdict::CounterEvidence,
            format!("{count} BIS rules in twelve months ({latest}) — a QUIET policy period"),
        )
    } else {
        (
            Verdict::Uninformative,
            format!("{count} BIS rules in twelve months ({latest}) — neither active nor quiet"),
        )
    };

    Falsifier {
        id: ID.into(),
        question: QUESTION.into(),
        reading,
        verdict,
        detail: format!(
            "Counted via the Federal Register API (keyless), BIS documents matching              \"advanced computing\" published in the trailing twelve months. A LOW count is              COUNTER-EVIDENCE here, which is the opposite of every other test in this panel: if              export controls were the binding constraint on AI revenue, the rule-making pace              would be high and tightening. It is not. WHAT THIS CANNOT SEE: (1) rule COUNT is not              rule SEVERITY — one sweeping rule can bind far more than six narrow ones, and this              test does not read the text; (2) enforcement and licensing practice can tighten              without any new rule, which is invisible here; (3) controls also constrain              COMPETITORS, which can help a US vendor's revenue rather than hurt it, so the              direction is not even unambiguous. Thresholds are coarse by design — this catches a              regime change, not month-to-month noise."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EdgarFact, Point, Provenance, Series};
    use std::collections::BTreeMap;

    fn fact(start: &str, end: &str, val: f64, days: i64) -> EdgarFact {
        EdgarFact {
            tag: "t".into(),
            start: start.into(),
            end: end.into(),
            val,
            form: "10-Q".into(),
            filed: end.into(),
            days,
        }
    }

    fn quarters(base: f64, mult: f64, n: usize) -> Vec<EdgarFact> {
        (0..n)
            .map(|i| {
                let y = 2023 + (i / 4);
                let m = (i % 4) * 3 + 3;
                fact(
                    &format!("{}-{:02}-01", y, m - 2),
                    &format!("{}-{:02}-30", y, m),
                    base * mult.powi(i as i32),
                    90,
                )
            })
            .collect()
    }

    fn series(vals: Vec<f64>, start_year: i32) -> Series {
        Series {
            provenance: Provenance {
                source: "t".into(),
                endpoint: "test://".into(),
                as_of: "2026-09-17".into(),
                retrieved_at: "2026-09-17T00:00:00Z".into(),
            },
            points: vals
                .into_iter()
                .enumerate()
                .map(|(i, v)| Point {
                    date: format!("{:04}-01-01", start_year + i as i32 / 12),
                    value: v,
                })
                .collect(),
        }
    }

    #[test]
    fn percentile_is_computed_against_the_whole_series() {
        let s: Vec<f64> = (0..100).map(|i| i as f64).collect();
        assert!((percentile_of(&s, 50.0).unwrap() - 50.0).abs() < 1.0);
        assert!((percentile_of(&s, 99.0).unwrap() - 99.0).abs() < 1.0);
        assert_eq!(percentile_of(&s, 0.0).unwrap(), 0.0);
    }

    #[test]
    fn percentile_refuses_a_series_too_short_to_interpret() {
        let s: Vec<f64> = (0..10).map(|i| i as f64).collect();
        assert!(
            percentile_of(&s, 5.0).is_none(),
            "a percentile on 10 points would not be meaningful"
        );
    }

    #[test]
    fn a_falling_backlog_ratio_is_counter_evidence() {
        // Backlog shrinking relative to billed cash: the good direction.
        let mut cf = CompanyFacts::default();
        cf.rpo = quarters(1000.0, 0.95, 12);
        cf.deferred_revenue = quarters(100.0, 1.10, 12);
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("X".to_string(), cf);
        o.edgar = m;
        let f = f1_revenue_arriving(&o);
        assert_eq!(
            f.verdict,
            Verdict::CounterEvidence,
            "got {:?}: {}",
            f.verdict,
            f.reading
        );
    }

    #[test]
    fn a_rising_backlog_ratio_is_consistent_with_the_thesis() {
        let mut cf = CompanyFacts::default();
        cf.rpo = quarters(1000.0, 1.15, 12);
        cf.deferred_revenue = quarters(100.0, 1.00, 12);
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("X".to_string(), cf);
        o.edgar = m;
        let f = f1_revenue_arriving(&o);
        assert_eq!(f.verdict, Verdict::ConsistentWithBubble);
    }

    #[test]
    fn tight_credit_is_counter_evidence_and_says_why_it_is_limited() {
        // Long history mostly wide, current value at the very bottom.
        let mut vals: Vec<f64> = (0..400).map(|i| 2.0 + (i as f64 % 40.0) / 20.0).collect();
        vals.push(1.43);
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("BAA10Y".to_string(), series(vals, 1990));
        o.fred = m;
        let f = f2_credit_stress(&o);
        assert_eq!(f.verdict, Verdict::CounterEvidence);
        // The crucial nuance must be stated, not just the verdict.
        assert!(
            f.detail
                .contains("NOT counter-evidence to a bubble EXISTING"),
            "must not let a reassuring reading be over-read: {}",
            f.detail
        );
    }

    #[test]
    fn wide_credit_is_consistent_with_the_thesis() {
        let mut vals: Vec<f64> = (0..400).map(|i| 1.0 + (i as f64 % 20.0) / 40.0).collect();
        vals.push(6.16);
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("BAA10Y".to_string(), series(vals, 1990));
        o.fred = m;
        assert_eq!(f2_credit_stress(&o).verdict, Verdict::ConsistentWithBubble);
    }

    #[test]
    fn a_short_credit_history_is_uninformative_not_a_verdict() {
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("BAA10Y".to_string(), series(vec![2.0, 2.1, 2.2], 2024));
        o.fred = m;
        assert_eq!(f2_credit_stress(&o).verdict, Verdict::Uninformative);
    }

    #[test]
    fn a_falling_capex_ratio_is_counter_evidence() {
        let mut cf = CompanyFacts::default();
        cf.capex = quarters(100.0, 0.95, 12);
        cf.revenue = quarters(100.0, 1.05, 12);
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("X".to_string(), cf);
        o.edgar = m;
        assert_eq!(f3_self_correcting(&o).verdict, Verdict::CounterEvidence);
    }

    #[test]
    fn a_rising_capex_ratio_is_consistent_with_the_thesis() {
        let mut cf = CompanyFacts::default();
        cf.capex = quarters(100.0, 1.10, 12);
        cf.revenue = quarters(100.0, 1.01, 12);
        let mut o = Observations::default();
        let mut m = BTreeMap::new();
        m.insert("X".to_string(), cf);
        o.edgar = m;
        assert_eq!(
            f3_self_correcting(&o).verdict,
            Verdict::ConsistentWithBubble
        );
    }

    #[test]
    fn no_data_yields_uninformative_verdicts_not_optimistic_ones() {
        // The failure mode that matters: with no data, the module must not default
        // to "everything is fine". Uninformative is its own answer.
        let o = Observations::default();
        let f = build(&o);
        assert_eq!(
            f.len(),
            4,
            "F4 (export controls) was added to widen the falsification side"
        );
        for x in &f {
            assert_eq!(
                x.verdict,
                Verdict::Uninformative,
                "{} must not claim a direction without data",
                x.id
            );
        }
        assert_eq!(counter_evidence_count(&f), 0);
    }

    #[test]
    fn every_falsifier_has_a_question_and_a_reading() {
        let o = Observations::default();
        for f in build(&o) {
            assert!(!f.question.is_empty(), "{} has no question", f.id);
            assert!(!f.reading.is_empty(), "{} has no reading", f.id);
            assert!(!f.detail.is_empty(), "{} has no detail", f.id);
        }
    }

    // ---- F4 export controls ------------------------------------------------

    #[test]
    fn a_quiet_export_control_period_is_counter_evidence() {
        // The whole point of this falsifier, and it INVERTS the usual reading: a
        // low rule count argues AGAINST the constraint narrative rather than for
        // calm.
        let mut obs = Observations::default();
        obs.bis_year_count = Some(2);
        obs.bis_latest_date = Some("2026-07-14".into());
        let f = f4_export_controls_binding(&obs);
        assert_eq!(
            f.verdict,
            Verdict::CounterEvidence,
            "2 rules in a year must read as counter-evidence, got {:?}",
            f.verdict
        );
        assert!(f.reading.contains("QUIET"), "got {}", f.reading);
    }

    #[test]
    fn an_active_rule_making_period_is_consistent_with_the_thesis() {
        let mut obs = Observations::default();
        obs.bis_year_count = Some(9);
        obs.bis_latest_date = Some("2026-09-01".into());
        let f = f4_export_controls_binding(&obs);
        assert_eq!(f.verdict, Verdict::ConsistentWithBubble);
        assert!(f.reading.contains("active"), "got {}", f.reading);
    }

    #[test]
    fn a_middling_count_forces_no_direction() {
        let mut obs = Observations::default();
        obs.bis_year_count = Some(4);
        let f = f4_export_controls_binding(&obs);
        assert_eq!(
            f.verdict,
            Verdict::Uninformative,
            "4 rules is neither active nor quiet; the test must not invent a direction"
        );
    }

    #[test]
    fn an_unmeasured_count_is_uninformative_not_calm() {
        // Absence of the data must never be reported as a finding of low activity.
        let obs = Observations::default();
        let f = f4_export_controls_binding(&obs);
        assert_eq!(f.verdict, Verdict::Uninformative);
        assert!(
            f.detail.contains("not a finding of calm"),
            "must distinguish a gap from a reading: {}",
            f.detail
        );
    }

    #[test]
    fn the_falsifier_states_what_it_cannot_see() {
        // Rule count is not rule severity; enforcement can tighten without a rule.
        let mut obs = Observations::default();
        obs.bis_year_count = Some(2);
        let f = f4_export_controls_binding(&obs);
        for needle in ["SEVERITY", "enforcement", "COMPETITORS"] {
            assert!(
                f.detail.contains(needle),
                "the detail must disclose the {:?} limitation: {}",
                needle,
                f.detail
            );
        }
    }

    #[test]
    fn the_panel_now_carries_four_tests() {
        // F4 was added to widen the falsification side, which the project's own
        // plan called thin.
        let obs = Observations::default();
        let all = build(&obs);
        assert_eq!(all.len(), 4, "expected four falsifiers, got {}", all.len());
        assert!(all.iter().any(|f| f.id == "export_controls"));
    }
}
