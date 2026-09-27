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

/// Current release: RESOLVED AT FETCH TIME, not a constant.
///
/// There used to be a `pub const URL` here reading
/// `.../eia860m/xls/july_generator2026.xlsx` — a hardcoded month, and the single wrong
/// path segment responsible for the model reading 96.7% instead of 100% from
/// 2026-09-25. EIA rotates the filename monthly and answers a stale one with status 200
/// and an HTML body, so no fixed filename can stay correct. The vintage is now resolved
/// by `open_newest_workbook`, which walks back from last month and tries both `/xls/`
/// and `/archive/xls/`.
///
/// `archive_url_dir()` below gives the pattern for provenance labelling.

/// Stable dated pattern for pinning a specific vintage (the archive directory).
pub fn archive_url(month: &str, year: u32) -> String {
    archive_path_url(month, year)
}

/// The archive DIRECTORY, for provenance when the vintage itself is resolved at
/// fetch time and therefore not known statically.
pub fn archive_url_dir() -> &'static str {
    "https://www.eia.gov/electricity/data/eia860m/archive/xls/<month>_generator<year>.xlsx"
}

/// The archive path for a dated vintage.
///
/// NOTE: EIA serves the CURRENT month under `/eia860m/xls/` and everything older
/// under `/eia860m/archive/xls/`. Requesting a month that has aged out of the
/// current directory does NOT 404 — EIA returns its HTML landing page with
/// `http=200` and `content-type: text/html` (measured: 55,725 bytes). A 200 is
/// therefore NOT evidence that the file exists, which is why `is_workbook` checks
/// the bytes rather than trusting the status code.
pub fn archive_path_url(month: &str, year: u32) -> String {
    format!(
        "https://www.eia.gov/electricity/data/eia860m/archive/xls/{}_generator{}.xlsx",
        month, year
    )
}

/// The CURRENT-month path for a dated vintage.
///
/// EIA publishes a new month here first and moves it to the archive path later, so a
/// month can exist in exactly one of the two directories. Measured 2026-09-27:
/// `august_generator2026.xlsx` was a real 14.0 MB workbook under `/xls/` and an HTML
/// landing page under `/archive/xls/`, while `july_generator2026.xlsx` was the
/// reverse. A resolver that consults only one of the two paths therefore either goes
/// stale (archive-only, as this one did on first implementation) or breaks on the
/// month rollover (current-only, the original bug).
pub fn current_path_url(month: &str, year: u32) -> String {
    format!(
        "https://www.eia.gov/electricity/data/eia860m/xls/{}_generator{}.xlsx",
        month, year
    )
}

/// Month names in EIA's filename order, index 0 = January.
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// The (month, year) candidates to try, newest first, starting one month BEHIND the
/// reference date.
///
/// WHY NOT THE CURRENT MONTH. EIA's current-month file appears late in the month and
/// the index page links filenames that do not yet resolve — measured 2026-09-26,
/// `/eia860m/xls/october_generator2026.xlsx` was linked on the index page and returned
/// the 55KB HTML landing page. So the newest month is routinely a file that does not
/// exist. Starting one month back targets a release EIA has actually published, and
/// the walk gives us a bounded descent if a given vintage is missing.
///
/// `months_back` is 1-based: 1 = last month, 2 = the month before that.
pub fn candidate_months(reference_ym: (u32, u32), count: usize) -> Vec<(String, u32)> {
    let (mut y, mut m) = reference_ym; // m: 1..=12
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        // step back one calendar month
        if m == 1 {
            m = 12;
            y -= 1;
        } else {
            m -= 1;
        }
        out.push((MONTHS[(m - 1) as usize].to_string(), y));
    }
    out
}

/// Is this byte slice actually an xlsx workbook, rather than an HTML error page?
///
/// Checks the ZIP local-file magic (`PK\x03\x04`) because that is what the reader
/// must see. EIA's failure mode here is a 200 with an HTML body, so checking the
/// status code or a content header would both pass a non-workbook through; only the
/// bytes are authoritative.
pub fn is_workbook(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"PK\x03\x04"
}

/// A short, honest description of what came back instead of a workbook, for the
/// failure message. The point is to let a reader tell "the source is down" from
/// "our URL is stale", which the previous generic message could not distinguish.
fn describe_non_workbook(bytes: &[u8]) -> String {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]);
    let looks_html = head
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("<!doctype")
        || head.to_ascii_lowercase().contains("<html");
    format!(
        "response was {} bytes of {}, not an xlsx workbook (zip magic absent). \
         A 200 status with an HTML body is EIA's response when a filename has aged out \
         of the current-month directory, which means the URL is stale rather than the \
         source being down.",
        bytes.len(),
        if looks_html { "HTML" } else { "non-xlsx data" }
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

/// Resolve and download the newest EIA-860M workbook that actually exists.
///
/// Returns `(bytes, month, year, url)` so the caller can label provenance with the
/// vintage it really read.
///
/// The walk starts one month behind the reference date and descends a bounded number
/// of months, accepting the first response whose bytes are a real workbook. Two
/// measured facts drive this shape:
///
///   1. EIA serves the current month under `/xls/` and older months under
///      `/archive/xls/`. Requesting a month that has aged out of the current
///      directory returns the HTML landing page with status 200, so the status is
///      not usable as a success signal.
///   2. The index page links filenames that do not yet resolve. On 2026-09-26 it
///      linked `/xls/october_generator2026.xlsx`, which returned that same 55KB HTML
///      page. A link is therefore not evidence either.
///
/// So existence is decided by `is_workbook` on the bytes, and the archive path is
/// tried for every candidate because a month that has been published is in the
/// archive regardless of whether it is still "current".
fn open_newest_workbook(
    f: &crate::http::Fetcher,
) -> Result<(Vec<u8>, String, u32, String), String> {
    resolve_newest_workbook(current_ym(), RESOLVE_MONTHS_BACK, |url| {
        f.get_bytes(url, UA)
    })
}

/// The resolution walk, with fetching injected.
///
/// Split out from `open_newest_workbook` so the WALK ITSELF is testable. An earlier
/// version of this change put the two-path logic inline and wrote a test against the
/// URL builders instead; that test passed even when the walk was reverted to
/// archive-only, i.e. it could not fail on the bug it was written for. Injecting the
/// fetcher lets a test present a month that exists in exactly one directory — which is
/// the real EIA condition — and assert which vintage comes back.
fn resolve_newest_workbook<F>(
    reference_ym: (u32, u32),
    months_back: usize,
    mut fetch: F,
) -> Result<(Vec<u8>, String, u32, String), String>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let mut attempts: Vec<String> = Vec::new();

    for (month, year) in candidate_months(reference_ym, months_back) {
        // BOTH directories are tried per month, because a month can exist in exactly
        // one of them: EIA moves a release from /xls/ to /archive/xls/ as it ages, so
        // the newest published month is usually current-only and the older ones are
        // archive-only. Measured 2026-09-27: august was /xls/-only, july /archive/-only.
        // Trying a single directory silently accepts a stale vintage — which is exactly
        // what this resolver did before the two-path walk was added.
        for url in [
            current_path_url(&month, year),
            archive_path_url(&month, year),
        ] {
            match fetch(&url) {
                Ok(bytes) => {
                    if is_workbook(&bytes) {
                        return Ok((bytes, month, year, url));
                    }
                    attempts.push(format!(
                        "{}_generator{}.xlsx [{}]: {}",
                        month,
                        year,
                        path_kind(&url),
                        describe_non_workbook(&bytes)
                    ));
                }
                Err(e) => attempts.push(format!(
                    "{}_generator{}.xlsx [{}]: {}",
                    month,
                    year,
                    path_kind(&url),
                    e
                )),
            }
        }
    }

    Err(format!(
        "no EIA-860M workbook found in the newest {} months. Tried, newest first, in both \
         the current-month and archive directories: {}. Every candidate failed on its \
         CONTENT, not on a status code: EIA answers a filename it no longer serves with \
         200 and an HTML landing page, so a missing file and a served file look identical \
         at the status level. If all candidates read as HTML, the archive naming scheme \
         has changed and the patterns in `current_path_url` / `archive_path_url` need \
         revisiting.",
        RESOLVE_MONTHS_BACK,
        attempts.join("; ")
    ))
}

/// Which directory a URL points at, for failure messages.
fn path_kind(url: &str) -> &'static str {
    if url.contains("/archive/xls/") {
        "archive"
    } else {
        "current"
    }
}

/// The reference (year, month) used to anchor vintage resolution.
///
/// Honours `BUBBLE_WATCH_NOW` (ISO-8601 `YYYY-MM-DD`) when set, which lets a test or
/// a replay pin the clock rather than depend on the wall time. Falls back to the
/// system clock.
fn current_ym() -> (u32, u32) {
    if let Ok(s) = std::env::var("BUBBLE_WATCH_NOW") {
        if let Some((y, m)) = parse_ym(&s) {
            return (y, m);
        }
    }
    let now = crate::now_iso8601();
    parse_ym(&now).unwrap_or((1970, 1))
}

/// Parse `YYYY-MM...` into `(year, month)`.
fn parse_ym(s: &str) -> Option<(u32, u32)> {
    let b = s.as_bytes();
    if b.len() < 7 || b[4] != b'-' {
        return None;
    }
    let y: u32 = s.get(0..4)?.parse().ok()?;
    let m: u32 = s.get(5..7)?.parse().ok()?;
    if (1..=12).contains(&m) {
        Some((y, m))
    } else {
        None
    }
}

/// How many months back the resolver will walk before giving up.
///
/// Four is enough for EIA's monthly cadence to survive one skipped release plus a
/// late-published month, and small enough that a wholesale naming change fails fast
/// instead of issuing a long chain of requests.
const RESOLVE_MONTHS_BACK: usize = 4;

/// Fetch the planned and cancelled inventories in one archive download.
pub fn planned_vs_cancelled(f: &crate::http::Fetcher) -> Result<(Series, Series, String), String> {
    // Resolve the vintage FIRST, because the filename is the thing that goes stale.
    // EIA moves the month; a hardcoded filename silently becomes an HTML landing page
    // served with a 200 (measured 2026-09-26: /xls/july_generator2026.xlsx returned
    // 55,725 bytes of text/html). So the month is derived and the bytes are checked.
    let (bytes, used_month, used_year, used_url) = open_newest_workbook(f)?;
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

    // `as_of` is DERIVED from the vintage actually read, not hardcoded. It was
    // hardcoded ("July 2026") until 2026-09-26, which meant the report would have
    // mislabelled the vintage the moment the resolver moved to a different month —
    // a staleness bug in the very field a reader uses to judge staleness.
    let mut mchars = used_month.chars();
    let cap = match mchars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + mchars.as_str(),
        None => used_month.clone(),
    };
    let as_of = format!("{} {}", cap, used_year);
    let mk = |label: &str, mw: f64| Series {
        provenance: Provenance {
            source: "eia-860m".into(),
            endpoint: format!("{} :: sheet '{}' :: {} MW", used_url, label, mw),
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
/// Sheet names in workbook order.
///
/// Made public (2026-09-20) so the LBNL interconnection-queue source can reuse it
/// rather than duplicating workbook navigation. The LBNL file has 43 sheets and the
/// one that matters ("37. IR to COD - all") is #40, so the sheet cannot be found by
/// position on sheet1 alone.
pub fn sheet_names(bytes: &[u8]) -> Result<Vec<String>, String> {
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
/// A worksheet's `<sheetData>` XML by zero-based sheet INDEX, plus its strings.
///
/// Public for the same reason as `sheet_names`.
pub fn sheet_xml(bytes: &[u8], idx0: usize) -> Result<(String, Vec<String>), String> {
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

    /// Both directories are addressable for the same vintage, and they are DIFFERENT
    /// paths. (This alone is not enough — see the walk test below, which is the one that
    /// can actually fail when the two-path logic is removed.)
    #[test]
    fn both_directories_are_addressable_and_distinct() {
        let cur = current_path_url("august", 2026);
        let arc = archive_path_url("august", 2026);
        assert_ne!(
            cur, arc,
            "the two directories must be different URLs or the walk is pointless"
        );
        assert!(cur.contains("/eia860m/xls/"), "current path: {}", cur);
        assert!(
            arc.contains("/eia860m/archive/xls/"),
            "archive path: {}",
            arc
        );
        assert_eq!(
            path_kind(&cur),
            "current",
            "a current-directory URL must be labelled as such"
        );
        assert_eq!(path_kind(&arc), "archive");
    }

    /// THE WALK ITSELF resolves to the newest vintage that exists, in EITHER directory.
    ///
    /// This is the test that can actually fail on the bug it guards. The real EIA
    /// condition on 2026-09-27 was: august existed ONLY under /xls/ (current), july ONLY
    /// under /archive/xls/. A resolver that consults one directory therefore returns a
    /// stale month while reporting success. The injected fetcher reproduces exactly that
    /// layout and asserts the answer is august — not july.
    #[test]
    fn the_walk_finds_a_month_that_lives_in_only_one_directory() {
        // A minimal valid-zip payload, so `is_workbook` accepts it.
        let workbook = b"PK\x03\x04pretend-this-is-a-real-xlsx".to_vec();

        // Reference 2026-09: the walk starts at august.
        // august: CURRENT only. july: ARCHIVE only. Everything else: HTML.
        let fetch = |url: &str| -> Result<Vec<u8>, String> {
            let is_aug_current = url.contains("/eia860m/xls/august_");
            let is_jul_archive = url.contains("/archive/xls/july_");
            if is_aug_current || is_jul_archive {
                Ok(workbook.clone())
            } else {
                Ok(b"<!doctype html><html>landing page</html>".to_vec())
            }
        };

        let (_, month, year, url) =
            resolve_newest_workbook((2026, 9), 4, fetch).expect("a workbook exists");
        assert_eq!(
            (month.as_str(), year),
            ("august", 2026),
            "must return the NEWEST existing vintage, not fall through to july"
        );
        assert!(
            url.contains("/eia860m/xls/august_"),
            "and must report the directory it actually read: {}",
            url
        );
    }

    /// The converse: when the newest month exists ONLY in the archive, it is still
    /// found. Guards against a fix that hardcodes the current directory.
    #[test]
    fn the_walk_finds_a_month_that_lives_only_in_the_archive() {
        let workbook = b"PK\x03\x04archive-payload".to_vec();
        let fetch = |url: &str| -> Result<Vec<u8>, String> {
            if url.contains("/archive/xls/august_") {
                Ok(workbook.clone())
            } else {
                Ok(b"<html>nope</html>".to_vec())
            }
        };
        let (_, month, _, url) =
            resolve_newest_workbook((2026, 9), 4, fetch).expect("a workbook exists");
        assert_eq!(month, "august");
        assert!(url.contains("/archive/xls/"), "url: {}", url);
    }

    /// An HTML 200 page never terminates the walk, and the walk is bounded. Every
    /// candidate reading as HTML must produce an error that says the failure was one of
    /// CONTENT, so a reader can tell a stale URL from a dead source.
    #[test]
    fn the_walk_never_accepts_html_and_fails_with_a_content_explanation() {
        let calls = std::cell::Cell::new(0);
        let fetch = |_: &str| -> Result<Vec<u8>, String> {
            calls.set(calls.get() + 1);
            Ok(b"<!doctype html><html>EIA landing page</html>".to_vec())
        };
        let err = resolve_newest_workbook((2026, 9), 3, fetch).unwrap_err();
        assert!(err.contains("CONTENT"), "must say content failed: {}", err);
        assert!(
            err.contains("stale") || err.contains("HTML"),
            "err: {}",
            err
        );
        // 3 months x 2 directories, and no more.
        assert_eq!(
            calls.get(),
            6,
            "must try both directories for each of the 3 months, and stop there"
        );
    }

    /// The walk starts one month BEHIND the reference date, because EIA links the
    /// current month's filename before the file resolves (measured: the September date
    /// list linked october_generator2026.xlsx and it returned the 55KB HTML page). And
    /// it descends in calendar order across a year boundary.
    #[test]
    fn candidate_months_descends_from_one_month_back() {
        assert_eq!(
            candidate_months((2026, 9), 4),
            vec![
                ("august".to_string(), 2026),
                ("july".to_string(), 2026),
                ("june".to_string(), 2026),
                ("may".to_string(), 2026),
            ]
        );
        // January must roll back into December of the previous year.
        assert_eq!(
            candidate_months((2026, 1), 2),
            vec![
                ("december".to_string(), 2025),
                ("november".to_string(), 2025)
            ]
        );
    }

    /// The guard that makes a 200 non-authoritative. EIA's staleness failure is
    /// `200 + text/html`, so this must reject an HTML body and accept a real workbook.
    #[test]
    fn a_200_html_landing_page_is_not_a_workbook() {
        let html = b"<!doctype html>\r\n<html><head><title>Electricity - U.S. Energy \
                     Information Administration (EIA)</title></head></html>";
        assert!(
            !is_workbook(html),
            "an HTML landing page must never pass as a workbook"
        );
        // and the rejection must explain itself in a way that separates 'stale URL'
        // from 'source down', since that is the distinction the old message lost.
        let msg = describe_non_workbook(html);
        assert!(msg.contains("HTML"), "must name HTML: {}", msg);
        assert!(
            msg.contains("stale"),
            "must point at staleness as the likely cause: {}",
            msg
        );

        // empty, and a truncated non-zip body, both fail closed.
        assert!(!is_workbook(b""));
        assert!(!is_workbook(b"PK"));
        assert!(!is_workbook(b"<html>"));

        // a real zip signature is accepted, so the guard is not simply always-false.
        assert!(
            is_workbook(b"PK\x03\x04rest-of-a-zip"),
            "a real zip magic must pass, or the guard would reject the real workbook"
        );
    }

    /// `as_of` is DERIVED from the fetched vintage rather than hardcoded. It read
    /// "July 2026" as a literal until 2026-09-26, which meant the field a reader uses
    /// to judge staleness would have gone stale itself the moment the resolver moved to
    /// a different month — and it had already moved, silently, to August.
    #[test]
    fn as_of_is_derived_from_the_vintage_not_hardcoded() {
        // The capitalisation helper used for the label.
        let cap = |m: &str| {
            let mut c = m.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => m.to_string(),
            }
        };
        assert_eq!(cap("august"), "August");
        assert_eq!(cap("july"), "July");
        assert_eq!(cap(""), "");
        // and the source no longer contains the old literal vintage claim
        let src = include_str!("eia.rs");
        assert!(
            !src.contains("let as_of = \"July 2026\""),
            "the vintage must be derived, not baked in"
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
