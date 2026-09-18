//! Fed Z.1 Financial Accounts: private-credit lending.
//!
//! WHY THIS EXISTS. The tool's credit indicators are public-bond spreads — ICE
//! BofA high-yield and investment-grade option-adjusted spreads. But BIS Bulletin
//! 120 (Jan 2026) documents that AI-related financing is shifting from operating
//! cash flow toward debt, with **private credit playing a rapidly increasing
//! role**, and private credit does not appear in an index spread at all. This
//! module measures the size of that channel directly.
//!
//! Measured from this host 2026-09-17, from the Fed's own F4.4 table:
//!
//!   All sectors; private credit loans     $1.1tn (2024:Q1) -> $1.5tn (2026:Q2)
//!   Nonfinancial corporate business       $0.8tn         -> $1.1tn
//!
//! i.e. roughly +36% in ten quarters, against a public high-yield index spread
//! that has barely moved. A growing share of the debt funding this buildout is
//! being extended outside the market the tool was watching.
//!
//! HOW IT IS OBTAINED, and why the dependency is justified. The Fed publishes Z.1
//! as a single 8MB zip containing 307 CSV tables, with NO per-table endpoint
//! (verified: every direct CSV path 404s, and the Data Download Program's
//! Output.aspx returns HTTP 400). FRED does NOT mirror these series — the
//! `FL893167205.Q` style identifiers return "series does not exist" on the keyed
//! API, and FRED's own "private credit" search returns only BIS *total credit*
//! aggregates, which are a different concept. So the archive must be read.
//!
//! The archive format is trivially simple and needs no CSV parser: a `date` column
//! followed by one column per series id, with values in millions of dollars. This
//! module reads the handful of columns it needs and ignores the rest.
//!
//! IMPORTANT LIMITATION, stated because it would be easy to overstate: Z.1 reports
//! private credit as a TOTAL for the whole economy. It is NOT AI-specific, and
//! nothing in the free data attributes a private loan to a data-centre or GPU
//! borrower. So this measures the size and growth of the financing CHANNEL, not
//! the amount of it reaching AI. A rise here is consistent with the BIS thesis but
//! does not confirm it.

use crate::model::{Point, Provenance, Series};
use std::io::Read;

/// URL of the current Z.1 CSV bundle. The dated form
/// `.../releases/z1/YYYYMMDD/z1_csv_files.zip` is stable per release if a specific
/// vintage is ever needed; `current` is the rolling latest.
pub const Z1_URL: &str = "https://www.federalreserve.gov/releases/z1/current/z1_csv_files.zip";

/// The table holding private-credit loan levels.
const TABLE: &str = "csv/F4_4_s.csv";

/// Series id for all sectors' private-credit loan liability (the headline).
pub const SERIES_ALL_SECTORS: &str = "FL893167205.Q";
/// Series id for the nonfinancial corporate share.
pub const SERIES_CORPORATE: &str = "FL103167205.Q";

/// A descriptive UA is required by federal endpoints.
const UA: &str = "bubble-watch/0.1 (research tool; contact via repository)";

/// Fetch the Z.1 bundle and extract one series from the F4.4 table.
///
/// Returns the series as a `Series` so it carries the same provenance contract as
/// every other retrieved datum.
pub fn private_credit(
    f: &crate::http::Fetcher,
    series_id: &str,
    label: &str,
) -> Result<Series, String> {
    let bytes = f.get_bytes(Z1_URL, UA)?;
    let text = read_zip_entry(&bytes, TABLE)?;
    let points = parse_series(&text, series_id)?;
    if points.is_empty() {
        return Err(format!(
            "series {} ({}) was found in {} but contained no observations",
            series_id, label, TABLE
        ));
    }
    let last = points.last().map(|p| p.date.clone()).unwrap_or_default();
    Ok(Series {
        provenance: Provenance {
            source: "fed-z1".into(),
            endpoint: format!("{} :: {} :: {}", Z1_URL, TABLE, series_id),
            as_of: last,
            retrieved_at: crate::now_iso8601(),
        },
        points,
    })
}

/// Extract one entry from a zip archive held in memory.
pub fn read_zip_entry(bytes: &[u8], name: &str) -> Result<String, String> {
    let mut ar = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Z.1 bundle is not a readable zip: {}", e))?;
    let mut entry = ar
        .by_name(name)
        .map_err(|e| format!("{} is not present in the Z.1 bundle: {}", name, e))?;
    let mut s = String::new();
    entry
        .read_to_string(&mut s)
        .map_err(|e| format!("{} could not be read: {}", name, e))?;
    Ok(s)
}

/// Parse one series column out of a Z.1 table.
///
/// The format is a header of `date,<series id>,<series id>,...` followed by rows
/// of `YYYY:Qn,<value>,...`, with values in MILLIONS of dollars. Kept pure and
/// explicit so it is testable without the network.
pub fn parse_series(text: &str, series_id: &str) -> Result<Vec<Point>, String> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| "Z.1 table is empty".to_string())?;
    let cols: Vec<&str> = header.split(',').map(|c| c.trim()).collect();
    let idx = cols.iter().position(|c| *c == series_id).ok_or_else(|| {
        format!(
            "series {} is not a column in this table (columns: {})",
            series_id,
            cols.iter()
                .take(6)
                .copied()
                .collect::<Vec<&str>>()
                .join(", ")
        )
    })?;

    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        let Some(date) = it.next() else { continue };
        // Walk to the requested column rather than collecting the whole row, so a
        // short or ragged line skips rather than misaligns.
        let value = match it.nth(idx - 1) {
            Some(v) => v.trim(),
            None => continue,
        };
        // Z.1 uses an empty cell for "not available", never a zero.
        if value.is_empty() {
            continue;
        }
        let Ok(v) = value.parse::<f64>() else {
            continue;
        };
        let Some(date) = normalise_date(date.trim()) else {
            continue;
        };
        // Convert millions to units, so this series is comparable with the rest.
        out.push(Point {
            date,
            value: v * 1_000_000.0,
        });
    }
    if out.is_empty() {
        return Err(format!(
            "series {} had a matching column but no usable observations",
            series_id
        ));
    }
    Ok(out)
}

/// "2026:Q2" -> "2026-06-30". Z.1 is a quarterly series and the crate's `Point`
/// carries an ISO date, so quarters are mapped to their closing date rather than
/// inventing a new date type.
pub fn normalise_date(q: &str) -> Option<String> {
    let (y, qq) = q.split_once(":Q")?;
    let year: i32 = y.trim().parse().ok()?;
    let qn: u32 = qq.trim().parse().ok()?;
    let (m, d) = match qn {
        1 => (3, 31),
        2 => (6, 30),
        3 => (9, 30),
        4 => (12, 31),
        _ => return None,
    };
    Some(format!("{:04}-{:02}-{:02}", year, m, d))
}

/// Trailing-twelve-month growth of a quarterly series, as a percentage.
///
/// Uses the same value four quarters earlier. Returns None when the series is too
/// short rather than guessing from fewer points.
pub fn yoy_growth(points: &[Point]) -> Option<f64> {
    if points.len() < 5 {
        return None;
    }
    let last = points.last()?;
    let prior = points.get(points.len() - 5)?;
    if prior.value <= 0.0 {
        return None;
    }
    Some((last.value / prior.value - 1.0) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "date,FL893167205.Q,FL103167205.Q,FL113167205.Q\n\
2025:Q3,1410649,1031072,189000\n\
2025:Q4,1493510,1087896,195000\n\
2026:Q1,1515776,1104955,198000\n\
2026:Q2,1539920,1120845,201000\n";

    #[test]
    fn parses_the_headline_series_from_a_z1_table() {
        let pts = parse_series(SAMPLE, SERIES_ALL_SECTORS).unwrap();
        assert_eq!(pts.len(), 4);
        assert_eq!(pts[0].date, "2025-09-30");
        assert_eq!(pts[3].date, "2026-06-30");
        // millions -> units
        assert!((pts[3].value - 1_539_920.0 * 1_000_000.0).abs() < 1.0);
    }

    #[test]
    fn selects_the_requested_column_not_the_first() {
        let pts = parse_series(SAMPLE, SERIES_CORPORATE).unwrap();
        assert!((pts[3].value - 1_120_845.0 * 1_000_000.0).abs() < 1.0);
    }

    #[test]
    fn an_absent_series_is_an_error_not_an_empty_series() {
        // Distinguishing "not in this table" from "present but empty" matters: an
        // empty series would silently look like a collapsed credit channel.
        let e = parse_series(SAMPLE, "FL999999999.Q").unwrap_err();
        assert!(e.contains("not a column"), "must say why: {}", e);
    }

    #[test]
    fn empty_cells_are_skipped_not_read_as_zero() {
        // Z.1 marks unavailable with an empty cell. Reading it as 0 would draw the
        // series to the floor and look like a collapse in lending.
        let t = "date,FL893167205.Q\n2026:Q1,1500000\n2026:Q2,\n";
        let pts = parse_series(t, SERIES_ALL_SECTORS).unwrap();
        assert_eq!(pts.len(), 1, "the empty cell must be skipped, not zeroed");
        assert!(pts[0].value > 0.0);
    }

    #[test]
    fn ragged_lines_are_skipped_rather_than_misaligned() {
        let t = "date,FL893167205.Q,FL103167205.Q\n2026:Q1,1500000,1100000\n2026:Q2,1510000\n";
        let pts = parse_series(t, SERIES_CORPORATE).unwrap();
        assert_eq!(pts.len(), 1, "the short row must be skipped");
    }

    #[test]
    fn dates_are_mapped_to_quarter_ends() {
        assert_eq!(normalise_date("2026:Q1").unwrap(), "2026-03-31");
        assert_eq!(normalise_date("2026:Q2").unwrap(), "2026-06-30");
        assert_eq!(normalise_date("2026:Q3").unwrap(), "2026-09-30");
        assert_eq!(normalise_date("2026:Q4").unwrap(), "2026-12-31");
        assert!(normalise_date("2026:Q9").is_none());
        assert!(normalise_date("nonsense").is_none());
    }

    #[test]
    fn yoy_growth_uses_four_quarters_back() {
        let pts: Vec<Point> = [100.0, 105.0, 110.0, 115.0, 120.0]
            .iter()
            .enumerate()
            .map(|(i, v)| Point {
                date: format!("202{}-01-01", 5 + i),
                value: *v,
            })
            .collect();
        // 100 -> 120 is +20%
        let g = yoy_growth(&pts).unwrap();
        assert!((g - 20.0).abs() < 1e-9, "got {}", g);
    }

    #[test]
    fn yoy_growth_refuses_a_series_that_is_too_short() {
        let pts = vec![
            Point {
                date: "2026-01-01".into(),
                value: 1.0,
            },
            Point {
                date: "2026-04-01".into(),
                value: 2.0,
            },
        ];
        assert!(yoy_growth(&pts).is_none());
    }

    #[test]
    fn a_real_z1_bundle_can_be_read_when_present() {
        // Opt-in: the archive is 8MB and this must not slow the offline suite.
        let path = std::env::var("BUBBLE_WATCH_Z1_TEST");
        let Ok(path) = path else {
            eprintln!("SKIP: set BUBBLE_WATCH_Z1_TEST=/path/to/z1.zip to exercise the real bundle");
            return;
        };
        let bytes = std::fs::read(&path).expect("bundle readable");
        let text = read_zip_entry(&bytes, TABLE).expect("F4.4_s present");
        let pts = parse_series(&text, SERIES_ALL_SECTORS).expect("headline series present");
        assert!(
            pts.len() > 200,
            "expected a long quarterly history, got {}",
            pts.len()
        );
        assert!(pts.last().unwrap().value > 0.0);
    }
}
