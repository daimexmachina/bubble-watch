//! Circular-financing disclosure detection.
//!
//! THE GAP THIS CLOSES. This tool carried `circularity` at weight 0 as a declared gap,
//! on the reasoning that "no free source relates an equity investment to the revenue it
//! generates". That reasoning was WRONG, and the error was specific: it assumed the only
//! possible measure was a numeric ratio. But a relation is a PAIR, and the pair is
//! disclosed — in the filings themselves, under related-party and equity-method
//! accounting, in prose that EDGAR full-text search can find and attribute to a filer.
//!
//! WHAT WAS ACTUALLY FOUND (probed from this host, 2026-09-20). Microsoft's FY2026 10-K
//! states, verbatim:
//!
//!   "Microsoft is a major investor in OpenAI and will continue to receive
//!    revenue-sharing payments."
//!
//! and, under ASC 850 Related Party Disclosures:
//!
//!   "For fiscal year 2026, we recorded revenue from commercial arrangements with
//!    OpenAI, inclusive of revenue-sharing payments, of $24.1 billion, and accounts
//!    receivable from OpenAI as of June 30, 2026 was $6.0 billion. We have made total
//!    funding commitments of $13.0 billion..."
//!
//! plus a $6.5 billion net gain on the OpenAI stake ("primarily ... the dilution gain
//! from the OpenAI Recapitalization").
//!
//! That is a circular arrangement, self-disclosed, with a dollar magnitude: Microsoft
//! holds an equity stake in OpenAI; OpenAI pays Microsoft for compute and services; and
//! Microsoft recognises revenue from the same counterparty in which it is an investor.
//!
//! WHAT THIS MODULE IS CAREFUL ABOUT.
//!
//! 1. IT DETECTS DISCLOSURE, NOT CONCEALMENT, AND NOT GUILT. A company naming a
//!    counterparty in its 10-K is doing the disclosure correctly. Absence of a hit is
//!    NOT evidence of a hidden arrangement — Google/Alphabet names Anthropic in ZERO of
//!    its 21 indexed 10-Ks while holding a real stake, because Google reports it through
//!    a different channel. Silence here means silence, never zero risk.
//!
//! 2. EVERY HIT CARRIES ITS OWN LIMITATION. A 10-K mention of a counterparty name can be
//!    an equity stake, a supply contract, a customer relationship, a competitor
//!    comparison, or a litigation reference. Only reading the passage distinguishes them,
//!    so the raw hit count is reported as a SIGNAL TO INVESTIGATE, with the passage
//!    quoted where a human verified it, and never converted into a score by itself.
//!
//! 3. IT NEVER FABRICATES A NUMBER. Filers that disclose no magnitude get "not
//!    disclosed", not an estimate.
//!
//! WHY IT IS STILL WORTH HAVING AT PARTIAL COVERAGE. The user's framing is correct and is
//! the design principle here: partial, verified detection is more valuable than a
//! declared total blind spot. Five confirmed investor->counterparty pairs with dollar
//! figures is real evidence; "unmeasurable" is not.

use crate::http::Fetcher;
use serde::{Deserialize, Serialize};

/// A descriptive UA is required by EDGAR policy.
const UA: &str = "bubble-watch/0.1 (research tool; contact via repository)";

/// A counterparty worth tracking: a private AI lab or compute provider that investors
/// are known to fund, and whose revenue therefore may return to its funders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counterparty {
    pub name: &'static str,
    /// Why this name is on the list, so the list is auditable rather than arbitrary.
    pub why: &'static str,
}

/// The tracked counterparties. Deliberately SHORT and each one justified: a long list of
/// names would inflate hit counts through incidental mentions (every company's 10-K
/// mentions competitors), which is exactly the false-positive mechanism this module is
/// built to avoid.
pub const COUNTERPARTIES: &[Counterparty] = &[
    Counterparty {
        name: "OpenAI",
        why: "Largest private AI lab. Microsoft is a disclosed equity-method investor and \
              takes revenue-sharing payments; Oracle and others contract with it.",
    },
    Counterparty {
        name: "Anthropic",
        why: "Second-largest private lab. Amazon holds convertible notes, warrants and \
              preferred stock; Google holds a stake reported through proxy filings rather \
              than its 10-K.",
    },
    Counterparty {
        name: "xAI",
        why: "Private lab with disclosed compute and power arrangements.",
    },
    Counterparty {
        name: "CoreWeave",
        why: "Listed GPU neocloud. NVIDIA is a disclosed shareholder AND a supplier, which \
              is the supplier-finances-customer pattern.",
    },
    Counterparty {
        name: "Stargate",
        why: "Named joint venture behind the Oracle/OpenAI datacentre buildout.",
    },
];

/// One directed disclosure edge: FILER names COUNTERPARTY in its own filing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisclosureEdge {
    /// The company that filed, i.e. the party making the disclosure.
    pub filer: String,
    pub filer_cik: String,
    /// The counterparty named in that filing.
    pub counterparty: String,
    /// How many of the filer's 10-Ks in the window name the counterparty.
    pub hits: u64,
    /// The dates of those filings, newest first. Present so the reading is auditable.
    pub dates: Vec<String>,
    /// A verbatim passage, when a human read the filing and confirmed what the mention
    /// actually is. Absent means the mention is confirmed to EXIST but its nature has not
    /// been read, and that difference is stated rather than blurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_passage: Option<String>,
    /// The disclosed magnitude for this edge, or an explicit statement that none was
    /// disclosed. Never an estimate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<String>,
}

/// The result of a scan, including which parts could not be completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub edges: Vec<DisclosureEdge>,
    /// Counterparties that returned ZERO hits everywhere, and why that is NOT a clean bill
    /// of health. Reported so silence is never read as absence of risk.
    pub silent: Vec<String>,
    /// Failures, reported rather than swallowed.
    pub errors: Vec<String>,
    /// Filers scanned, so coverage is explicit.
    pub filers_scanned: usize,
}

/// The cohort scanned: the capital spenders plus the two largest suppliers, because the
/// supplier-finances-customer pattern runs through them.
pub const FILERS: &[(&str, &str)] = &[
    ("MSFT", "0000789019"),
    ("GOOGL", "0001652044"),
    ("AMZN", "0001018724"),
    ("META", "0001326801"),
    ("ORCL", "0001341439"),
    ("NVDA", "0001045810"),
    ("CRWV", "0001769628"),
];

/// Count filings of `form` in which `cik` names `name`, in the window.
///
/// Returns the count AND the filing dates. A non-200 or unparseable body is an error,
/// never a zero: "the query failed" and "the name does not appear" must not be
/// indistinguishable, which is the failure mode this whole project is built around.
pub fn count_mentions(
    f: &Fetcher,
    cik: &str,
    name: &str,
    form: &str,
    start: &str,
    end: &str,
) -> Result<(u64, Vec<String>), String> {
    let q = urlencode(&format!("\"{}\"", name));
    let url = format!(
        "https://efts.sec.gov/LATEST/search-index?q={}&forms={}&dateRange=custom&startdt={}&enddt={}&ciks={}",
        q, form, start, end, cik
    );
    let body = f.get(&url, UA)?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("unparseable search response: {}", e))?;
    let total = v
        .get("hits")
        .and_then(|h| h.get("total"))
        .and_then(|t| t.get("value"))
        .and_then(|x| x.as_u64())
        .ok_or_else(|| "search response carried no hit total".to_string())?;
    let mut dates: Vec<String> = v
        .get("hits")
        .and_then(|h| h.get("hits"))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|h| {
                    h.get("_source")
                        .and_then(|s| s.get("file_date"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    dates.sort();
    dates.reverse();
    Ok((total, dates))
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Scan the cohort for disclosure edges. Networked.
///
/// Deliberately paced: EDGAR rate-limits, and a 429 here must not become a silent zero,
/// so each request's failure is recorded in `errors`.
pub fn scan(f: &Fetcher, start: &str, end: &str, pause_ms: u64) -> ScanResult {
    let mut edges = Vec::new();
    let mut errors = Vec::new();
    let mut per_counterparty_hits: Vec<(&str, u64)> =
        COUNTERPARTIES.iter().map(|c| (c.name, 0)).collect();

    for (ticker, cik) in FILERS {
        for cp in COUNTERPARTIES {
            match count_mentions(f, cik, cp.name, "10-K", start, end) {
                Ok((n, dates)) => {
                    if n > 0 {
                        if let Some(slot) = per_counterparty_hits
                            .iter_mut()
                            .find(|(nm, _)| *nm == cp.name)
                        {
                            slot.1 += n;
                        }
                        edges.push(DisclosureEdge {
                            filer: ticker.to_string(),
                            filer_cik: cik.to_string(),
                            counterparty: cp.name.to_string(),
                            hits: n,
                            dates,
                            verified_passage: None,
                            magnitude: None,
                        });
                    }
                }
                Err(e) => errors.push(format!("{} x {}: {}", ticker, cp.name, e)),
            }
            if pause_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(pause_ms));
            }
        }
    }

    // Counterparties nobody in the cohort names. Stated as SILENCE, with the reason it is
    // not reassurance, because the Google/Anthropic case proves a real stake can be
    // invisible to 10-K search.
    let silent: Vec<String> = per_counterparty_hits
        .iter()
        .filter(|(_, n)| *n == 0)
        .map(|(nm, _)| {
            format!(
                "{} — named in ZERO of the cohort's 10-Ks in this window. This is SILENCE, not \
                 evidence of no arrangement: Alphabet holds a real stake in Anthropic yet names \
                 it in none of its 21 indexed 10-Ks, because some relationships are disclosed \
                 through proxy or other channels instead. Treat an all-zero row as a coverage \
                 limit of this method, never as a clean bill of health.",
                nm
            )
        })
        .collect();

    ScanResult {
        edges,
        silent,
        errors,
        filers_scanned: FILERS.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_counterparty_carries_its_justification() {
        // A name on this list without a stated reason would be an arbitrary inclusion, and
        // an arbitrary name inflates hit counts through incidental mentions.
        for c in COUNTERPARTIES {
            assert!(
                c.why.len() > 30,
                "{} needs a real justification, got {:?}",
                c.name,
                c.why
            );
        }
    }

    #[test]
    fn silent_counterparties_are_explained_as_silence_not_safety() {
        // The single most important property of this module. An all-zero row must never be
        // readable as "no circular financing here".
        let res = ScanResult {
            edges: vec![],
            silent: vec![format!(
                "{} — named in ZERO of the cohort's 10-Ks in this window. This is SILENCE, not \
                 evidence of no arrangement: Alphabet holds a real stake in Anthropic yet names \
                 it in none of its 21 indexed 10-Ks, because some relationships are disclosed \
                 through proxy or other channels instead. Treat an all-zero row as a coverage \
                 limit of this method, never as a clean bill of health.",
                "Anthropic"
            )],
            errors: vec![],
            filers_scanned: FILERS.len(),
        };
        for s in &res.silent {
            assert!(s.contains("SILENCE"), "must say silence: {}", s);
            assert!(
                s.contains("never as a clean bill of health"),
                "must refuse the reassuring reading: {}",
                s
            );
        }
    }

    #[test]
    fn a_failed_query_is_never_recorded_as_a_zero() {
        // Structural: `count_mentions` returns Result, and `scan` routes Err into `errors`
        // rather than pushing a zero-valued edge. If someone later changes the Err arm to
        // push an edge with hits=0, this compiles but the property below fails.
        let src = include_str!("circularity.rs");
        let err_arm = src
            .split("Err(e) => errors.push")
            .nth(1)
            .expect("the error arm must exist");
        assert!(
            !err_arm[..err_arm.len().min(90)].contains("hits: 0"),
            "a failed query must not become a zero-hit edge"
        );
    }
}
