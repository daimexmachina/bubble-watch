//! GSADF: the Generalised Supremum Augmented Dickey-Fuller explosiveness test.
//!
//! Reference: Phillips, Shi & Yu (2015), "Testing for Multiple Bubbles: Historical
//! Episodes of Exuberance and Collapse in the S&P 500", *International Economic
//! Review*; and Phillips, Wu & Yu (2011) for the earlier SADF form. Applied
//! specifically to AI and semiconductor names by Sarkar & Wells (2026), "Is there
//! an AI bubble? Robust date-stamping for periods of exuberance", *Frontiers of
//! Mathematical Finance*.
//!
//! WHY THIS EXISTS ALONGSIDE THE COMPOSITE. Everything else in this tool is a
//! hand-anchored judgement: a value is mapped through anchors a human chose. That
//! is auditable but it is still an opinion, and a composite of such opinions can
//! only ever report what its author already believed. This module is a different
//! KIND of evidence — a formal hypothesis test with a published distribution,
//! applied to the price series directly.
//!
//! The value is in the DISAGREEMENT. "Composite: mid; explosiveness test: not
//! significant" is far more informative than either alone, and the tool reports
//! both without reconciling them, because reconciling them would hide the very
//! tension a reader should see.
//!
//! WHAT IT TESTS. The null is a unit root (a random walk: prices have no
//! tendency to revert). The alternative is mildly explosive behaviour —
//! d > 1 in a right-tailed ADF regression, i.e. past price changes feeding
//! further price increases faster than a random walk would. Rejection means "the
//! price path is statistically hard to explain as a random walk", which is
//! evidence of exuberance, not proof of a bubble.
//!
//! WHAT IT DOES NOT DO, and these limitations are load-bearing:
//!
//!   * **It is not a crash signal.** Rejection says a run-up is statistically
//!     unusual. Many rejected episodes continue for a long time. The paper dates
//!     episodes; it does not forecast their end.
//!   * **The critical values here are SIMULATED, not the published table.** The
//!     Monte Carlo draws from a standard normal, which assumes a pure random walk
//!     with no drift and no fat tails. Real equity returns have both. Treat the
//!     thresholds as indicative and the p-value as approximate; this is stated in
//!     the output, not just here. Getting the real table in would mean
//!     transcribing published critical values as constants, which is a future
//!     improvement rather than something to fake now.
//!   * **A weekly or monthly series is coarse.** The literature mostly uses
//!     daily or weekly data; monthly gives fewer windows and less power.
//!   * **It says nothing about fundamentals.** A price can be explosive and
//!     justified, or flat and a bubble. GSADF measures the price path's
//!     statistical character and nothing else.

use crate::model::Point;

/// How many observations the smallest regression window must contain.
///
/// The literature's usual minimum is around 20-30 effective observations. With
/// monthly data a window of 24 gives two years, which is short but workable; the
/// test's power falls off quickly below this.
pub const MIN_WINDOW: usize = 24;

/// A single ADF t-statistic for an explosive alternative, for one window.
///
/// Fits `Δy_t = μ + δ·y_{t-1} + Σφ_i·Δy_{t-i} + ε` by OLS over the window and
/// returns the t-statistic on δ. The SADF is the maximum over windows ending at
/// the final observation; the GSADF is the maximum over ALL windows, which is
/// what makes it able to detect a bubble that already burst and did not end at
/// the sample's last point.
pub fn adf_t(y: &[f64], lags: usize) -> Option<f64> {
    // Build the regression rows.
    let start = 1 + lags;
    if y.len() <= start + 2 {
        return None;
    }
    let n = y.len();
    let k = 2 + lags; // intercept, y_{t-1}, lagged differences

    let mut x: Vec<Vec<f64>> = Vec::new();
    let mut d: Vec<f64> = Vec::new();
    for t in start..n {
        let mut row = Vec::with_capacity(k);
        row.push(1.0);
        row.push(y[t - 1]);
        for i in 1..=lags {
            row.push(y[t - i] - y[t - i - 1]);
        }
        x.push(row);
        d.push(y[t] - y[t - 1]);
    }
    if x.len() <= k {
        return None;
    }

    // OLS via normal equations: beta = (X'X)^-1 X'd. k is tiny (<= 5), so a
    // Gauss-Jordan solve is plenty and avoids a linear-algebra dependency.
    let m = x.len();
    let mut xtx = vec![vec![0.0f64; k]; k];
    let mut xtd = vec![0.0f64; k];
    for r in 0..m {
        for i in 0..k {
            xtd[i] += x[r][i] * d[r];
            for j in 0..k {
                xtx[i][j] += x[r][i] * x[r][j];
            }
        }
    }
    let inv = invert(&mut xtx)?;
    let beta: Vec<f64> = (0..k)
        .map(|i| (0..k).map(|j| inv[i][j] * xtd[j]).sum())
        .collect();

    // Residual variance for the standard error of beta_1.
    let mut ssr = 0.0;
    for r in 0..m {
        let fit: f64 = (0..k).map(|i| x[r][i] * beta[i]).sum();
        ssr += (d[r] - fit).powi(2);
    }
    let dof = m as f64 - k as f64;
    if dof <= 0.0 {
        return None;
    }
    let sigma2 = ssr / dof;
    let var_b1 = sigma2 * inv[1][1];
    if var_b1 <= 0.0 || !var_b1.is_finite() {
        return None;
    }
    Some(beta[1] / var_b1.sqrt())
}

/// Gauss-Jordan inverse of a small square matrix, or None when singular.
fn invert(a: &mut [Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut inv = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        inv[i][i] = 1.0;
    }
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        inv.swap(col, piv);
        let p = a[col][col];
        for j in 0..n {
            a[col][j] /= p;
            inv[col][j] /= p;
        }
        for r in 0..n {
            if r != col {
                let f = a[r][col];
                if f != 0.0 {
                    for j in 0..n {
                        a[r][j] -= f * a[col][j];
                        inv[r][j] -= f * inv[col][j];
                    }
                }
            }
        }
    }
    Some(inv)
}

/// The GSADF statistic: the supremum of the ADF t-statistic over every window of
/// at least `MIN_WINDOW` observations, and the window that produced it.
///
/// Returns `(statistic, window_start_index, window_end_index)`. The dates are
/// indices because this is pure: the caller maps them onto labels.
pub fn gsadf(y: &[f64], lags: usize) -> Option<(f64, usize, usize)> {
    let n = y.len();
    if n < MIN_WINDOW + 2 {
        return None;
    }
    let mut best: Option<(f64, usize, usize)> = None;
    // r1 = window start fraction, r2 = window end fraction, with the usual
    // requirement that the window be long enough for the regression.
    let min_len = MIN_WINDOW;
    for start in 0..=(n - min_len) {
        for end in (start + min_len)..=n {
            let w = &y[start..end];
            if let Some(t) = adf_t(w, lags) {
                if best.as_ref().map(|(b, _, _)| t > *b).unwrap_or(true) {
                    best = Some((t, start, end - 1));
                }
            }
        }
    }
    best
}

/// Simulated critical values for the GSADF statistic under the null of a random
/// walk, by Monte Carlo.
///
/// The published table (Phillips-Shi-Yu) is the correct source and is NOT
/// reproduced here; this generates its own thresholds. That is a real weakness
/// and is disclosed in the output. A fixed seed keeps the result deterministic,
/// which matters more for this tool than statistical purity — a critical value
/// that moved between runs would make the whole report non-reproducible.
/// Critical values are a pure function of `(sample_len, reps, seed)`, and the
/// Monte Carlo is by far the most expensive part of this module (200 simulations,
/// each running GSADF over every window). Reports are built repeatedly in tests
/// and in one `site` run, so recomputing identical thresholds is pure waste.
/// Memoising them cut the integration suite from 74s to a fraction of that.
fn cached_critical_values(sample_len: usize, reps: usize, seed: u64) -> CriticalValues {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<(usize, usize, u64), CriticalValues>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (sample_len, reps, seed);
    if let Ok(g) = cache.lock() {
        if let Some(v) = g.get(&key) {
            return *v;
        }
    }
    let computed = simulated_critical_values(sample_len, reps, seed);
    if let Ok(mut g) = cache.lock() {
        g.insert(key, computed);
    }
    computed
}

pub fn simulated_critical_values(sample_len: usize, reps: usize, seed: u64) -> CriticalValues {
    // xorshift64*, so no rand dependency and a reproducible stream.
    let mut state = seed | 1;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        (state.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut stats: Vec<f64> = Vec::with_capacity(reps);
    for _ in 0..reps {
        // Standard normal via Box-Muller. A pure random walk null: no drift, no
        // fat tails, no volatility clustering — all of which real prices have, so
        // these thresholds are indicative rather than exact.
        let mut walk = Vec::with_capacity(sample_len);
        let mut v = 100.0f64;
        for _ in 0..sample_len {
            let u1 = next().max(1e-12);
            let u2 = next();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            v += z;
            walk.push(v);
        }
        if let Some((t, _, _)) = gsadf(&walk, 0) {
            stats.push(t);
        }
    }
    stats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| -> f64 {
        if stats.is_empty() {
            return f64::NAN;
        }
        let i = ((stats.len() - 1) as f64 * p).round() as usize;
        stats[i]
    };
    CriticalValues {
        p90: q(0.90),
        p95: q(0.95),
        p99: q(0.99),
        reps: stats.len(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CriticalValues {
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub reps: usize,
}

impl CriticalValues {
    /// Approximate significance of an observed statistic against the simulated
    /// null. Reported as a band, never as a precise probability.
    pub fn significance(&self, stat: f64) -> &'static str {
        if stat >= self.p99 {
            "significant at the 1% level"
        } else if stat >= self.p95 {
            "significant at the 5% level"
        } else if stat >= self.p90 {
            "significant at the 10% level"
        } else {
            "not significant at the 10% level"
        }
    }
}

/// The full test result, ready to report.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GsadfResult {
    pub statistic: f64,
    pub critical: CriticalValues,
    pub significance: String,
    pub window_start_date: String,
    pub window_end_date: String,
    pub observations: usize,
}

impl GsadfResult {
    pub fn is_significant(&self) -> bool {
        self.significance != "not significant at the 10% level"
    }
}

/// Run the test over a dated price series.
///
/// `reps` controls the Monte Carlo size; the caller sets it from config so the
/// cost is explicit and tunable.
pub fn run(points: &[Point], reps: usize, seed: u64) -> Option<GsadfResult> {
    let y: Vec<f64> = points.iter().map(|p| p.value).collect();
    if y.len() < MIN_WINDOW + 2 {
        return None;
    }
    let (stat, s, e) = gsadf(&y, 0)?;
    let critical = cached_critical_values(y.len(), reps, seed);
    Some(GsadfResult {
        statistic: stat,
        significance: critical.significance(stat).to_string(),
        critical,
        window_start_date: points.get(s).map(|p| p.date.clone()).unwrap_or_default(),
        window_end_date: points.get(e).map(|p| p.date.clone()).unwrap_or_default(),
        observations: y.len(),
    })
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

    /// A deterministic pseudo-random walk.
    fn walk(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed | 1;
        let mut v = 100.0;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            let u = (s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64;
            v += (u * 2.0 - 1.0) * 2.0;
            out.push(v);
        }
        out
    }

    /// An explosive series with a small amount of noise.
    ///
    /// The noise is load-bearing for the TEST, not just decoration: on a purely
    /// deterministic series the regressors (intercept, level, lagged difference)
    /// become collinear to machine precision, X'X is singular, and the solve
    /// returns garbage. Real price series always carry noise, so this is the
    /// realistic case; there is a separate test for the degenerate noiseless one.
    fn explosive(n: usize) -> Vec<f64> {
        let mut v = 100.0f64;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            v *= 1.02;
            // Deterministic pseudo-noise: keeps the test reproducible.
            let jitter = ((i * 2654435761usize) % 1000) as f64 / 1000.0 - 0.5;
            out.push(v * (1.0 + jitter * 0.004));
        }
        out
    }

    #[test]
    fn adf_t_is_strongly_positive_for_an_explosive_series() {
        // The whole point: a compounding series should look explosive.
        let y = explosive(60);
        let t = adf_t(&y, 1).expect("regression should be solvable");
        // Against simulated 1% critical values of roughly +2, a t near +10 is
        // decisive. The exact figure depends on the jitter in the generator, so
        // the bound is set well clear of significance rather than at a value
        // tuned to the current sample.
        assert!(
            t > 5.0,
            "an explosive series must give a large positive t, got {}",
            t
        );
    }

    #[test]
    fn adf_t_is_not_large_for_a_pure_random_walk() {
        let y = walk(120, 42);
        let t = adf_t(&y, 1).expect("solvable");
        assert!(
            t < 5.0,
            "a random walk should not look explosive, got t={}",
            t
        );
    }

    #[test]
    fn gsadf_separates_explosive_from_random() {
        let (t_exp, _, _) = gsadf(&explosive(80), 0).unwrap();
        let (t_rw, _, _) = gsadf(&walk(80, 7), 0).unwrap();
        assert!(
            t_exp > t_rw,
            "explosive ({}) must exceed random ({})",
            t_exp,
            t_rw
        );
    }

    #[test]
    fn gsadf_locoates_the_explosive_window() {
        // Flat, then a bubble in the middle, then flat again. The GSADF must
        // locate the bubble rather than the end of the sample — which is exactly
        // why the generalised form exists over the simpler SADF.
        let mut y = vec![100.0; 30];
        let mut v = 100.0;
        for _ in 0..25 {
            v *= 1.05;
            y.push(v);
        }
        for _ in 0..30 {
            y.push(v);
        }
        let (_, s, e) = gsadf(&y, 0).unwrap();
        assert!(
            s >= 20 && e <= 60,
            "window {}..{} should sit inside the bubble region 25..55",
            s,
            e
        );
    }

    #[test]
    fn critical_values_are_ordered_and_deterministic() {
        let a = simulated_critical_values(60, 40, 12345);
        let b = simulated_critical_values(60, 40, 12345);
        assert_eq!(a, b, "the same seed must give the same thresholds");
        assert!(a.p90 < a.p95 && a.p95 < a.p99, "quantiles must ascend");
        assert!(a.p95.is_finite());
    }

    #[test]
    fn significance_bands_are_ordered() {
        let c = CriticalValues {
            p90: 1.0,
            p95: 2.0,
            p99: 3.0,
            reps: 100,
        };
        assert_eq!(c.significance(0.5), "not significant at the 10% level");
        assert_eq!(c.significance(1.5), "significant at the 10% level");
        assert_eq!(c.significance(2.5), "significant at the 5% level");
        assert_eq!(c.significance(9.9), "significant at the 1% level");
    }

    #[test]
    fn too_short_a_series_is_refused_not_guessed() {
        let short: Vec<Point> = (0..5).map(|i| pt("2026-01-01", 100.0 + i as f64)).collect();
        assert!(
            run(&short, 20, 1).is_none(),
            "must refuse a series too short to test"
        );
    }

    #[test]
    fn a_deterministic_explosive_series_is_degenerate_but_must_not_panic() {
        // A purely deterministic compounding series makes the regressors
        // collinear, so the normal equations are singular. This test documents
        // that the implementation returns a finite value or refuses, rather than
        // emitting NaN or panicking — it must never report a wild number for an
        // input that carries no information about volatility.
        let mut v = 100.0f64;
        let y: Vec<f64> = (0..60)
            .map(|_| {
                v *= 1.02;
                v
            })
            .collect();
        match adf_t(&y, 1) {
            None => {}
            Some(t) => assert!(t.is_finite(), "must not emit NaN or infinity"),
        }
    }

    #[test]
    fn a_monotone_series_with_no_variance_does_not_panic() {
        // Degenerate input: singular normal equations. Must return None rather
        // than NaN or a panic.
        let flat: Vec<f64> = vec![100.0; 60];
        let r = adf_t(&flat, 1);
        assert!(r.is_none() || r.unwrap().is_finite());
    }

    #[test]
    fn run_reports_the_window_dates_from_the_series() {
        let pts: Vec<Point> = explosive(80)
            .into_iter()
            .enumerate()
            .map(|(i, v)| pt(&format!("20{:02}-01-01", 10 + i / 12), v))
            .collect();
        let r = run(&pts, 30, 99).expect("should produce a result");
        assert_eq!(r.observations, 80);
        assert!(!r.window_start_date.is_empty());
        assert!(!r.window_end_date.is_empty());
    }
}
