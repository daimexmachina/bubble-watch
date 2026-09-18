//! Source registry: the only module allowed to touch the network.

pub mod edgar;
pub mod fred;
pub mod fulltext;
pub mod yahoo;

use crate::http::Fetcher;
use crate::model::{Observations, Series, SourceFailure};
use std::collections::BTreeMap;

/// Every market series the model may want, with the range/interval each needs.
///
/// `key` is what the indicator code looks up; `symbol` is what Yahoo wants
/// (percent-encoded for indices).
pub struct YahooSpec {
    pub key: &'static str,
    pub symbol: &'static str,
    pub range: &'static str,
    pub interval: &'static str,
}

pub const YAHOO_SPECS: &[YahooSpec] = &[
    YahooSpec {
        key: "SPX_5y_mo",
        symbol: "%5EGSPC",
        range: "5y",
        interval: "1mo",
    },
    // Long monthly history for the GSADF explosiveness test. The test needs a
    // rolling window with enough observations to be meaningful, and a 5-year
    // series gives only ~40 usable windows. Max range gives decades, which is
    // what makes the test comparable to the published literature.
    YahooSpec {
        key: "SPX_MAX_mo",
        symbol: "%5EGSPC",
        range: "max",
        interval: "1mo",
    },
    YahooSpec {
        key: "SPX_1y_d",
        symbol: "%5EGSPC",
        range: "1y",
        interval: "1d",
    },
    YahooSpec {
        key: "RSP_1y_d",
        symbol: "RSP",
        range: "1y",
        interval: "1d",
    },
    YahooSpec {
        key: "SPY_1y_d",
        symbol: "SPY",
        range: "1y",
        interval: "1d",
    },
    YahooSpec {
        key: "VIX_1y_d",
        symbol: "%5EVIX",
        range: "1y",
        interval: "1d",
    },
    YahooSpec {
        key: "TNX_1y_d",
        symbol: "%5ETNX",
        range: "1y",
        interval: "1d",
    },
];

/// FRED series the model may want. All optional by construction.
pub const FRED_SPECS: &[&str] = &[
    "BAMLH0A0HYM2",
    "BAMLC0A0CM",
    "DGS10",
    // Moody's Baa yield less the 10-year Treasury. Added because it carries
    // history from 1986 whereas the ICE BofA spreads above only start in 2023,
    // so the credit indicators otherwise have no 2000 or 2008 episode to
    // compare against.
    "BAA10Y",
];

/// Fetch everything. Partial failure is normal and expected: the returned
/// `Observations` records what succeeded and what did not, and scoring proceeds
/// on whatever is real.
pub fn fetch_all(f: &Fetcher, offline: bool) -> Observations {
    let mut obs = Observations {
        retrieved_at: crate::now_iso8601(),
        ..Default::default()
    };

    for spec in YAHOO_SPECS {
        match yahoo::chart(f, spec.symbol, spec.range, spec.interval) {
            Ok(s) => {
                obs.yahoo.insert(spec.key.to_string(), s);
            }
            Err(e) => obs.failures.push(SourceFailure {
                source: "yahoo".into(),
                endpoint: format!("{} ({})", spec.symbol, spec.key),
                reason: e,
            }),
        }
    }

    for id in FRED_SPECS {
        match fred::series(f, id) {
            Ok((s, transport)) => {
                obs.fred.insert(id.to_string(), s);
                obs.fred_transports.insert(id.to_string(), transport);
            }
            Err(e) => obs.failures.push(SourceFailure {
                source: "fred".into(),
                endpoint: id.to_string(),
                reason: e,
            }),
        }
    }

    // Primary-market filing volume: the broad IPO wave that no per-company
    // concept can see. Stored under its own key so it can never be mistaken for
    // a ticker series.
    match fulltext::registrations(f, &crate::now_date(), 365) {
        Ok(series) => {
            obs.yahoo.insert("S1_REGISTRATIONS_1Y".to_string(), series);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "sec-edgar-fulltext".into(),
            endpoint: "S-1 registration count, trailing 1 year".into(),
            reason: e,
        }),
    }

    for (ticker, cik, name) in edgar::COHORT {
        let cf = edgar::company(f, ticker, cik, name);
        // Record a failure when a filer yielded nothing at all, so the gap is
        // visible rather than looking like a company with no capex.
        if cf.capex.is_empty() && cf.cfo.is_empty() && cf.revenue.is_empty() {
            obs.failures.push(SourceFailure {
                source: "edgar".into(),
                endpoint: format!("CIK{} ({})", cik, ticker),
                reason: "no XBRL facts retrieved".into(),
            });
            // Deliberately DO NOT insert an empty container: presence in this
            // map must mean real facts were retrieved, otherwise a downstream
            // reader cannot tell "no data" from "no filer".
            continue;
        }
        obs.edgar.insert(ticker.to_string(), cf);
    }

    let _ = offline;
    obs
}

/// Summarize health per source for the report, so a reader can see at a glance
/// whether the number rests on everything or on a degraded subset.
pub fn source_health(obs: &Observations) -> Vec<crate::model::SourceHealth> {
    use crate::model::SourceHealth;
    let mut out = Vec::new();

    // The S-1 count lives in the same map but is NOT a yahoo series, so exclude
    // it here — otherwise it would inflate yahoo's retrieved count and hide a
    // real yahoo failure behind it.
    let yahoo_ok = obs
        .yahoo
        .keys()
        .filter(|k| !k.starts_with("S1_REGISTRATIONS"))
        .count();
    let yahoo_fail = obs.failures.iter().filter(|x| x.source == "yahoo").count();
    out.push(SourceHealth {
        name: "yahoo".into(),
        status: status_for(yahoo_ok, yahoo_fail),
        ok_count: yahoo_ok,
        failed_count: yahoo_fail,
        detail: format!(
            "{} of {} chart series retrieved",
            yahoo_ok,
            YAHOO_SPECS.len()
        ),
    });

    let edgar_ok = obs.edgar.len();
    let edgar_fail = obs.failures.iter().filter(|x| x.source == "edgar").count();
    out.push(SourceHealth {
        name: "sec-edgar".into(),
        status: status_for(edgar_ok, edgar_fail),
        ok_count: edgar_ok,
        failed_count: edgar_fail,
        detail: format!(
            "{} of {} filers with usable XBRL facts",
            edgar_ok,
            edgar::COHORT.len()
        ),
    });

    let fred_ok = obs.fred.len();
    let fred_fail = obs.failures.iter().filter(|x| x.source == "fred").count();
    let keyed = fred::key_configured();
    out.push(SourceHealth {
        name: "fred".into(),
        status: if fred_ok == 0 {
            "unavailable".to_string()
        } else {
            status_for(fred_ok, fred_fail)
        },
        ok_count: fred_ok,
        failed_count: fred_fail,
        detail: if fred_ok == 0 {
            format!(
                "OPTIONAL source: {} of {} series retrieved. {} Credit and macro indicators \
                 are reported as gaps, and the composite renormalizes over what remains.",
                fred_ok,
                FRED_SPECS.len(),
                if keyed {
                    "Key is configured, but requests are still failing."
                } else {
                    "No FRED_API_KEY set — the anonymous CSV endpoint is flaky from this host \
                     (measured 0/9) while the keyed API host answered 5/5, so a key is the fix."
                }
            )
        } else {
            let transports: Vec<&str> = obs.fred_transports.values().map(|t| t.as_str()).collect();
            format!(
                "{} of {} series retrieved via {}{}",
                fred_ok,
                FRED_SPECS.len(),
                transports.join(", "),
                if keyed { "" } else { " (no FRED_API_KEY set)" }
            )
        },
    });

    let ft_ok = obs
        .yahoo
        .keys()
        .filter(|k| k.starts_with("S1_REGISTRATIONS"))
        .count();
    let ft_fail = obs
        .failures
        .iter()
        .filter(|x| x.source == "sec-edgar-fulltext")
        .count();
    out.push(SourceHealth {
        name: "sec-edgar-fulltext".into(),
        status: status_for(ft_ok, ft_fail),
        ok_count: ft_ok,
        failed_count: ft_fail,
        detail: "S-1 registration count, trailing 1 year (primary-market supply)".into(),
    });

    out
}

fn status_for(ok: usize, failed: usize) -> String {
    if ok == 0 {
        "unavailable".into()
    } else if failed > 0 {
        "degraded".into()
    } else {
        "ok".into()
    }
}

/// Helper for indicators that need a series and must report a clean gap if it
/// is missing.
pub fn need<'a>(m: &'a BTreeMap<String, Series>, key: &str) -> Result<&'a Series, String> {
    m.get(key)
        .ok_or_else(|| format!("series '{}' unavailable from source", key))
}
