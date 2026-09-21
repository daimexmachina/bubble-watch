//! LBNL interconnection queue — how long it actually takes to energise a project.
//!
//! ## What this measures, and why it belongs here
//!
//! Every other physical indicator in this model counts what is BUILT or SPENT
//! (data-centre construction, cancelled capacity). None measures the **delay
//! between proposing a project and energising it**, which is the binding
//! constraint on whether announced AI capacity can arrive at all.
//!
//! ## It reads AGAINST the thesis, and that is the point
//!
//! A lengthening queue is physical evidence that demand for power is real and
//! ahead of supply: projects are waiting, not being abandoned. That is a
//! supply-constraint reading, not a demand-failure one, so a LONGER delay argues
//! the capex is being absorbed rather than that the boom is hollow. It therefore
//! doubles as falsification evidence, which the project's own plan called thin.
//!
//! ## Verified behaviour (probed live 2026-09-20)
//!
//! | URL | UA | status | size |
//! |---|---|---|---|
//! | `emp.lbl.gov/sites/default/files/2026-05/LBNL_Ix_Queue_Data_File_thru2025.xlsx` | browser | **200** | **15,571,236 B** |
//! | same URL | none / curl default | **403** | 5,644 B |
//! | `emp.lbl.gov/queues` (landing page) | any | **403** | Cloudflare |
//!
//! ⚠ **The browser User-Agent is REQUIRED.** Without it the file 403s, which is
//! what made this source look dead on first check. The landing page is Cloudflare-
//! blocked regardless; only the data file is reachable.
//!
//! ## The series (parsed from the raw workbook, 2026-09-20)
//!
//! Sheet **40**, named `37. IR to COD - all` — NOT sheet1, which is an
//! introduction. Median months from interconnection request to commercial
//! operation, by in-service year:
//!
//! | year | n | median |
//! |---|---|---|
//! | 2005 | 63 | 17.7 |
//! | 2020 | 227 | 46.1 |
//! | 2024 | 335 | 62.7 |
//! | **2025** | **294** | **60.8** |

use crate::http::{Fetcher, UA_WEB};

/// The queue-data workbook. Reached only with a browser User-Agent.
pub const URL: &str =
    "https://emp.lbl.gov/sites/default/files/2026-05/LBNL_Ix_Queue_Data_File_thru2025.xlsx";

/// The sheet holding median request-to-operation months by in-service year.
///
/// Located by NAME, not position: the workbook has 43 sheets and this is #40.
/// A positional lookup against sheet1 silently reads an introduction page.
pub const SHEET: &str = "37. IR to COD - all";

/// Median months from interconnection request to commercial operation, one year.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnergisationYear {
    pub year: i32,
    /// Projects in the cohort.
    pub n: u32,
    pub mean_months: f64,
    pub median_months: f64,
    pub p75_months: f64,
}

/// Parse the rows of the IR-to-COD sheet into per-year medians.
///
/// The sheet interleaves several tables, so this cannot assume a single header
/// row. Instead it takes every row whose FIRST cell is a plausible four-digit
/// in-service year and whose remaining cells are the (n, mean, p25, median, p75)
/// figures. That is robust to the sheet carrying additional summary tables above
/// or below, which it does.
///
/// Returns an error when no year rows are found, rather than an empty series: an
/// empty series would read as "no queue delay", which is the opposite of a gap.
pub fn parse_sheet(sheet: &str, strings: &[String]) -> Result<Vec<EnergisationYear>, String> {
    let mut out = Vec::new();
    for row in crate::sources::census::xlsx_rows(sheet) {
        let cells = row_cells(row, strings);
        if cells.is_empty() {
            continue;
        }
        let Some(year) = cells[0].parse::<i32>().ok() else {
            continue;
        };
        // A plausible in-service year. The bound also excludes the small integers
        // the sheet uses for counts in its other tables.
        if !(1990..=2100).contains(&year) {
            continue;
        }
        // Expect: year, n, mean, p25, median, p75 — the observed layout.
        if cells.len() < 5 {
            continue;
        }
        let nums: Vec<Option<f64>> = cells[1..].iter().map(|c| c.parse::<f64>().ok()).collect();
        let (n, mean, median, p75) = match (nums.first(), nums.get(1), nums.get(3), nums.get(4)) {
            (Some(Some(n)), Some(Some(mean)), Some(Some(med)), Some(Some(p75))) => {
                (*n, *mean, *med, *p75)
            }
            _ => continue,
        };
        out.push(EnergisationYear {
            year,
            n: n as u32,
            mean_months: mean,
            median_months: median,
            p75_months: p75,
        });
    }
    if out.is_empty() {
        return Err(format!(
            "no year rows found in the '{SHEET}' sheet — the workbook layout may have changed"
        ));
    }
    out.sort_by_key(|e| e.year);
    Ok(out)
}

/// Extract a row's cell values, resolving shared-string references.
fn row_cells(row: &str, strings: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(i) = rest.find("<c") {
        let body = &rest[i..];
        let Some(gt) = body.find('>') else { break };
        let head = &body[..gt];
        let Some(end) = body.find("</c>") else { break };
        let inner = &body[gt + 1..end];
        let is_str = head.contains("t=\"s\"");
        let v = inner
            .find("<v>")
            .and_then(|a| {
                let after = &inner[a + 3..];
                after.find("</v>").map(|b| after[..b].to_string())
            })
            .unwrap_or_default();
        let val = if is_str {
            v.parse::<usize>()
                .ok()
                .and_then(|idx| strings.get(idx).cloned())
                .unwrap_or_default()
        } else {
            v
        };
        out.push(val);
        rest = &body[end + 4..];
    }
    out
}

/// Year-over-year change in median months, between the two most recent years that
/// carry a usable cohort.
///
/// Returns `None` when fewer than two years are usable. The cohort size matters:
/// a median from a handful of projects is noise, so years with tiny cohorts are
/// skipped rather than allowed to set the trend.
pub fn recent_change(years: &[EnergisationYear], min_cohort: u32) -> Option<(i32, f64, i32, f64)> {
    let usable: Vec<&EnergisationYear> = years.iter().filter(|y| y.n >= min_cohort).collect();
    if usable.len() < 2 {
        return None;
    }
    let last = usable[usable.len() - 1];
    let prev = usable[usable.len() - 2];
    Some((prev.year, prev.median_months, last.year, last.median_months))
}

/// Fetch and parse the queue workbook.
pub fn fetch(f: &Fetcher) -> Result<Vec<EnergisationYear>, String> {
    // UA_WEB is a browser-like agent, which this host REQUIRES: without it the
    // request 403s. That is not a politeness preference here, it is the difference
    // between data and a block page.
    let bytes = f.get_bytes(URL, UA_WEB)?;
    let names = crate::sources::eia::sheet_names(&bytes)?;
    let idx = names
        .iter()
        .position(|n| n.trim() == SHEET)
        .ok_or_else(|| {
            format!(
                "the '{SHEET}' sheet is not in the workbook; the release layout may have changed. \
                 Sheets present: {}",
                names.len()
            )
        })?;
    let (xml, strings) = crate::sources::eia::sheet_xml(&bytes, idx)?;
    parse_sheet(&xml, &strings)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row in the shape `census::xlsx_rows` actually returns: cells WRAPPED in
    /// `<row>`.
    ///
    /// My first version emitted bare `<c>` cells and every test failed with "no
    /// year rows" — the helper had been written to match my assumption rather than
    /// the real parser's contract. `xlsx_rows` finds `<row`, takes everything up to
    /// `</row>`, so an unwrapped row is invisible to it.
    fn numeric_row(vals: &[&str]) -> String {
        let cells: String = vals
            .iter()
            .map(|v| format!("<c><v>{}</v></c>", v))
            .collect();
        format!("<row>{}</row>", cells)
    }

    #[test]
    fn year_rows_are_extracted_with_their_medians() {
        // Mirrors the real layout: year, n, mean, p25, median, p75.
        let sheet = numeric_row(&["2024", "335", "62.08", "42.94", "62.71", "80.26"]);
        let out = parse_sheet(&sheet, &[]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].year, 2024);
        assert_eq!(out[0].n, 335);
        assert!((out[0].median_months - 62.71).abs() < 1e-9);
    }

    #[test]
    fn the_real_measured_values_reproduce() {
        // Taken from the actual workbook on 2026-09-20. If the parse drifts, this
        // fails rather than quietly reporting a different number.
        let sheet = format!(
            "{}{}{}",
            numeric_row(&["2023", "265", "57.84", "40.87", "55.75", "73.00"]),
            numeric_row(&["2024", "335", "62.08", "42.94", "62.71", "80.26"]),
            numeric_row(&["2025", "294", "62.44", "42.28", "60.82", "86.07"])
        );
        let out = parse_sheet(&sheet, &[]).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[2].year, 2025);
        assert_eq!(out[2].n, 294);
        assert!((out[2].median_months - 60.82).abs() < 0.01);
    }

    #[test]
    fn other_tables_in_the_same_sheet_are_ignored() {
        // The sheet interleaves tables and carries header rows. Only rows whose
        // first cell is a plausible YEAR may be taken: a small integer from a
        // count column must not be read as a year.
        let sheet = format!(
            "<row><c t=\"s\"><v>0</v></c>{}</row>{}",
            // a count table whose first cell is 12, not a year
            {
                let cells: String = ["12", "8", "19"]
                    .iter()
                    .map(|v| format!("<c><v>{}</v></c>", v))
                    .collect();
                cells
            },
            // and a count-only table below it
            numeric_row(&["8", "19", "14"])
        );
        let e = parse_sheet(&sheet, &["Year".to_string()]).unwrap_err();
        assert!(
            e.contains("no year rows"),
            "must refuse rather than invent a year row: {}",
            e
        );
    }

    #[test]
    fn a_row_with_too_few_cells_is_skipped() {
        let sheet = numeric_row(&["2025", "294", "62.44"]);
        let e = parse_sheet(&sheet, &[]).unwrap_err();
        assert!(e.contains("no year rows"), "got {}", e);
    }

    #[test]
    fn output_is_year_ordered() {
        let sheet = format!(
            "{}{}",
            numeric_row(&["2025", "294", "62.44", "42.28", "60.82", "86.07"]),
            numeric_row(&["2020", "227", "46.5", "33.0", "46.10", "56.30"])
        );
        let out = parse_sheet(&sheet, &[]).unwrap();
        assert_eq!(out[0].year, 2020, "oldest first");
        assert_eq!(out[1].year, 2025);
    }

    #[test]
    fn shared_string_cells_resolve_rather_than_reading_an_index() {
        // A shared-string cell holds an INDEX; emitting the index would print a
        // number where a label belongs.
        let sheet = "<row><c t=\"s\"><v>0</v></c></row>";
        let cells = row_cells("<c t=\"s\"><v>0</v></c>", &["Introduction".to_string()]);
        assert_eq!(cells, vec!["Introduction".to_string()]);
        // And an out-of-range index degrades to empty rather than panicking.
        let cells2 = row_cells(sheet, &[]);
        assert_eq!(cells2, vec![String::new()]);
    }

    #[test]
    fn a_trend_needs_two_usable_cohorts() {
        let one = vec![EnergisationYear {
            year: 2025,
            n: 294,
            mean_months: 62.4,
            median_months: 60.8,
            p75_months: 86.1,
        }];
        assert!(
            recent_change(&one, 100).is_none(),
            "one year is not a trend"
        );
    }

    #[test]
    fn tiny_cohorts_do_not_set_the_trend() {
        // A median from three projects is noise and must not drive the reading.
        let noisy = vec![
            EnergisationYear {
                year: 2024,
                n: 335,
                mean_months: 62.1,
                median_months: 62.7,
                p75_months: 80.3,
            },
            EnergisationYear {
                year: 2025,
                n: 3,
                mean_months: 12.0,
                median_months: 9.0,
                p75_months: 10.0,
            },
        ];
        assert!(
            recent_change(&noisy, 100).is_none(),
            "a 3-project cohort must not be used as the latest year"
        );
    }

    #[test]
    fn the_change_reports_both_years_and_both_medians() {
        let yrs = vec![
            EnergisationYear {
                year: 2024,
                n: 335,
                mean_months: 62.1,
                median_months: 62.7,
                p75_months: 80.3,
            },
            EnergisationYear {
                year: 2025,
                n: 294,
                mean_months: 62.4,
                median_months: 60.8,
                p75_months: 86.1,
            },
        ];
        let (py, pm, ly, lm) = recent_change(&yrs, 100).unwrap();
        assert_eq!((py, ly), (2024, 2025));
        assert!((pm - 62.7).abs() < 0.1 && (lm - 60.8).abs() < 0.1);
    }
}
