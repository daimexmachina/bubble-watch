//! Backtest harness: can any of this data actually PREDICT a bubble bursting?
//!
//! WHY THIS EXISTS. The tool measures bubble STRESS — a level. The obvious next
//! question is whether it can predict a TURN. That question deserves an empirical
//! answer before any predictor is built, and this module is the answer: it tests
//! candidate precursors against the three real peaks available in the data.
//!
//! THE HONEST RESULT IS NEGATIVE, AND IT IS RECORDED RATHER THAN BURIED.
//!
//! Sample: 489 months (1986-01 to 2026-09) from BAA10Y, 169 from SPX. Three peaks:
//! 2000-03 (dot-com), 2007-10 (GFC), 2020-02 (COVID). Base rate: 0.6% of months.
//!
//! | signal                              | episodes | peaks caught | precision |
//! |-------------------------------------|----------|--------------|-----------|
//! | A  credit widening 3m > 0.25pp      |       31 |          3/3 |      10%  |
//! | C  widening FROM A TIGHT BASE       |        4 |          1/3 |      25%  |
//! | E  SPX 10% off its 12m high         |       25 |          1/3 |       4%  |
//!
//! What that means:
//!
//!   * **A catches every peak and is still nearly useless.** It fires in 16% of months,
//!     so any two-year window contains it. "Caught 3/3" sounds strong and is not: a
//!     signal that is usually on will be on before anything.
//!   * **C is the sharpest credit signal** — 1% of months, echoing the config's own
//!     claim that widening from a tight base is the informative case — but it fired
//!     before only one of three peaks, and n=3 is far too small to distinguish 25%
//!     precision from luck.
//!   * **E confirms rather than predicts.** An equity drawdown fires at or after a peak.
//!     Useful as corroboration, worthless as a warning.
//!
//! THE CONCLUSION: **this data can REJECT a predictor but cannot VALIDATE one.** Three
//! peaks is not a sample; it is three anecdotes. Any threshold tuned on them is
//! curve-fitted, and reporting one as though it were calibrated would be precisely the
//! fabricated precision this project refuses.
//!
//! SO WHAT IS THE POINT OF THE MODULE? Two things, both real:
//!
//!   1. It **falsifies** the tempting claim that credit momentum predicts peaks. It
//!      does not, at any usable precision, and now that is measured rather than
//!      assumed. That is the difference between this tool and financial astrology.
//!   2. It provides the **harness** — `episodes`, `precision`, `lead_months` — so that
//!      as the daily archive accumulates, thresholds can be tested without rebuilding
//!      the machinery. The limitation is the DATA, not the method, and the method is
//!      what this module contributes.
//!
//! WHAT WOULD CHANGE THE ANSWER: more peaks. A predictor cannot be fitted on three
//! events, but it can be tested on twenty. That means either a longer series (SPX from
//! the 1870s via Shiller) or a broader universe (peaks in other assets and markets),
//! and neither is in hand today.

use crate::model::Point;

/// One candidate precursor and its evaluated performance. Reported honestly, including
/// when the result is bad.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Backtest {
    pub signal: String,
    /// Distinct periods during which the signal was on.
    pub episodes: usize,
    /// How many of the reference peaks it fired before, within the horizon.
    pub peaks_caught: usize,
    /// Reference peaks available in total.
    pub peaks_total: usize,
    /// Share of distinct episodes that preceded a peak. Low is the norm.
    pub precision_pct: f64,
    /// Share of all months the signal was on. A signal that is usually on catches
    /// everything by accident, which is why this is reported next to precision.
    pub months_on_pct: f64,
    /// Median months between the signal firing and the peak.
    pub median_lead_months: Option<f64>,
    /// What the reader should conclude.
    pub verdict: String,
}

/// A reference peak: a month label and what it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Peak {
    pub month: String,
    pub label: String,
}

/// The three peaks available in the data. Deliberately labelled as ANECDOTES rather
/// than a sample, because that is what three events are.
pub fn reference_peaks() -> Vec<Peak> {
    vec![
        Peak {
            month: "2000-03".into(),
            label: "dot-com".into(),
        },
        Peak {
            month: "2007-10".into(),
            label: "GFC".into(),
        },
        Peak {
            month: "2020-02".into(),
            label: "COVID".into(),
        },
    ]
}

/// Collapse month-end observations to a strictly ordered monthly grid.
///
/// The last observation for each calendar month wins, which matches how a monthly
/// series is conventionally read and avoids a month appearing twice.
pub fn monthly_grid(points: &[Point]) -> Vec<(String, f64)> {
    use std::collections::BTreeMap;
    let mut m: BTreeMap<String, f64> = BTreeMap::new();
    for p in points {
        if p.date.len() >= 7 {
            m.insert(p.date[..7].to_string(), p.value);
        }
    }
    m.into_iter().collect()
}

/// Contiguous runs where the signal is true, as (start_index, end_index) pairs.
///
/// A run is an EPISODE, not a month: counting months would make any long-lived signal
/// look like a predictor simply by being on more often.
pub fn episodes(flags: &[bool]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut run: Option<usize> = None;
    for (i, f) in flags.iter().enumerate() {
        match (*f, run) {
            (true, None) => run = Some(i),
            (false, Some(a)) => {
                out.push((a, i - 1));
                run = None;
            }
            _ => {}
        }
    }
    if let Some(a) = run {
        out.push((a, flags.len() - 1));
    }
    out
}

/// Months between a signal's first firing and a peak, or None when the peak is not
/// after the signal.
pub fn lead_months(months: &[String], signal_start: usize, peak: &str) -> Option<i64> {
    let pi = months.iter().position(|m| m == peak)?;
    let si = signal_start;
    let diff = pi as i64 - si as i64;
    if diff >= 0 {
        Some(diff)
    } else {
        None
    }
}

/// Evaluate a signal against the reference peaks.
///
/// `horizon_months` bounds how far ahead a firing counts: a signal that fires a decade
/// early has not predicted anything useful.
pub fn evaluate(
    name: &str,
    months: &[String],
    flags: &[bool],
    peaks: &[Peak],
    horizon_months: i64,
) -> Backtest {
    let eps = episodes(flags);
    let mut caught = 0usize;
    let mut leads: Vec<i64> = Vec::new();
    for (a, b) in &eps {
        for p in peaks {
            // The signal must have ENDED at or before the peak, and STARTED within the
            // horizon of it. An episode still running at the peak does not count as
            // having preceded it.
            if let Some(pi) = months.iter().position(|m| *m == p.month) {
                if *b <= pi && (pi as i64 - *a as i64) <= horizon_months {
                    caught += 1;
                    leads.push(pi as i64 - *a as i64);
                    break;
                }
            }
        }
    }
    leads.sort_unstable();
    let months_on = flags.iter().filter(|f| **f).count();
    let precision = if eps.is_empty() {
        0.0
    } else {
        caught as f64 / eps.len() as f64 * 100.0
    };

    let verdict = if eps.is_empty() {
        "never fired: cannot be assessed".to_string()
    } else if precision < 15.0 {
        format!(
            "fires in {:.1}% of months and preceded only {} of {} peaks — catching a peak is close \
             to inevitable for a signal that is usually on, so this is NOT a usable predictor",
            months_on as f64 / flags.len().max(1) as f64 * 100.0,
            caught,
            peaks.len()
        )
    } else {
        format!(
            "preceded {} of {} peaks. With only {} reference events this cannot be validated, only \
             noted — a threshold tuned on three occurrences is curve-fitted",
            caught,
            peaks.len(),
            peaks.len()
        )
    };

    Backtest {
        signal: name.to_string(),
        episodes: eps.len(),
        peaks_caught: caught,
        peaks_total: peaks.len(),
        precision_pct: precision,
        months_on_pct: months_on as f64 / flags.len().max(1) as f64 * 100.0,
        median_lead_months: leads.get(leads.len() / 2).map(|v| *v as f64),
        verdict,
    }
}

/// Rolling change over `k` months, with the first `k` entries false (not enough
/// history rather than zero change).
pub fn rising_by(values: &[f64], k: usize, threshold: f64) -> Vec<bool> {
    (0..values.len())
        .map(|i| i >= k && (values[i] - values[i - k]) > threshold)
        .collect()
}

/// True where the value sits in the bottom quartile of its own trailing window.
pub fn bottom_quartile(values: &[f64], window: usize) -> Vec<bool> {
    (0..values.len())
        .map(|i| {
            let w = &values[i.saturating_sub(window)..=i];
            if w.is_empty() {
                return false;
            }
            let mut s = w.to_vec();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            values[i] <= s[s.len() / 4]
        })
        .collect()
}

/// Deterministic xorshift64*, so the null model is reproducible run to run. A
/// randomised test that produced a different verdict each run would be worse than
/// useless.
fn next_rand(state: &mut u64) -> f64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    (state.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
}

/// How many reference peaks a flag series precedes within `horizon` months.
///
/// An episode must have ENDED at or before the peak: a signal still running at the peak
/// has not preceded it, or a permanently-on signal would "predict" every peak.
pub fn peaks_preceded(set: &ShillerSet, flags: &[bool], horizon: i64) -> usize {
    let mut caught = 0usize;
    for (a, b) in episodes(flags) {
        for pk in &set.peaks {
            if let Some(pi) = set.index_of(pk) {
                if b <= pi && (pi as i64 - a as i64) <= horizon {
                    caught += 1;
                    break;
                }
            }
        }
    }
    caught
}

/// The result of a NULL-MODEL test: is a signal better than a coin with the same
/// on-rate?
///
/// This is the test that matters, and the one the three-peak version could not run.
/// "Caught N peaks" is meaningless alone: with a 0.7% base rate, a signal ON half the
/// time sits before many peaks simply by covering many months. The right question is
/// whether STRUCTURE beats a random signal firing the SAME NUMBER of months.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NullTest {
    pub signal: String,
    pub months_on: usize,
    pub peaks_caught: usize,
    pub peaks_total: usize,
    /// Median peaks caught by random signals with the same on-rate.
    pub random_median: usize,
    /// 95th percentile of that null distribution.
    pub random_p95: usize,
    /// Share of random draws that matched or beat the real signal. Near 1 = no signal.
    pub p_value: f64,
}

impl NullTest {
    pub fn beats_chance(&self) -> bool {
        self.p_value < 0.05
    }

    pub fn verdict(&self) -> String {
        if self.beats_chance() {
            format!(
                "beats chance at p={:.3} — worth investigating further, though one signal \
                 surviving one test is not evidence of a usable predictor",
                self.p_value
            )
        } else {
            format!(
                "INDISTINGUISHABLE FROM CHANCE (p={:.3}): a random signal firing the same {} months \
                 catches a median of {} of {} peaks. The structure adds nothing.",
                self.p_value, self.months_on, self.random_median, self.peaks_total
            )
        }
    }
}

/// Run the null model: draw `trials` random flag sets with the SAME number of ON months
/// and compare how many peaks each precedes.
///
/// Sampling is without replacement, so a draw cannot switch a month on twice and the
/// on-rate is matched exactly — which is the whole point. Comparing against a random
/// signal of DIFFERENT on-rate would prove nothing either way.
pub fn null_test(
    name: &str,
    set: &ShillerSet,
    flags: &[bool],
    horizon: i64,
    trials: usize,
    seed: u64,
) -> NullTest {
    let n = flags.len();
    let months_on = flags.iter().filter(|f| **f).count();
    let observed = peaks_preceded(set, flags, horizon);

    let mut state = seed | 1;
    let mut pool: Vec<usize> = (0..n).collect();
    let mut draws: Vec<usize> = Vec::with_capacity(trials);
    for _ in 0..trials {
        let mut picked = vec![false; n];
        for placed in 0..months_on.min(n) {
            // jitter the pool each trial so the draws are not identical
            let j = placed + (next_rand(&mut state) * (n - placed) as f64) as usize;
            let j = j.min(n - 1);
            pool.swap(placed, j);
            picked[pool[placed]] = true;
        }
        draws.push(peaks_preceded(set, &picked, horizon));
    }
    draws.sort_unstable();
    let median = draws.get(draws.len() / 2).copied().unwrap_or(0);
    let p95 = draws
        .get(((draws.len() as f64 * 0.95) as usize).min(draws.len() - 1))
        .copied()
        .unwrap_or(0);
    let at_least = draws.iter().filter(|d| **d >= observed).count();
    let p_value = (at_least + 1) as f64 / (draws.len() + 1) as f64;

    NullTest {
        signal: name.to_string(),
        months_on,
        peaks_caught: observed,
        peaks_total: set.peaks.len(),
        random_median: median,
        random_p95: p95,
        p_value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(date: &str, value: f64) -> Point {
        Point {
            date: date.into(),
            value,
        }
    }

    #[test]
    fn monthly_grid_collapses_to_one_point_per_month_keeping_the_last() {
        let pts = vec![
            pt("2026-01-05", 1.0),
            pt("2026-01-28", 2.0),
            pt("2026-02-10", 3.0),
        ];
        let g = monthly_grid(&pts);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], ("2026-01".to_string(), 2.0));
    }

    #[test]
    fn episodes_count_runs_not_months() {
        // The distinction that matters: a signal on for 6 consecutive months is ONE
        // episode, so precision is not inflated by a long run.
        let f = vec![false, true, true, true, false, true, false];
        assert_eq!(episodes(&f), vec![(1, 3), (5, 5)]);
    }

    #[test]
    fn an_episode_is_closed_at_the_end() {
        let f = vec![false, true, true];
        assert_eq!(episodes(&f), vec![(1, 2)]);
    }

    #[test]
    fn a_signal_that_never_fires_has_an_empty_episode_list() {
        assert!(episodes(&[false, false, false]).is_empty());
    }

    #[test]
    fn a_signal_still_running_at_the_peak_does_not_count_as_preceding_it() {
        let months: Vec<String> = (0..10).map(|i| format!("2020-{:02}", i + 1)).collect();
        let peaks = vec![Peak {
            month: "2020-06".into(),
            label: "t".into(),
        }];
        // on from index 5 through the end, so it starts before the peak but never ends
        let flags: Vec<bool> = (0..10).map(|i| i >= 5).collect();
        let b = evaluate("still running", &months, &flags, &peaks, 24);
        assert_eq!(
            b.peaks_caught, 0,
            "an un-ended episode has not preceded anything"
        );
    }

    #[test]
    fn a_briefly_firing_signal_before_a_peak_is_credited() {
        let months: Vec<String> = (0..10).map(|i| format!("2020-{:02}", i + 1)).collect();
        let peaks = vec![Peak {
            month: "2020-06".into(),
            label: "t".into(),
        }];
        let flags: Vec<bool> = (0..10).map(|i| i == 2).collect();
        let b = evaluate("brief", &months, &flags, &peaks, 24);
        assert_eq!(b.peaks_caught, 1);
    }

    #[test]
    fn a_peak_beyond_the_horizon_is_not_credited() {
        let months: Vec<String> = (0..40)
            .map(|i| format!("20{:02}-01", 20 + i / 12))
            .collect();
        let peaks = vec![Peak {
            month: "2023-03".into(),
            label: "far".into(),
        }];
        let flags: Vec<bool> = (0..40).map(|i| i == 0).collect();
        let b = evaluate("too early", &months, &flags, &peaks, 12);
        assert_eq!(b.peaks_caught, 0);
    }

    #[test]
    fn rising_by_has_no_signal_before_enough_history() {
        let v = vec![1.0, 2.0, 3.0, 10.0];
        let f = rising_by(&v, 3, 1.0);
        assert!(
            !f[0] && !f[1] && !f[2],
            "first k entries cannot be assessed"
        );
        assert!(f[3]);
    }

    #[test]
    fn bottom_quartile_identifies_the_low_end() {
        let v: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let f = bottom_quartile(&v, 100);
        // The maximum can never be in its own bottom quartile.
        assert!(!f[99], "the series maximum is not bottom-quartile");
        // A strictly rising series is only bottom-quartile at the very start, while its
        // trailing window is small. By index 3 the window is [1,2,3,4] and 4 is above
        // the quartile boundary of 2, so it must NOT qualify — the earlier assertion
        // here was wrong and the implementation was right.
        assert!(f[0], "the first point is trivially bottom-quartile");
        assert!(!f[3], "a rising series leaves the bottom quartile quickly");
        // And a genuine low point inside a long window does qualify.
        let mut w = vec![50.0; 100];
        w[60] = 1.0; // an outlier low among a stable series
        assert!(bottom_quartile(&w, 100)[60]);
    }

    #[test]
    fn the_falsification_result_is_reproducible_on_the_shipped_fixture() {
        // Guards the headline finding: credit momentum catches peaks but with low
        // precision, so it must not be shipped as a predictor. If this ever starts
        // reporting high precision, either the data or the harness changed.
        let path = std::path::Path::new("tests/fixtures/observations.json");
        if !path.exists() {
            return;
        }
        let text = std::fs::read_to_string(path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let arr = match v
            .get("fred")
            .and_then(|f| f.get("BAA10Y"))
            .and_then(|s| s.get("points"))
        {
            Some(a) => a,
            None => return,
        };
        let pts: Vec<Point> = arr
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| {
                Some(Point {
                    date: p.get("date")?.as_str()?.to_string(),
                    value: p.get("value")?.as_f64()?,
                })
            })
            .collect();
        let grid = monthly_grid(&pts);
        let months: Vec<String> = grid.iter().map(|(m, _)| m.clone()).collect();
        let values: Vec<f64> = grid.iter().map(|(_, v)| *v).collect();
        let flags = rising_by(&values, 3, 0.25);
        let b = evaluate("credit momentum", &months, &flags, &reference_peaks(), 24);
        assert!(
            b.precision_pct < 30.0,
            "credit momentum must not look precise on this data, got {:.0}%",
            b.precision_pct
        );
        assert!(
            b.months_on_pct > 5.0,
            "it should be on often enough that catching peaks is unsurprising"
        );
    }
}

// ---------------------------------------------------------------------------
// SHILLER EXTENSION: 155 years instead of 36, and the null model that matters.
// ---------------------------------------------------------------------------

/// The Shiller reference set, loaded from the committed fixture.
///
/// 1,833 monthly observations (1871-01 to 2023-09) and 13 crash peaks, against the 3
/// available from 1986 onward. Extracted from Shiller's `ie_data.xls`, which is legacy
/// OLE2/BIFF rather than xlsx, so the extraction used a one-off Python step and the
/// RESULT is committed as JSON — the test suite needs neither the download nor a
/// legacy-format reader.
pub struct ShillerSet {
    pub months: Vec<String>,
    pub prices: Vec<f64>,
    pub peaks: Vec<String>,
}

impl ShillerSet {
    /// Load from `tests/fixtures/shiller_spx.json`, or None when absent.
    pub fn load() -> Option<ShillerSet> {
        let path = std::path::Path::new("tests/fixtures/shiller_spx.json");
        let text = std::fs::read_to_string(path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        let months = v
            .get("months")?
            .as_array()?
            .iter()
            .filter_map(|p| Some(p.as_array()?.first()?.as_str()?.to_string()))
            .collect();
        let prices = v
            .get("months")?
            .as_array()?
            .iter()
            .filter_map(|p| p.as_array()?.get(1)?.as_f64())
            .collect();
        let peaks = v
            .get("peaks")?
            .as_array()?
            .iter()
            .filter_map(|p| Some(p.get("peak")?.as_str()?.to_string()))
            .collect();
        Some(ShillerSet {
            months,
            prices,
            peaks,
        })
    }

    pub fn index_of(&self, month: &str) -> Option<usize> {
        self.months.iter().position(|m| m == month)
    }

    /// Trailing 12-month return, in percent. NaN for the first 12 months.
    pub fn momentum_12m(&self) -> Vec<f64> {
        (0..self.prices.len())
            .map(|i| {
                if i < 12 {
                    f64::NAN
                } else {
                    (self.prices[i] / self.prices[i - 12] - 1.0) * 100.0
                }
            })
            .collect()
    }

    /// Months from `i` to the NEXT peak, or None if no peak follows.
    pub fn months_to_next_peak(&self, i: usize) -> Option<i64> {
        self.peaks
            .iter()
            .filter_map(|pk| self.index_of(pk))
            .filter(|pi| *pi >= i)
            .map(|pi| pi as i64 - i as i64)
            .min()
    }
}

#[cfg(test)]
mod shiller_tests {
    use super::*;
    #[test]
    fn the_shiller_fixture_loads_with_155_years_and_13_peaks() {
        let Some(set) = ShillerSet::load() else {
            eprintln!("SKIP: shiller fixture absent");
            return;
        };
        assert!(
            set.months.len() > 1800,
            "expected ~1833 months, got {}",
            set.months.len()
        );
        assert_eq!(set.months.len(), set.prices.len());
        assert!(
            set.peaks.len() >= 12,
            "expected 13 peaks, got {}",
            set.peaks.len()
        );
        assert_eq!(set.months[0], "1871-01");
        // the famous ones must be present, or the peak rule is wrong
        for pk in ["1929-09", "1987-08", "2000-08", "2007-10", "2021-12"] {
            assert!(
                set.peaks.contains(&pk.to_string()),
                "missing reference peak {}",
                pk
            );
        }
    }

    #[test]
    fn a_signal_is_indistinguishable_from_chance_on_155_years() {
        // THE headline finding. With 13 peaks over 155 years, a 12-month momentum
        // threshold does not beat a random signal that fires the same number of months.
        let Some(set) = ShillerSet::load() else {
            eprintln!("SKIP: shiller fixture absent");
            return;
        };
        let mom = set.momentum_12m();
        let flags: Vec<bool> = mom.iter().map(|m| m.is_finite() && *m > 25.0).collect();
        let t = null_test("12m return > 25%", &set, &flags, 24, 300, 12345);
        assert!(
            !t.beats_chance(),
            "momentum must NOT beat chance on this data, got p={:.3} (caught {} vs random median {})",
            t.p_value, t.peaks_caught, t.random_median
        );
        assert!(
            t.months_on > 100,
            "the signal should be on often enough that catching peaks is unsurprising"
        );
    }

    #[test]
    fn the_null_model_is_deterministic() {
        // A randomised test that produced different verdicts run to run would be worse
        // than useless, so the seed is fixed and this asserts it.
        let Some(set) = ShillerSet::load() else {
            eprintln!("SKIP: shiller fixture absent");
            return;
        };
        let mom = set.momentum_12m();
        let flags: Vec<bool> = mom.iter().map(|m| m.is_finite() && *m > 25.0).collect();
        let a = null_test("x", &set, &flags, 24, 100, 7);
        let b = null_test("x", &set, &flags, 24, 100, 7);
        assert_eq!(a, b);
        let c = null_test("x", &set, &flags, 24, 100, 8);
        assert!(
            c.random_median > 0,
            "a different seed still draws random signals"
        );
    }

    #[test]
    fn a_random_signal_catches_peaks_by_construction() {
        // Guards the reason the null model exists: a signal that is on more often
        // catches more peaks, with no structure at all. If this ever fails, the null
        // model itself is broken.
        let Some(set) = ShillerSet::load() else {
            eprintln!("SKIP: shiller fixture absent");
            return;
        };
        let n = set.months.len();
        let rare: Vec<bool> = (0..n).map(|i| i % 50 == 0).collect(); // ~2% on
        let often: Vec<bool> = (0..n).map(|i| i % 3 == 0).collect(); // ~33% on
        let t_rare = null_test("rare", &set, &rare, 24, 200, 1);
        let t_often = null_test("often", &set, &often, 24, 200, 1);
        assert!(
            t_often.peaks_caught > t_rare.peaks_caught,
            "being on more often should catch more peaks purely by coverage: {} vs {}",
            t_often.peaks_caught,
            t_rare.peaks_caught
        );
    }

    #[test]
    fn months_to_next_peak_is_none_after_the_final_peak() {
        let Some(set) = ShillerSet::load() else {
            eprintln!("SKIP: shiller fixture absent");
            return;
        };
        let last = set.months.len() - 1;
        // 2021-12 is the final reference peak, so the last month follows it
        assert!(set.months[last] > "2021-12".to_string());
        assert!(set.months_to_next_peak(last).is_none());
        // and a month before the 1929 peak must resolve to something
        if let Some(i) = set.index_of("1929-01") {
            assert!(set.months_to_next_peak(i).is_some());
        }
    }
}
