//! Plain-English summary.
//!
//! Written to be readable by someone with no finance background. Rules that this
//! module must keep to:
//!
//!   * short sentences, common words, no jargon (there is a test that rejects a
//!     list of finance terms outright);
//!   * every number comes from the report itself, so the wording cannot drift
//!     away from the data;
//!   * it NEVER claims to predict timing — it says the opposite, in plain words;
//!   * anything unmeasurable is stated as unmeasurable, not glossed over.
//!
//! It is generated, not hand-written, for the same reason the rest of the tool
//! is: if the numbers move, the paragraph describing them must move with them.

use crate::model::{IndicatorReading, Reading, Report};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LaymanSummary {
    pub what_this_is: String,
    pub the_score: String,
    pub what_is_stretched: String,
    pub what_is_calm: String,
    pub what_we_cannot_measure: String,
    pub about_timing: String,
    /// Whether things are getting better or worse. This is the section that
    /// answers the question the credit indicators were always meant to answer.
    pub direction_of_travel: String,
    /// The permanent holes in the model, which do not depend on today's data.
    /// These are stated prominently rather than listed as data quality, because
    /// the most consequential one is a real blind spot rather than a failed
    /// fetch — the report can be at 100% coverage and still not see it.
    pub known_blind_spots: String,
    pub bottom_line: String,
}

/// Friendly wording for each indicator, so the summary never shows a raw id or a
/// technical label.
fn plain_name(id: &str) -> &'static str {
    match id {
        "valuation_stretch" => "how expensive stocks are compared with their own long-run trend",
        "concentration" => "how much the market depends on a few giant companies",
        "breadth" => "how many companies are rising, not just the biggest ones",
        "issuance" => "how many new shares companies are selling",
        "volatility" => "how jumpy the market is",
        "credit_hy" => "what weaker companies pay to borrow money",
        "credit_ig" => "what strong companies pay to borrow money",
        "capex_vs_cashflow" => "how much AI companies spend building data centres compared with the cash their business brings in",
        "funding_gap" => "how much AI companies spend compared with what they sell",
        "leverage" => "how much debt these companies are carrying",
        "foreign_interest" => "how much foreign money owns US shares",
        "circularity" => "money moving in circles between the companies involved",
        "primary_market_supply" => "how many companies are filing to sell shares for the first time",
        "backlog_quality" => "whether the big orders companies have booked are turning into money actually collected",
        "private_credit_growth" => "how fast lending outside the public bond market is growing",
        "datacenter_construction" => "how fast data-centre building is growing, as measured by the government",
        "grid_cancellations" => "how much announced power-plant capacity has been abandoned",
        "narrative_saturation" => "how many companies are talking about AI in their official filings",
        "depreciation_subsidy" => "whether companies are stretching how slowly they write down their equipment to flatter their profits",
        "frontier_premium" => "how far ahead the best paid-for AI model is compared with the best free one",
        _ => "one of the measurements",
    }
}

/// Describe a 0-100 stress number in words a non-specialist can act on.
fn stress_words(stress: f64) -> &'static str {
    match stress {
        s if s < 20.0 => "low",
        s if s < 35.0 => "fairly low",
        s if s < 50.0 => "middling",
        s if s < 65.0 => "high",
        s if s < 80.0 => "very high",
        _ => "at an extreme",
    }
}

fn scored(r: &IndicatorReading) -> Option<f64> {
    if r.weight <= 0.0 {
        return None; // declared blind spot, never scored
    }
    match &r.reading {
        Reading::Scored { stress, .. } => Some(*stress),
        Reading::Unavailable { .. } => None,
    }
}

/// Join phrases into a readable list: "a", "a and b", "a, b and c".
fn join_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

pub fn summarize(r: &Report) -> LaymanSummary {
    let what_this_is = "This report tries to answer one question: how much does the AI boom look \
like a bubble right now? It scores a set of measurements from 0 to 100. A higher score means more \
of the warning signs are showing."
        .to_string();

    // Which band the score falls in, in plain words.
    let phase_plain = match r.phase.as_str() {
        "early" => "early",
        "mid" => "middle",
        "late" => "late",
        "critical" => "extreme",
        _ => "unclear",
    };

    // Count the indicators that actually contributed, not the weight total —
    // "weight 100" is not "100 measurements", and printing the weight here was
    // simply wrong.
    let scored_count = r
        .indicators
        .iter()
        .filter(|i| i.weight > 0.0 && i.reading.is_available())
        .count();
    let declared_gaps = r.indicators.iter().filter(|i| i.weight <= 0.0).count();

    let the_score = format!(
        "The score today is {:.0} out of 100, which puts it in the {} range. For scale: a score \
above 55 would look more like the late stage of a bubble, and only true extremes have pushed \
past 75. It is built from {} different measurements{}. That is not a huge number, and it is \
why this report tells you what it cannot check as well as what it can.",
        r.composite,
        phase_plain,
        scored_count,
        if declared_gaps == 1 {
            ", plus 1 thing it cannot measure at all".to_string()
        } else if declared_gaps > 1 {
            format!(", plus {} things it cannot measure at all", declared_gaps)
        } else {
            String::new()
        }
    );

    // Anything at or above 50 is genuinely stretched; sort by how much it
    // actually moves the final number.
    let mut stretched: Vec<(&IndicatorReading, f64)> = r
        .indicators
        .iter()
        .filter_map(|i| scored(i).map(|s| (i, s)))
        .filter(|(_, s)| *s >= 50.0)
        .collect();
    stretched.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let what_is_stretched = if stretched.is_empty() {
        "Nothing on the list is currently flashing red. No single measurement scored in the \
high range. That does not prove there is no bubble. It means the things we can measure are \
not showing one yet."
            .to_string()
    } else {
        let items: Vec<String> = stretched
            .iter()
            .take(4)
            .map(|(i, s)| format!("{} ({} out of 100)", plain_name(&i.id), s.round() as i64))
            .collect();
        format!(
            "These measurements are the ones showing strain: {}. Read those as the places where \
the boom is genuinely stretched, rather than the whole picture being stretched.",
            join_list(&items)
        )
    };

    // Calm side: clearly low readings, lowest first.
    let mut calm: Vec<(&IndicatorReading, f64)> = r
        .indicators
        .iter()
        .filter_map(|i| scored(i).map(|s| (i, s)))
        .filter(|(_, s)| *s < 35.0)
        .collect();
    calm.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let what_is_calm = if calm.is_empty() {
        "Nothing is currently sitting in the calm range.".to_string()
    } else {
        let items: Vec<String> = calm
            .iter()
            .take(4)
            .map(|(i, s)| format!("{} ({} out of 100)", plain_name(&i.id), s.round() as i64))
            .collect();
        format!(
            "These measurements look calm: {}. The important thing here is what that means for \
the market's MOOD versus the money being SPENT. Prices are not behaving like a classic bubble \
even where spending is stretched.",
            join_list(&items)
        )
    };

    // Blind spots: real gaps (excluding the permanent weight-0 declarations).
    let gaps: Vec<String> = r
        .data_quality
        .unavailable
        .iter()
        .filter(|u| u.weight > 0.0)
        .map(|u| plain_name(&u.id).to_string())
        .collect();

    let what_we_cannot_measure = if gaps.is_empty() {
        "Every measurement this report attempted today, it managed to take. That is not the same \
as measuring everything that matters — the things it cannot measure at all are set out above."
            .to_string()
    } else {
        format!(
            "There are things this report could not check today: {}. They are left blank rather \
than guessed at, and the score is adjusted to account for their absence.",
            join_list(&gaps)
        )
    };

    // Timing. The single most misused output, so it is stated plainly and
    // without hedging buried in jargon.
    let about_timing = match r.analog.range_months {
        Some(rg) => format!(
            "This report cannot tell you when a bubble might burst. Nobody can. The historical \
comparison puts setups like today's roughly {:.0} to {:.0} months before a peak in the two past \
episodes it is based on. That is a description of what similar situations have looked like. It \
is not a forecast, and it is built on only two examples, so treat it as a rough yardstick at \
best. Bubbles can also run for years longer than anyone expects.",
            rg[0], rg[1]
        ),
        None => "This report cannot tell you when a bubble might burst, and today it is not even \
offering a historical comparison, because too much of the data is missing for that comparison \
to mean anything. What it can tell you is how stretched things look right now."
            .to_string(),
    };

    // One-sentence takeaway, assembled from the two sides.
    let top = stretched.first().map(|(i, s)| (*s, i.id.as_str()));
    let bottom_line = match top {
        Some((s, id)) => format!(
            "In one line: the money being spent looks {} ({}), while the market's mood looks \
calmer than that would suggest. A stretch between spending and pricing like this can go on \
for a long time, and it can also end suddenly. Nothing here tells you which.",
            stress_words(s),
            plain_name(id)
        ),
        None => "In one line: none of the things we can measure look like a bubble in its final \
stages right now. That is not the same as being safe."
            .to_string(),
    };

    // Direction of travel. Written in plain words, and it must never imply a
    // trend the tool refused to compute — when there is no delta, this says so
    // and explains why, rather than staying silent.
    let direction_of_travel = match &r.trend.delta {
        Some(d) => {
            let change = match d.direction {
                crate::model::Direction::Flat => {
                    "barely changed, which is worth saying plainly: the numbers are stable, not \
getting better or worse"
                }
                crate::model::Direction::Rising => {
                    "has got worse, meaning more of the warning signs are showing than before"
                }
                crate::model::Direction::Falling => {
                    "has got better, meaning fewer of the warning signs are showing than before"
                }
            };
            // Name what moved most, but only when something moved meaningfully.
            let mover = d
                .indicators
                .iter()
                .find(|i| i.direction != crate::model::Direction::Flat)
                .map(|i| {
                    format!(
                        " The biggest single change was in {} ({} points {}).",
                        plain_name(&i.id),
                        (i.delta.abs() * 10.0).round() / 10.0,
                        if i.delta > 0.0 { "worse" } else { "better" }
                    )
                })
                .unwrap_or_default();
            let phase = if d.phase_changed {
                format!(
                    " The overall description also moved from '{}' to '{}', which is the kind of \
change worth paying attention to.",
                    plain_phase(&d.phase_then),
                    plain_phase(&d.phase_now)
                )
            } else {
                String::new()
            };
            format!(
                "Comparing today with {} ({} days ago): the overall score {} ({} points).{} That \
comparison is only made against a previous reading taken under the same conditions, so an \
apples-to-apples comparison is guaranteed{}.",
                d.baseline_date,
                d.elapsed_days.round() as i64,
                change,
                format_args!("{:+.1}", d.composite_delta),
                mover,
                phase
            )
        }
        None => {
            // Say that it cannot be computed, and why. Silence here would look
            // like "nothing has changed", which is a different claim entirely.
            //
            // THE RAW REASON IS DELIBERATELY NOT EMBEDDED HERE. It used to be interpolated in
            // full, and because every rejected archive entry contributes its own sentence the
            // plain-English block grew into the single largest wall of text on the page (~3,300
            // words of near-identical refusal). The specifics belong in the technical detail,
            // which is collapsed; this block states the situation in plain terms and says where
            // the detail is. It must stay short enough to actually be read.
            "This report cannot yet say whether things are getting better or worse — there is no \
earlier reading it is willing to compare against. It will not compare two readings taken under \
different conditions, because the difference would then reflect the conditions rather than the \
market. Rather than show you a number that means nothing, it says nothing and tells you why; the \
exact reason is in the collapsed note below and in the caveats at the foot of this page. Run it \
again on another day and a comparison will appear."
                .to_string()
        }
    };

    // Declared blind spots: weight-0 entries, which are permanent properties of
    // the model rather than failures on this run. Written out in full because the
    // circularity one materially limits what a low score can mean.
    let declared: Vec<&crate::model::UnavailableItem> = r
        .data_quality
        .unavailable
        .iter()
        .filter(|u| u.weight <= 0.0)
        .collect();

    let circularity_note = "One thing this report cannot see at all is money going in circles. In this boom, large technology companies have taken ownership stakes in AI businesses, and those businesses then buy computing power, chips and cloud services back from the companies that funded them. A chip maker invests in the very customers who buy its chips. Sometimes the same dollars get counted as sales by more than one company in the chain. This report cannot measure any of that, because it needs to know who invested in whom and how much of a company's sales came from its own backer, and that relationship is not published in a form a program can read reliably. It matters twice over. First, a sale is not independent proof that outside demand exists. Second, if the young company's value falls, it can hit its backer's profits and its sales at the same time. If anything this gap makes the score look better than it should, because spending that is really money moving in a circle is counted here as ordinary business investment. Treat a calm reading as less reassuring than usual on this point.";

    let known_blind_spots = if declared.is_empty() {
        "This report measures a fixed list of things and claims nothing beyond it.".to_string()
    } else {
        let mut out = String::new();
        let has_circularity = declared.iter().any(|u| u.id == "circularity");
        if has_circularity {
            out.push_str(circularity_note);
            out.push(' ');
        }
        let others: Vec<String> = declared
            .iter()
            .filter(|u| u.id != "circularity")
            .map(|u| plain_name(&u.id).to_string())
            .collect();
        if !others.is_empty() {
            out.push_str(&format!(
                "There {} {} it also cannot measure at all: {}. Those are left blank rather than guessed at, and they never affect the score. The difference between these and the gaps listed elsewhere is that these are permanent — they would still be missing on a day when every source answered.",
                if others.len() == 1 { "is one more thing" } else { "are further things" },
                if others.len() == 1 { "" } else { "" },
                join_list(&others)
            ));
        }
        out.trim().to_string()
    };

    LaymanSummary {
        what_this_is,
        the_score,
        what_is_stretched,
        what_is_calm,
        what_we_cannot_measure,
        about_timing,
        direction_of_travel,
        known_blind_spots,
        bottom_line,
    }
}

/// Plain wording for a phase id, so the summary never shows a raw label.
fn plain_phase(id: &str) -> &'static str {
    match id {
        "early" => "early",
        "mid" => "middle",
        "late" => "late",
        "critical" => "extreme",
        _ => "unclear",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AnalogWindow, DataQuality, IndicatorReading, Provenance, Reading, Report, Trend,
        UnavailableItem,
    };

    fn prov() -> Provenance {
        Provenance {
            source: "t".into(),
            endpoint: "t".into(),
            as_of: "2026-09-17".into(),
            retrieved_at: "2026-09-17T00:00:00Z".into(),
        }
    }

    fn ind(id: &str, weight: f64, stress: Option<f64>) -> IndicatorReading {
        IndicatorReading {
            id: id.into(),
            label: id.into(),
            weight,
            rationale: String::new(),
            reading: match stress {
                Some(s) => Reading::Scored {
                    stress: s,
                    value: 1.0,
                    unit: "u".into(),
                    detail: String::new(),
                    provenance: prov(),
                },
                None => Reading::Unavailable {
                    reason: "down".into(),
                },
            },
            contribution: None,
        }
    }

    fn report(inds: Vec<IndicatorReading>, composite: f64, months: Option<[f64; 2]>) -> Report {
        let unavailable: Vec<UnavailableItem> = inds
            .iter()
            .filter(|i| !i.reading.is_available())
            .map(|i| UnavailableItem {
                id: i.id.clone(),
                label: i.label.clone(),
                weight: i.weight,
                reason: "down".into(),
            })
            .collect();
        Report {
            tool: "bubble-watch".into(),
            version: "0".into(),
            generated_at: "2026-09-17T00:00:00Z".into(),
            composite,
            coverage: 1.0,
            confidence: "high".into(),
            phase: "early".into(),
            phase_label: "Early".into(),
            phase_detail: "d".into(),
            analog: AnalogWindow {
                method: "historical_analog".into(),
                is_probability: false,
                analogs: vec!["a".into(), "b".into()],
                derivation: "d".into(),
                caveat: "c".into(),
                range_months: months,
                band: months.map(|_| "early".to_string()),
                band_note: None,
            },
            trend: Trend::empty("no history in this test"),
            // None in this fixture: with no archive there is no attribution to make, which is
            // exactly the case the drift module reports as "no attribution is possible" rather
            // than as a zero.
            drift: None,
            band_margin: None,
            exposure: Vec::new(),
            falsifiers: Vec::new(),
            judgments: Vec::new(),
            explosiveness: None,
            indicators: inds,
            data_quality: DataQuality {
                total_weight: 100.0,
                available_weight: 100.0,
                coverage: 1.0,
                unavailable,
                notes: vec![],
            },
            sources: vec![],
            headline: String::new(),
            caveats: vec![],
            disclaimer: String::new(),
            layman: LaymanSummary {
                what_this_is: String::new(),
                the_score: String::new(),
                what_is_stretched: String::new(),
                what_is_calm: String::new(),
                what_we_cannot_measure: String::new(),
                about_timing: String::new(),
                direction_of_travel: String::new(),
                known_blind_spots: String::new(),
                bottom_line: String::new(),
            },
        }
    }

    #[test]
    fn mentions_the_actual_score() {
        let r = report(
            vec![ind("volatility", 8.0, Some(20.0))],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        assert!(
            s.the_score.contains("32"),
            "score must appear: {}",
            s.the_score
        );
        assert!(s.the_score.contains("early"));
    }

    #[test]
    fn counts_measurements_not_weight() {
        // Regression: this once reported the weight total (100) as the number of
        // measurements, which is simply false.
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("credit_hy", 12.0, Some(16.0)),
                ind("foreign_interest", 0.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.the_score.contains("2 different measurements"),
            "must count the scored indicators: {}",
            s.the_score
        );
        assert!(
            !s.the_score.contains("100 separate measurements"),
            "must not report the weight total as a count"
        );
        assert!(
            s.the_score.contains("1 thing it cannot measure"),
            "declared gaps should be acknowledged separately: {}",
            s.the_score
        );
    }

    #[test]
    fn avoids_finance_jargon() {
        // The whole point of this module. If a term from this list appears, the
        // summary has failed its one job.
        let r = report(
            vec![
                ind("credit_hy", 12.0, Some(16.0)),
                ind("capex_vs_cashflow", 14.0, Some(42.0)),
                ind("funding_gap", 8.0, Some(68.0)),
            ],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        let all = format!(
            "{} {} {} {} {} {} {} {} {}",
            s.what_this_is,
            s.the_score,
            s.what_is_stretched,
            s.what_is_calm,
            s.what_we_cannot_measure,
            s.about_timing,
            s.direction_of_travel,
            s.known_blind_spots,
            s.bottom_line
        )
        .to_lowercase();
        for term in [
            "composite",
            "log-linear",
            "option-adjusted",
            "renormaliz",
            "oas",
            "percentile",
            "basis point",
            "z-score",
            "hyperscaler",
            "capex",
            "spread",
            "correlation",
            "volatility",
            "drawdown",
        ] {
            assert!(
                !all.contains(term),
                "jargon leaked into the summary: {}",
                term
            );
        }
    }

    #[test]
    fn never_claims_to_predict_when() {
        let r = report(
            vec![ind("volatility", 8.0, Some(20.0))],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        assert!(
            s.about_timing.contains("cannot tell you when"),
            "must state plainly that it cannot time the market"
        );
        assert!(
            s.about_timing.contains("not a forecast"),
            "must state plainly that the comparison is not a forecast"
        );
    }

    #[test]
    fn reports_gaps_rather_than_hiding_them() {
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("credit_hy", 12.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.what_we_cannot_measure.contains("could not check"),
            "a missing measurement must be admitted: {}",
            s.what_we_cannot_measure
        );
    }

    #[test]
    fn says_so_when_nothing_is_stretched() {
        let r = report(vec![ind("volatility", 8.0, Some(10.0))], 10.0, None);
        let s = summarize(&r);
        assert!(s.what_is_stretched.contains("Nothing on the list"));
        assert!(s.bottom_line.contains("not the same as being safe"));
    }

    #[test]
    fn names_the_most_stretched_measurement() {
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("funding_gap", 8.0, Some(68.0)),
            ],
            40.0,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.what_is_stretched
                .contains("spend compared with what they sell"),
            "the biggest strain should be named: {}",
            s.what_is_stretched
        );
    }

    #[test]
    fn every_configured_indicator_has_a_plain_name() {
        // Regression guard. A new indicator that falls through to "one of the
        // measurements" produces a summary sentence that names nothing, which is
        // useless to the non-specialist this module exists for. Caught exactly
        // that when backlog_quality shipped.
        let path = std::path::Path::new("config/indicators.toml");
        if !path.exists() {
            return;
        }
        let text = std::fs::read_to_string(path).unwrap();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("id = \"") {
                if let Some(id) = rest.strip_suffix('\"') {
                    assert_ne!(
                        plain_name(id),
                        "one of the measurements",
                        "indicator '{}' has no plain-English name; add it to plain_name()",
                        id
                    );
                }
            }
        }
    }

    #[test]
    fn ignores_weight_zero_declared_gaps() {
        // foreign_interest is a permanent blind spot at weight 0; it must not be
        // reported as a missing measurement, and must never be scored.
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("foreign_interest", 0.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            !s.what_we_cannot_measure.contains("foreign"),
            "a declared weight-0 gap is a permanent disclosure, not a data failure"
        );
    }

    #[test]
    fn is_deterministic() {
        let mk = || {
            report(
                vec![
                    ind("a", 10.0, Some(50.0)),
                    ind("b", 10.0, Some(50.0)),
                    ind("c", 10.0, Some(20.0)),
                ],
                40.0,
                Some([30.0, 72.0]),
            )
        };
        assert_eq!(summarize(&mk()), summarize(&mk()));
    }
}
