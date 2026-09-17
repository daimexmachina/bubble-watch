//! Inline-SVG charts for the served site.
//!
//! No JavaScript, no external resources, no chart library: the report must render
//! with no network access at all, because it is most interesting exactly when
//! something is happening in the market. Everything here is arithmetic on real
//! archived points and a handful of SVG primitives.
//!
//! Two honesty rules shape every chart in this module, and both exist to stop a
//! reader drawing a stronger conclusion than the data supports.
//!
//! 1. **The composite chart uses a FIXED 0-100 scale with the documented phase
//!    bands shaded.** Auto-scaling to the data range is the standard trick and it
//!    is exactly what makes a chart lie: a move from 32 to 34 would fill the
//!    whole plot and read as a dramatic surge. On a fixed scale it reads as what
//!    it is — a small move well inside the early band.
//!
//! 2. **A missing measurement BREAKS the line; it is never interpolated across.**
//!    An indicator that could not be measured on a given day is a gap in the data,
//!    not a zero, and not a straight line between the days either side of it. A
//!    continuous stroke across a gap is a fabricated observation.

use crate::model::TrendPoint;

/// Chart canvas height, in viewBox units.
const H: f64 = 150.0;
/// Plot width. Charts are drawn at a fixed logical width and scaled by the
/// container, so they are legible on a phone without a resize script.
const W: f64 = 900.0;
/// Room under the plot for date labels.
const AXIS_H: f64 = 18.0;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// "2026-09-17" -> "09-17". Axis labels are short by design; the full date is in
/// the tooltip and the table beneath.
fn short_date(d: &str) -> String {
    let parts: Vec<&str> = d.split('-').collect();
    if parts.len() == 3 {
        format!("{}-{}", parts[1], parts[2])
    } else {
        d.to_string()
    }
}

/// Evenly spaced tick indices, always including the first and last point so the
/// reader can see the actual span covered.
fn tick_indices(n: usize, want: usize) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    if n <= want {
        return (0..n).collect();
    }
    let mut v = Vec::new();
    for k in 0..want {
        let i = (k as f64 * (n - 1) as f64 / (want - 1) as f64).round() as usize;
        if !v.contains(&i) {
            v.push(i);
        }
    }
    v
}

/// Shared <defs>: an arrowhead marker used by the annotation overlays.
fn defs() -> &'static str {
    "<defs><marker id='arw' viewBox='0 0 8 8' refX='7' refY='4' markerWidth='6' markerHeight='6' \
     orient='auto'><path d='M0,0 L8,4 L0,8 z' fill='#c62828'/></marker></defs>"
}

/// The composite over time, on a FIXED 0-100 scale, with the phase bands shaded
/// and labelled at the right edge.
///
/// The fixed scale is the point of this chart. Auto-scaling would make a two-point
/// move fill the plot; here, the distance between 32 and 34 is visibly small
/// against the 0-100 range and the shaded bands give it meaning.
pub fn composite_chart(points: &[TrendPoint]) -> String {
    // Phase bands, matching the thresholds in config/indicators.toml. They are
    // drawn as background so the line is read against the documented boundaries.
    let bands = [
        (0.0, 35.0, "#e8f5e9", "early"),
        (35.0, 55.0, "#fffde7", "mid"),
        (55.0, 75.0, "#fff3e0", "late"),
        (75.0, 100.0, "#fdecea", "critical"),
    ];
    let y = |v: f64| -> f64 { H - (v.clamp(0.0, 100.0) / 100.0) * H };

    let mut band_svg = String::new();
    let mut band_labels = String::new();
    for (lo, hi, colour, label) in bands {
        let (y_top, y_bot) = (y(hi), y(lo));
        band_svg.push_str(&format!(
            "<rect x='0' y='{y:.1}' width='{w:.1}' height='{h:.1}' fill='{c}'/>",
            y = y_top,
            w = W,
            h = (y_bot - y_top).max(0.0),
            c = colour
        ));
        band_labels.push_str(&format!(
            "<text x='{x:.1}' y='{ty:.1}' font-size='9' fill='#999'>{l}</text>",
            x = W - 2.0,
            ty = (y_top + y_bot) / 2.0 + 3.0,
            l = label
        ));
    }

    // Horizontal gridlines every 25 points.
    let mut grid = String::new();
    for v in [25.0f64, 50.0, 75.0] {
        grid.push_str(&format!(
            "<line x1='0' y1='{yy:.1}' x2='{w:.1}' y2='{yy:.1}' stroke='#00000012' stroke-width='1'/>\
             <text x='3' y='{ty:.1}' font-size='9' fill='#aaa'>{v:.0}</text>",
            yy = y(v),
            w = W,
            ty = y(v) - 2.0,
            v = v
        ));
    }

    if points.len() < 2 {
        return format!(
            "<svg viewBox='-2 -2 904 {vh}' width='100%' height='{vh}' role='img' \
             aria-label='composite score over recorded runs'>\
             {bands}{grid}{labels}\
             <text x='{cx:.1}' y='{cy:.1}' font-size='12' fill='#777' text-anchor='middle'>\
             Not enough recorded runs to chart a trend yet — a line needs at least two dated runs.\
             </text></svg>",
            vh = H + AXIS_H + 4.0,
            bands = band_svg,
            grid = grid,
            labels = band_labels,
            cx = W / 2.0,
            cy = H / 2.0
        );
    }

    let n = points.len();
    let x_at = |i: usize| -> f64 {
        if n == 1 {
            W / 2.0
        } else {
            i as f64 / (n - 1) as f64 * W
        }
    };

    let poly: String = points
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{:.1},{:.1} ", x_at(i), y(p.composite)))
        .collect();

    // Date ticks.
    let mut ticks = String::new();
    for i in tick_indices(n, 6) {
        ticks.push_str(&format!(
            "<text x='{x:.1}' y='{ty:.1}' font-size='9.5' fill='#888' text-anchor='middle'>{d}</text>",
            x = x_at(i),
            ty = H + 13.0,
            d = esc(&short_date(&points[i].date))
        ));
    }

    // Points, plus the newest value called out.
    let mut dots = String::new();
    for (i, p) in points.iter().enumerate() {
        dots.push_str(&format!(
            "<circle cx='{x:.1}' cy='{yy:.1}' r='2.6' fill='#1a73e8'/>",
            x = x_at(i),
            yy = y(p.composite)
        ));
    }
    let last = points.last().unwrap();
    let (lx, ly) = (x_at(n - 1), y(last.composite));
    let first = &points[0];
    let delta = last.composite - first.composite;
    let delta_txt = format!(
        "{:+.1} over {} day(s)",
        delta,
        crate::history::days_between(&first.date, &last.date)
            .unwrap_or(0.0)
            .round() as i64
    );

    format!(
        "<svg viewBox='-2 -2 904 {vh}' width='100%' height='{vh}' role='img' \
         aria-label='composite score over recorded runs'>\
         {bands}{grid}{labels}{ticks}\
         <polyline fill='none' stroke='#1a73e8' stroke-width='2.2' points='{poly}'/>\
         {dots}\
         <circle cx='{lx:.1}' cy='{ly:.1}' r='5' fill='#1a73e8' stroke='#fff' stroke-width='1.5'/>\
         <text x='{tx:.1}' y='{ty:.1}' font-size='11' fill='#1a73e8' text-anchor='end'>{val:.1}</text>\
         <text x='-1' y='-6' font-size='10' fill='#777'>0-100 fixed scale · {d}</text>\
         </svg>",
        vh = H + AXIS_H + 4.0,
        bands = band_svg,
        grid = grid,
        labels = band_labels,
        ticks = ticks,
        poly = poly.trim(),
        dots = dots,
        lx = lx,
        ly = ly,
        tx = lx,
        ty = (ly - 9.0).max(11.0),
        val = last.composite,
        d = esc(&delta_txt)
    )
}

/// One indicator's stress over time.
///
/// Unavailable days BREAK the line. The gaps are computed as runs of consecutive
/// missing values and the polyline is split at each one, so no stroke is ever
/// drawn between two days whose intervening values were never measured.
pub fn indicator_chart(points: &[TrendPoint], id: &str, label: &str) -> String {
    let y = |v: f64| -> f64 { H - (v.clamp(0.0, 100.0) / 100.0) * H };

    // Gather (index, stress) for days where this indicator was actually scored.
    let present: Vec<(usize, f64)> = points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.stresses.get(id).map(|v| (i, *v)))
        .collect();

    if present.len() < 2 {
        return format!(
            "<div class='chart-empty'>Not enough recorded days with a reading for <b>{l}</b> to \
             chart a trend. {n} day(s) have one. Days when this could not be measured are left \
             blank rather than drawn as zero.</div>",
            l = esc(label),
            n = present.len()
        );
    }

    let n = points.len();
    let x_at = |i: usize| -> f64 {
        if n == 1 {
            W / 2.0
        } else {
            i as f64 / (n - 1) as f64 * W
        }
    };

    // Split into contiguous runs of present days. A run of one point draws a dot
    // with no line, which is the honest rendering of an isolated measurement.
    let mut strokes: Vec<String> = Vec::new();
    let mut run: Vec<(usize, f64)> = Vec::new();
    let mut prev_idx: Option<usize> = None;
    for (i, v) in &present {
        let contiguous = prev_idx.map(|p| *i == p + 1).unwrap_or(true);
        if !contiguous && !run.is_empty() {
            strokes.push(run_to_polyline(&run, &x_at, &y));
            run.clear();
        }
        run.push((*i, *v));
        prev_idx = Some(*i);
    }
    if !run.is_empty() {
        strokes.push(run_to_polyline(&run, &x_at, &y));
    }
    let lines: String = strokes.join("");

    // Dots on every measured day, so isolated points are still visible.
    let mut dots = String::new();
    for (i, v) in &present {
        dots.push_str(&format!(
            "<circle cx='{x:.1}' cy='{yy:.1}' r='2.4' fill='#6a1b9a'/>",
            x = x_at(*i),
            yy = y(*v)
        ));
    }

    // Mark the gap counts explicitly, so a broken line is explained rather than
    // looking like a rendering fault.
    let missing = points.len() - present.len();
    // Left-aligned, and the "latest" note below is right-aligned: both sat on the
    // same baseline at first and overlapped into an unreadable smear.
    let gap_note = if missing == 0 {
        String::new()
    } else {
        format!(
            "<text x='0' y='-6' font-size='10' fill='#a1887f' text-anchor='start'>\
             {m} day(s) with no reading — the line breaks there</text>",
            m = missing
        )
    };

    let mut ticks = String::new();
    for i in tick_indices(n, 6) {
        ticks.push_str(&format!(
            "<text x='{x:.1}' y='{ty:.1}' font-size='9.5' fill='#888' text-anchor='middle'>{d}</text>",
            x = x_at(i),
            ty = H + 13.0,
            d = esc(&short_date(&points[i].date))
        ));
    }

    let last_v = present.last().unwrap().1;
    let first_v = present.first().unwrap().1;

    format!(
        "<svg viewBox='-2 -12 904 {vh}' width='100%' height='{vh}' role='img' \
         aria-label='{lbl} stress over recorded runs'>{defs}\
         <line x1='0' y1='{ymid:.1}' x2='{w:.1}' y2='{ymid:.1}' stroke='#00000010' stroke-width='1'/>\
         <line x1='0' y1='{ytop:.1}' x2='{w:.1}' y2='{ytop:.1}' stroke='#00000008' stroke-width='1'/>\
         <text x='3' y='{ty1:.1}' font-size='9' fill='#aaa'>0</text>\
         <text x='3' y='{ty2:.1}' font-size='9' fill='#aaa'>50</text>\
         <text x='3' y='{ty3:.1}' font-size='9' fill='#aaa'>100</text>\
         {lines}{dots}{ticks}{gap}\
         <text x='{w:.1}' y='-6' font-size='10' fill='#6a1b9a' text-anchor='end' \
         opacity='0.85'>latest {lv:.1} ({d:+.1} over the recorded span)</text>\
         </svg>",
        vh = H + AXIS_H + 16.0,
        defs = defs(),
        w = W,
        ymid = y(50.0),
        ytop = y(100.0),
        ty1 = y(0.0) - 2.0,
        ty2 = y(50.0) - 2.0,
        ty3 = y(100.0) + 9.0,
        lines = lines,
        dots = dots,
        ticks = ticks,
        gap = gap_note,
        lv = last_v,
        d = last_v - first_v,
        lbl = esc(label)
    )
}

/// Turn a contiguous run of points into a polyline, or a single dot if the run is
/// one point long (a line needs two).
fn run_to_polyline(
    run: &[(usize, f64)],
    x_at: &dyn Fn(usize) -> f64,
    y: &dyn Fn(f64) -> f64,
) -> String {
    if run.len() < 2 {
        return String::new();
    }
    let pts: String = run
        .iter()
        .map(|(i, v)| format!("{:.1},{:.1} ", x_at(*i), y(*v)))
        .collect();
    format!(
        "<polyline fill='none' stroke='#6a1b9a' stroke-width='2' points='{}'/>",
        pts.trim()
    )
}

/// "Movement since the previous recorded run", for the direction-of-travel panel.
///
/// A horizontal divergence chart: each indicator is a bar from its previous reading
/// to its current one, so the eye reads direction and magnitude together. Every bar
/// carries the day count, because a movement means nothing without its elapsed time.
pub fn movement_chart(deltas: &[(String, String, f64, f64)], days: f64) -> String {
    if deltas.is_empty() {
        return "<div class='chart-empty'>No indicator was measured on both runs, so there is \
                nothing to chart. A measurement missing on one side is not treated as a change of \
                zero.</div>"
            .to_string();
    }
    let row_h = 26.0;
    let h = (deltas.len() as f64 * row_h).max(row_h) + 8.0;
    // Two columns of labels, then a symmetric axis around x=0.
    let label_w = 190.0;
    let axis_x = label_w + 250.0;
    let half = 230.0;
    // Scale to the largest absolute change in this run, so a small week still
    // reads clearly - but print the range, because this axis IS data-scaled and
    // the reader must be told that (unlike the composite chart, which is fixed).
    let max_abs = deltas
        .iter()
        .map(|d| d.2.abs())
        .fold(0.0f64, f64::max)
        .max(1.0);

    let mut bars = String::new();
    for (k, (id, label, delta, now)) in deltas.iter().enumerate() {
        let cy = row_h * k as f64 + row_h / 2.0;
        let len = (delta.abs() / max_abs) * half;
        let (x, w, colour) = if *delta >= 0.0 {
            (axis_x, len, "#ef6c00")
        } else {
            (axis_x - len, len, "#2e7d32")
        };
        let text_x = if *delta >= 0.0 {
            axis_x + 6.0
        } else {
            axis_x - 6.0
        };
        let anchor = if *delta >= 0.0 { "start" } else { "end" };
        bars.push_str(&format!(
            "<text x='{lx:.1}' y='{ty:.1}' font-size='11' fill='#555'>{lbl}</text>\
             <rect x='{x:.1}' y='{y:.1}' width='{w:.1}' height='13' fill='{c}' rx='2'/>\
             <text x='{tx:.1}' y='{ty:.1}' font-size='10' fill='#777' text-anchor='{a}'>\
             {d:+.1} → {n:.1}</text>",
            lx = 0.0,
            ty = cy + 4.0,
            lbl = esc(&short_label(label, id)),
            x = x,
            y = cy - 6.5,
            w = w.max(0.5),
            c = colour,
            tx = text_x,
            a = anchor,
            d = delta,
            n = now
        ));
    }

    format!(
        "<svg viewBox='-2 -2 {vw} {vh}' width='100%' height='{vh}' role='img' \
         aria-label='change per indicator since the previous run'>{defs}\
         <line x1='{ax:.1}' y1='0' x2='{ax:.1}' y2='{h:.1}' stroke='#bbb' stroke-width='1'/>\
         <text x='{ax:.1}' y='-4' font-size='9.5' fill='#999' text-anchor='middle'>no change</text>\
         <text x='{ax2:.1}' y='-4' font-size='9.5' fill='#bbb' text-anchor='end'>← better · worse →</text>\
         <text x='{lx:.1}' y='{h2:.1}' font-size='9.5' fill='#aaa'>axis scaled to the largest \
         change this run: ±{m:.1} points over {dd:.0} day(s)</text>\
         {bars}</svg>",
        vw = axis_x + half + 130.0,
        vh = h + 16.0,
        defs = defs(),
        ax = axis_x,
        ax2 = axis_x + half + 120.0,
        h = h,
        h2 = h + 12.0,
        lx = 0.0,
        m = max_abs,
        dd = days,
        bars = bars
    )
}

/// Indicator labels are long ("how much AI companies spend compared with what they
/// sell"); chart axes need something that fits. Falls back to the id.
fn short_label(label: &str, id: &str) -> String {
    let l = if label.is_empty() { id } else { label };
    if l.chars().count() <= 34 {
        return l.to_string();
    }
    let cut: String = l.chars().take(31).collect();
    format!("{}…", cut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn pt(date: &str, composite: f64, stresses: &[(&str, f64)]) -> TrendPoint {
        let mut m = BTreeMap::new();
        for (k, v) in stresses {
            m.insert((*k).to_string(), *v);
        }
        TrendPoint {
            date: date.into(),
            generated_at: format!("{}T00:00:00Z", date),
            composite,
            coverage: 1.0,
            phase: "early".into(),
            stresses: m,
        }
    }

    #[test]
    fn composite_chart_is_self_contained() {
        let pts = vec![pt("2026-09-10", 30.0, &[]), pt("2026-09-11", 34.0, &[])];
        let c = composite_chart(&pts);
        assert!(c.starts_with("<svg"));
        assert!(c.contains("polyline"));
        assert!(!c.contains("<script"));
        assert!(!c.contains("href"));
    }

    #[test]
    fn composite_chart_uses_a_fixed_scale_not_a_data_scaled_one() {
        // A two-point move from 32 to 34 must NOT fill the plot. On the fixed
        // 0-100 scale its y-span must be a small fraction of the height.
        let pts = vec![pt("2026-09-10", 32.0, &[]), pt("2026-09-11", 34.0, &[])];
        let c = composite_chart(&pts);
        let poly = c
            .split("points='")
            .nth(1)
            .unwrap()
            .split('\'')
            .next()
            .unwrap();
        let ys: Vec<f64> = poly
            .split_whitespace()
            .map(|p| p.split(',').nth(1).unwrap().parse::<f64>().unwrap())
            .collect();
        let span = (ys[0] - ys[1]).abs();
        assert!(
            span < H * 0.1,
            "a 2-point move must look small on a 0-100 scale, but spanned {} of {}",
            span,
            H
        );
    }

    #[test]
    fn composite_chart_shades_and_labels_every_phase_band() {
        let pts = vec![pt("2026-09-10", 30.0, &[]), pt("2026-09-11", 80.0, &[])];
        let c = composite_chart(&pts);
        for label in ["early", "mid", "late", "critical"] {
            assert!(c.contains(label), "band {} must be labelled", label);
        }
    }

    #[test]
    fn composite_chart_says_so_with_fewer_than_two_points() {
        let one = vec![pt("2026-09-10", 30.0, &[])];
        let c = composite_chart(&one);
        assert!(c.contains("Not enough recorded runs"));
        assert!(!c.contains("polyline"));
    }

    #[test]
    fn indicator_chart_breaks_the_line_at_a_missing_day() {
        // Measured on days 0 and 1, missing on day 2, measured again on day 3.
        // The line MUST NOT be drawn straight across day 2.
        let pts = vec![
            pt("2026-09-10", 30.0, &[("credit_hy", 20.0)]),
            pt("2026-09-11", 31.0, &[("credit_hy", 22.0)]),
            pt("2026-09-12", 32.0, &[]), // not measured
            pt("2026-09-13", 33.0, &[("credit_hy", 40.0)]),
        ];
        let c = indicator_chart(&pts, "credit_hy", "credit");
        let strokes = c.matches("<polyline").count();
        assert_eq!(
            strokes, 1,
            "only the two contiguous days may form a line; an isolated point gets a dot only"
        );
        // Three dots: days 10, 11 and 13 were measured. Day 12 is a gap and gets
        // no marker at all — plotting it as zero would be a fabricated value.
        assert_eq!(c.matches("<circle").count(), 3);
        assert!(
            c.contains("the line breaks there"),
            "the gap must be explained"
        );
        assert!(
            c.contains("1 day(s) with no reading"),
            "the count of missing days must be stated"
        );
    }

    #[test]
    fn indicator_chart_draws_dots_when_no_two_days_are_contiguous() {
        let pts = vec![
            pt("2026-09-10", 30.0, &[("credit_hy", 20.0)]),
            pt("2026-09-11", 31.0, &[]),
            pt("2026-09-12", 32.0, &[("credit_hy", 40.0)]),
        ];
        let c = indicator_chart(&pts, "credit_hy", "credit");
        assert_eq!(
            c.matches("<polyline").count(),
            0,
            "no contiguous pair exists"
        );
        assert_eq!(c.matches("<circle").count(), 2, "both points still visible");
    }

    #[test]
    fn indicator_chart_refuses_to_chart_a_single_reading() {
        let pts = vec![
            pt("2026-09-10", 30.0, &[("credit_hy", 20.0)]),
            pt("2026-09-11", 31.0, &[]),
        ];
        let c = indicator_chart(&pts, "credit_hy", "credit");
        assert!(c.contains("Not enough recorded days"));
        assert!(c.contains("left blank rather than drawn as zero"));
    }

    #[test]
    fn movement_chart_states_that_its_axis_is_data_scaled() {
        // Unlike the composite chart, this axis IS fitted to the data, so the
        // reader must be told rather than left to assume a fixed scale.
        let d = vec![(
            "valuation_stretch".to_string(),
            "Valuation".to_string(),
            5.0,
            40.0,
        )];
        let c = movement_chart(&d, 7.0);
        assert!(c.contains("axis scaled to the largest change"));
        assert!(c.contains("over 7 day(s)"));
    }

    #[test]
    fn movement_chart_explains_an_empty_comparison() {
        let c = movement_chart(&[], 7.0);
        assert!(c.contains("No indicator was measured on both runs"));
        assert!(c.contains("not treated as a change of zero"));
    }

    #[test]
    fn charts_never_emit_nan_or_infinity() {
        let pts = vec![
            pt("2026-09-10", 0.0, &[("credit_hy", 0.0)]),
            pt("2026-09-11", 100.0, &[("credit_hy", 100.0)]),
        ];
        for c in [
            composite_chart(&pts),
            indicator_chart(&pts, "credit_hy", "c"),
            movement_chart(&[("a".into(), "A".into(), 0.0, 0.0)], 0.0),
        ] {
            assert!(!c.contains("NaN"), "NaN in chart output");
            assert!(!c.contains("inf"), "infinity in chart output");
        }
    }
}
