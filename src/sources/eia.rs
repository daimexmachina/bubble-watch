//! EIA-860M: planned vs cancelled electricity generating capacity.
//!
//! WHY THIS EXISTS. The power grid is the physical constraint on the AI buildout —
//! data centres need enormous amounts of firm electricity, and interconnection
//! queues are the practical bottleneck. This measures the pipeline of generation
//! that developers have ANNOUNCED and then ABANDONED, which is the leading edge of a
//! capex cycle turning. Cancelled capacity is intent that did not survive contact
//! with reality, and it shows up before the spending does.
//!
//! Measured from this host 2026-09-17, from the July 2026 EIA-860M release:
//!
//!   Operating                    28,318 units   1,408,847 MW
//!   Planned                       2,341 units     291,138 MW
//!   Canceled or Postponed         1,734 units     184,141 MW
//!   Retired                       7,299 units     296,459 MW
//!
//!   cancellation ratio = cancelled / (planned + cancelled) = 38.7%
//!
//!   cancelled capacity by technology, largest first:
//!     natural gas combined cycle  59,466 MW
//!     natural gas combustion turbine 31,064 MW
//!     solar photovoltaic          28,517 MW
//!     onshore wind                15,752 MW
//!     conventional steam coal     15,409 MW
//!
//! WHAT THE CANCELLATION RATIO DOES AND DOES NOT SAY, because this is easy to
//! over-read. The sheets are cumulative inventories, not a flow: "Canceled or
//! Postponed" accumulates every cancelled project currently listed, so a HIGH ratio
//! means many announced projects have been abandoned over the whole accumulation
//! window, NOT that cancellations are spiking right now. This tool measures a LEVEL
//! of abandoned intent, not a rate of change, and the output says so. Detecting the
//! RATE would need successive monthly releases compared against each other, which is
//! a deliberate future step rather than something to imply now.
//!
//! The same caveat applies to "Planned": it is everything currently announced,
//! including projects that will themselves be cancelled.
//!
//! SOURCE. EIA publishes only xlsx, keyless-API-inaccessible (`api.eia.gov/v2`
//! returns a "Missing Key" page and no EIA key is configured), so the workbook is
//! read directly with the shared xlsx reader built for the Census C30 work. The
//! archive exposes a stable dated pattern, so a specific vintage can be pinned:
//! `.../eia860m/archive/xls/<month>_generator<year>.xlsx`.

use crate::model::{Point, Provenance, Series};

/// Current release. EIA moves the month, so this is the one guessed filename in the
/// project and it is verified to exist rather than assumed present.
pub const URL: &str = "https://www.eia.gov/electricity/data/eia860m/xls/july_generator2026.xlsx";

/// Stable dated pattern for pinning a specific vintage.
pub fn archive_url(month: &str, year: u32) -> String {
    format!(
        "https://www.eia.gov/electricity/data/eia860m/archive/xls/{}_generator{}.xlsx",
        month, year
    )
}

const UA: &str = "bubble-watch/0.1 (research tool; contact via repository)";

/// Header text identifying the capacity column, matched whitespace-insensitively.
const MW_HEADER: &str = "Nameplate Capacity";
const TECH_HEADER: &str = "Technology";

/// One row of the inventory, reduced to what this module needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub capacity_mw: f64,
    pub technology: String,
}

/// Parse one sheet of an EIA-860M workbook.
///
/// Finds the header row by looking for the capacity column rather than assuming a
/// fixed row index, so a release that inserts a title line does not silently
/// misparse into an empty inventory.
pub fn parse_sheet(sheet: &str, strings: &[String]) -> Result<Vec<Unit>, String> {
    let rows = crate::sources::census::xlsx_rows(sheet);
    let mut mw_col: Option<usize> = None;
    let mut tech_col: Option<usize> = None;
    let mut header_seen = false;
    let mut out = Vec::new();

    for row in rows {
        let cells: Vec<(String, String, Option<f64>)> = crate::sources::census::row_cells(row)
            .iter()
            .map(|c| crate::sources::census::parse_cell(c, strings))
            .collect();

        if !header_seen {
            // The header row is the first one that names the capacity column.
            let hit = cells
                .iter()
                .position(|(_, t, _)| normalise(t).starts_with(&normalise(MW_HEADER)));
            if let Some(i) = hit {
                mw_col = Some(i);
                tech_col = cells
                    .iter()
                    .position(|(_, t, _)| normalise(t).starts_with(&normalise(TECH_HEADER)));
                header_seen = true;
            }
            continue;
        }

        let Some(mi) = mw_col else { continue };
        let Some((_, _, Some(mw))) = cells.get(mi).cloned() else {
            continue;
        };
        if mw <= 0.0 {
            continue;
        }
        let tech = tech_col
            .and_then(|ti| cells.get(ti))
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default();
        out.push(Unit {
            capacity_mw: mw,
            technology: tech,
        });
    }

    if !header_seen {
        return Err(format!(
            "no header row naming '{}' was found in this EIA-860M sheet; the release format may \
             have changed, so this is a failure rather than an empty inventory",
            MW_HEADER
        ));
    }
    Ok(out)
}

fn normalise(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Total capacity in MW.
pub fn total_mw(units: &[Unit]) -> f64 {
    units.iter().map(|u| u.capacity_mw).sum()
}

/// Cancellation ratio: cancelled / (planned + cancelled), as a percentage.
///
/// Returns None when either side is empty, because a ratio against zero is not a
/// reading — it is a division by nothing, and reporting 100% would be the most
/// alarming possible answer to a missing dataset.
pub fn cancellation_ratio(planned: &[Unit], cancelled: &[Unit]) -> Option<f64> {
    let p = total_mw(planned);
    let c = total_mw(cancelled);
    if p <= 0.0 || c <= 0.0 {
        return None;
    }
    Some(c / (p + c) * 100.0)
}

/// Fetch the planned and cancelled inventories in one archive download.
pub fn planned_vs_cancelled(f: &crate::http::Fetcher) -> Result<(Series, Series, String), String> {
    let bytes = f.get_bytes(URL, UA)?;
    let (strings, _) = crate::sources::census::open_xlsx(&bytes)?;

    // The workbook holds several sheets; the reader above returns the first. EIA's
    // sheet order is stable (Operating, Planned, Retired, Canceled or Postponed), so
    // the sheets are located by NAME rather than by position.
    let names = sheet_names(&bytes)?;
    let find =
        |want: &str| -> Option<usize> { names.iter().position(|n| n.eq_ignore_ascii_case(want)) };
    let planned_idx =
        find("Planned").ok_or_else(|| "no 'Planned' sheet in the workbook".to_string())?;
    let cancelled_idx = find("Canceled or Postponed")
        .ok_or_else(|| "no 'Canceled or Postponed' sheet".to_string())?;

    let planned = parse_sheet(&sheet_xml(&bytes, planned_idx)?.0, &strings)?;
    let (cancelled_xml, cancelled_strings) = sheet_xml(&bytes, cancelled_idx)?;
    let cancelled = parse_sheet(&cancelled_xml, &cancelled_strings)?;

    let p_mw = total_mw(&planned);
    let c_mw = total_mw(&cancelled);
    if p_mw <= 0.0 || c_mw <= 0.0 {
        return Err(format!(
            "planned {} MW and cancelled {} MW were parsed; one side being zero means the \
             workbook layout changed rather than that nothing is planned",
            p_mw, c_mw
        ));
    }
    let ratio = cancellation_ratio(&planned, &cancelled)
        .ok_or_else(|| "cancellation ratio could not be formed".to_string())?;

    let as_of = "July 2026".to_string();
    let mk = |label: &str, mw: f64| Series {
        provenance: Provenance {
            source: "eia-860m".into(),
            endpoint: format!("{} :: sheet '{}' :: {} MW", URL, label, mw),
            as_of: as_of.clone(),
            retrieved_at: crate::now_iso8601(),
        },
        points: vec![Point {
            date: "2026-07-01".into(),
            value: mw,
        }],
    };
    Ok((
        mk("Planned", p_mw),
        mk("Canceled or Postponed", c_mw),
        format!("{:.1}", ratio),
    ))
}

/// Sheet names, in workbook order.
fn sheet_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut ar = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("not a readable xlsx: {}", e))?;
    let mut e = ar
        .by_name("xl/workbook.xml")
        .map_err(|e| format!("no workbook.xml: {}", e))?;
    let mut s = String::new();
    use std::io::Read;
    e.read_to_string(&mut s).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut rest = s.as_str();
    while let Some(i) = rest.find("<sheet ") {
        let body = &rest[i..];
        let Some(end) = body.find("/>") else { break };
        let tag = &body[..end];
        if let Some(ni) = tag.find("name=\"") {
            let after = &tag[ni + 6..];
            if let Some(q) = after.find('"') {
                out.push(after[..q].to_string());
            }
        }
        rest = &body[end + 2..];
    }
    Ok(out)
}

/// The worksheet XML at a given sheet index.
///
/// EIA-860M and the Census workbook differ here: Census puts its only sheet at
/// `sheet1.xml`, while EIA has seven sheets at `sheetN.xml`. The index is 1-based in
/// the filename, so the caller passes a 0-based index and this adds one.
fn sheet_xml(bytes: &[u8], idx0: usize) -> Result<(String, Vec<String>), String> {
    let mut ar = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("not a readable xlsx: {}", e))?;
    let name = format!("xl/worksheets/sheet{}.xml", idx0 + 1);
    let mut s = String::new();
    {
        let mut e = ar
            .by_name(&name)
            .map_err(|e| format!("{} not in the workbook: {}", name, e))?;
        use std::io::Read;
        e.read_to_string(&mut s).map_err(|e| e.to_string())?;
    }
    let mut shared = String::new();
    if let Ok(mut e) = ar.by_name("xl/sharedStrings.xml") {
        use std::io::Read;
        e.read_to_string(&mut shared).map_err(|e| e.to_string())?;
    }
    let mut strings = Vec::new();
    let mut rest = shared.as_str();
    while let Some(i) = rest.find("<si>") {
        let body = &rest[i + 4..];
        let Some(end) = body.find("</si>") else { break };
        strings.push(crate::sources::census::si_text(&body[..end]));
        rest = &body[end + 5..];
    }
    Ok((s, strings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(mw: f64, tech: &str) -> Unit {
        Unit {
            capacity_mw: mw,
            technology: tech.into(),
        }
    }

    #[test]
    fn cancellation_ratio_is_cancelled_over_the_sum() {
        let p = vec![unit(75.0, "Solar")];
        let c = vec![unit(25.0, "Gas")];
        assert!((cancellation_ratio(&p, &c).unwrap() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn cancellation_ratio_refuses_a_zero_denominator() {
        // Reporting 100% here would be the most alarming possible answer to a
        // missing dataset, which is exactly the wrong failure.
        let p: Vec<Unit> = vec![];
        let c = vec![unit(25.0, "Gas")];
        assert!(cancellation_ratio(&p, &c).is_none());
        let p2 = vec![unit(75.0, "Solar")];
        let c2: Vec<Unit> = vec![];
        assert!(cancellation_ratio(&p2, &c2).is_none());
    }

    #[test]
    fn totals_sum_capacity() {
        let u = vec![unit(10.0, "a"), unit(32.5, "b")];
        assert!((total_mw(&u) - 42.5).abs() < 1e-9);
    }

    #[test]
    fn a_sheet_without_the_capacity_header_is_an_error() {
        let mut buf = Vec::new();
        {
            use std::io::Write;
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            w.start_file("xl/sharedStrings.xml", opts).unwrap();
            w.write_all(b"<sst><si><t>Something</t></si></sst>")
                .unwrap();
            w.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            w.write_all(
                b"<worksheet><sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c></row>\
                  </sheetData></worksheet>",
            )
            .unwrap();
            w.finish().unwrap();
        }
        let (strings, sheet) = crate::sources::census::open_xlsx(&buf).unwrap();
        let e = parse_sheet(&sheet, &strings).unwrap_err();
        assert!(e.contains("no header row"), "must say why: {}", e);
    }

    #[test]
    fn archive_url_follows_eias_dated_pattern() {
        assert_eq!(
            archive_url("july", 2026),
            "https://www.eia.gov/electricity/data/eia860m/archive/xls/july_generator2026.xlsx"
        );
    }

    #[test]
    fn the_real_workbook_parses_when_present() {
        let Ok(path) = std::env::var("BUBBLE_WATCH_EIA_TEST") else {
            eprintln!("SKIP: set BUBBLE_WATCH_EIA_TEST=/path/to/eia860m.xlsx to exercise it");
            return;
        };
        let bytes = std::fs::read(&path).expect("workbook readable");
        let names = sheet_names(&bytes).expect("sheet names");
        assert!(names.iter().any(|n| n == "Planned"), "sheets: {:?}", names);
        let pi = names.iter().position(|n| n == "Planned").unwrap();
        let (px, ps) = sheet_xml(&bytes, pi).unwrap();
        let planned = parse_sheet(&px, &ps).unwrap();
        assert!(
            total_mw(&planned) > 100_000.0,
            "planned capacity should be large, got {} MW",
            total_mw(&planned)
        );

        let ci = names
            .iter()
            .position(|n| n == "Canceled or Postponed")
            .unwrap();
        let (cx, cs) = sheet_xml(&bytes, ci).unwrap();
        let cancelled = parse_sheet(&cx, &cs).unwrap();
        assert!(total_mw(&cancelled) > 10_000.0);

        let r = cancellation_ratio(&planned, &cancelled).unwrap();
        assert!(r > 0.0 && r < 100.0, "ratio out of range: {}", r);
    }
}
