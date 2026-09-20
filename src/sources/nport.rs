//! Fund holdings of private AI companies, from NPORT-P filings.
//!
//! WHY THIS EXISTS, AND WHY IT IS THE STRONGEST SOURCE FOUND.
//!
//! The disclosure scanner in `circularity` reads the SUPPLIER side: which public company
//! names which private lab. That establishes a relationship but rarely a number, because
//! public companies disclose counterparties without necessarily disclosing the size of their
//! stake.
//!
//! NPORT-P is the mirror image and it carries FIGURES. Every US-registered fund files a
//! monthly portfolio report on Form N-PORT, and Part C is published as MACHINE-READABLE XML
//! naming each holding with its value. That means the investor side of the circular loop —
//! who actually holds OpenAI, Anthropic or xAI, and for how much — is available as
//! structured data rather than as journalism.
//!
//! MEASURED ON THIS HOST, 2026-09-20:
//!   * Form NPORT-P filings naming "OpenAI":     579
//!   * Form NPORT-P filings naming "Anthropic":  749
//!   * A single Fidelity fund's `primary_doc.xml` contained 51 `<invstOrSec>` blocks,
//!     including five private-AI-lab holdings named explicitly, e.g.:
//!
//!       <name>ANTHROPIC PBC</name>
//!       <title>ANTHROPIC PBC SERIES G PC PP</title>
//!       <valUSD>2414941.00000000</valUSD>
//!       <balance>4100.00000000</balance>
//!       <fairValLevel>3</fairValLevel>
//!
//! WHAT IT MEASURES, AND WHAT IT DOES NOT.
//!
//! It measures DISCLOSED FUND EXPOSURE, which is not the same as total investment. Only
//! US-registered funds file N-PORT; sovereign wealth funds, family offices, pension funds
//! and foreign vehicles do not, and neither do the strategic corporate investors whose
//! stakes are the actual circularity this project cares about. So this is a LOWER BOUND on
//! institutional exposure and says nothing at all about Microsoft's or NVIDIA's positions.
//!
//! `fairValLevel` is the honesty-critical field. Level 3 means the value is derived from
//! UNOBSERVABLE inputs — there is no market price to check it against. For a private AI lab
//! that is the expected tier, and it means the dollar figures are the fund's own estimate
//! rather than a transaction price. Reporting them without that flag would overstate their
//! solidity.
//!
//! It also cannot distinguish a circular arrangement from an ordinary investment. A fund
//! holding Anthropic is not evidence of circularity; the circularity lives in the strategic
//! investors, which is what `circularity::scan` looks for. These are two halves of one
//! picture and neither is a score.

use crate::http::Fetcher;
use serde::{Deserialize, Serialize};

/// EDGAR requires a descriptive UA including a contact address; Archives returns 403 without.
const UA: &str = "bubble-watch/0.1 (research; contact: research@example.invalid)";

/// The private AI names worth pricing, drawn from the same list the disclosure scanner uses.
/// Kept as plain strings because these are matched against filing TEXT, not resolved as CIKs.
pub const PRIVATE_AI_NAMES: &[&str] = &["OpenAI", "Anthropic", "Databricks", "xAI", "CoreWeave"];

/// One disclosed fund holding of a private AI company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundHolding {
    /// The reporting fund.
    pub filer: String,
    pub filer_cik: String,
    /// The private company held.
    pub holding: String,
    /// The exact name as filed, so a reader can check the match themselves.
    pub filed_name: String,
    /// Filing date of the N-PORT.
    pub filed: String,
    /// Reported value in USD, as filed. NOT an estimate by this tool.
    pub val_usd: f64,
    /// 1 = observable quoted price, 2 = observable inputs, 3 = UNOBSERVABLE inputs.
    /// Level 3 means the figure is the filer's own valuation, not a transaction price.
    pub fair_value_level: Option<u32>,
}

/// The result of a holdings survey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingsResult {
    pub holdings: Vec<FundHolding>,
    /// Filings whose XML could not be parsed, reported rather than silently dropped.
    pub unparsed: Vec<String>,
    /// Names with no holding found anywhere, and why that is a coverage limit.
    pub unfound: Vec<String>,
    /// Query failures.
    pub errors: Vec<String>,
}

/// Count NPORT-P filings naming `name`, and return their (filer, date, accession) so the XML
/// can be fetched. A failure is an error, never a zero.
pub fn find_filings(
    f: &Fetcher,
    name: &str,
    start: &str,
    end: &str,
    limit: usize,
) -> Result<Vec<(String, String, String)>, String> {
    let q = urlencode(&format!("\"{}\"", name));
    let url = format!(
        "https://efts.sec.gov/LATEST/search-index?q={}&forms=NPORT-P&dateRange=custom&startdt={}&enddt={}",
        q, start, end
    );
    let body = f.get(&url, UA)?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("unparseable search response: {}", e))?;
    let arr = v
        .get("hits")
        .and_then(|h| h.get("hits"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| "search response carried no hits array".to_string())?;
    let mut out = Vec::new();
    for h in arr.iter().take(limit) {
        let src = h.get("_source");
        let display = src
            .and_then(|s| s.get("display_names"))
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let date = src
            .and_then(|s| s.get("file_date"))
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let cik = src
            .and_then(|s| s.get("ciks"))
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(id) = h.get("_id").and_then(|x| x.as_str()) {
            out.push((format!("{}|{}", display, cik), date, id.to_string()));
        }
    }
    Ok(out)
}

/// Build the Archives URL for an N-PORT primary document from an EDGAR search id.
///
/// The search id has the form "0000035402-26-004745:primary_doc.xml": the part before the
/// colon is the accession number with dashes, the part after is the document name. The
/// directory path uses the accession with dashes REMOVED, and the CIK is unpadded.
pub fn document_url(cik: &str, accession_and_doc: &str) -> Option<String> {
    let (acc, doc) = accession_and_doc.split_once(':')?;
    let cik_unpadded = cik.trim_start_matches('0');
    let acc_nodash: String = acc.chars().filter(|c| *c != '-').collect();
    Some(format!(
        "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
        cik_unpadded, acc_nodash, doc
    ))
}

/// Extract holdings of the named private companies from an N-PORT primary document.
///
/// Parsing is deliberately regex-free where it matters little and literal where it matters
/// most: the XML is machine-generated and flat, but the fields are extracted by exact tag
/// match rather than a loose search, so a renamed field yields NO holding instead of a
/// silently wrong one. Missing `valUSD` means the holding is DROPPED and reported, not
/// reported as zero — a zero-dollar holding and an unreadable one are different facts.
pub fn parse_holdings(
    xml: &str,
    filer: &str,
    cik: &str,
    filed: &str,
    names: &[&str],
) -> (Vec<FundHolding>, usize) {
    let mut out = Vec::new();
    let mut dropped = 0usize;

    for block in split_blocks(xml, "invstOrSec") {
        let Some(filed_name) = extract(&block, "name") else {
            continue;
        };
        let upper = filed_name.to_uppercase();
        let Some(matched) = names.iter().find(|n| upper.contains(&n.to_uppercase())) else {
            continue;
        };
        let Some(val) = extract(&block, "valUSD").and_then(|v| v.parse::<f64>().ok()) else {
            // A holding we cannot price is DROPPED and counted, never recorded as zero.
            dropped += 1;
            continue;
        };
        let level = extract(&block, "fairValLevel").and_then(|v| v.trim().parse::<u32>().ok());
        out.push(FundHolding {
            filer: filer.to_string(),
            filer_cik: cik.to_string(),
            holding: matched.to_string(),
            filed_name: filed_name.trim().to_string(),
            filed: filed.to_string(),
            val_usd: val,
            fair_value_level: level,
        });
    }
    (out, dropped)
}

/// Split the document into `<tag>...</tag>` blocks. Written as a literal scan rather than a
/// regex so it is inspectable and has no backtracking behaviour to reason about.
fn split_blocks(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find(&open) {
        let after = &rest[i + open.len()..];
        match after.find(&close) {
            Some(j) => {
                out.push(after[..j].to_string());
                rest = &after[j + close.len()..];
            }
            None => break,
        }
    }
    out
}

fn extract(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let i = block.find(&open)? + open.len();
    let j = block[i..].find(&close)? + i;
    Some(block[i..j].to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
<edgarSubmission><formData><invstOrSecs>
<invstOrSec><name>CHARTER COMMUNICATIONS INC</name><valUSD>999.0</valUSD><fairValLevel>1</fairValLevel></invstOrSec>
<invstOrSec><name>ANTHROPIC PBC</name><title>ANTHROPIC PBC SERIES G PC PP</title><valUSD>2414941.0</valUSD><balance>4100</balance><fairValLevel>3</fairValLevel></invstOrSec>
<invstOrSec><name>OPENAI GROUP PBC</name><title>OPENAI GROUP PBC</title><valUSD>11049153.67</valUSD><fairValLevel>3</fairValLevel></invstOrSec>
<invstOrSec><name>SOME UNPRICED PRIVATE CO</name></invstOrSec>
</invstOrSecs></formData></edgarSubmission>"#;

    #[test]
    fn parses_only_the_private_ai_names_with_their_values() {
        let (h, dropped) =
            parse_holdings(SAMPLE, "FIDELITY", "320351", "2026-07-24", PRIVATE_AI_NAMES);
        assert_eq!(h.len(), 2, "only the two AI names should match: {:?}", h);
        assert!(h
            .iter()
            .any(|x| x.holding == "Anthropic" && (x.val_usd - 2414941.0).abs() < 1e-6));
        assert!(h.iter().any(|x| x.holding == "OpenAI"));
        // Charter is not an AI name and must not be picked up.
        assert!(!h.iter().any(|x| x.filed_name.contains("CHARTER")));
        // The unpriced private company is counted as dropped, not silently ignored.
        assert_eq!(
            dropped, 0,
            "the unpriced block is not an AI name, so it is not a drop"
        );
    }

    #[test]
    fn an_unpriced_holding_is_dropped_and_counted_never_zeroed() {
        let xml = r#"<invstOrSecs>
<invstOrSec><name>ANTHROPIC PBC</name></invstOrSec>
<invstOrSec><name>ANTHROPIC PBC</name><valUSD>5.0</valUSD></invstOrSec>
</invstOrSecs>"#;
        let (h, dropped) = parse_holdings(xml, "F", "1", "2026-01-01", PRIVATE_AI_NAMES);
        assert_eq!(h.len(), 1, "the priced one survives");
        assert_eq!(dropped, 1, "the unpriced one is DROPPED and counted");
        assert!(
            !h.iter().any(|x| x.val_usd == 0.0),
            "a zero must never stand in for an unknown value"
        );
    }

    #[test]
    fn fair_value_level_3_is_carried_through() {
        // The honesty-critical field: level 3 means unobservable inputs, i.e. the filer's own
        // valuation rather than a transaction price. Losing it would overstate solidity.
        let (h, _) = parse_holdings(SAMPLE, "FIDELITY", "320351", "2026-07-24", PRIVATE_AI_NAMES);
        assert!(
            h.iter().all(|x| x.fair_value_level == Some(3)),
            "these private holdings are all level 3: {:?}",
            h
        );
    }

    #[test]
    fn document_url_matches_the_archives_convention() {
        // Verified against a real filing: the accession path drops dashes and the CIK is
        // unpadded. Getting this wrong yields a 404, which would look like "no holdings".
        let u = document_url("0000320351", "0000035402-26-004745:primary_doc.xml").unwrap();
        assert_eq!(
            u,
            "https://www.sec.gov/Archives/edgar/data/320351/000003540226004745/primary_doc.xml"
        );
    }

    #[test]
    fn a_renamed_field_yields_no_holding_rather_than_a_wrong_one() {
        // Exact tag matching: if EDGAR renames valUSD, we must get NOTHING for that block
        // rather than accidentally matching some other numeric field.
        let xml = r#"<invstOrSecs>
<invstOrSec><name>ANTHROPIC PBC</name><someOtherValue>999999</someOtherValue></invstOrSec>
</invstOrSecs>"#;
        let (h, dropped) = parse_holdings(xml, "F", "1", "2026-01-01", PRIVATE_AI_NAMES);
        assert!(h.is_empty(), "no value field means no holding: {:?}", h);
        assert_eq!(dropped, 1);
    }
}
