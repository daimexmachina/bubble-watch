//! Integration tests.
//!
//! Two classes:
//!   * OFFLINE tests that must always pass — they exercise the honesty
//!     contract using fixtures and never touch the network.
//!   * LIVE tests that hit real Yahoo/EDGAR and are skipped with a clear
//!     message when the network is unavailable. They never assert a specific
//!     market value, because that would make the suite fail whenever the market
//!     moves — they assert shape and internal consistency instead.

use bubble_watch::config::Config;
use bubble_watch::http::Fetcher;
use bubble_watch::indicators::{self, Ctx};
use bubble_watch::model::{Observations, Reading};
use std::path::PathBuf;

fn cfg() -> Config {
    Config::load(std::path::Path::new("config/indicators.toml")).expect("shipped config must load")
}

fn fixture_dir() -> PathBuf {
    PathBuf::from("tests/fixtures")
}

/// Build an Observations from committed fixture files. Lets the whole scoring
/// path be tested deterministically with no network at all.
fn fixture_obs() -> Option<Observations> {
    let dir = fixture_dir();
    let path = dir.join("observations.json");
    if !path.exists() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

#[test]
fn offline_fixture_run_produces_an_auditable_report() {
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: tests/fixtures/observations.json not present");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-16T00:00:00Z");

    // The composite must be reconstructable from the reported contributions.
    let sum: f64 = r.indicators.iter().filter_map(|i| i.contribution).sum();
    assert!(
        (sum - r.composite).abs() < 1e-6,
        "contributions ({}) must sum to the composite ({})",
        sum,
        r.composite
    );

    // Coverage must be internally consistent with the available weights.
    assert!((0.0..=1.0).contains(&r.coverage));
    assert!(r.data_quality.available_weight <= r.data_quality.total_weight);

    // Every scored reading must carry provenance with a non-empty source.
    for i in &r.indicators {
        if let Reading::Scored { provenance, .. } = &i.reading {
            assert!(!provenance.source.is_empty(), "{} lacks a source", i.id);
            assert!(
                !provenance.endpoint.is_empty(),
                "{} lacks an endpoint",
                i.id
            );
        }
    }
}

#[test]
fn report_is_reproducible_from_the_same_fixture() {
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let mk = || {
        let ctx = Ctx { obs: &obs, cfg: &c };
        let readings = indicators::evaluate_all(&ctx);
        bubble_watch::report::build(readings, &obs, &c, "2026-09-16T00:00:00Z")
    };
    let a = bubble_watch::report::json::to_json(&mk(), false);
    let b = bubble_watch::report::json::to_json(&mk(), false);
    assert_eq!(a, b, "identical inputs must produce byte-identical output");
}

#[test]
fn offline_mode_never_returns_fabricated_data() {
    // With an empty cache and offline set, every source must report a failure
    // and no indicator may produce a score.
    let tmp = std::env::temp_dir().join("bw-offline-test-cache");
    let _ = std::fs::remove_dir_all(&tmp);
    let f = Fetcher::new(true, Some(tmp.clone()));
    let obs = bubble_watch::sources::fetch_all(&f, true);
    assert!(obs.yahoo.is_empty(), "offline must not invent yahoo data");
    assert!(obs.edgar.is_empty(), "offline must not invent edgar data");

    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-16T00:00:00Z");
    assert_eq!(r.coverage, 0.0);
    assert!(
        r.headline.contains("NO COMPOSITE PRODUCED"),
        "a total data failure must be reported as such, not as a zero score"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn every_unavailable_weight_is_documented_in_caveats_or_quality() {
    let c = cfg();
    let obs = Observations::default();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-16T00:00:00Z");

    for i in &r.indicators {
        if let Reading::Unavailable { .. } = &i.reading {
            assert!(
                r.data_quality.unavailable.iter().any(|u| u.id == i.id),
                "unavailable indicator {} must appear in data_quality.unavailable",
                i.id
            );
        }
    }
}

#[test]
fn live_sources_are_reachable_or_the_test_says_why() {
    // Networked test: opt in explicitly so `cargo test` stays fast and offline.
    //   BUBBLE_WATCH_LIVE=1 cargo test -- --nocapture
    // It asserts SHAPE, never a specific market value, so a moving market can
    // never make the suite fail.
    if std::env::var("BUBBLE_WATCH_LIVE").is_err() {
        eprintln!("SKIP live test: set BUBBLE_WATCH_LIVE=1 to run networked assertions");
        return;
    }
    let tmp = std::env::temp_dir().join(format!("bw-live-{}", std::process::id()));
    let f = Fetcher::new(false, Some(tmp.clone()));
    let obs = bubble_watch::sources::fetch_all(&f, false);

    if obs.yahoo.is_empty() {
        eprintln!(
            "SKIP live test: no yahoo series retrieved. Failures: {:?}",
            obs.failures
                .iter()
                .filter(|x| x.source == "yahoo")
                .map(|x| &x.reason)
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&tmp);
        return;
    }

    // Assert shape, never a specific price.
    let spx = obs.yahoo.get("SPX_5y_mo").expect("SPX 5y series present");
    assert!(spx.points.len() > 24, "need enough history for a trend fit");
    assert!(spx.latest_value().unwrap() > 0.0);
    assert!(!spx.provenance.endpoint.is_empty());

    // EDGAR: at least one filer should yield capex facts when the network is up.
    let with_capex = obs.edgar.values().filter(|c| !c.capex.is_empty()).count();
    if with_capex == 0 {
        eprintln!("NOTE: EDGAR returned no capex facts this run (may be rate-limited)");
    } else {
        for (t, c) in &obs.edgar {
            for f in &c.capex {
                assert!(f.days > 0, "{} capex fact {} has no duration", t, f.tag);
                assert!(!f.end.is_empty());
            }
        }
    }

    // Internal consistency of the full pipeline on live data.
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-16T00:00:00Z");
    assert!((0.0..=100.0).contains(&r.composite));
    let sum: f64 = r.indicators.iter().filter_map(|i| i.contribution).sum();
    assert!((sum - r.composite).abs() < 1e-6);

    let _ = std::fs::remove_dir_all(&tmp);
}
