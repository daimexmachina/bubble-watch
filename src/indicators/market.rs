//! Market-price indicators: valuation stretch, concentration, breadth, volatility.
//!
//! All four are proxies, and each one says so in its `detail` field. They are
//! kept because they map onto Capital Economics' indicators and because price
//! data is the most reliable thing we can actually get for free — but a reader
//! should never mistake them for the ideal measures.

use super::{Ctx, Indicator};
use crate::model::{Provenance, Reading, Series};
use std::collections::HashMap;

/// Deviation of price from its own log-linear trend.
///
/// Chosen over a simple moving average because a log-linear fit is scale-free
/// and captures the exponential character of a long equity uptrend; a linear
/// trend on a compounding index systematically understates stretch late in a
/// bull market.
pub fn log_trend_deviation_pct(s: &Series) -> Option<f64> {
    let v: Vec<f64> = s.values();
    if v.len() < 24 || v.iter().any(|x| *x <= 0.0) {
        return None;
    }
    let ys: Vec<f64> = v.iter().map(|x| x.ln()).collect();
    let n = ys.len() as f64;
    let xs: Vec<f64> = (0..ys.len()).map(|i| i as f64).collect();
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let num: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let den: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    if den.abs() < f64::EPSILON {
        return None;
    }
    let b = num / den;
    let a = my - b * mx;
    let last_x = (ys.len() - 1) as f64;
    let fitted = a + b * last_x;
    let last = *ys.last()?;
    Some(((last - fitted).exp() - 1.0) * 100.0)
}

/// Percentage change over the whole series window.
pub fn pct_change(s: &Series) -> Option<f64> {
    let v = s.values();
    if v.len() < 2 {
        return None;
    }
    let first = *v.first()?;
    let last = *v.last()?;
    if first.abs() < f64::EPSILON {
        return None;
    }
    Some((last / first - 1.0) * 100.0)
}

/// Latest value of a ratio series versus the mean of its trailing `window`
/// observations, as a percentage. Negative means the ratio has fallen below its
/// own recent average (breadth narrowing).
pub fn ratio_last_vs_sma_pct(a: &Series, b: &Series, window: usize) -> Option<(f64, f64)> {
    let bmap: HashMap<&str, f64> = b
        .points
        .iter()
        .map(|p| (p.date.as_str(), p.value))
        .collect();
    let ratios: Vec<f64> = a
        .points
        .iter()
        .filter_map(|p| bmap.get(p.date.as_str()).map(|bv| p.value / bv))
        .collect();
    if window == 0 || ratios.len() < window {
        return None;
    }
    let last = *ratios.last()?;
    let slice = &ratios[ratios.len() - window..];
    let mean = slice.iter().sum::<f64>() / window as f64;
    if mean.abs() < f64::EPSILON {
        return None;
    }
    Some((last, (last / mean - 1.0) * 100.0))
}

/// Annualized realized volatility of daily log returns over the last `window`
/// observations, in percent.
pub fn realized_vol_pct(s: &Series, window: usize) -> Option<f64> {
    let v = s.values();
    if v.len() < window + 1 {
        return None;
    }
    let tail = &v[v.len() - (window + 1)..];
    let rets: Vec<f64> = tail
        .windows(2)
        .filter_map(|w| {
            if w[0] > 0.0 && w[1] > 0.0 {
                Some((w[1] / w[0]).ln())
            } else {
                None
            }
        })
        .collect();
    if rets.len() < 2 {
        return None;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (rets.len() - 1) as f64;
    Some(var.sqrt() * 252f64.sqrt() * 100.0)
}

fn prov_of(s: &Series) -> Provenance {
    s.provenance.clone()
}

// ---------------------------------------------------------------------------

pub struct ValuationStretch;

impl Indicator for ValuationStretch {
    fn id(&self) -> &'static str {
        "valuation_stretch"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };
        let s = match ctx.obs.yahoo.get("SPX_5y_mo") {
            Some(s) => s,
            None => {
                return Reading::Unavailable {
                    reason: "S&P 500 5-year monthly series unavailable from yahoo".into(),
                }
            }
        };
        let Some(dev) = log_trend_deviation_pct(s) else {
            return Reading::Unavailable {
                reason: "insufficient history to fit a 5-year trend".into(),
            };
        };
        let stress = crate::score::interpolate(dev, &ic.anchors);

        // The 10Y yield is the theoretically right counterpart for an equity
        // risk premium, but we have no free EPS series, so it is reported as
        // context and deliberately NOT scored here.
        let tnx = ctx
            .obs
            .yahoo
            .get("TNX_1y_d")
            .and_then(|t| t.latest_value())
            .map(|v| format!("{:.2}%", v))
            .unwrap_or_else(|| "unavailable".into());

        Reading::Scored {
            stress,
            value: dev,
            unit: ic.unit.clone(),
            detail: format!(
                "S&P 500 is {:+.1}% versus its 5-year log-linear trend (last {:.0}). \
                 PROXY: measures price stretch, not earnings. Context, not scored: 10-year \
                 Treasury yield {} — a rising long yield compresses the equity risk premium \
                 without any change in price.",
                dev,
                s.latest_value().unwrap_or(f64::NAN),
                tnx
            ),
            provenance: prov_of(s),
        }
    }
}

pub struct Concentration;

impl Indicator for Concentration {
    fn id(&self) -> &'static str {
        "concentration"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };
        let (Some(spy), Some(rsp)) = (ctx.obs.yahoo.get("SPY_1y_d"), ctx.obs.yahoo.get("RSP_1y_d"))
        else {
            return Reading::Unavailable {
                reason: "SPY or RSP series unavailable from yahoo".into(),
            };
        };
        let (Some(cs), Some(cr)) = (pct_change(spy), pct_change(rsp)) else {
            return Reading::Unavailable {
                reason: "insufficient price history for a 12-month comparison".into(),
            };
        };
        let spread = cs - cr;
        let stress = crate::score::interpolate(spread, &ic.anchors);
        Reading::Scored {
            stress,
            value: spread,
            unit: ic.unit.clone(),
            detail: format!(
                "Cap-weight SPY {:+.1}% vs equal-weight RSP {:+.1}% over 12 months — a \
                 {:+.1}pp gap. PROXY: a large-cap-versus-small-cap regime measure, not a \
                 free-float market-cap share. A NEGATIVE gap means the average constituent is \
                 outperforming the index, which is a healthy-breadth reading and scores as low \
                 stress.",
                cs, cr, spread
            ),
            provenance: prov_of(spy),
        }
    }
}

pub struct Breadth;

impl Indicator for Breadth {
    fn id(&self) -> &'static str {
        "breadth"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };
        let (Some(rsp), Some(spy)) = (ctx.obs.yahoo.get("RSP_1y_d"), ctx.obs.yahoo.get("SPY_1y_d"))
        else {
            return Reading::Unavailable {
                reason: "RSP or SPY series unavailable from yahoo".into(),
            };
        };
        let Some((ratio, dev)) = ratio_last_vs_sma_pct(rsp, spy, 200) else {
            return Reading::Unavailable {
                reason: "insufficient overlapping history for a 200-day breadth measure".into(),
            };
        };
        let stress = crate::score::interpolate(dev, &ic.anchors);
        Reading::Scored {
            stress,
            value: dev,
            unit: ic.unit.clone(),
            detail: format!(
                "RSP/SPY ratio {:.4} sits {:+.1}% versus its own 200-day average. \
                 PROXY: relative performance, not an advance/decline line or the percentage \
                 of stocks above their 200-day. Falling below average indicates a narrowing \
                 rally; above average indicates broad participation.",
                ratio, dev
            ),
            provenance: prov_of(rsp),
        }
    }
}

pub struct Volatility;

impl Indicator for Volatility {
    fn id(&self) -> &'static str {
        "volatility"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };
        let Some(vix) = ctx.obs.yahoo.get("VIX_1y_d") else {
            return Reading::Unavailable {
                reason: "VIX series unavailable from yahoo".into(),
            };
        };
        let Some(level) = vix.latest_value() else {
            return Reading::Unavailable {
                reason: "VIX series empty".into(),
            };
        };
        let stress = crate::score::interpolate(level, &ic.anchors);
        let rv = ctx
            .obs
            .yahoo
            .get("SPX_1y_d")
            .and_then(|s| realized_vol_pct(s, 20))
            .map(|v| format!("{:.1}%", v))
            .unwrap_or_else(|| "unavailable".into());
        Reading::Scored {
            stress,
            value: level,
            unit: ic.unit.clone(),
            detail: format!(
                "VIX at {:.1}. Cross-check: 20-day annualized realized volatility of the S&P 500 \
                 is {}. The anchors are deliberately shallow at the low end: a calm market is \
                 mildly bubble-like (complacency) rather than strongly so, because sustained \
                 HIGH volatility more often marks a peak already passed.",
                level, rv
            ),
            provenance: prov_of(vix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;

    fn series(vals: &[f64]) -> Series {
        Series {
            provenance: Provenance {
                source: "t".into(),
                endpoint: "t".into(),
                as_of: "2026-09-16".into(),
                retrieved_at: "2026-09-16T00:00:00Z".into(),
            },
            points: vals
                .iter()
                .enumerate()
                .map(|(i, v)| Point {
                    date: format!("d{}", i),
                    value: *v,
                })
                .collect(),
        }
    }

    #[test]
    fn trend_deviation_is_zero_on_a_perfect_exponential() {
        // Exactly exponential: value = 2^i, so log-linear fits perfectly.
        let vals: Vec<f64> = (0..60).map(|i| 2f64.powi(i)).collect();
        let d = log_trend_deviation_pct(&series(&vals)).unwrap();
        assert!(d.abs() < 1e-6, "expected ~0 deviation, got {}", d);
    }

    #[test]
    fn trend_deviation_is_positive_when_price_runs_above_trend() {
        let mut vals: Vec<f64> = (0..60).map(|i| 1.01f64.powi(i)).collect();
        let n = vals.len();
        vals[n - 1] *= 1.25; // blow-off top
        let d = log_trend_deviation_pct(&series(&vals)).unwrap();
        assert!(d > 15.0, "blow-off should read as stretched, got {}", d);
    }

    #[test]
    fn trend_deviation_needs_enough_history() {
        assert!(log_trend_deviation_pct(&series(&[1.0, 2.0, 3.0])).is_none());
    }

    #[test]
    fn pct_change_basic() {
        let c = pct_change(&series(&[100.0, 110.0])).unwrap();
        assert!((c - 10.0).abs() < 1e-9);
    }

    #[test]
    fn ratio_below_sma_is_negative() {
        // a rises sharply then pulls back relative to a flat b, so the latest
        // ratio sits below its own recent average.
        let a = series(&[10.0, 12.0, 14.0, 16.0, 13.0]);
        let b = series(&[10.0, 10.0, 10.0, 10.0, 10.0]);
        let (_, dev) = ratio_last_vs_sma_pct(&a, &b, 4).unwrap();
        assert!(dev < 0.0, "expected below-average ratio, got {}", dev);
    }

    #[test]
    fn ratio_above_sma_is_positive() {
        let a = series(&[10.0, 10.0, 10.0, 10.0, 14.0]);
        let b = series(&[10.0, 10.0, 10.0, 10.0, 10.0]);
        let (_, dev) = ratio_last_vs_sma_pct(&a, &b, 4).unwrap();
        assert!(dev > 0.0, "expected above-average ratio, got {}", dev);
    }

    #[test]
    fn ratio_aligns_on_dates_not_positions() {
        let mut a = series(&[10.0, 20.0, 30.0]);
        a.points[0].date = "2026-01-01".into();
        a.points[1].date = "2026-01-02".into();
        a.points[2].date = "2026-01-03".into();
        let mut b = series(&[1.0, 1.0, 1.0]);
        b.points[0].date = "2026-01-03".into();
        b.points[1].date = "2026-01-04".into();
        b.points[2].date = "2026-01-05".into();
        // Only 2026-01-03 overlaps, and window 3 > 1 overlap -> None.
        assert!(ratio_last_vs_sma_pct(&a, &b, 3).is_none());
    }

    #[test]
    fn realized_vol_is_zero_for_a_flat_series() {
        let v = realized_vol_pct(&series(&[100.0; 25]), 20).unwrap();
        assert!(v.abs() < 1e-9);
    }
}
