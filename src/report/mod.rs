pub mod html;
pub mod json;

use crate::config::Config;
use crate::model::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DISCLAIMER: &str = "This tool is a state descriptor, not a forecast and not investment \
advice. It measures where a set of published indicators currently sit relative to documented \
historical reference points, and it says so plainly when it cannot measure something. It has no \
ability to tell you if or when a market reversal will occur, and it should not be the sole basis \
for any financial decision.";

/// Assemble the final report from evaluated readings. Pure.
pub fn build(
    readings: Vec<IndicatorReading>,
    obs: &Observations,
    cfg: &Config,
    generated_at: &str,
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
         share); issuance uses reported share counts (net of buybacks, and blind to IPOs outside \
         the cohort). Each indicator's row names its own limitation."
            .into(),
    );
    caveats.push(
        "A CALM READING IS NOT EVIDENCE OF SAFETY. Bubbles are generally identifiable only in \
         hindsight, and tight credit spreads or healthy breadth are consistent with both 'no \
         bubble' and 'the complacent phase of one'."
            .into(),
    );

    let headline = match comp_opt {
        Some(c) => format!(
            "Composite bubble-stress {:.1}/100 ({} phase) on {:.0}% weighted coverage, {} \
             confidence. {} of {} weight available; {} indicator(s) unavailable and contributing \
             nothing.",
            c,
            phase.id(),
            coverage * 100.0,
            confidence,
            available_weight,
            total_weight,
            unavailable.len()
        ),
        None => "NO COMPOSITE PRODUCED: no configured indicator could be measured from the \
                 available sources. This is reported as a total data failure rather than a score \
                 of zero, because zero would imply a measurement that was never taken."
            .to_string(),
    };

    Report {
        tool: "bubble-watch".into(),
        version: VERSION.into(),
        generated_at: generated_at.into(),
        composite,
        coverage,
        confidence: confidence.to_string(),
        phase: phase.id().to_string(),
        phase_label: phase.label().to_string(),
        phase_detail: phase.detail().to_string(),
        analog,
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
