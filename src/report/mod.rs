pub mod charts;
pub mod html;
pub mod json;
pub mod layman;

use crate::config::Config;
use crate::model::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DISCLAIMER: &str = "This tool is a state descriptor, not a forecast and not investment \
advice. It measures where a set of published indicators currently sit relative to documented \
historical reference points, and it says so plainly when it cannot measure something. It has no \
ability to tell you if or when a market reversal will occur, and it should not be the sole basis \
for any financial decision.";

/// Assemble the final report from evaluated readings. Pure.
///
/// History is not consulted: the trend is reported as absent with a reason, so
/// a caller that wants direction of travel must use `build_with_history`.
/// Keeping this signature means every existing caller keeps working unchanged.
pub fn build(
    readings: Vec<IndicatorReading>,
    obs: &Observations,
    cfg: &Config,
    generated_at: &str,
) -> Report {
    build_with_history(readings, obs, cfg, generated_at, &[], false, false)
}

/// Assemble the final report, including direction of travel against a baseline
/// drawn from `archive`.
///
/// PURE — the caller loads the archive and appends the new run. `recorded`
/// records whether this run was written to the archive, and `warnings` carries
/// any non-fatal problems found while reading it.
///
/// `history_consulted` says whether the CALLER actually read the run archive.
/// It exists because an empty slice is ambiguous: a command that never looks at
/// history and a command that looked and found nothing both pass `&[]`, and the
/// honest explanation of "no direction of travel" is completely different in
/// the two cases. Passing `false` makes the report say so instead of claiming
/// the history is empty.
///
/// The trend NEVER enters the composite. It is derived from the composite, so
/// scoring it inside the composite would make the score partly a function of its
/// own past; the composite is byte-identical with and without history, and
/// `tests/integration_tests.rs` asserts that.
pub fn build_with_history(
    readings: Vec<IndicatorReading>,
    obs: &Observations,
    cfg: &Config,
    generated_at: &str,
    archive: &[TrendPoint],
    recorded: bool,
    history_consulted: bool,
) -> Report {
    let mut readings = readings;
    let (comp_opt, coverage) = crate::score::composite(&readings);
    crate::score::attribute(&mut readings);

    let composite = comp_opt.unwrap_or(0.0);
    let confidence = crate::score::confidence(
        coverage,
        cfg.coverage_floor.low_below,
        cfg.coverage_floor.high_above,
    );

    let phase = match comp_opt {
        Some(c) => crate::phase::Phase::classify(c, cfg),
        None => crate::phase::Phase::Early,
    };

    let analog = crate::phase::analog_window(comp_opt, coverage, cfg);

    // Per-company exposure. A different level of analysis from the composite —
    // market-level configuration versus who carries the risk — so it is context
    // and never folded in.
    let exposure = crate::exposure::build(obs, &crate::history::date_of(generated_at));

    // Declared judgment. Reported beside the composite, never inside it.
    let judgments = crate::subjective::build();

    // Falsification tests. Reported beside the composite, never inside it.
    let falsifiers = crate::falsifiers::build(obs);

    // GSADF explosiveness test. A second, independent method: a formal test with
    // a published pedigree, rather than an anchored judgement. Reported alongside
    // the composite and never reconciled with it, because the tension between the
    // two is exactly what a reader should see.
    let mut gsadf_note: Option<String> = None;
    let explosiveness = if cfg.gsadf.enabled {
        let series_key = if obs.yahoo.contains_key("SPX_MAX_mo") {
            "SPX_MAX_mo"
        } else if obs.yahoo.contains_key("SPX_5y_mo") {
            "SPX_5y_mo"
        } else {
            ""
        };
        match obs.yahoo.get(series_key) {
            None => {
                gsadf_note = Some(
                    "the explosiveness test DID NOT RUN: no long-history price series was \
                     retrieved. That is a missing second method, not evidence either way, and \
                     the composite above is unaffected by its absence."
                        .into(),
                );
                None
            }
            Some(s) => {
                let r = crate::gsadf::run(&s.points, cfg.gsadf.monte_carlo_reps, cfg.gsadf.seed);
                if r.is_none() {
                    gsadf_note = Some(format!(
                        "the explosiveness test DID NOT RUN: the available series ({} \
                         observations) is too short to test. That is a missing second method, \
                         not evidence either way.",
                        s.points.len()
                    ));
                }
                r
            }
        }
    } else {
        gsadf_note = Some("the explosiveness test is disabled in config.".into());
        None
    };

    // Direction of travel. Computed from the composite that was just produced,
    // and deliberately NOT fed back into it.
    let trend = if !history_consulted {
        // This command path never reads the archive at all. Say THAT, rather than
        // claiming no previous run exists — which would be false whenever a
        // previous run does exist, and would promise a comparison that this
        // command can never produce however many times it is re-run.
        crate::model::Trend::empty(
            "direction of travel was NOT computed because this command does not consult the \
             run archive. Use `report` or `trend` for a comparison against a recorded run. \
             This is a property of the command, not a statement about whether history exists.",
        )
    } else {
        let current = crate::history::point_from(
            &readings,
            composite,
            coverage,
            phase.id(),
            generated_at,
            &cfg.meta.schema_version,
        );
        crate::history::compute(
            archive,
            &current,
            &readings,
            &cfg.trend,
            recorded,
            Vec::new(),
        )
    };

    // Model-versus-market attribution over the archive. Cheap, pure, and reported beside the
    // trend so a reader cannot see the history table without also seeing that most of its
    // movement is the instrument.
    let drift = if archive.is_empty() {
        None
    } else {
        Some(crate::drift::attribute_with_cfg(archive, cfg))
    };

    // Data quality: what is missing, and how much weight it carried.
    let total_weight = cfg.total_weight();
    let available_weight: f64 = readings
        .iter()
        .filter(|r| r.reading.is_available() && r.weight > 0.0)
        .map(|r| r.weight)
        .sum();

    let unavailable: Vec<UnavailableItem> = readings
        .iter()
        .filter(|r| !r.reading.is_available())
        .map(|r| UnavailableItem {
            id: r.id.clone(),
            label: r.label.clone(),
            weight: r.weight,
            reason: match &r.reading {
                Reading::Unavailable { reason } => reason.clone(),
                _ => String::new(),
            },
        })
        .collect();

    let sources = crate::sources::source_health(obs);

    // Caveats are generated from the actual state of the data, not boilerplate.
    let mut caveats = Vec::new();

    // Equity-only bias when credit is missing.
    let credit_missing = readings
        .iter()
        .any(|r| (r.id == "credit_hy" || r.id == "credit_ig") && !r.reading.is_available());
    let credit_present = readings
        .iter()
        .any(|r| (r.id == "credit_hy" || r.id == "credit_ig") && r.reading.is_available());
    if credit_missing {
        if credit_present {
            caveats.push(
                "PARTIAL CREDIT COVERAGE: one of the two credit-spread indicators is \
                 unavailable. The composite therefore leans more heavily on equity-price \
                 measures than intended."
                    .into(),
            );
        } else {
            caveats.push(
                "EQUITY-PRICE BIAS: both credit-spread indicators are unavailable (FRED is an \
                 optional source and is not currently answering from this host). The composite \
                 is consequently computed almost entirely from equity-market data, which is the \
                 half of the picture that reflects sentiment rather than financing conditions. \
                 Treat the level as provisional and compare only against other runs with the \
                 same coverage."
                    .into(),
            );
        }
    }

    // Comparability warning whenever coverage is imperfect.
    if coverage < 0.999 {
        caveats.push(format!(
            "COMPARABILITY: this run has {:.0}% weighted coverage. Because the composite is \
             renormalized over available weight, a run with different coverage is NOT directly \
             comparable to this one at the decimal level. Compare direction and the \
             per-indicator stresses, or re-run with all sources healthy.",
            coverage * 100.0
        ));
    }

    let fallbacks: Vec<String> = Vec::new();
    if !fallbacks.is_empty() {
        caveats.push(format!(
            "FILING BASIS: some cohort fundamentals came from annual filings rather than four \
             summed quarters ({}).",
            fallbacks.join(", ")
        ));
    }

    caveats.push(
        "PROXIES IN USE: valuation is price-stretch versus trend (no free earnings series); \
         concentration and breadth use cap-weight versus equal-weight returns (not a free-float \
         share). Issuance is no longer a proxy: it is computed from reported cash flows (cash \
         raised from stock issuance minus cash paid to repurchase stock, over operating cash flow), \
         which are available for every scored filer except AMZN, whose equity-issuance concept does \
         not exist because it does not issue equity. Each indicator's row names its own limitation."
            .into(),
    );
    // Double-counting disclosure. The config carries the full note, but a reader
    // of the report must not read these two rows as independent confirmations, so
    // it is stated where the numbers are.
    caveats.push(
        "NOT INDEPENDENT: `concentration` and `breadth` measure the same underlying quantity — \
         equal-weight versus cap-weight performance — over different windows. Measured correlation \
         is -0.70 at 12 months and -0.93 at 6 months, i.e. up to ~86% shared variance. Their \
         combined weight was reduced from 22 to 11 at methodology 1.2 so the model does not count \
         one phenomenon as two pieces of evidence. Do NOT read agreement between the two as \
         corroboration."
            .into(),
    );
    if let Some(n) = &gsadf_note {
        // A second method that silently vanishes is worse than one that reports
        // nothing: a reader would infer agreement where there is only absence.
        caveats.push(format!("EXPLOSIVENESS TEST: {}", n));
    }
    caveats.push(format!(
        "METHODOLOGY VERSION {}. The tool records which version of the model produced each run, \
         and refuses to compute a direction of travel across a version change: redefining an \
         indicator changes what the composite MEANS, so a difference across such a change would \
         report an accounting change as a market move. This version replaced the share-count \
         issuance proxy with reported cash flows and added primary-market supply; runs recorded \
         before it are preserved but cannot serve as baselines.",
        cfg.meta.schema_version
    ));
    caveats.push(
        "A CALM READING IS NOT EVIDENCE OF SAFETY. Bubbles are generally identifiable only in \
         hindsight, and cheap credit or healthy breadth are consistent with both 'no bubble' and \
         'the quiet phase of one'. More generally: this tool reports the STATE of a set of \
         indicators, and state extremity carries no timing information."
            .into(),
    );
    caveats.push(
        "CREDIT IS SCORED AS LEVEL, NOT COMPLACENCY. A wide spread reads as high stress and a \
         tight one as low. Scoring tight spreads as the bubble signal is a defensible \
         alternative (credit tightened into the dot-com peak) but is deliberately not used here: \
         HY OAS has a ~3.0% median over the available record, so a level-based complacency score \
         would read most of the last decade as a bubble — a near-constant with no timing value. \
         The informative credit signal is the DIRECTION OF TRAVEL from a tight base. Measured \
         effect of the alternative reading on this composite: 32.4 -> 44.2, i.e. early -> late."
            .into(),
    );

    // Direction of travel: when no delta could be produced, state the refusal and WHY it
    // was a refusal rather than missing data.
    //
    // NO RATIONALE IS APPENDED HERE. Each rejection in `history::is_eligible` now carries
    // its own explanation, because the causes are genuinely different: a same-day run, an
    // insufficient elapsed gap, a methodology change, and a coverage mismatch are four
    // distinct reasons and only one of them is about coverage. This site previously bolted
    // one fixed coverage rationale onto every refusal, which meant an archive rejected
    // purely for a methodology change at identical coverage was explained with a cause
    // that had not occurred. Explaining the wrong cause is the same class of error as
    // inventing the number.
    if trend.delta.is_none() {
        if let Some(reason) = &trend.reason {
            // `trim_end_matches('.')`: reasons may end in a full stop, and
            // "{}. ..." would otherwise emit a doubled stop.
            caveats.push(format!(
                "DIRECTION OF TRAVEL NOT REPORTED: {}.",
                reason.trim_end_matches('.')
            ));
        }
    }

    let headline = match comp_opt {
        Some(c) => {
            // Append direction of travel only when it is actually available, so
            // the headline never implies a trend that was refused.
            let dir = match &trend.delta {
                Some(d) => format!(
                    " Direction of travel: {} {:+.1} over {:.0} day(s) against the {} baseline \
                     ({}% coverage).",
                    d.direction.as_str(),
                    d.composite_delta,
                    d.elapsed_days,
                    d.baseline_date,
                    d.baseline_coverage * 100.0
                ),
                None => String::new(),
            };
            format!(
                "Composite bubble-stress {:.1}/100 ({} phase) on {:.0}% weighted coverage, {} \
                 confidence. {} of {} weight available; {} indicator(s) unavailable and contributing \
                 nothing.{}",
                c,
                phase.id(),
                coverage * 100.0,
                confidence,
                available_weight,
                total_weight,
                unavailable.len(),
                dir
            )
        }
        None => "NO COMPOSITE PRODUCED: no configured indicator could be measured from the \
                 available sources. This is reported as a total data failure rather than a score \
                 of zero, because zero would imply a measurement that was never taken."
            .to_string(),
    };

    // The plain-English summary is generated from the same numbers as the rest
    // of the report, so it cannot drift out of sync with the data.
    let placeholder = Report {
        tool: "bubble-watch".into(),
        version: VERSION.into(),
        generated_at: generated_at.into(),
        composite,
        coverage,
        confidence: confidence.to_string(),
        phase: phase.id().to_string(),
        phase_label: phase.label().to_string(),
        phase_detail: phase.detail().to_string(),
        layman: crate::report::layman::LaymanSummary {
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
        analog,
        trend,
        drift,
        exposure,
        falsifiers,
        judgments,
        explosiveness,
        indicators: readings,
        data_quality: DataQuality {
            total_weight,
            available_weight,
            coverage,
            unavailable,
            notes: vec![
                "Unavailable indicators are never imputed, defaulted, or scored.".into(),
                "The composite is renormalized over available weight, so a degraded source \
                 reduces coverage rather than shifting the level."
                    .into(),
            ],
        },
        sources,
        headline,
        caveats,
        disclaimer: DISCLAIMER.into(),
    };

    let layman = crate::report::layman::summarize(&placeholder);

    Report {
        layman,
        ..placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Provenance;

    fn cfg() -> Config {
        Config::load(std::path::Path::new("config/indicators.toml"))
            .unwrap_or_else(|_| panic!("shipped config must load"))
    }

    fn scored(id: &str, weight: f64, stress: f64) -> IndicatorReading {
        IndicatorReading {
            id: id.into(),
            label: id.into(),
            weight,
            rationale: String::new(),
            reading: Reading::Scored {
                stress,
                value: 1.0,
                unit: "u".into(),
                detail: "d".into(),
                provenance: Provenance {
                    source: "t".into(),
                    endpoint: "t".into(),
                    as_of: "2026-09-16".into(),
                    retrieved_at: "2026-09-16T00:00:00Z".into(),
                },
            },
            contribution: None,
        }
    }

    fn unavailable(id: &str, weight: f64) -> IndicatorReading {
        IndicatorReading {
            id: id.into(),
            label: id.into(),
            weight,
            rationale: String::new(),
            reading: Reading::Unavailable {
                reason: "down".into(),
            },
            contribution: None,
        }
    }

    #[test]
    fn no_score_does_not_become_zero_with_a_headline_claim() {
        let c = cfg();
        let obs = Observations::default();
        let r = build(
            vec![
                unavailable("credit_hy", 12.0),
                unavailable("capex_vs_cashflow", 14.0),
            ],
            &obs,
            &c,
            "2026-09-16T00:00:00Z",
        );
        assert_eq!(r.coverage, 0.0);
        assert!(r.headline.contains("NO COMPOSITE PRODUCED"));
    }

    #[test]
    fn credit_outage_raises_an_equity_bias_caveat() {
        let c = cfg();
        let obs = Observations::default();
        let r = build(
            vec![
                scored("valuation_stretch", 14.0, 50.0),
                unavailable("credit_hy", 12.0),
                unavailable("credit_ig", 6.0),
            ],
            &obs,
            &c,
            "2026-09-16T00:00:00Z",
        );
        assert!(
            r.caveats.iter().any(|x| x.contains("EQUITY-PRICE BIAS")),
            "must disclose that the reading leans on equity data: {:?}",
            r.caveats
        );
    }

    #[test]
    fn imperfect_coverage_raises_comparability_warning() {
        let c = cfg();
        let obs = Observations::default();
        let r = build(
            vec![
                scored("valuation_stretch", 14.0, 50.0),
                unavailable("credit_hy", 12.0),
            ],
            &obs,
            &c,
            "2026-09-16T00:00:00Z",
        );
        assert!(r.caveats.iter().any(|x| x.contains("COMPARABILITY")));
    }

    #[test]
    fn report_is_deterministic_for_identical_inputs() {
        let c = cfg();
        let obs = Observations::default();
        let mk = || {
            vec![
                scored("valuation_stretch", 14.0, 55.0),
                scored("concentration", 12.0, 30.0),
            ]
        };
        let a = build(mk(), &obs, &c, "2026-09-16T00:00:00Z");
        let b = build(mk(), &obs, &c, "2026-09-16T00:00:00Z");
        assert_eq!(
            crate::report::json::to_json(&a, false),
            crate::report::json::to_json(&b, false)
        );
    }

    #[test]
    fn html_renders_without_external_resources() {
        let c = cfg();
        let obs = Observations::default();
        let r = build(
            vec![scored("valuation_stretch", 14.0, 42.0)],
            &obs,
            &c,
            "2026-09-16T00:00:00Z",
        );
        let h = crate::report::html::render(&r);
        assert!(h.starts_with("<!DOCTYPE html>"));
        assert!(!h.contains("<script"), "must not require JavaScript");

        // Absence of external RESOURCE LOADING is the real requirement: the file
        // must render with no network access. Provenance citations are plain
        // text and are explicitly allowed — asserting on "https://" would both
        // forbid the citations and, worse, pass vacuously whenever a fixture
        // happened to carry an empty endpoint.
        assert!(
            !h.contains("src=\"http") && !h.contains("href=\"http"),
            "must not load external resources"
        );
        assert!(
            !h.contains("@import"),
            "must not import external stylesheets"
        );
        assert!(
            !h.contains("rel=\"stylesheet\""),
            "must not link an external stylesheet"
        );
    }

    #[test]
    fn html_states_that_the_analog_window_is_not_a_probability() {
        // Timing is the most misusable output in the tool, so the VISIBLE page
        // must carry the disclaimer in plain words — matching the JSON's
        // is_probability:false rather than relying on a soft caveat.
        let c = cfg();
        let obs = Observations::default();
        let mut reading = scored("valuation_stretch", 14.0, 42.0);
        if let Reading::Scored { provenance, .. } = &mut reading.reading {
            provenance.endpoint = "t".into();
        }
        let r = build(vec![reading], &obs, &c, "2026-09-16T00:00:00Z");
        let h = crate::report::html::render(&r);
        if r.analog.range_months.is_some() {
            assert!(
                h.contains("not a probability"),
                "a rendered time range must be labelled as not a probability"
            );
            assert!(
                h.contains("not a prediction"),
                "a rendered time range must be labelled as not a prediction"
            );
        }
    }

    #[test]
    fn html_allows_provenance_citations_but_never_loads_them() {
        // Regression guard for the above: a report WITH a real http endpoint in
        // its provenance must still render, and must still not load anything.
        let c = cfg();
        let obs = Observations::default();
        let mut reading = scored("valuation_stretch", 14.0, 42.0);
        if let Reading::Scored { provenance, .. } = &mut reading.reading {
            provenance.endpoint =
                "https://query1.finance.yahoo.com/v8/finance/chart/%5EGSPC".into();
        }
        let r = build(vec![reading], &obs, &c, "2026-09-16T00:00:00Z");
        let h = crate::report::html::render(&r);
        assert!(
            h.contains("query1.finance.yahoo.com"),
            "the citation must appear so the number is traceable"
        );
        assert!(
            !h.contains("src=\"http") && !h.contains("href=\"http"),
            "a citation must never become a resource load"
        );
    }
}
