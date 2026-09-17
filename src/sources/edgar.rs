//! SEC EDGAR XBRL `companyconcept` client.
//!
//! Endpoint: https://data.sec.gov/api/xbrl/companyconcept/CIK{10-digit}/{taxonomy}/{tag}.json
//!
//! Requirements and hazards established by probing the live host:
//!
//!   1. EDGAR *requires* a descriptive User-Agent. Without it requests are
//!      rejected. (`UA_EDGAR` supplies one.)
//!   2. Rate limit is nominally 10 requests/second; we stay well under it.
//!   3. THE TRAP: different companies report the same economic quantity under
//!      different tags, and a naive "first tag that responds" fallback silently
//!      returns *stale* data. During probing, that approach produced MSFT revenue
//!      from 2010 and META revenue from 2018. The resolver below therefore
//!      fetches every candidate tag and selects the one whose most recent fact
//!      end-date is newest — then keeps that tag's full series.
//!
//! We use `companyconcept` per tag rather than `companyfacts` because a single
//! `companyfacts` payload for a mega-cap runs to tens of megabytes, which is
//! needlessly heavy on this host.

use crate::http::{FetchError, Fetcher, UA_EDGAR};
use crate::model::{CompanyFacts, EdgarFact};

/// Retry budget for SEC endpoints. EDGAR's rate limit is nominally 10 req/s; two
/// attempts is enough for a transient blip without stalling the run.
pub const EDGAR_RETRIES: u32 = 2;

pub const COHORT: &[(&str, &str, &str)] = &[
    ("MSFT", "0000789019", "Microsoft"),
    ("GOOGL", "0001652044", "Alphabet"),
    ("AMZN", "0001018724", "Amazon"),
    ("META", "0001326801", "Meta"),
    ("ORCL", "0001341439", "Oracle"),
];

/// Tag candidates by economic quantity, most-preferred first.
pub const TAGS_CAPEX: &[&str] = &[
    "PaymentsToAcquirePropertyPlantAndEquipment",
    "PaymentsToAcquireProductiveAssets",
];

pub const TAGS_CFO: &[&str] = &[
    "NetCashProvidedByUsedInOperatingActivities",
    "NetCashProvidedByUsedInOperatingActivitiesContinuingOperations",
];

pub const TAGS_REVENUE: &[&str] = &[
    "RevenueFromContractWithCustomerExcludingAssessedTax",
    "RevenueFromContractWithCustomerIncludingAssessedTax",
    "Revenues",
];

pub const TAGS_DEBT: &[&str] = &["LongTermDebtNoncurrent", "LongTermDebt"];

/// Cash returned to shareholders by buying back stock.
///
/// Verified 2026-09-17 from this host: returns HTTP 200 for all five scored
/// cohort members, so the buyback leg needs no proxy.
pub const TAGS_BUYBACK: &[&str] = &["PaymentsForRepurchaseOfCommonStock"];

/// Cash raised by issuing common stock.
///
/// Note the real absence: AMZN does not report this concept under any candidate
/// tag (it does not issue equity), so its issuance leg is genuinely unavailable.
/// That is handled as a per-filer gap rather than filled with zero — see
/// `CompanyFacts` and the `issuance` indicator.
pub const TAGS_ISSUANCE: &[&str] = &["ProceedsFromIssuanceOfCommonStock"];

/// Share-count tags.
///
/// HAZARD, verified against the live API: GOOGL and META do **not** publish
/// `dei:EntityCommonStockSharesOutstanding` (it 404s for them), while MSFT,
/// AMZN, ORCL and NVDA do. Alphabet and Meta instead report the
/// `us-gaap:CommonStockSharesOutstanding` / `CommonStockSharesIssued` concepts.
/// Resolving only the `dei` tag silently dropped two of five filers from the
/// issuance indicator — a quiet, invisible hole in the data. Both taxonomies are
/// therefore resolved together, and the most-recent-end rule picks the winner.
pub const TAGS_SHARES_DEI: &[&str] = &["EntityCommonStockSharesOutstanding"];
pub const TAGS_SHARES_USGAAP: &[&str] = &[
    "CommonStockSharesOutstanding",
    "CommonStockSharesIssued",
    "WeightedAverageNumberOfDilutedSharesOutstanding",
];

/// Fetch the facts for one tag. Returns `Ok(None)` when the tag does not exist
/// for this filer (a normal outcome, not an error).
pub fn concept(
    f: &Fetcher,
    cik: &str,
    taxonomy: &str,
    tag: &str,
    unit: &str,
) -> Result<Option<Vec<EdgarFact>>, String> {
    let url = format!(
        "https://data.sec.gov/api/xbrl/companyconcept/CIK{}/{}/{}.json",
        cik, taxonomy, tag
    );
    let body = match f.get_raw(&url, UA_EDGAR, EDGAR_RETRIES) {
        Ok(b) => b,
        // A missing concept is a normal, deterministic absence — not a failure.
        Err(FetchError::NotFound) => return Ok(None),
        Err(e) => return Err(e.message()),
    };
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("bad JSON {}: {}", url, e))?;

    let Some(arr) = v
        .get("units")
        .and_then(|u| u.get(unit))
        .and_then(|a| a.as_array())
    else {
        return Ok(None);
    };

    let mut facts = Vec::new();
    for item in arr {
        let (Some(start), Some(end)) = (
            item.get("start").and_then(|s| s.as_str()),
            item.get("end").and_then(|s| s.as_str()),
        ) else {
            continue; // instant facts (e.g. shares at a date) handled separately
        };
        let Some(val) = item.get("val").and_then(|v| v.as_f64()) else {
            continue;
        };
        let days = days_between(start, end);
        facts.push(EdgarFact {
            tag: tag.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            val,
            form: item
                .get("form")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            filed: item
                .get("filed")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            days,
        });
    }
    if facts.is_empty() {
        return Ok(None);
    }
    facts.sort_by(|a, b| a.end.cmp(&b.end));
    Ok(Some(facts))
}

/// Instant facts (no `start`), used for point-in-time balances such as shares
/// outstanding or debt at a period end.
pub fn concept_instant(
    f: &Fetcher,
    cik: &str,
    taxonomy: &str,
    tag: &str,
    unit: &str,
) -> Result<Option<Vec<EdgarFact>>, String> {
    let url = format!(
        "https://data.sec.gov/api/xbrl/companyconcept/CIK{}/{}/{}.json",
        cik, taxonomy, tag
    );
    let body = match f.get_raw(&url, UA_EDGAR, EDGAR_RETRIES) {
        Ok(b) => b,
        Err(FetchError::NotFound) => return Ok(None),
        Err(e) => return Err(e.message()),
    };
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("bad JSON {}: {}", url, e))?;
    let Some(arr) = v
        .get("units")
        .and_then(|u| u.get(unit))
        .and_then(|a| a.as_array())
    else {
        return Ok(None);
    };
    let mut facts = Vec::new();
    for item in arr {
        let Some(end) = item.get("end").and_then(|s| s.as_str()) else {
            continue;
        };
        let Some(val) = item.get("val").and_then(|v| v.as_f64()) else {
            continue;
        };
        facts.push(EdgarFact {
            tag: tag.to_string(),
            start: end.to_string(),
            end: end.to_string(),
            val,
            form: item
                .get("form")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .into(),
            filed: item
                .get("filed")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .into(),
            days: 0,
        });
    }
    if facts.is_empty() {
        return Ok(None);
    }
    facts.sort_by(|a, b| a.end.cmp(&b.end));
    Ok(Some(facts))
}

/// Resolve a quantity across candidate tags by **most recent fact end-date**.
///
/// This is the fix for the stale-tag trap described in the module comment: a
/// company that switched tags years ago keeps publishing the old tag's history,
/// so "first tag that answers" returns ancient data.
///
/// Candidates may span multiple taxonomies (see `TAGS_SHARES_*`), so each entry
/// is a `(taxonomy, tag)` pair.
pub fn resolve(
    f: &Fetcher,
    cik: &str,
    tags: &[(&str, &str)],
    unit: &str,
    instant: bool,
) -> Result<Option<Vec<EdgarFact>>, String> {
    let mut best: Option<Vec<EdgarFact>> = None;
    let mut errors: Vec<String> = Vec::new();
    let mut attempted = 0usize;

    for (taxonomy, tag) in tags {
        attempted += 1;
        let got = if instant {
            concept_instant(f, cik, taxonomy, tag, unit)
        } else {
            concept(f, cik, taxonomy, tag, unit)
        };
        match got {
            Ok(Some(facts)) => {
                let newest = facts
                    .iter()
                    .map(|x| x.end.clone())
                    .max()
                    .unwrap_or_default();
                let better = match &best {
                    None => true,
                    Some(cur) => {
                        let cur_newest =
                            cur.iter().map(|x| x.end.clone()).max().unwrap_or_default();
                        newest > cur_newest
                    }
                };
                if better {
                    best = Some(facts);
                }
            }
            Ok(None) => {}
            Err(e) => errors.push(format!("{}:{}: {}", taxonomy, tag, e)),
        }
    }

    if best.is_none() && !errors.is_empty() && errors.len() == attempted {
        return Err(errors.join("; "));
    }
    Ok(best)
}

/// Resolve share counts across BOTH the `dei` and `us-gaap` taxonomies.
///
/// Verified against the live API: `dei:EntityCommonStockSharesOutstanding`
/// exists for MSFT, AMZN, ORCL and NVDA but 404s for GOOGL and META, which
/// report `us-gaap:CommonStockSharesOutstanding` instead. Resolving only the dei
/// tag silently dropped two of five filers. The most-recent-end rule picks the
/// better candidate whichever taxonomy supplies it.
fn resolve_shares(f: &Fetcher, cik: &str) -> Result<Option<Vec<EdgarFact>>, String> {
    let mut inst: Vec<(&str, &str)> = TAGS_SHARES_DEI.iter().map(|t| ("dei", *t)).collect();
    inst.extend(TAGS_SHARES_USGAAP[..2].iter().map(|t| ("us-gaap", *t)));

    match resolve(f, cik, &inst, "shares", true) {
        Ok(Some(v)) => Ok(Some(v)),
        Ok(None) => {
            // Last resort: diluted weighted-average share count is a duration
            // fact, so it needs the period-series path rather than the instant one.
            let dur: Vec<(&str, &str)> = TAGS_SHARES_USGAAP[2..]
                .iter()
                .map(|t| ("us-gaap", *t))
                .collect();
            resolve(f, cik, &dur, "shares", false)
        }
        Err(e) => Err(e),
    }
}

/// Gather everything the cohort indicators need for one filer.
pub fn company(f: &Fetcher, ticker: &str, cik: &str, name: &str) -> CompanyFacts {
    let mut cf = CompanyFacts {
        cik: cik.to_string(),
        name: name.to_string(),
        ..Default::default()
    };

    if let Ok(Some(v)) = resolve(f, cik, &usgaap(TAGS_CAPEX), "USD", false) {
        cf.capex = v;
    }
    if let Ok(Some(v)) = resolve(f, cik, &usgaap(TAGS_CFO), "USD", false) {
        cf.cfo = v;
    }
    if let Ok(Some(v)) = resolve(f, cik, &usgaap(TAGS_REVENUE), "USD", false) {
        cf.revenue = v;
    }
    if let Ok(Some(v)) = resolve_shares(f, cik) {
        cf.shares = v;
    }
    if let Ok(Some(v)) = resolve(f, cik, &usgaap(TAGS_DEBT), "USD", true) {
        cf.debt = v;
    }
    if let Ok(Some(v)) = resolve(f, cik, &usgaap(TAGS_BUYBACK), "USD", false) {
        cf.buyback = v;
    }
    if let Ok(Some(v)) = resolve(f, cik, &usgaap(TAGS_ISSUANCE), "USD", false) {
        cf.issuance = v;
    }

    let _ = ticker;
    cf
}

/// Map bare tag names onto the us-gaap taxonomy.
///
/// The taxonomy string is a literal, so the result borrows nothing from `tags`.
fn usgaap(tags: &[&'static str]) -> Vec<(&'static str, &'static str)> {
    tags.iter().map(|t| ("us-gaap", *t)).collect()
}

/// Whole days between two YYYY-MM-DD dates.
pub fn days_between(start: &str, end: &str) -> i64 {
    let parse = |s: &str| -> Option<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
    };
    match (parse(start), parse(end)) {
        (Some(a), Some(b)) => (b - a).num_days(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_counts_are_right() {
        assert_eq!(days_between("2026-01-01", "2026-03-31"), 89);
        assert_eq!(days_between("2025-07-01", "2026-06-30"), 364);
    }

    #[test]
    fn cohort_has_five_filers_with_ten_digit_ciks() {
        assert_eq!(COHORT.len(), 5);
        for (_, cik, _) in COHORT {
            assert_eq!(cik.len(), 10, "CIK must be zero-padded to 10 digits");
            assert!(cik.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn share_tags_cover_both_taxonomies() {
        // Regression guard: GOOGL and META publish no dei share tag. If the
        // us-gaap fallbacks are ever removed, the issuance indicator silently
        // loses two of five filers.
        assert!(TAGS_SHARES_DEI.contains(&"EntityCommonStockSharesOutstanding"));
        assert!(TAGS_SHARES_USGAAP.contains(&"CommonStockSharesOutstanding"));
    }
}
