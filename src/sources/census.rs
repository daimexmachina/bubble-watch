//! Census C30: data-center construction spending.
//!
//! WHY THIS MATTERS MORE THAN IT LOOKS. Every other indicator in this tool is a
//! price, a credit spread, or a company's financial statements — all of them
//! financial claims about the buildout. This is the buildout itself, measured in
//! dollars actually spent on steel and concrete by the US government's own
//! construction survey. It is the only PHYSICAL, financial-market-independent
//! series in the model, and it therefore cannot be moved by sentiment.
//!
//! Measured from this host 2026-09-17, from the Census C30 release:
//!
//!   data-center construction, full-year totals
//!     2015   $2,745M        2023  $19,995M
//!     2019   $8,483M        2024  $34,797M   (+74.0% YoY)
//!     2021   $9,949M        2025  $49,737M   (+42.9% YoY)
//!     2022  $12,583M        2026  $37,222M   (7 months)
//!
//!   latest month (Jul-2026, preliminary) $6,551M, +58.5% year over year.
//!
//! Census added "Data center" as its own line item only in the May 2024 release,
//! with estimates back to January 2014. That is a short history compared with the
//! market series, and it matters for the anchors: there is no data-center series
//! covering 2000 or 2008, so the scale describes the range THIS boom has produced
//! rather than a full cycle.
//!
//! WHY AN XML PARSER AND NOT A CSV. The C30 release is published as xlsx (and PDF)
//! with no CSV and no keyless API: the EITS API endpoint requires a key
//! (`api.census.gov/data/timeseries/eits/vip` returns a "Missing Key" page), and
//! FRED carries the aggregate construction series but NOT the data-center line
//! item. So the xlsx must be read, and an xlsx is a zip of XML. `flate2` and `zip`
//! are already direct dependencies for the Fed Z.1 work, so the incremental cost is
//! the parsing logic and nothing else.
//!
//! The parser is deliberately narrow: it finds the column whose header is
//! "Data center", then reads that column from every row by cell REFERENCE (the
//! `r="J5"` attribute) rather than by position. Reading by position would silently
//! misalign the moment Census inserts a column, which is exactly the class of bug
//! that has already bitten this project three times.

use crate::model::{Point, Provenance, Series};
use std::io::Read;

/// Current release of the private-construction time series (not seasonally
/// adjusted, monthly, millions of dollars). Census publishes a new file each month
/// under the same name.
pub const URL: &str = "https://www.census.gov/construction/c30/xlsx/privtime.xlsx";

const UA: &str = "bubble-watch/0.1 (research tool; contact via repository)";

/// The line item to extract.
pub const LINE_ITEM: &str = "Data center";

/// Fetch and parse the data-center construction series.
pub fn data_center(f: &crate::http::Fetcher) -> Result<Series, String> {
    let bytes = f.get_bytes(URL, UA)?;
    let (points, as_of) = parse_xlsx(&bytes, LINE_ITEM)?;
    if points.is_empty() {
        return Err(format!(
            "the C30 workbook parsed but contained no observations for '{}'",
            LINE_ITEM
        ));
    }
    Ok(Series {
        provenance: Provenance {
            source: "census-c30".into(),
            endpoint: format!("{} :: sheet Priv NSA :: column '{}'", URL, LINE_ITEM),
            as_of,
            retrieved_at: crate::now_iso8601(),
        },
        points,
    })
}

/// Collapse runs of whitespace to a single space, for header comparison.
fn normalise_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Column letter for a zero-based index: 0 -> A, 25 -> Z, 26 -> AA.
pub fn col_letter(mut i: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (i % 26) as u8) as char);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s
}

/// Zero-based index for a column letter. Inverse of `col_letter`.
pub fn col_index(letters: &str) -> Option<usize> {
    if letters.is_empty() || !letters.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let mut n = 0usize;
    for c in letters.chars() {
        n = n * 26 + (c as usize - 'A' as usize + 1);
    }
    Some(n - 1)
}

/// Read the shared-strings table and the first worksheet's `<sheetData>` out of an
/// xlsx held in memory. Shared by every xlsx source, so the workbook handling has
/// exactly one implementation.
pub fn open_xlsx(bytes: &[u8]) -> Result<(Vec<String>, String), String> {
    let mut ar = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("not a readable xlsx: {}", e))?;
    let mut read = |name: &str| -> Result<String, String> {
        let mut e = ar
            .by_name(name)
            .map_err(|e| format!("{} missing from the workbook: {}", name, e))?;
        let mut s = String::new();
        e.read_to_string(&mut s)
            .map_err(|e| format!("{} unreadable: {}", name, e))?;
        Ok(s)
    };
    let shared = read("xl/sharedStrings.xml")?;
    let sheet = read("xl/worksheets/sheet1.xml")?;
    let mut strings = Vec::new();
    let mut rest = shared.as_str();
    while let Some(i) = rest.find("<si>") {
        let body = &rest[i + 4..];
        let Some(end) = body.find("</si>") else { break };
        strings.push(si_text(&body[..end]));
        rest = &body[end + 5..];
    }
    Ok((strings, sheet))
}

/// Split a worksheet into its rows' inner XML.
pub fn xlsx_rows(sheet: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = sheet;
    while let Some(i) = rest.find("<row") {
        let body = &rest[i..];
        let Some(gt) = body.find('>') else { break };
        let Some(close) = body.find("</row>") else {
            break;
        };
        out.push(&body[gt + 1..close]);
        rest = &body[close + 6..];
    }
    out
}

/// Read one cell: (column letters, resolved text, numeric value).
///
/// Extracted as a single correct implementation because the inline version in the
/// two loops advanced by the wrong offset and silently skipped every other cell —
/// which made the first data row parse as all-empty while the header row happened
/// to work.
pub fn parse_cell(cell_xml: &str, strings: &[String]) -> (String, String, Option<f64>) {
    let letters: String = match cell_xml.find("r=\"") {
        Some(i) => cell_xml[i + 3..]
            .chars()
            .take_while(|c| c.is_ascii_uppercase())
            .collect(),
        None => String::new(),
    };
    // The <v> payload, whatever its type.
    let vtext: Option<&str> = cell_xml.find("<v>").and_then(|v| {
        cell_xml[v + 3..]
            .find("</v>")
            .map(|e| &cell_xml[v + 3..v + 3 + e])
    });
    if cell_xml.contains("t=\"s\"") {
        let text = vtext
            .and_then(|t| t.parse::<usize>().ok())
            .and_then(|i| strings.get(i).cloned())
            .unwrap_or_default();
        (letters, text, None)
    } else {
        let num = vtext.and_then(|t| t.trim().parse::<f64>().ok());
        (letters, vtext.unwrap_or("").trim().to_string(), num)
    }
}

/// Split a row's XML into its cell elements, advancing past each CLOSING tag.
pub fn row_cells(row: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(i) = rest.find("<c ") {
        let c = &rest[i..];
        let end = match c.find("</c>") {
            Some(e) => e + 4,
            None => c.find("/>").map(|e| e + 2).unwrap_or(c.len()),
        };
        out.push(&c[..end]);
        rest = &c[end..];
    }
    out
}

/// Extract `<t>` text from one `<si>` shared-string entry.
pub fn si_text(si: &str) -> String {
    let mut out = String::new();
    let mut rest = si;
    while let Some(i) = rest.find("<t") {
        let after = &rest[i..];
        let Some(gt) = after.find('>') else { break };
        let body = &after[gt + 1..];
        let Some(end) = body.find("</t>") else { break };
        out.push_str(&body[..end]);
        rest = &body[end + 4..];
    }
    // The sheet uses _x000D_ for embedded carriage returns in some headers.
    out.replace("_x000D_", " ")
        .replace('\n', " ")
        .trim()
        .to_string()
}

/// Parse the C30 workbook: find the target column by HEADER TEXT, then read that
/// column from every data row by cell reference.
pub fn parse_xlsx(bytes: &[u8], line_item: &str) -> Result<(Vec<Point>, String), String> {
    let mut ar = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("C30 workbook is not a readable xlsx: {}", e))?;

    let read =
        |ar: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str| -> Result<String, String> {
            let mut e = ar
                .by_name(name)
                .map_err(|e| format!("{} missing from the workbook: {}", name, e))?;
            let mut s = String::new();
            e.read_to_string(&mut s)
                .map_err(|e| format!("{} unreadable: {}", name, e))?;
            Ok(s)
        };

    let shared = read(&mut ar, "xl/sharedStrings.xml")?;
    let sheet = read(&mut ar, "xl/worksheets/sheet1.xml")?;

    // Shared strings, in index order.
    let strings: Vec<String> = {
        let mut v = Vec::new();
        let mut rest = shared.as_str();
        while let Some(i) = rest.find("<si>") {
            let body = &rest[i + 4..];
            let Some(end) = body.find("</si>") else { break };
            v.push(si_text(&body[..end]));
            rest = &body[end + 5..];
        }
        v
    };

    // Locate the header row: the first row whose cells include the line item text.
    let mut target_col: Option<usize> = None;
    let mut seen_header = false;
    let mut data_rows: Vec<&str> = Vec::new();
    let mut rest = sheet.as_str();
    while let Some(i) = rest.find("<row") {
        let body = &rest[i..];
        let Some(gt) = body.find('>') else { break };
        let Some(close) = body.find("</row>") else {
            break;
        };
        let row = &body[gt + 1..close];
        if seen_header {
            data_rows.push(row);
        } else {
            for cell in row_cells(row) {
                let (letters, text, _) = parse_cell(cell, &strings);
                // Whitespace-insensitive: the C30 header carries an embedded
                // carriage return that normalises to a variable number of spaces,
                // so a literal comparison would break on a formatting change.
                if normalise_ws(&text).eq_ignore_ascii_case(&normalise_ws(line_item)) {
                    target_col = col_index(&letters);
                    seen_header = true;
                }
            }
        }
        rest = &body[close + 6..];
    }

    let col = target_col.ok_or_else(|| {
        format!(
            "no column headed '{}' was found in the C30 workbook — the release format may have \
             changed, so this is reported as a failure rather than guessed at",
            line_item
        )
    })?;
    let want_letter = col_letter(col);

    let mut points = Vec::new();
    let mut newest_raw = String::new();
    let mut newest_key = (0i32, 0u32);
    let mut rows_without_value = 0usize;
    for row in data_rows {
        let mut date_raw = String::new();
        let mut value: Option<f64> = None;
        for cell in row_cells(row) {
            let (letters, text, num) = parse_cell(cell, &strings);
            if letters == "A" {
                date_raw = if text.is_empty() { String::new() } else { text };
            } else if letters == want_letter {
                value = num;
            }
        }
        let (Some(key), Some(v)) = (parse_month(&date_raw), value) else {
            if !date_raw.is_empty() {
                rows_without_value += 1;
            }
            continue;
        };
        if key > newest_key {
            newest_key = key;
            newest_raw = date_raw.clone();
        }
        points.push(Point {
            date: key_to_date(key),
            value: v * 1_000_000.0, // millions -> units
        });
    }
    // A column that exists but yields no numbers is a format problem, not a flat
    // series. Fail loudly rather than returning something that charts as zero.
    if points.is_empty() && rows_without_value > 0 {
        return Err(format!(
            "column '{}' was found but {} data rows had a date and no numeric value; the \
             release format may have changed",
            line_item, rows_without_value
        ));
    }

    points.sort_by(|a, b| a.date.cmp(&b.date));
    points.dedup_by(|a, b| a.date == b.date);
    Ok((points, newest_raw))
}

/// "Jul-26p" / "Jul-25r" / "Jul-26" -> (year, month). The trailing `p`/`r` marks
/// preliminary/revised and is ignored, since it does not change the period.
pub fn parse_month(s: &str) -> Option<(i32, u32)> {
    let s = s.trim();
    if s.len() < 5 {
        return None;
    }
    let mon = &s[..3];
    // The form is "Jul-26p": month, hyphen, two-digit year, optional p/r suffix.
    let rest = &s[3..];
    let yy = rest.strip_prefix('-')?;
    let yy = &yy[..2.min(yy.len())];
    if yy.len() != 2 || !yy.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year: i32 = yy.parse().ok()?;
    let month = match mon.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    Some((2000 + year, month))
}

fn key_to_date(key: (i32, u32)) -> String {
    format!("{:04}-{:02}-01", key.0, key.1)
}

/// Year-over-year growth using the value twelve months earlier, matched by DATE
/// rather than by index so a missing month cannot silently shift the comparison.
pub fn yoy(points: &[Point]) -> Option<f64> {
    let last = points.last()?;
    let (y, m): (i32, u32) = (last.date[0..4].parse().ok()?, last.date[5..7].parse().ok()?);
    let want = if y > 0 && m >= 1 {
        format!("{:04}-{:02}-01", y - 1, m)
    } else {
        return None;
    };
    let prior = points.iter().find(|p| p.date == want)?;
    if prior.value <= 0.0 {
        return None;
    }
    Some((last.value / prior.value - 1.0) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_letters_round_trip() {
        for i in [0usize, 1, 9, 25, 26, 27, 51, 52, 701] {
            let l = col_letter(i);
            assert_eq!(col_index(&l), Some(i), "col {} -> {} -> ?", i, l);
        }
        assert_eq!(col_letter(9), "J");
        assert_eq!(col_index("J"), Some(9));
    }

    #[test]
    fn month_parsing_handles_the_preliminary_and_revised_suffixes() {
        assert_eq!(parse_month("Jul-26p"), Some((2026, 7)));
        assert_eq!(parse_month("Jun-26r"), Some((2026, 6)));
        assert_eq!(parse_month("Jan-14"), Some((2014, 1)));
        assert_eq!(parse_month("garbage"), None);
        assert_eq!(parse_month("Xxx-26"), None);
    }

    #[test]
    fn shared_string_text_strips_embedded_carriage_returns() {
        assert_eq!(si_text("<t>Data_x000D_ center</t>"), "Data  center");
        assert_eq!(si_text("<t>Plain</t>"), "Plain");
        // rich-text runs
        assert_eq!(
            si_text("<r><t>Da</t></r><r><t>ta center</t></r>"),
            "Data center"
        );
    }

    #[test]
    fn a_missing_column_is_an_error_not_an_empty_series() {
        // A workbook with no matching header must FAIL loudly. Returning an empty
        // series would look like a collapse in construction spending.
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            use std::io::Write;
            w.start_file("xl/sharedStrings.xml", opts).unwrap();
            w.write_all(b"<sst><si><t>Date</t></si><si><t>Office</t></si></sst>")
                .unwrap();
            w.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            w.write_all(
                b"<worksheet><sheetData>\
                  <row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c></row>\
                  <row r=\"2\"><c r=\"A2\" t=\"s\"><v>0</v></c><c r=\"B2\"><v>100</v></c></row>\
                  </sheetData></worksheet>",
            )
            .unwrap();
            w.finish().unwrap();
        }
        let e = parse_xlsx(&buf, "Data center").unwrap_err();
        assert!(e.contains("no column headed"), "must say why: {}", e);
    }

    #[test]
    fn a_synthetic_workbook_parses_by_cell_reference() {
        // The target column is NOT contiguous with the date column, so this also
        // proves the reader indexes by reference rather than by position.
        let mut buf = Vec::new();
        {
            use std::io::Write;
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            w.start_file("xl/sharedStrings.xml", opts).unwrap();
            w.write_all(
                b"<sst><si><t>Date</t></si><si><t>Office</t></si><si><t>Data_x000D_center</t></si>\
                  <si><t>Jul-26p</t></si><si><t>Jul-25</t></si></sst>",
            )
            .unwrap();
            w.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            // header: A=Date, B=Office, C=Data center
            w.write_all(
                b"<worksheet><sheetData>\
                  <row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c><c r=\"C1\" t=\"s\"><v>2</v></c></row>\
                  <row r=\"2\"><c r=\"A2\" t=\"s\"><v>3</v></c><c r=\"B2\"><v>10683</v></c><c r=\"C2\"><v>6551</v></c></row>\
                  <row r=\"3\"><c r=\"A3\" t=\"s\"><v>4</v></c><c r=\"B3\"><v>8772</v></c><c r=\"C3\"><v>4133</v></c></row>\
                  </sheetData></worksheet>",
            )
            .unwrap();
            w.finish().unwrap();
        }
        let (pts, as_of) = parse_xlsx(&buf, "Data center").unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].date, "2025-07-01");
        assert!((pts[0].value - 4_133.0 * 1_000_000.0).abs() < 1.0);
        assert_eq!(pts[1].date, "2026-07-01");
        assert!((pts[1].value - 6_551.0 * 1_000_000.0).abs() < 1.0);
        assert_eq!(as_of, "Jul-26p");
    }

    #[test]
    fn yoy_matches_by_date_not_by_index() {
        // A missing month must NOT shift the comparison onto the wrong period.
        let pts: Vec<Point> = [
            ("2025-07-01", 100.0),
            ("2025-09-01", 120.0), // August absent
            ("2026-07-01", 150.0),
        ]
        .iter()
        .map(|(d, v)| Point {
            date: (*d).into(),
            value: *v,
        })
        .collect();
        let g = yoy(&pts).unwrap();
        assert!((g - 50.0).abs() < 1e-9, "expected +50%, got {}", g);
    }

    #[test]
    fn yoy_is_none_without_a_year_earlier() {
        let pts = vec![Point {
            date: "2026-07-01".into(),
            value: 150.0,
        }];
        assert!(yoy(&pts).is_none());
    }

    #[test]
    fn the_real_workbook_parses_when_present() {
        // Opt-in: the file is ~170KB and this must not slow the offline suite.
        let Ok(path) = std::env::var("BUBBLE_WATCH_C30_TEST") else {
            eprintln!("SKIP: set BUBBLE_WATCH_C30_TEST=/path/to/privtime.xlsx to exercise it");
            return;
        };
        let bytes = std::fs::read(&path).expect("workbook readable");
        let (pts, as_of) = parse_xlsx(&bytes, LINE_ITEM).expect("data-center column present");
        assert!(
            pts.len() > 100,
            "expected a long monthly history, got {}",
            pts.len()
        );
        assert!(pts.last().unwrap().value > 0.0);
        assert!(!as_of.is_empty());
        let g = yoy(&pts).expect("a year-earlier observation should exist");
        assert!(
            g > 0.0,
            "data-center construction has been growing, got {}%",
            g
        );
    }
}
