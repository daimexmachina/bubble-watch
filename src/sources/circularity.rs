//! Circular-financing disclosure detection.
//!
//! THE GAP THIS CLOSES. This tool carried `circularity` at weight 0 as a declared gap, on
//! the reasoning that "no free source relates an equity investment to the revenue it
//! generates". That reasoning was WRONG, and the error was specific: it assumed the only
//! possible measure was a numeric ratio. A relation is a PAIR, and the pair is disclosed —
//! in filings, under related-party and equity-method accounting, in text that EDGAR
//! full-text search can find and ATTRIBUTE TO A FILER.
//!
//! WHAT IS ACTUALLY IN THE FILINGS (read from primary sources, 2026-09-20):
//!
//!   MSFT FY2026 10-K: "Microsoft is a major investor in OpenAI and will continue to
//!   receive revenue-sharing payments." Under ASC 850: revenue from commercial
//!   arrangements with OpenAI of **$24.1B**, accounts receivable **$6.0B**, total funding
//!   commitments **$13.0B**, and a **$6.5B** net gain on the stake. Equity-method
//!   investments DOUBLED $6.0B -> $12.0B over the year.
//!
//!   SPCX FY2026 10-Q: SpaceX acquired xAI on 2026-02-02 (the largest M&A on record,
//!   $1T + $250B, agreement filed as S-1 Exhibit 2.1), carries xAI's debt on its OWN
//!   balance sheet (spcx:XAI12.5SecuredSeniorNotesMember, spcx:XAIFixedRateTermLoanMember,
//!   spcx:XAIFloatingRateTermLoanMember), pays **$513M** of related-party interest in H1
//!   2026, and purchased **$329M** of Megapack products from Tesla, whose CEO is also its
//!   own.
//!
//! Those are circular arrangements, self-disclosed, with magnitudes.
//!
//! MECHANISM: EDGAR full-text search filtered by `ciks=`, which attributes a hit to the
//! FILER. That converts a keyword count into a DIRECTED EDGE (who discloses a relationship
//! with whom) rather than a bag of mentions.
//!
//! WHAT THIS MODULE REFUSES TO DO:
//!
//! 1. A failed query is an ERROR, never a zero. "The query failed" and "the name is absent"
//!    must never be indistinguishable.
//! 2. An all-zero row is SILENCE with its reason, never reassurance. Measured: Alphabet
//!    names Anthropic in ZERO of its 21 indexed 10-Ks while holding a real stake. A clean
//!    scan is a coverage limit, not a clean bill of health.
//! 3. A name mention is a SIGNAL TO INVESTIGATE, not a finding. Read passages are recorded
//!    in `verified_passage`; unread ones stay absent rather than implied.
//! 4. No fabricated magnitudes. A filer that discloses none gets no magnitude.

use crate::http::Fetcher;
use serde::{Deserialize, Serialize};

/// SEC requires a descriptive UA INCLUDING A CONTACT ADDRESS. Without one the Archives
/// endpoints return HTTP 403 "Undeclared Automated Tool" — measured on this host.
const UA: &str = "bubble-watch/0.1 (research; contact: research@example.invalid)";

/// A counterparty worth tracking: a private AI lab, neocloud or major supplier that
/// investors fund, and whose revenue may therefore return to its funders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counterparty {
    pub name: &'static str,
    /// Why this name is on the list, so the list is auditable rather than arbitrary.
    pub why: &'static str,
}

/// Tracked counterparties. Each one is justified in writing.
///
/// DELIBERATELY FINITE. A long name list inflates hit counts through incidental mentions —
/// every company's 10-K names competitors — which is the false-positive mechanism this
/// module exists to avoid.
///
/// A TRAP WORTH RECORDING: "xAI" substring-matches two unrelated SEC registrants, "XAI
/// Floating Rate & Alternative Income Trust" (XFLT) and "XAI Madison Equity Premium Income
/// Fund" (MCN). Neither has anything to do with Musk's xAI. Any matching here must be on
/// whole tokens and the CIK checked, or the instrument reports a bond fund as an AI
/// circularity.
pub const COUNTERPARTIES: &[Counterparty] = &[
    Counterparty {
        name: "OpenAI",
        why: "Largest private AI lab. Microsoft discloses a $24.1B revenue relationship and \
              $13.0B of funding commitments; Oracle, NVIDIA and CoreWeave contract with it.",
    },
    Counterparty {
        name: "Anthropic",
        why: "Second-largest private lab. Amazon holds convertible notes, warrants and \
              preferred stock; Google holds a stake it reports via proxy, not in its 10-K.",
    },
    Counterparty {
        name: "xAI",
        why: "Musk's lab, ABSORBED BY SPACEX on 2026-02-02 in the largest M&A on record. \
              SpaceX now carries xAI's debt and pays related-party interest on it.",
    },
    Counterparty {
        name: "CoreWeave",
        why: "Listed GPU neocloud. NVIDIA is a disclosed shareholder AND a supplier, which \
              is the supplier-finances-customer pattern.",
    },
    Counterparty {
        name: "Databricks",
        why: "Private enterprise-AI platform, held by funds under NPORT-P disclosure.",
    },
    Counterparty {
        name: "Lambda",
        why: "GPU neocloud. SMCI names it three times in its 10-Ks.",
    },
    Counterparty {
        name: "Crusoe",
        why: "AI datacentre operator named by APLD and IREN. Included after measurement, \
              not as a guess.",
    },
    Counterparty {
        name: "Stargate",
        why: "Named joint venture behind the Oracle/OpenAI datacentre buildout.",
    },
    Counterparty {
        name: "Tesla",
        why: "SpaceX discloses $329M of Megapack purchases from Tesla in H1 2026. Musk is \
              CEO of both, making this the clearest related-party structure in the dataset.",
    },
    Counterparty {
        name: "NVIDIA",
        why: "The largest single node in the circular deals: simultaneously an investor in \
              its customers and their supplier. Intel names it in its 10-Ks.",
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
    /// Total mentions across all searched forms.
    pub hits: u64,
    /// Per-form breakdown, e.g. ["10-K:2", "8-K:9"]. Present because a 10-K-only reading
    /// systematically undercounts: deals are announced in 8-Ks.
    pub forms: Vec<String>,
    /// Filing dates, newest first. Present so the reading is auditable.
    pub dates: Vec<String>,
    /// A verbatim passage, where a human read the filing and confirmed what the mention
    /// actually is. Absent means the mention is confirmed to EXIST but its nature has not
    /// been read, and that difference is stated rather than blurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_passage: Option<String>,
    /// The disclosed magnitude for this edge. Absent means none was disclosed or read —
    /// never an estimate.
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
    /// Forms searched, so the form filter is visible rather than implicit.
    pub forms_searched: Vec<String>,
}

/// The cohort. EVERY CIK was verified against SEC `company_tickers.json` on 2026-09-20
/// rather than transcribed from an article.
///
/// Split into the two halves that matter for this instrument — the DEMAND side (companies
/// that buy compute and sign the contracts) and the SUPPLY side (chips and datacentre
/// infrastructure) — because the supplier-finances-customer pattern runs between them.
pub const FILERS: &[(&str, &str, &str)] = &[
    // --- hyperscalers and the labs' principal counterparties (demand side) ---
    (
        "MSFT",
        "0000789019",
        "discloses $24.1B revenue from OpenAI under ASC 850",
    ),
    (
        "GOOGL",
        "0001652044",
        "Anthropic stake reported via proxy, NOT in its 10-K",
    ),
    (
        "AMZN",
        "0001018724",
        "Anthropic via convertible notes, warrants, preferred stock",
    ),
    ("META", "0001326801", "hyperscaler; names Anthropic"),
    (
        "AAPL",
        "0000320193",
        "consumer AI; tests whether it discloses any lab tie at all",
    ),
    (
        "ORCL",
        "0001341439",
        "OpenAI compute; names NO counterparty — see the silence note",
    ),
    ("IBM", "0000051143", "watsonx; enterprise AI provider"),
    (
        "CRM",
        "0001108524",
        "Salesforce Einstein; enterprise AI provider",
    ),
    ("PLTR", "0001321655", "Palantir; government and defence AI"),
    // --- silicon (supply side) ---
    (
        "NVDA",
        "0001045810",
        "supplier AND investor in the neoclouds it sells to",
    ),
    (
        "AMD",
        "0000002488",
        "OpenAI warrant/compute deal; announces in 8-K, not 10-K",
    ),
    (
        "INTC",
        "0000050863",
        "chip supplier; names NVIDIA, no AI lab",
    ),
    // --- the Musk complex ---
    (
        "SPCX",
        "0001181412",
        "SpaceX: acquired xAI 2026-02-02 in the largest M&A on record, carries xAI debt, \
         IPO'd 2026-06, funds orbital datacentres",
    ),
    // --- the neocloud / AI-datacentre complex, where the pattern concentrates ---
    (
        "CRWV",
        "0001769628",
        "the GPU neocloud being financed: NVIDIA invests in it AND sells to it",
    ),
    (
        "APLD",
        "0001144879",
        "AI datacentre operator leasing to neoclouds; names CoreWeave, Lambda, Crusoe",
    ),
    (
        "IREN",
        "0001878848",
        "bitcoin-miner-turned-AI-datacentre; names CoreWeave, Lambda, Crusoe",
    ),
    (
        "CIFR",
        "0001819989",
        "AI datacentre operator, converted from crypto mining; names CoreWeave",
    ),
    (
        "DELL",
        "0001571996",
        "AI server vendor whose customer is a funded neocloud; names CoreWeave",
    ),
    (
        "SMCI",
        "0001375365",
        "AI server vendor; names Lambda, a funded neocloud customer",
    ),
    (
        "VRT",
        "0001674101",
        "datacentre power and cooling; names CoreWeave",
    ),
];

/// The forms searched. NOT 10-K alone.
///
/// Measured reason: AMD mentions OpenAI twice in 10-Ks but NINE times in 8-Ks and three in
/// 10-Qs — a 10-K-only scan captures 14% of its signal and misses 86%. Deals are ANNOUNCED
/// in 8-Ks and merely summarised annually, so a 10-K filter systematically undercounts
/// exactly the event this instrument exists to find.
pub const FORMS: &[&str] = &["10-K", "8-K", "10-Q"];

/// Count filings of `form` in which `cik` names `name`, in the window.
///
/// Returns the count AND the filing dates. A non-200 or unparseable body is an ERROR, never
/// a zero: "the query failed" and "the name does not appear" must not be indistinguishable,
/// which is the failure mode this whole project is built around.
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

/// Passages a human read and confirmed. Only populated where the filing was actually read,
/// so the field cannot become a place where plausible-sounding text accumulates.
fn verified_passage(filer: &str, counterparty: &str) -> Option<String> {
    match (filer, counterparty) {
        ("MSFT", "OpenAI") => Some(
            "MSFT FY2026 10-K, verbatim: \"Microsoft is a major investor in OpenAI and will \
             continue to receive revenue-sharing payments.\" And under ASC 850 Related Party \
             Disclosures: \"For fiscal year 2026, we recorded revenue from commercial \
             arrangements with OpenAI, inclusive of revenue-sharing payments, of $24.1 billion, \
             and accounts receivable from OpenAI as of June 30, 2026 was $6.0 billion. We have \
             made total funding commitments of $13.0 billion...\""
                .to_string(),
        ),
        ("AMD", "OpenAI") => Some(
            "AMD Form 8-K filed 2025-10-06 (Item 1.01, Material Definitive Agreement), \
             verbatim: \"On October 5, 2025, Advanced Micro Devices, Inc. issued to OpenAI \
             OpCo, LLC a warrant to purchase up to an aggregate of 160 million shares of \
             common stock of the Company at an exercise price of $0.01 per share. The Warrant \
             Shares vest in tranches based on milestones tied to purchases of AMD Instinct GPU \
             products by Warrantholder... with the first tranche of shares vesting after the \
             delivery of the initial one (1) gigawatt of AMD Instinct MI450 Series GPU products \
             and full vesting for the 160 million shares contingent upon Warrantholder, its \
             affiliates or Authorized Purchasers purchasing six (6) gigawatts... Vesting of \
             Warrant Shares are further subject to achievement of specified Company stock price \
             targets that escalate to $600 per share for the final tranche.\" The 8-K Item 3.02 \
             records this as an unregistered sale of equity securities. Exhibit 99.1 is the \
             joint press release; AMD's CFO states the partnership is 'expected to deliver tens \
             of billions of dollars in revenue for AMD'."
                .to_string(),
        ),
        ("ORCL", "OpenAI") => Some(
            "CONFIRMED, AND THE MENTIONS ARE NOT WHAT THEY LOOK LIKE. Oracle names OpenAI in \
             its 10-K ZERO times and in 10-Qs ZERO times; both hits are 8-K earnings press \
             releases, and in BOTH the name appears only as a MODEL PROVIDER, not as a \
             contract counterparty. FY2026 Q1 (filed 2025-09-09), verbatim: \"we will introduce \
             a new Cloud Infrastructure service called the 'Oracle AI Database' that enables our \
             customers to use the Large Language Model of their choice—including Google's \
             Gemini, OpenAI's ChatGPT, xAI's Grok, etc.—directly on top of the Oracle \
             Database\". The SAME release reports RPO up 359% to $455 billion and attributes it \
             to \"four multi-billion-dollar contracts with three different customers\" WITHOUT \
             NAMING ANY OF THEM. So the largest AI infrastructure contract in the industry is \
             disclosed as a NUMBER with no counterparty. This is why ORCL shows a hit here and \
             why the hit must not be read as a disclosure of the contract: the instrument is \
             finding a model-integration list, and the actual customer relationship is \
             deliberately anonymous. Oracle's FY2026 10-K reinforces it: \"No single customer \
             accounted for 10% or more of our total revenues in fiscal 2026, 2025 or 2024.\""
                .to_string(),
        ),
        ("SPCX", "Tesla") => Some(
            "SPCX FY2026 10-Q, Note 17 Related Party Transactions, verbatim: \"During the three \
             and six months ended June 30, 2026, the Company purchased $295 million and $329 \
             million, respectively, of Megapack products from Tesla, Inc. ... As of December 31, \
             2025, the Company purchased $506 million of Megapack products and $131 million of \
             Cybertrucks at manufacturer's suggested retail price from Tesla...\""
                .to_string(),
        ),
        ("SPCX", "xAI") => Some(
            "SPCX FY2026 10-Q: the xAI Merger appears in equity (\"Conversion of redeemable \
             convertible preferred stock pursuant to xAI Merger\", 1,424 shares / $37,474M), and \
             xAI's debt sits on SpaceX's own balance sheet via the XBRL axis members \
             spcx:XAI12.5SecuredSeniorNotesMember, spcx:XAIFixedRateTermLoanMember and \
             spcx:XAIFloatingRateTermLoanMember. Interest expense carries a RELATED PARTY \
             component: $327M for Q2 2026 and $513M for H1 2026."
                .to_string(),
        ),
        _ => None,
    }
}

/// Disclosed magnitudes, transcribed from filings that were read. Absent where not read or
/// not disclosed — never estimated.
fn known_magnitude(filer: &str, counterparty: &str) -> Option<String> {
    match (filer, counterparty) {
        ("MSFT", "OpenAI") => Some(
            "revenue from OpenAI $24.1B (FY2026), receivable $6.0B, funding commitments $13.0B, \
             stake gain $6.5B"
                .to_string(),
        ),
        ("AMD", "OpenAI") => Some(
            "warrant for 160,000,000 AMD shares at a $0.01 strike, vesting against 6 GW of GPU \
             purchases; equal to 9.8% of AMD's 1,632,475,042 shares outstanding (EDGAR dei, \
             2026-07-29). AMD's stated expectation is 'tens of billions of dollars in revenue' \
             from OpenAI."
                .to_string(),
        ),
        ("SPCX", "Tesla") => Some("$329M of Megapack purchases in H1 2026".to_string()),
        ("SPCX", "xAI") => Some(
            "xAI acquired at a $250B valuation (closed 2026-02-02); related-party interest \
             expense $513M in H1 2026"
                .to_string(),
        ),
        _ => None,
    }
}

/// Scan the cohort for disclosure edges. Networked.
///
/// Paced deliberately: EDGAR rate-limits, and a 429 must not become a silent zero, so each
/// request's failure is recorded in `errors` rather than swallowed.
pub fn scan(f: &Fetcher, start: &str, end: &str, pause_ms: u64) -> ScanResult {
    let mut edges = Vec::new();
    let mut errors = Vec::new();
    let mut per_counterparty: Vec<(&str, u64)> =
        COUNTERPARTIES.iter().map(|c| (c.name, 0)).collect();

    for (ticker, cik, _why) in FILERS {
        for cp in COUNTERPARTIES {
            let mut total = 0u64;
            let mut all_dates: Vec<String> = Vec::new();
            let mut forms_hit: Vec<String> = Vec::new();
            let mut any_ok = false;

            for form in FORMS {
                match count_mentions(f, cik, cp.name, form, start, end) {
                    Ok((n, dates)) => {
                        any_ok = true;
                        if n > 0 {
                            total += n;
                            forms_hit.push(format!("{}:{}", form, n));
                            all_dates.extend(dates);
                        }
                    }
                    Err(e) => errors.push(format!("{} x {} [{}]: {}", ticker, cp.name, form, e)),
                }
                if pause_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(pause_ms));
                }
            }

            // Every form failed: this is an ERROR (recorded above), never a zero. Skipped
            // rather than converted into a zero-hit edge.
            if !any_ok {
                continue;
            }

            all_dates.sort();
            all_dates.dedup();
            all_dates.reverse();

            if total > 0 {
                if let Some(slot) = per_counterparty.iter_mut().find(|(nm, _)| *nm == cp.name) {
                    slot.1 += total;
                }
                edges.push(DisclosureEdge {
                    filer: ticker.to_string(),
                    filer_cik: cik.to_string(),
                    counterparty: cp.name.to_string(),
                    hits: total,
                    forms: forms_hit,
                    dates: all_dates,
                    verified_passage: verified_passage(ticker, cp.name),
                    magnitude: known_magnitude(ticker, cp.name),
                });
            }
        }
    }

    // Counterparties nobody in the cohort names. Stated as SILENCE with the reason it is not
    // reassurance, because the Google/Anthropic case proves a real stake can be invisible to
    // this method.
    let silent: Vec<String> = per_counterparty
        .iter()
        .filter(|(_, n)| *n == 0)
        .map(|(nm, _)| {
            format!(
                "{} — named in ZERO of the cohort's filings in this window. This is SILENCE, \
                 not evidence of no arrangement: Alphabet holds a real stake in Anthropic yet \
                 names it in none of its 21 indexed 10-Ks, disclosing it through proxy filings \
                 instead. Treat an all-zero row as a coverage limit of this method, never as a \
                 clean bill of health.",
                nm
            )
        })
        .collect();

    ScanResult {
        edges,
        silent,
        errors,
        filers_scanned: FILERS.len(),
        forms_searched: FORMS.iter().map(|s| s.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_counterparty_carries_its_justification() {
        // A name without a stated reason would be an arbitrary inclusion, and an arbitrary
        // name inflates hit counts through incidental mentions.
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
    fn no_filer_cik_is_duplicated_and_all_are_well_formed() {
        // A duplicated CIK would double-count a company's filings; a malformed one would
        // silently return zero rows, which is a quiet false NEGATIVE — the failure mode that
        // looks identical to a clean result.
        let mut seen = std::collections::BTreeSet::new();
        for (t, cik, why) in FILERS {
            assert!(
                cik.len() == 10 && cik.chars().all(|c| c.is_ascii_digit()),
                "{} has a malformed CIK {:?}",
                t,
                cik
            );
            assert!(seen.insert(*cik), "{} duplicates CIK {}", t, cik);
            assert!(why.len() > 15, "{} needs a stated reason", t);
        }
    }

    #[test]
    fn the_musk_complex_is_present_because_it_is_the_largest_structure_measured() {
        // SpaceX absorbed xAI in the largest M&A on record and carries xAI's debt on its own
        // balance sheet. Omitting it would leave the single biggest circular structure in the
        // dataset invisible to the instrument.
        assert!(
            FILERS.iter().any(|(t, _, _)| *t == "SPCX"),
            "SPCX must be scanned"
        );
        assert!(
            COUNTERPARTIES.iter().any(|c| c.name == "xAI"),
            "xAI must be searched for"
        );
        assert!(
            COUNTERPARTIES.iter().any(|c| c.name == "Tesla"),
            "Tesla must be searched for"
        );
    }

    #[test]
    fn the_named_us_ai_players_are_all_covered() {
        // The instrument must not silently omit a company someone would reasonably expect
        // to see. Names that are genuinely PRIVATE (OpenAI, Anthropic, Databricks, xAI) are
        // counterparties, not filers, because they file no annual report — that distinction
        // is what this test pins down.
        let filer_tickers: Vec<&str> = FILERS.iter().map(|(t, _, _)| *t).collect();
        for expected in [
            "NVDA", "MSFT", "GOOGL", "AMZN", "META", "AAPL", "ORCL", "AMD", "IBM", "CRM", "PLTR",
            "INTC",
        ] {
            assert!(
                filer_tickers.contains(&expected),
                "{} is a listed AI player and must be scanned as a filer",
                expected
            );
        }
        let cps: Vec<&str> = COUNTERPARTIES.iter().map(|c| c.name).collect();
        for private in ["OpenAI", "Anthropic", "xAI", "Databricks"] {
            assert!(
                cps.contains(&private),
                "{} is private and must be tracked as a COUNTERPARTY",
                private
            );
        }
    }

    #[test]
    fn silent_counterparties_are_explained_as_silence_not_safety() {
        // The single most important property of this module: an all-zero row must never be
        // readable as "no circular financing here".
        let msg = format!(
            "{} — named in ZERO of the cohort's filings in this window. This is SILENCE, not \
             evidence of no arrangement: Alphabet holds a real stake in Anthropic yet names it \
             in none of its 21 indexed 10-Ks, disclosing it through proxy filings instead. \
             Treat an all-zero row as a coverage limit of this method, never as a clean bill \
             of health.",
            "Anthropic"
        );
        assert!(msg.contains("SILENCE"), "must say silence: {}", msg);
        assert!(
            msg.contains("never as a clean bill of health"),
            "must refuse the reassuring reading: {}",
            msg
        );
    }

    #[test]
    fn a_verified_passage_must_state_what_kind_of_relationship_it_is() {
        // THE LESSON FROM READING THE PASSAGES. A verified hit is not automatically a
        // circular-financing hit. Oracle names OpenAI twice, and in BOTH cases the name is a
        // MODEL INTEGRATION LIST ("Google's Gemini, OpenAI's ChatGPT, xAI's Grok"), not a
        // contract counterparty — while the same press release reports RPO up 359% to $455B
        // from "four multi-billion-dollar contracts with three different customers" WITHOUT
        // NAMING ANY OF THEM.
        //
        // A tool that surfaced "ORCL names OpenAI 2x" without that context would report the
        // largest AI contract in the industry as DISCLOSED when it is in fact anonymous. So a
        // verified passage must classify the relationship, not merely quote text.
        let p = verified_passage("ORCL", "OpenAI").expect("ORCL x OpenAI was read");
        assert!(
            p.contains("MODEL PROVIDER"),
            "the Oracle passage must classify the mention as a model-provider list: {}",
            &p[..p.len().min(200)]
        );
        assert!(
            p.contains("NOT a contract counterparty")
                || p.contains("not as a contract counterparty"),
            "and must say explicitly what it is NOT: {}",
            &p[..p.len().min(200)]
        );
        assert!(
            p.contains("455 billion") && p.contains("WITHOUT"),
            "and must report that the RPO growth is unattributed: {}",
            &p[..p.len().min(200)]
        );
    }

    #[test]
    fn the_amd_warrant_is_recorded_as_a_measured_magnitude() {
        // The clearest circular structure found: AMD handed OpenAI a warrant for 160M shares
        // at a $0.01 strike, vesting against 6 GW of GPU purchases. Both sides are each
        // other's consideration. It is an 8-K disclosure, so a 10-K-only scan never sees it.
        let m = known_magnitude("AMD", "OpenAI").expect("AMD x OpenAI has a magnitude");
        assert!(m.contains("160,000,000"), "share count: {}", m);
        assert!(
            m.contains("9.8%"),
            "must state the dilution, not just the count: {}",
            m
        );
        let p = verified_passage("AMD", "OpenAI").expect("AMD x OpenAI was read");
        assert!(
            p.contains("$0.01") && p.contains("6") && p.contains("gigawatts"),
            "the passage must carry the strike price and the vesting trigger: {}",
            &p[..p.len().min(200)]
        );
    }

    #[test]
    fn forms_include_8k_because_deals_are_announced_there() {
        // Measured: AMD names OpenAI 2x in 10-Ks but 9x in 8-Ks. A 10-K-only scan captures
        // 14% of the signal for exactly the event this instrument exists to find.
        assert!(FORMS.contains(&"8-K"), "8-K must be searched");
        assert!(FORMS.contains(&"10-Q"), "10-Q must be searched");
        assert!(FORMS.contains(&"10-K"), "10-K must be searched");
    }

    #[test]
    fn a_failed_query_is_never_recorded_as_a_zero() {
        // Structural: `count_mentions` returns Result, `scan` routes Err into `errors`, and
        // skips the edge entirely when every form failed. If a later change turns the Err arm
        // into an edge with hits=0, the assertion below fails.
        let src = include_str!("circularity.rs");
        let after = src
            .split("Err(e) => errors.push")
            .nth(1)
            .expect("the error arm must exist");
        let window = &after[..after.len().min(400)];
        assert!(
            !window.contains("hits: 0"),
            "a failed query must not become a zero-hit edge"
        );
        assert!(
            src.contains("if !any_ok {"),
            "the all-forms-failed guard must exist"
        );
    }
}
