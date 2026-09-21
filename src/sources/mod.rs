//! Source registry: the only module allowed to touch the network.

pub mod census;
pub mod circularity;
pub mod edgar;
pub mod eia;
pub mod federalregister;
pub mod fred;
pub mod fulltext;
pub mod lbnl;
pub mod nport;
pub mod openrouter;
pub mod yahoo;
pub mod z1;

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

    // Fed Z.1 private-credit lending. One 8MB archive yields both series, so it
    // is fetched once and both columns are parsed from it.
    match z1::private_credit(f, z1::SERIES_ALL_SECTORS, "all sectors private credit") {
        Ok(series) => {
            obs.yahoo.insert("PRIVATE_CREDIT_ALL".to_string(), series);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "fed-z1".into(),
            endpoint: format!("{} :: F4.4 {}", z1::Z1_URL, z1::SERIES_ALL_SECTORS),
            reason: e,
        }),
    }

    // Census C30 data-center construction. The only PHYSICAL, financial-market-
    // independent series in the model: dollars actually spent on steel and concrete,
    // from the government's own construction survey.
    match census::data_center(f) {
        Ok(series) => {
            obs.yahoo
                .insert("DATACENTER_CONSTRUCTION".to_string(), series);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "census-c30".into(),
            endpoint: format!("{} :: '{}'", census::URL, census::LINE_ITEM),
            reason: e,
        }),
    }

    // EDGAR AI-mention census: a genuine census of how many filings discuss AI,
    // used as a hype measure with a long baseline. Two years are fetched so the
    // indicator can form a growth rate without hardcoding history.
    {
        let this_year = crate::now_date()[0..4].parse::<i32>().unwrap_or(2026);
        let mut census: Vec<(String, u32, u64)> = Vec::new();
        for year in [this_year - 1, this_year] {
            match fulltext::phrase_census(f, "artificial intelligence", "10-K", year) {
                Ok(n) => census.push(("10-K".to_string(), year as u32, n)),
                Err(e) => obs.failures.push(SourceFailure {
                    source: "sec-edgar-fulltext".into(),
                    endpoint: format!("10-K AI-mention census {}", year),
                    reason: e,
                }),
            }
        }
        if !census.is_empty() {
            obs.ai_census = census;
            obs.ai_census_provenance = Some(crate::model::Provenance {
                source: "sec-edgar-fulltext".into(),
                endpoint: "efts.sec.gov/LATEST/search-index forms=10-K phrase=\"artificial \
                           intelligence\""
                    .into(),
                as_of: crate::now_date(),
                retrieved_at: crate::now_iso8601(),
            });
        }
    }

    // EIA-860M planned vs cancelled generating capacity. The physical constraint on
    // the buildout is firm electricity; this measures announced generation that was
    // then abandoned, which turns before the spending does.
    match eia::planned_vs_cancelled(f) {
        Ok((planned, cancelled, ratio)) => {
            obs.yahoo.insert("EIA_PLANNED".to_string(), planned);
            obs.yahoo.insert("EIA_CANCELLED".to_string(), cancelled);
            obs.eia_ratio = Some(ratio);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "eia-860m".into(),
            endpoint: format!("{} :: Planned / Canceled or Postponed", eia::URL),
            reason: e,
        }),
    }

    // Rest-of-world holdings of US corporate equities — the previously declared
    // "unmeasurable" blind spot. Same archive, different table; the Fetcher cache
    // means the 8MB download is reused rather than repeated.
    match z1::series_from_m3(f, z1::SERIES_FOREIGN_EQUITY) {
        Ok(series) => {
            obs.yahoo.insert("FOREIGN_US_EQUITY".to_string(), series);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "fed-z1".into(),
            endpoint: format!("{} :: M3s {}", z1::Z1_URL, z1::SERIES_FOREIGN_EQUITY),
            reason: e,
        }),
    }

    // Peer filers for the depreciation test only. Fetched into a SEPARATE map so
    // they cannot alter the five indicators that iterate `obs.edgar`.
    for (ticker, cik, name) in edgar::ACCOUNTING_PEERS {
        let cf = edgar::company(f, ticker, cik, name);
        obs.edgar_peers.insert(ticker.to_string(), cf);
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

    // ---- demand-side sources (added 2026-09-20) --------------------------
    //
    // These are the model's first measurements of whether AI compute is USED,
    // where the demand sits, and what constrains it. Each degrades to a gap.

    match openrouter::fetch_daily(f) {
        Ok(days) => {
            // Keep ONLY the most recent COMPLETE day. The feed back-fills
            // incompletely: on 2026-09-20 six of seven dates carried 1-10 rows
            // against 544 for the one complete day, so storing the newest date
            // unconditionally would read a stub as a demand collapse.
            let floor = days.iter().map(|d| d.model_count).max().unwrap_or(0) / 2;
            if let Some(latest) = openrouter::latest_complete_day(&days, floor.max(1)) {
                obs.openrouter_latest_day = Some(latest.clone());
                obs.openrouter_provenance = Some(crate::model::Provenance {
                    source: "openrouter".into(),
                    endpoint: openrouter::RANKINGS_MODELS.to_string(),
                    as_of: latest.date.clone(),
                    retrieved_at: obs.retrieved_at.clone(),
                });
            } else {
                obs.failures.push(SourceFailure {
                    source: "openrouter".into(),
                    endpoint: openrouter::RANKINGS_MODELS.into(),
                    reason: "no COMPLETE day in the rankings window — every date had fewer \
                             than half the maximum row count, so none is safe to read"
                        .into(),
                });
            }
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "openrouter".into(),
            endpoint: openrouter::RANKINGS_MODELS.into(),
            reason: e,
        }),
    }

    match openrouter::fetch_weekly(f) {
        Ok(weeks) => {
            // The newest week is routinely PARTIAL (measured 5.8e10 against
            // 1.29e14, a ~2,200x artefact). Without this the growth calculation
            // reported 0.0x over a year in which usage grew ~24x.
            obs.openrouter_weeks = openrouter::drop_partial_trailing_week(&weeks, 10.0);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "openrouter".into(),
            endpoint: openrouter::RANKINGS_CHART.into(),
            reason: e,
        }),
    }

    // US export-control activity. This feeds a FALSIFIER, not the composite:
    // two BIS rule documents in twelve months is a QUIET policy period, and the
    // most recent rule EASES access for one jurisdiction. Counting that as stress
    // would be counting a non-event.
    match federalregister::bis_rule_count(f) {
        Ok((count, latest)) => {
            obs.bis_year_count = Some(count);
            match latest {
                Some(d) => {
                    obs.bis_latest_date = Some(d.clone());
                    obs.bis_provenance = Some(crate::model::Provenance {
                        source: "federalregister".into(),
                        endpoint: federalregister::QUERY_URL.into(),
                        as_of: d,
                        retrieved_at: obs.retrieved_at.clone(),
                    });
                }
                // A count of zero has no newest date, which is expected rather
                // than a failure: the count itself is the measurement.
                None => {}
            }
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "federalregister".into(),
            endpoint: federalregister::QUERY_URL.into(),
            reason: e,
        }),
    }

    // LBNL interconnection queue. NOTE: this host REQUIRES a browser User-Agent;
    // without one the 15.5MB workbook returns 403, which makes the source look
    // dead. The landing page is Cloudflare-blocked regardless.
    match lbnl::fetch(f) {
        Ok(years) => {
            obs.lbnl_provenance = Some(crate::model::Provenance {
                source: "lbnl".into(),
                endpoint: lbnl::URL.into(),
                as_of: years.last().map(|y| y.year.to_string()).unwrap_or_default(),
                retrieved_at: obs.retrieved_at.clone(),
            });
            obs.lbnl_years = Some(years);
        }
        Err(e) => obs.failures.push(SourceFailure {
            source: "lbnl".into(),
            endpoint: lbnl::URL.into(),
            reason: e,
        }),
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
        .filter(|k| {
            !k.starts_with("S1_REGISTRATIONS")
                && !k.starts_with("PRIVATE_CREDIT")
                && !k.starts_with("FOREIGN_US_EQUITY")
                && !k.starts_with("DATACENTER")
                && !k.starts_with("EIA_")
        })
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
    let z1_ok = obs
        .yahoo
        .keys()
        .filter(|k| k.starts_with("PRIVATE_CREDIT") || k.starts_with("FOREIGN_US_EQUITY"))
        .count();
    let z1_fail = obs.failures.iter().filter(|x| x.source == "fed-z1").count();
    out.push(SourceHealth {
        name: "fed-z1".into(),
        status: status_for(z1_ok, z1_fail),
        ok_count: z1_ok,
        failed_count: z1_fail,
        detail: "Fed Z.1 F4.4 private-credit lending (8MB archive)".into(),
    });

    let c30_ok = obs
        .yahoo
        .keys()
        .filter(|k| k.starts_with("DATACENTER"))
        .count();
    let c30_fail = obs
        .failures
        .iter()
        .filter(|x| x.source == "census-c30")
        .count();
    let eia_ok = obs.yahoo.keys().filter(|k| k.starts_with("EIA_")).count();
    let eia_fail = obs
        .failures
        .iter()
        .filter(|x| x.source == "eia-860m")
        .count();
    out.push(SourceHealth {
        name: "eia-860m".into(),
        status: status_for(eia_ok, eia_fail),
        ok_count: eia_ok,
        failed_count: eia_fail,
        detail: "EIA-860M planned vs cancelled generating capacity (xlsx)".into(),
    });

    out.push(SourceHealth {
        name: "census-c30".into(),
        status: status_for(c30_ok, c30_fail),
        ok_count: c30_ok,
        failed_count: c30_fail,
        detail: "Census C30 data-center construction spending (monthly)".into(),
    });

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
