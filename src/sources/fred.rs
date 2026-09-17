//! FRED CSV client — an OPTIONAL source.
//!
//! During probing, `fred.stlouisfed.org/graph/fredgraph.csv` answered fine and
//! then began failing on *every* series (HTTP/2 `INTERNAL_ERROR`, then read
//! timeouts) after a modest burst of requests. That is consistent with anonymous
//! rate-limiting. It may well recover.
//!
//! Therefore: FRED is never required. Every failure is converted into a
//! reported coverage gap. The macro/credit indicators that depend on it go
//! `Unavailable` and the composite renormalizes over what is left — the
//! headline number stays honest about resting on fewer inputs.

use crate::http::{Fetcher, UA_FRED};
use crate::model::{Point, Provenance, Series};

/// FRED has been observed to answer this host for a while and then stop
/// entirely. It gets a short, single-attempt budget so a dead FRED costs the run
/// a few seconds rather than minutes — and the circuit breaker then stops the
/// remaining series from re-trying the same dead host.
pub const FRED_RETRIES: u32 = 1;

/// CSV format: `DATE,VALUE` with an empty value on holidays (".").
///
/// NOTE: `fredgraph.csv` serves roughly a three-year trailing window regardless
/// of the `cosd`/`coed` parameters (verified against the live endpoint). That is
/// ample for a current-reading indicator, but it means this client must never be
/// used for a long historical backtest without checking the returned span.
pub fn series(f: &Fetcher, id: &str) -> Result<Series, String> {
    let url = format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={}", id);
    let body = f
        .get_raw(&url, UA_FRED, FRED_RETRIES)
        .map_err(|e| e.message())?;

    let mut points = Vec::new();
    for line in body.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        let (Some(date), Some(val)) = (it.next(), it.next()) else {
            continue;
        };
        let val = val.trim();
        if val.is_empty() || val == "." {
            continue; // non-observations stay non-observations
        }
        let Ok(v) = val.parse::<f64>() else { continue };
        points.push(Point {
            date: date.trim().to_string(),
            value: v,
        });
    }

    if points.is_empty() {
        return Err(format!("FRED {} returned no usable observations", id));
    }

    let as_of = points.last().map(|p| p.date.clone()).unwrap_or_default();
    Ok(Series {
        provenance: Provenance {
            source: "fred".into(),
            endpoint: url,
            as_of,
            retrieved_at: crate::now_iso8601(),
        },
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv_with_holiday_gaps() {
        // Mirrors the real fredgraph.csv shape, including a "." non-observation.
        let body = "observation_date,DGS10\n2026-09-10,5.02\n2026-09-11,.\n2026-09-12,5.01\n";
        let mut pts: Vec<Point> = Vec::new();
        for line in body.lines().skip(1) {
            let mut it = line.split(',');
            if let (Some(d), Some(v)) = (it.next(), it.next()) {
                let v = v.trim();
                if v.is_empty() || v == "." {
                    continue;
                }
                if let Ok(val) = v.parse::<f64>() {
                    pts.push(Point {
                        date: d.into(),
                        value: val,
                    });
                }
            }
        }
        assert_eq!(pts.len(), 2, "the '.' row must be skipped, not zero-filled");
        assert_eq!(pts[1].value, 5.01);
    }
}
