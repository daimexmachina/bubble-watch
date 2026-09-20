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
fn offline_mode_scores_only_fixture_backed_indicators() {
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

    // COVERAGE IS NOT ZERO, AND THAT IS CORRECT. Two indicators (`frontier_premium` and
    // `circularity`) read COMMITTED data rather than the network, so they legitimately score
    // with every source offline. Committed data with recorded provenance is source data, not
    // fabricated data — the distinction this test protects is between REAL data and INVENTED
    // data, not between online and offline.
    //
    // THE BOUND IS COMPUTED, NOT HARDCODED. It was a literal 0.10, which went stale the moment
    // circularity's weight was raised from 6 to 12 — the behaviour was correct and the constant
    // was not. Deriving it from the fixture-backed weights means the test keeps asserting the
    // real property ("offline coverage equals exactly the fixture-backed share") instead of
    // breaking whenever a weight changes for unrelated reasons.
    let total_weight = c.total_weight();
    let fixture_weight: f64 = r
        .indicators
        .iter()
        .filter(|i| matches!(i.id.as_str(), "frontier_premium" | "circularity"))
        .map(|i| i.weight)
        .sum();
    let expected_ceiling = fixture_weight / total_weight;
    assert!(
        r.coverage <= expected_ceiling + 1e-9,
        "offline coverage {:.3} exceeded the fixture-backed share {:.3} — something \
         network-backed scored with the network off",
        r.coverage,
        expected_ceiling
    );
    assert!(
        r.coverage > 0.0,
        "fixture-backed indicators should still score offline"
    );
    for i in &r.indicators {
        // FIXTURE-BACKED indicators legitimately score offline, because their input is
        // committed with recorded provenance rather than fetched. The distinction this test
        // protects is REAL versus INVENTED data, not online versus offline.
        //
        //   frontier_premium — reads tests/fixtures/arena_frontier_gap.json (LMArena parquet
        //                      extracted once, because the source is 56MB of parquet);
        //   circularity      — reads src/circular_rubric.rs, whose every entry carries a
        //                      citation to a filing that was actually read. It performs no
        //                      fetch, so there is nothing for offline mode to disable.
        let fixture_backed = matches!(i.id.as_str(), "frontier_premium" | "circularity");
        if !fixture_backed && i.reading.is_available() {
            panic!(
                "indicator '{}' produced a score with all sources offline — that would be \
                 fabricated data",
                i.id
            );
        }
    }
    // And the claim is still reported honestly rather than as a confident number.
    assert!(
        r.confidence == "low",
        "a low-coverage composite must not claim better than low confidence, got {}",
        r.confidence
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
fn the_proxy_disclosure_matches_what_is_actually_a_proxy() {
    // Regression guard: the caveat used to claim issuance "uses reported share
    // counts". When issuance was rebuilt on cash-flow data, that sentence became
    // false — a report that misdescribes its own method is worse than no caveat.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-17T12:00:00Z");
    let proxies = r
        .caveats
        .iter()
        .find(|x| x.contains("PROXIES IN USE"))
        .expect("a proxy disclosure must exist");
    assert!(
        !proxies.contains("issuance uses reported share counts"),
        "issuance is no longer share-count based; the disclosure must say so: {}",
        proxies
    );
    assert!(
        proxies.contains("cash flows"),
        "the disclosure must describe the current method: {}",
        proxies
    );
}

#[test]
fn the_report_states_its_own_methodology_version() {
    // The archive refuses to compare across a methodology change, so the report
    // must say which version produced it — otherwise a reader comparing two
    // saved reports by hand has no way to know they are incomparable.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-17T12:00:00Z");
    let m = r
        .caveats
        .iter()
        .find(|x| x.contains("METHODOLOGY VERSION"))
        .unwrap_or_else(|| panic!("must disclose its methodology: {:?}", r.caveats));
    assert!(
        m.contains(&c.meta.schema_version),
        "must name the actual version {}: {}",
        c.meta.schema_version,
        m
    );
}

#[test]
fn the_report_discloses_the_concentration_breadth_double_counting() {
    // Regression guard. These two indicators are algebraically the same quantity
    // (equal-weight vs cap-weight) at measured r = -0.70 to -0.93. They were once
    // carried at 22 combined weight as though they were two independent
    // confirmations, which overstates how much evidence the composite actually
    // aggregates. If the caveat or the reduced weight is ever removed, this fails.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-17T12:00:00Z");

    assert!(
        r.caveats.iter().any(|x| x.contains("NOT INDEPENDENT")),
        "the correlation between concentration and breadth must be disclosed: {:?}",
        r.caveats
    );

    // The combined weight must stay at roughly one indicator's worth, not two.
    let conc = c
        .indicator("concentration")
        .expect("concentration configured")
        .weight;
    let brd = c.indicator("breadth").expect("breadth configured").weight;
    assert!(
        conc + brd <= 12.0,
        "combined weight {} is too high for a pair with up to 86% shared variance",
        conc + brd
    );
}

#[test]
fn a_missing_explosiveness_test_is_disclosed_not_silent() {
    // Regression guard. GSADF depends on a long price history that Yahoo
    // rate-limits intermittently. If that fetch fails the test simply returns
    // nothing, and without this disclosure a reader would see no result and infer
    // agreement with the composite rather than ABSENCE of a second opinion.
    let c = cfg();
    let obs = Observations::default(); // no price series at all
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-17T12:00:00Z");

    assert!(r.explosiveness.is_none(), "no series means no test");
    assert!(
        r.caveats
            .iter()
            .any(|x| x.contains("EXPLOSIVENESS TEST") && x.contains("DID NOT RUN")),
        "a missing second method must be stated: {:?}",
        r.caveats
    );
}

#[test]
fn every_scored_value_is_commensurate_with_its_own_unit() {
    // A VALUE AND A UNIT THAT DISAGREE ABOUT SCALE IS A SILENT MISREADING WAITING TO HAPPEN.
    //
    // `circularity` shipped with `unit = "pct of the expressible rubric scale"` while `value`
    // carried the raw point total (230) — a number in POINTS labelled as a PERCENTAGE. Nothing
    // failed, because both fields are free-form; the report simply stated two things that
    // cannot both be true, and a consuming script would have taken it at face value.
    //
    // This asserts the property that was violated, for EVERY scored indicator: where a unit
    // claims a percentage, the value must actually lie on a percentage scale.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixture absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let r = bubble_watch::report::build(readings, &obs, &c, "2026-09-20T00:00:00Z");

    for i in &r.indicators {
        if !i.reading.is_available() {
            continue;
        }
        let (Some(v), Some(u)) = (i.reading.value(), i.reading.unit()) else {
            continue;
        };
        let unit_l = u.to_lowercase();
        let claims_pct = unit_l.contains("pct")
            || unit_l.contains("percent")
            || unit_l.trim_start().starts_with('%');
        if !claims_pct {
            continue;
        }
        // A UNIT CAN DESCRIBE A LEVEL OR A CHANGE, AND ONLY LEVELS ARE BOUNDED 0-100.
        // `breadth` legitimately reports -2.67 "pct" — the percentage CHANGE in the
        // equal-weight/cap-weight ratio, which is signed and can exceed 100 in principle. So
        // the assertion distinguishes the two: a bounded share must lie within 0-100, while a
        // signed change must simply NOT be a raw point total masquerading as a percentage.
        let is_change = unit_l.contains("change")
            || unit_l.contains("spread")
            || unit_l.contains("gap")
            || unit_l.contains("deviation")
            || unit_l.contains("vs ")
            || unit_l.contains("minus");
        if is_change {
            // A change is unbounded in sign but should still be of plausible magnitude: a
            // figure in the thousands would indicate points being passed off as percent.
            assert!(
                v.abs() <= 1000.0,
                "indicator '{}' has unit {:?} (a change) but value {} — that magnitude suggests \
                 the value is not on the scale the unit claims",
                i.id,
                u,
                v
            );
        } else {
            assert!(
                (0.0..=100.0).contains(&v),
                "indicator '{}' has unit {:?} but value {} — a bounded share must lie within \
                 0-100, so either the unit or the value is describing the wrong quantity",
                i.id,
                u,
                v
            );
        }
    }
}

#[test]
fn the_docs_do_not_contradict_the_build() {
    // DOCS THAT STATE NUMBERS DRIFT THE MOMENT THOSE NUMBERS CHANGE, AND NOTHING FAILS WHEN THEY DO.
    // This has now happened twice on this project — README claimed 234 tests and weight 146, then
    // 263 and twelve entries, after the build had moved past both. Each time the stale figure was
    // in a PUBLIC document describing the model's own weights, which is the worst place for it.
    //
    // The test reads what the docs CLAIM and compares against what the build IS, for the facts
    // that change most often: the test count is excluded (it changes on every test added, which
    // would make this test self-defeating), but the WEIGHT TOTAL and the VERIFIED EDGE COUNT are
    // both asserted, because those are substantive model facts a reader would rely on.
    use std::fs;
    let readme = fs::read_to_string("README.md").expect("README.md must exist");
    let spec = fs::read_to_string("SPEC.md").expect("SPEC.md must exist");

    // The rubric's entry count must match what SPEC claims.
    let n_entries = bubble_watch::circular_rubric::VERIFIED.len();
    let words = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve", "thirteen", "fourteen", "fifteen",
    ];
    let word = words.get(n_entries).copied().unwrap_or("many");
    let claimed = format!("{} verified", word);
    // Case-insensitive: the docs write "Thirteen verified" at the start of a sentence and
    // "thirteen" mid-sentence. Demanding one casing would break on a copy-edit that changed
    // nothing substantive, which is the kind of brittle assertion that trains people to edit
    // tests instead of reading failures.
    assert!(
        spec.to_lowercase().contains(&claimed),
        "SPEC must describe the verified edge count correctly: expected the phrase {:?} for {} \
         entries. If the count changed, update SPEC (and README's table).",
        claimed,
        n_entries
    );

    // The weight total must match what README claims.
    let c = cfg();
    let total = c.total_weight();
    assert!(
        readme.contains(&format!("Weight totals {}", total)),
        "README must state the real weight total ({}). Update it if weights changed.",
        total
    );

    // NEITHER DOCUMENT MAY STATE AN AGGREGATE EXPOSURE TOTAL.
    //
    // An earlier revision claimed a combined figure that was written without being derived, and
    // checking it showed no single total is honest: the set mixes dollar figures with percentage
    // shares, restates some commitments, includes third-party capital that sits on OTHER balance
    // sheets, and contains two REFUTATIONS that must contribute nothing. See SPEC 15.8.
    //
    // This asserts the rule survives future edits. It looks for the SHAPE of an aggregate claim —
    // a dollar figure immediately followed by a total-ish phrase — rather than an exact string,
    // so rewording the surrounding prose will not defeat it.
    for (name, doc) in [("README.md", &readme), ("SPEC.md", &spec)] {
        let lower = doc.to_lowercase();
        for phrase in [
            "of disclosed exposure",
            "combined exposure",
            "total exposure",
            "of exposure across",
        ] {
            if let Some(i) = lower.find(phrase) {
                let window = &lower[i.saturating_sub(60)..i];
                assert!(
                    !window.contains('$') || !window.chars().any(|c| c.is_ascii_digit()),
                    "{} appears to state an aggregate exposure total near {:?} — no single total \
                     is honest here (SPEC 15.8). State each figure with its source instead.",
                    name,
                    &doc[i.saturating_sub(60)..(i + phrase.len()).min(doc.len())]
                );
            }
        }
    }

    // And neither document may still call circularity a declared gap.
    for (name, doc) in [("README.md", &readme), ("SPEC.md", &spec)] {
        assert!(
            !doc.contains("`circularity` | 0 |"),
            "{} still lists circularity at weight 0 — it is scored now",
            name
        );
    }
}

#[test]
fn the_pre_v11_trend_defaults_match_the_shipped_config() {
    // `TrendCfg::defaults()` exists so a config written before v1.1 still loads and behaves
    // the same way, and its doc comment claims it matches the shipped `[trend]` block. Once
    // min_gap_days was changed from 1.0 to 0.9 in the shipped config, that claim was false:
    // an old config would have measured the elapsed-time guard differently from a current
    // one, silently. Assert the equality so the two cannot drift apart again.
    let c = cfg();
    let d = bubble_watch::config::TrendCfg::defaults();
    assert!(
        (c.trend.min_gap_days - d.min_gap_days).abs() < 1e-12,
        "shipped min_gap_days {} != defaults {}",
        c.trend.min_gap_days,
        d.min_gap_days
    );
    assert!(
        (c.trend.coverage_tolerance_pp - d.coverage_tolerance_pp).abs() < 1e-12,
        "shipped coverage_tolerance_pp {} != defaults {}",
        c.trend.coverage_tolerance_pp,
        d.coverage_tolerance_pp
    );
    assert!(
        (c.trend.flat_band - d.flat_band).abs() < 1e-12,
        "shipped flat_band {} != defaults {}",
        c.trend.flat_band,
        d.flat_band
    );
    assert_eq!(c.trend.sparkline_points, d.sparkline_points);
}

#[test]
fn a_fixture_backed_indicator_refuses_to_report_a_stale_value() {
    // The frontier premium reads a committed fixture, so it CANNOT update itself. Without
    // the staleness guard it would report the same gap forever while retrieved_at showed
    // today's date. This asserts the guard is wired, and — because the fixture ships with
    // the repo — that it is not already stale, which would silently drop an indicator from
    // every future run.
    let Some(p) = bubble_watch::frontier::load() else {
        eprintln!("SKIP: arena fixture absent");
        return;
    };
    let last = p.last().expect("series non-empty");
    let today = bubble_watch::now_date();
    let age = bubble_watch::sources::edgar::days_between(&last.date, &today);
    assert!(
        age <= 120,
        "the committed LMArena fixture is {} days old (latest {}). Re-extract it, or the \
         frontier premium will be reported as a gap on every run from now on.",
        age,
        last.date
    );
}

#[test]
fn a_genuinely_empty_archive_still_says_so_when_history_was_consulted() {
    // The complement of the test below: when the caller DID open the archive and it is
    // genuinely empty, the first-run explanation is the correct one. The fix must not
    // have replaced an untrue message with a vague one.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixture absent");
        return;
    };
    let c = cfg();
    let ctx = bubble_watch::indicators::Ctx { obs: &obs, cfg: &c };
    let readings = bubble_watch::indicators::evaluate_all(&ctx);

    let r = bubble_watch::report::build_with_history(
        readings,
        &obs,
        &c,
        "2026-09-19T00:00:00Z",
        &[],
        false,
        true,
    );
    let reason = r.trend.reason.expect("reason present");
    assert!(
        reason.contains("No previous run is recorded yet"),
        "a consulted-but-empty archive should get the first-run wording: {}",
        reason
    );
}

#[test]
fn a_command_that_ignores_history_never_claims_history_is_empty() {
    // `score` and `explain` never read the archive, but they render the same report
    // structure as `report`. With an empty archive slice the report used to say "No
    // previous run is recorded yet ... run the tool again on a later day and a comparison
    // will appear." Both halves were false in that case: previous runs DID exist, and
    // re-running the command could never produce a comparison because it does not open
    // the archive at all.
    //
    // The property: a report built without consulting history must describe the COMMAND,
    // not make a claim about the archive.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixture absent");
        return;
    };
    let c = cfg();
    let ctx = bubble_watch::indicators::Ctx { obs: &obs, cfg: &c };
    let readings = bubble_watch::indicators::evaluate_all(&ctx);

    let ignored = bubble_watch::report::build(readings.clone(), &obs, &c, "2026-09-19T00:00:00Z");
    let reason = ignored
        .trend
        .reason
        .expect("a reason must be present when no delta is computed");

    assert!(
        !reason.contains("No previous run is recorded yet"),
        "must not assert the archive is empty when the archive was never opened: {}",
        reason
    );
    assert!(
        !reason.contains("Run the tool again on a later day"),
        "must not promise a comparison that this command can never produce: {}",
        reason
    );
    assert!(
        reason.contains("does not consult the run archive"),
        "must say why, and it is a property of the command: {}",
        reason
    );
    // And the delta must genuinely be absent, not a zero.
    assert!(ignored.trend.delta.is_none());
}

#[test]
fn history_never_changes_the_composite() {
    // THE load-bearing test for v1.1. The trend is derived FROM the composite,
    // so if it could also feed back into the composite, the score would partly
    // be a function of its own past — and a published, audited number would
    // silently change. The composite must be byte-identical with and without
    // history.
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);

    let without = bubble_watch::report::build(readings.clone(), &obs, &c, "2026-09-17T12:00:00Z");

    // Match the baseline's coverage to this run's actual coverage, so the test
    // exercises a COMPARABLE baseline. (On the committed fixtures coverage is
    // not 100%, because FRED is absent — which is itself a useful reminder that
    // a coverage-matched baseline is a real constraint, not a formality.)
    let archive = vec![bubble_watch::model::TrendPoint {
        date: "2026-08-01".into(),
        generated_at: "2026-08-01T12:00:00Z".into(),
        composite: 90.0, // deliberately extreme: it must not drag the score
        coverage: without.coverage,
        phase: "critical".into(),
        methodology_version: c.meta.schema_version.clone(),
        stresses: std::collections::BTreeMap::new(),
    }];
    let with = bubble_watch::report::build_with_history(
        readings,
        &obs,
        &c,
        "2026-09-17T12:00:00Z",
        &archive,
        true,
        true,
    );

    assert_eq!(
        without.composite, with.composite,
        "history must never move the composite"
    );
    assert_eq!(without.coverage, with.coverage);
    assert_eq!(without.phase, with.phase);
    // Per-indicator stresses must be untouched too.
    for (a, b) in without.indicators.iter().zip(with.indicators.iter()) {
        assert_eq!(a.reading, b.reading, "indicator {} was altered", a.id);
    }
    // The extreme baseline must still be reported as a delta, not scored into
    // the level.
    let d = with.trend.delta.expect("an eligible baseline was supplied");
    assert!(
        d.composite_delta < 0.0,
        "delta is a comparison, not a score"
    );
}

#[test]
fn a_delta_is_never_emitted_without_elapsed_days_or_matching_coverage() {
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);

    // Baseline with materially different coverage: must be refused.
    let bad = vec![bubble_watch::model::TrendPoint {
        date: "2026-08-01".into(),
        generated_at: "2026-08-01T12:00:00Z".into(),
        composite: 40.0,
        coverage: 0.50,
        phase: "mid".into(),
        methodology_version: c.meta.schema_version.clone(),
        stresses: std::collections::BTreeMap::new(),
    }];
    let r = bubble_watch::report::build_with_history(
        readings.clone(),
        &obs,
        &c,
        "2026-09-17T12:00:00Z",
        &bad,
        true,
        true,
    );
    assert!(
        r.trend.delta.is_none(),
        "a coverage-mismatched baseline must never produce a delta"
    );
    assert!(
        r.caveats
            .iter()
            .any(|x| x.contains("DIRECTION OF TRAVEL NOT REPORTED")),
        "the refusal must be disclosed in the caveats"
    );

    // Every emitted delta must carry a positive elapsed time.
    let good = vec![bubble_watch::model::TrendPoint {
        date: "2026-08-01".into(),
        generated_at: "2026-08-01T12:00:00Z".into(),
        composite: 40.0,
        coverage: r.coverage,
        phase: "mid".into(),
        methodology_version: c.meta.schema_version.clone(),
        stresses: std::collections::BTreeMap::new(),
    }];
    let r2 = bubble_watch::report::build_with_history(
        readings,
        &obs,
        &c,
        "2026-09-17T12:00:00Z",
        &good,
        true,
        true,
    );
    if let Some(d) = &r2.trend.delta {
        assert!(
            d.elapsed_days > 0.0,
            "a delta needs elapsed time to mean anything"
        );
    }
}

#[test]
fn the_html_trend_card_needs_no_javascript_and_loads_nothing() {
    let Some(obs) = fixture_obs() else {
        eprintln!("SKIP: fixtures absent");
        return;
    };
    let c = cfg();
    let ctx = Ctx { obs: &obs, cfg: &c };
    let readings = indicators::evaluate_all(&ctx);
    let archive = vec![bubble_watch::model::TrendPoint {
        date: "2026-08-01".into(),
        generated_at: "2026-08-01T12:00:00Z".into(),
        composite: 28.0,
        coverage: indicators::evaluate_all(&ctx)
            .iter()
            .filter(|r| r.reading.is_available() && r.weight > 0.0)
            .map(|r| r.weight)
            .sum::<f64>()
            / c.total_weight(),
        phase: "early".into(),
        methodology_version: c.meta.schema_version.clone(),
        stresses: std::collections::BTreeMap::new(),
    }];
    let r = bubble_watch::report::build_with_history(
        readings,
        &obs,
        &c,
        "2026-09-17T12:00:00Z",
        &archive,
        true,
        true,
    );
    let h = bubble_watch::report::html::render(&r);
    assert!(h.contains("Direction of travel"));
    assert!(!h.contains("<script"), "must still need no JavaScript");
    assert!(
        !h.contains("src=\"http") && !h.contains("href=\"http"),
        "must still load nothing external"
    );
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
