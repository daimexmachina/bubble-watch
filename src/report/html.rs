//! Self-contained HTML dashboard.
//!
//! No CDN, no external fonts, no JavaScript dependencies, no network calls. The
//! file opens standalone in any browser and renders entirely from inline CSS and
//! a small amount of inline SVG. That constraint matters: a report that needs
//! the network to display is a report that breaks exactly when a market event
//! makes it most interesting.

use crate::model::{IndicatorReading, Reading, Report};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Phase pill background. Amber and orange are LIGHT colours, so they get dark
/// text (see `phase_text_color`) rather than white - white on amber measures
/// 1.97:1, far below the 4.5:1 legibility threshold, and reads as a smear.
fn phase_color(phase: &str) -> &'static str {
    match phase {
        "early" => "#1b5e20",
        "mid" => "#f9a825",
        "late" => "#ef6c00",
        "critical" => "#b3261e",
        _ => "#5f5f5f",
    }
}

/// Pill text colour, chosen per background so every combination clears 4.5:1.
fn phase_text_color(phase: &str) -> &'static str {
    match phase {
        // Dark backgrounds take white text.
        "early" | "critical" => "#ffffff",
        // Light backgrounds take near-black text.
        "mid" | "late" => "#3a2100",
        _ => "#ffffff",
    }
}

/// Horizontal gauge with the phase boundaries marked, so a reader can see where
/// the score sits relative to the documented bands rather than only the number.
fn gauge(score: f64) -> String {
    let w = 900.0;
    let x = (score.clamp(0.0, 100.0) / 100.0) * w;
    let mut marks = String::new();
    for (v, label) in [
        (35.0, "early|mid"),
        (55.0, "mid|late"),
        (75.0, "late|critical"),
    ] {
        let mx = v / 100.0 * w;
        marks.push_str(&format!(
            "<line x1='{mx:.1}' y1='0' x2='{mx:.1}' y2='46' stroke='#ffffff' stroke-width='2' stroke-dasharray='4 3'/>\
             <text class='ax' x='{mx:.1}' y='60' font-size='10' text-anchor='middle'>{lbl}</text>",
            mx = mx,
            lbl = label
        ));
    }
    format!(
        r##"<svg viewBox="-2 -2 904 74" width="100%" height="78" role="img" aria-label="composite score gauge">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#2e7d32"/><stop offset="35%" stop-color="#f9a825"/>
      <stop offset="55%" stop-color="#ef6c00"/><stop offset="75%" stop-color="#c62828"/>
      <stop offset="100%" stop-color="#4a0000"/>
    </linearGradient>
  </defs>
  <rect x="0" y="10" width="900" height="26" fill="url(#g)" rx="3"/>
  {marks}
  <polygon points="{px:.1},44 {lx:.1},4 {rx:.1},4" fill="#111111" stroke="#ffffff" stroke-width="1.5"/>
</svg>"##,
        marks = marks,
        px = x,
        lx = (x - 7.0).max(0.0),
        rx = (x + 7.0).min(900.0)
    )
}

fn reading_row(r: &IndicatorReading, total_weight: f64) -> String {
    let (status_cls, status_txt, value_txt, stress_txt, detail) = match &r.reading {
        Reading::Scored {
            stress,
            value,
            unit,
            detail,
            provenance,
        } => (
            "ok",
            format!("as of {}", esc(&provenance.as_of)),
            format!("{:.4} {}", value, esc(unit)),
            format!("{:.1}", stress),
            format!(
                "{} <span class='prov'>[{}, {}]</span>",
                esc(detail),
                esc(&provenance.source),
                esc(&provenance.endpoint)
            ),
        ),
        Reading::Unavailable { reason } => (
            "gap",
            "GAP".to_string(),
            "—".to_string(),
            "—".to_string(),
            esc(reason),
        ),
    };

    let contrib = match r.contribution {
        Some(c) => format!("{:.2}", c),
        None => "—".to_string(),
    };
    let pct = if total_weight > 0.0 {
        r.weight / total_weight * 100.0
    } else {
        0.0
    };

    format!(
        r#"<tr class="{cls}">
  <td class="id">{id}<div class="lbl">{label}</div></td>
  <td class="num">{wt:.0}%</td>
  <td class="num stress">{stress}</td>
  <td class="num">{contrib}</td>
  <td class="val">{value}</td>
  <td class="st">{st}</td>
  <td class="dt">{detail}</td>
</tr>"#,
        cls = status_cls,
        id = esc(&r.id),
        label = esc(&r.label),
        wt = pct,
        stress = stress_txt,
        contrib = contrib,
        value = value_txt,
        st = status_txt,
        detail = detail
    )
}

/// The declared-judgment panel.
///
/// Placed AFTER the falsifiers and before the indicator tables. Deliberately grouped
/// with the other out-of-composite material so the report has a clear shape: score,
/// then what would disprove it, then what is belief rather than measurement.
fn judgments_card(r: &Report) -> String {
    if r.judgments.is_empty() {
        return String::new();
    }
    let against = crate::subjective::against_count(&r.judgments);
    let jonly = crate::subjective::judgment_only_count(&r.judgments);
    let mut rows = String::new();
    for j in &r.judgments {
        let cls = match j.bears {
            crate::subjective::Bears::Supports => "j-sup",
            crate::subjective::Bears::Against => "j-aga",
            crate::subjective::Bears::Ambiguous => "j-amb",
        };
        rows.push_str(&format!(
            "<tr class='{cls}'><td><div class='j-claim'>{claim}</div>\
             <div class='j-meta'>{bears} &middot; {basis} &middot; {conf} confidence</div>\
             <div class='j-ev'><b>Evidence:</b> {ev}</div>\
             <div class='j-fal'><b>How this could be wrong:</b> {fal}</div></td></tr>",
            cls = cls,
            claim = esc(&j.claim),
            bears = esc(j.bears.as_str()),
            basis = esc(j.basis.as_str()),
            conf = esc(j.confidence.as_str()),
            ev = esc(&j.evidence),
            fal = esc(&j.must_not)
        ));
    }
    format!(
        "<div class='card judg'><h3 style='margin-top:0;font-size:15px'>Declared judgment and \
         unscored evidence</h3>\
         <p style='margin:0 0 10px;font-size:13.5px'>{n} entries: <b>{against}</b> bear against the \
         bubble thesis, <b>{jonly}</b> are interpretation with no measured number behind them. \
         Every entry states how it could be shown wrong &mdash; an entry that could not be \
         falsified would make any outcome confirm it, which is the reasoning this report exists \
         to refuse.</p>\
         <table class='jtab'><tbody>{rows}</tbody></table>\
         <p style='margin:12px 0 0;font-size:12px;color:#777'>This section is <b>not</b> part of \
         the score. Judgment is declared here rather than hidden inside the indicator anchors, \
         where it would be indistinguishable from arithmetic.</p></div>",
        n = r.judgments.len(),
        against = against,
        jonly = jonly,
        rows = rows
    )
}

/// The falsification panel: what would show the thesis is WRONG.
///
/// Placed directly after the score, deliberately high on the page. A reader who
/// sees only "35/100, mid" and scrolls away has been given the alarm without the
/// counter-evidence, which is the most misleading way to present this material.
fn falsifiers_card(r: &Report) -> String {
    if r.falsifiers.is_empty() {
        return String::new();
    }
    let against = crate::falsifiers::counter_evidence_count(&r.falsifiers);
    let total = r.falsifiers.len();

    let mut rows = String::new();
    for f in &r.falsifiers {
        let (cls, label) = match f.verdict {
            crate::falsifiers::Verdict::CounterEvidence => ("counter", "AGAINST"),
            crate::falsifiers::Verdict::ConsistentWithBubble => ("forb", "for"),
            crate::falsifiers::Verdict::Uninformative => ("uninf", "n/a"),
        };
        rows.push_str(&format!(
            "<tr class='f-{cls}'><td class='f-mark'>{label}</td><td><div class='f-q'>{q}</div>\
             <div class='f-r'>{reading}</div><div class='f-d'>{detail}</div></td></tr>",
            cls = cls,
            label = label,
            q = esc(&f.question),
            reading = esc(&f.reading),
            detail = esc(&f.detail)
        ));
    }

    format!(
        "<div class='card fals'><h3 style='margin-top:0;font-size:15px'>What would show this is wrong</h3>\
         <p style='margin:0 0 10px;font-size:13.5px'><b>{against} of {total}</b> of the tool's own \
         falsification tests currently read as <b>counter-evidence</b> to the bubble thesis. These are \
         measurements chosen to DISPROVE it, and the answers are reported whichever way they fall.</p>\
         <table class='ftab'><tbody>{rows}</tbody></table>\
         <p style='margin:12px 0 0;font-size:12px;color:#777'>These are <b>not</b> part of the score. \
         Averaging evidence for and against into one number would merge opposite meanings, so they are \
         reported beside it. &ldquo;n/a&rdquo; means the data cannot support a direction &mdash; stated rather than forced.</p></div>",
        against = against,
        total = total,
        rows = rows
    )
}

/// The GSADF explosiveness panel.
///
/// Printed next to the composite and never reconciled with it: when the two
/// disagree, that disagreement is the finding.
fn explosiveness_card(r: &Report) -> String {
    let Some(e) = &r.explosiveness else {
        return String::new();
    };
    let colour = if e.is_significant() {
        "#c62828"
    } else {
        "#2e7d32"
    };
    format!(
        "<div class='card expl'><h3 style='margin-top:0;font-size:15px'>Explosiveness test (GSADF)</h3>\
         <div class='expl-stat' style='color:{col}'>{stat:.2}</div>\
         <div style='font-size:13.5px;margin-bottom:6px'><b>{sig}</b> &mdash; simulated 5% critical value {p95:.2}</div>\
         <div style='font-size:12.5px;color:#555'>Window tested: <b>{ws}</b> to <b>{we}</b> over {n} monthly observations.</div>\
         <p style='margin:12px 0 0;font-size:12.5px;color:#777'>A formal hypothesis test on the price \
         series &mdash; a different KIND of evidence from every indicator in the score above, which is a \
         hand-anchored judgement. <b>It is deliberately not part of the composite.</b> If it disagrees with \
         the score, that disagreement is the point: the two measure different things and neither is a \
         forecast. Critical values are simulated here rather than taken from the published table, so treat \
         them as indicative.</p></div>",
        col = colour,
        stat = e.statistic,
        sig = esc(&e.significance),
        p95 = e.critical.p95,
        ws = esc(&e.window_start_date),
        we = esc(&e.window_end_date),
        n = e.observations
    )
}

/// The per-company exposure table.
///
/// Three distinct absent-value states are rendered differently, because
/// conflating them would be false: "not disclosed at all" (MSFT has no purchase
/// obligation concept), "disclosed but stale" (AMZN's latest figure is 810 days
/// old), and "not applicable". None is ever shown as 0.00.
fn exposure_card(r: &Report) -> String {
    if r.exposure.is_empty() {
        return String::new();
    }
    let mut rows = String::new();
    for (i, e) in r.exposure.iter().enumerate() {
        let cell = |v: Option<f64>| match v {
            Some(x) => format!("{:.2}", x),
            None => "<span class='nd'>not disclosed</span>".to_string(),
        };
        rows.push_str(&format!(
            "<tr><td class='num'>{rank}</td><td class='id'>{t}</td>\
             <td class='num'>{debt}</td><td class='num'>{near}</td><td class='num'>{lease}</td>\
             <td class='num'>{comm}</td><td class='num'>{rpo}</td></tr>",
            rank = i + 1,
            t = esc(&e.ticker),
            debt = cell(e.debt_to_cfo),
            near = cell(e.near_term_to_cfo),
            lease = cell(e.lease_to_cfo),
            comm = cell(e.commitments_to_cfo),
            rpo = cell(e.rpo_to_revenue),
        ));
        // Notes go in their OWN row directly beneath the company, so each caveat
        // stays attached to the figures it qualifies. Emitting them as a div
        // inside tbody is invalid nesting and browsers hoisted them above the
        // whole table, separating every caveat from its row.
        if !e.notes.is_empty() {
            let items: String = e
                .notes
                .iter()
                .map(|n| format!("<li>{}</li>", esc(n)))
                .collect();
            rows.push_str(&format!(
                "<tr class='exp-nr'><td></td><td colspan='6'><div class='exp-note'>\
                 <span class='exp-nh'>{} caveats:</span><ul>{}</ul></div></td></tr>",
                esc(&e.ticker),
                items
            ));
        }
    }
    format!(
        "<div class='card exp'><h3 style='margin-top:0;font-size:15px'>Who is most exposed</h3>\
         <p style='margin:0 0 10px;font-size:13px'>Ratios to operating cash flow, most exposed first. \
         This answers a different question from the score above &mdash; who carries the risk, rather than how \
         bubble-like the configuration is &mdash; so it is <b>never folded into the composite</b>.</p>\
         <table><thead><tr><th>#</th><th>Company</th><th>debt/CFO</th><th>due&nbsp;&lt;1y/CFO</th>\
         <th>leases/CFO</th><th>commitments/CFO</th><th>RPO/revenue</th></tr></thead><tbody>{rows}</tbody></table>\
         <p style='margin:12px 0 0;font-size:12px;color:#777'>A missing figure is <b>named</b>, never shown as \
         0.00: an absent disclosure is not a small number. Near-term debt is the &ldquo;who is tested first&rdquo; \
         column, and for this cohort it is small for every company, so a maturity wall is not the \
         mechanism in this cycle &mdash; the unconditional commitments are.</p></div>",
        rows = rows
    )
}

/// One block of the plain-English summary.
fn layman_block(title: &str, body: &str) -> String {
    format!(
        "<div class='lm-block'><div class='lm-h'>{}</div><div class='lm-b'>{}</div></div>",
        esc(title),
        esc(body)
    )
}

fn dir_color(d: &str) -> &'static str {
    match d {
        "rising" => "#ef6c00",
        "falling" => "#2e7d32",
        _ => "#616161",
    }
}

/// The composite trend chart. Delegates to `report::charts`, which draws it on a
/// FIXED 0-100 scale with the phase bands shaded — auto-scaling to the data range
/// would make a two-point move look dramatic, which is the classic way a chart
/// misleads.
fn sparkline(points: &[crate::model::TrendPoint], _limit: usize) -> String {
    crate::report::charts::composite_chart(points)
}

#[allow(dead_code)]
fn sparkline_legacy(points: &[crate::model::TrendPoint], limit: usize) -> String {
    if points.len() < 2 {
        return "<div class='spark-empty'>Not enough recorded runs yet to draw a trend. A line \
                needs at least two dated runs; re-running the tool on a later day adds one.</div>"
            .to_string();
    }
    let pts: Vec<&crate::model::TrendPoint> = points.iter().rev().take(limit).rev().collect();
    let w = 900.0f64;
    let h = 120.0f64;
    // Scale the y-axis to the data with padding, but never to a zero-height
    // range: if every run has the same score the line is drawn mid-height.
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for p in &pts {
        lo = lo.min(p.composite);
        hi = hi.max(p.composite);
    }
    let (lo, hi) = if (hi - lo).abs() < 0.5 {
        ((hi - 5.0).max(0.0), (hi + 5.0).min(100.0))
    } else {
        ((lo - 2.0).max(0.0), (hi + 2.0).min(100.0))
    };
    let span = (hi - lo).max(1e-6);
    let n = pts.len();
    let x_at = |i: usize| -> f64 {
        if n == 1 {
            w / 2.0
        } else {
            i as f64 / (n - 1) as f64 * w
        }
    };
    let y_at = |v: f64| -> f64 { h - ((v - lo) / span) * h };

    let mut poly = String::new();
    for (i, p) in pts.iter().enumerate() {
        poly.push_str(&format!("{:.1},{:.1} ", x_at(i), y_at(p.composite)));
    }

    // Phase guide lines, so the shape can be read against the documented bands.
    let mut guides = String::new();
    for v in [35.0f64, 55.0, 75.0] {
        if v > lo && v < hi {
            let y = y_at(v);
            guides.push_str(&format!(
                "<line x1='0' y1='{y:.1}' x2='{w:.1}' y2='{y:.1}' stroke='#ccc' stroke-width='1' \
                 stroke-dasharray='3 4'/><text class='ax' x='2' y='{ty:.1}' font-size='9'>{v:.0}</text>",
                y = y,
                w = w,
                ty = y - 2.0,
                v = v
            ));
        }
    }

    // Mark the newest point.
    let last = pts.last().unwrap();
    let lx = x_at(n - 1);
    let ly = y_at(last.composite);

    format!(
        r##"<svg viewBox="-2 -2 904 {vh}" width="100%" height="{vh}" role="img" aria-label="composite score over recorded runs">
  {guides}
  <polyline fill="none" stroke="#1a73e8" stroke-width="2.5" points="{poly}"/>
  <circle cx="{lx:.1}" cy="{ly:.1}" r="4.5" fill="#1a73e8"/>
  <text x="{lx:.1}" y="{ty:.1}" font-size="11" fill="#1a73e8" text-anchor="end">{val:.1}</text>
</svg>"##,
        vh = h + 4.0,
        guides = guides,
        poly = poly.trim(),
        lx = lx,
        ly = ly,
        ty = (ly - 8.0).max(10.0),
        val = last.composite
    )
}

/// A multi-run dashboard: the series over time plus every recorded run.
///
/// This is the landing page of the served site. It is deliberately separate from
/// the per-run report: the report is a snapshot of one day, the dashboard is the
/// history of snapshots.
///
/// `latest` is the newest full report, used only for its plain-English framing.
/// `None` is legitimate (no report generated yet) and the page says so rather
/// than rendering a hollow shell.
///
/// `have_page` reports whether a full report page exists for a given date. A
/// date without one is rendered as plain text, never as a link — a link to a
/// file that does not exist is exactly the kind of false claim this project
/// exists to avoid.
pub fn render_dashboard(
    points: &[crate::model::TrendPoint],
    latest: Option<&Report>,
    sparkline_points: usize,
    have_page: &dyn Fn(&str) -> bool,
) -> String {
    let series = sparkline(points, sparkline_points);

    let mut rows = String::new();
    for p in points.iter().rev() {
        // Per-indicator movement against the immediately preceding run, shown so
        // the table answers "what changed" without opening each report.
        let prev = points
            .iter()
            .filter(|q| q.date < p.date)
            .max_by(|a, b| a.date.cmp(&b.date));
        let mut chips = String::new();
        if let Some(q) = prev {
            let mut moved: Vec<(&String, f64)> = p
                .stresses
                .iter()
                .filter_map(|(k, v)| q.stresses.get(k).map(|old| (k, v - old)))
                .collect();
            moved.sort_by(|a, b| {
                b.1.abs()
                    .partial_cmp(&a.1.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for (k, d) in moved.into_iter().take(3) {
                if d.abs() < 0.5 {
                    continue;
                }
                let col = if d > 0.0 { "#ef6c00" } else { "#2e7d32" };
                chips.push_str(&format!(
                    "<span class='chip' style='color:{col}'>{id} {d:+.1}</span>",
                    col = col,
                    id = esc(k),
                    d = d
                ));
            }
        }
        rows.push_str(&format!(
            "<tr><td>{datelink}</td><td class='num'>{comp:.1}</td>\
             <td class='num'>{cov:.0}%</td><td><span class='phase' style='background:{col};color:{tcol}'>{ph}</span></td>\
             <td class='chips'>{chips}</td></tr>",
            datelink = if have_page(&p.date) {
                format!(
                    "<a href='{}.html'>{}</a>",
                    esc(&p.date),
                    esc(&p.date)
                )
            } else {
                // No page for this date: show the date as plain text and say
                // why, rather than emitting a link that would 404.
                format!(
                    "{} <span class='dt' title='no archived report page for this date'>(summary only)</span>",
                    esc(&p.date)
                )
            },
            comp = p.composite,
            cov = p.coverage * 100.0,
            col = phase_color(&p.phase),
            tcol = phase_text_color(&p.phase),
            ph = esc(&p.phase),
            chips = chips
        ));
    }

    let body_rows = if rows.is_empty() {
        "<tr><td colspan='5' class='dt'>No runs recorded yet. Run the tool once and this page will \
         fill in.</td></tr>"
            .to_string()
    } else {
        rows
    };

    // Per-indicator charts: one per measurement that appears anywhere in the
    // archive, using the newest run's label for the title.
    let mut ids: Vec<String> = Vec::new();
    for p in points.iter().rev() {
        for k in p.stresses.keys() {
            if !ids.contains(k) {
                ids.push(k.clone());
            }
        }
    }
    ids.sort();
    let label_for = |id: &str| -> String {
        latest
            .and_then(|r| r.indicators.iter().find(|i| i.id == id))
            .map(|i| i.label.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let mut ind_charts = String::new();
    for id in &ids {
        ind_charts.push_str(&format!(
            "<div class='ind-chart'><div class='ind-h'>{label}</div><div class='ind-id'>{id}</div>             {chart}</div>",
            label = esc(&label_for(id)),
            id = esc(id),
            chart = crate::report::charts::indicator_chart(points, id, &label_for(id))
        ));
    }
    if ind_charts.is_empty() {
        ind_charts =
            "<p class='chart-empty'>No measurements have been recorded yet.</p>".to_string();
    }

    let headline = latest
        .map(|r| {
            format!(
                "<div class='card'><h3 style='margin-top:0;font-size:15px'>Latest reading ({date})</h3>\
                 <div class='score'>{score:.1}<small>/100 stress</small></div>\
                 <div style='margin:8px 0'><span class='phase' style='background:{col};color:{tcol}'>{ph}</span></div>\
                 <p class='lm-b' style='margin:10px 0 0'>{plain}</p></div>",
                date = esc(&r.generated_at),
                score = r.composite,
                col = phase_color(&r.phase),
                tcol = phase_text_color(&r.phase),
                ph = esc(&r.phase),
                plain = esc(&r.layman.direction_of_travel)
            )
        })
        .unwrap_or_else(|| {
            "<div class='card'><h3 style='margin-top:0;font-size:15px'>Latest reading</h3>\
             <p class='dt'>No report has been generated yet. Run <code>bubble-watch report</code> \
             and this page will show the current reading.</p></div>"
                .to_string()
        });

    format!(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>bubble-watch — history</title>
<style>
:root {{ color-scheme: light dark; }}
* {{ box-sizing: border-box; }}
body {{ margin:0; padding:28px; font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
  background:#fafafa; color:#1a1a1a; max-width:1100px; }}
@media (prefers-color-scheme: dark) {{ body {{ background:#121212; color:#e8e8e8; }}
  table {{ border-color:#333 !important; }} th {{ background:#1e1e1e !important; }}
  td, th {{ border-color:#2a2a2a !important; }} .card {{ background:#1a1a1a !important; border-color:#2e2e2e !important; }}
  .spark-empty {{ background:#1e1e1e !important; color:#aaa !important; }}
  /* Chart text and band fills. Scoped to the band class: an unscoped
     `svg rect` rule here also dimmed the movement-chart bars to near-invisible. */
  svg .band {{ opacity:.22; }}
  svg .ax {{ fill:#bbbbbb; }}
  svg .barlab {{ fill:#dddddd; }}
  svg .gapnote {{ fill:#d7a98c; }}
  svg .latest {{ fill:#c9a7e0; }} }}
h1 {{ font-size:20px; margin:0 0 2px; }}
.sub {{ color:#777; font-size:12px; margin-bottom:20px; }}
.card {{ background:#fff; border:1px solid #e2e2e2; border-radius:8px; padding:18px; margin-bottom:18px; }}
.score {{ font-size:44px; font-weight:700; line-height:1; letter-spacing:-1px; }}
.score small {{ font-size:15px; font-weight:400; color:#888; }}
.phase {{ display:inline-block; padding:2px 9px; border-radius:999px; color:#fff; font-weight:600; font-size:11px; text-transform:uppercase; letter-spacing:.5px; }}
table {{ border-collapse:collapse; width:100%; font-size:13px; }}
th {{ text-align:left; background:#f2f2f2; padding:8px; border-bottom:1px solid #ddd; font-size:11px; text-transform:uppercase; letter-spacing:.4px; color:#666; }}
td {{ padding:8px; border-bottom:1px solid #eee; vertical-align:middle; }}
td.num {{ text-align:right; font-variant-numeric:tabular-nums; white-space:nowrap; }}
td.dt {{ color:#555; font-size:12px; overflow-wrap:anywhere; }}
/* Provenance endpoints are long unbroken URLs; without this the table is forced
   wider than the page and clips instead of wrapping. */
.prov {{ overflow-wrap:anywhere; }}
td.chips {{ font-size:11.5px; }}
.chip {{ display:inline-block; margin-right:7px; white-space:nowrap; font-variant-numeric:tabular-nums; }}
a {{ color:#1a73e8; text-decoration:none; font-weight:600; }}
a:hover {{ text-decoration:underline; }}
.lm-b {{ font-size:13.5px; line-height:1.55; }}
.spark-empty {{ font-size:12.5px; color:#777; background:#f7f7f7; border-radius:4px; padding:9px 11px; margin:0; }}
code {{ background:#f1f1f1; padding:1px 5px; border-radius:3px; font-size:12.5px; }}
.disc {{ color:#777; font-size:12px; border-top:1px solid #e2e2e2; padding-top:12px; }}
/* Falsification panel. Colour-coded by verdict so the AGAINST rows are
   impossible to miss, since they are the ones a reader most needs to see. */
.card.fals {{ border-left:4px solid #2e7d32; }}
table.ftab {{ margin:0; }}
table.ftab td {{ border-bottom:1px solid #eee; padding:9px 8px; vertical-align:top; }}
td.f-mark {{ font-weight:700; font-size:10.5px; letter-spacing:.5px; white-space:nowrap; width:74px; }}
tr.f-counter td.f-mark {{ color:#2e7d32; }}
tr.f-forb td.f-mark {{ color:#ef6c00; }}
tr.f-uninf td.f-mark {{ color:#888888; }}
tr.f-counter {{ background:rgba(46,125,50,.05); }}
.f-q {{ font-weight:600; font-size:13px; }}
.f-r {{ font-size:12.5px; color:#333; margin-top:2px; }}
.f-d {{ font-size:11.5px; color:#777; margin-top:4px; line-height:1.5; }}
.card.expl {{ border-left:4px solid #1a73e8; }}
details.tech {{ margin-top:10px; font-size:12px; }}
details.tech summary {{ cursor:pointer; color:#777; }}
details.tech .f-d {{ margin-top:6px; padding:8px 10px; background:#f7f7f7; border-radius:4px; }}
@media (prefers-color-scheme: dark) {{ details.tech .f-d {{ background:#1e1e1e; }} }}
.expl-stat {{ font-size:36px; font-weight:700; line-height:1; letter-spacing:-1px; margin-bottom:4px; }}
.card.exp {{ border-left:4px solid #6a1b9a; }}
.card.judg {{ border-left:4px solid #5f6368; }}
table.jtab td {{ border-bottom:1px solid #eee; padding:10px 8px; vertical-align:top; }}
.j-claim {{ font-weight:600; font-size:13px; }}
.j-meta {{ font-size:10.5px; text-transform:uppercase; letter-spacing:.4px; color:#888; margin:2px 0 5px; }}
.j-ev, .j-fal {{ font-size:12px; color:#444; margin-top:4px; line-height:1.5; }}
.j-fal {{ color:#7a3e12; }}
tr.j-sup {{ background:rgba(239,108,0,.05); }}
tr.j-aga {{ background:rgba(46,125,50,.06); }}
@media (prefers-color-scheme: dark) {{
  table.jtab td {{ border-color:#2a2a2a; }}
  .j-ev {{ color:#ccc; }} .j-fal {{ color:#e0a878; }}
}}
.nd {{ color:#b3261e; font-size:11px; font-style:italic; }}
.exp-note {{ font-size:11.5px; color:#777; }}
.exp-nr td {{ padding-top:0; border-bottom:1px solid #eee; }}
.exp-nh {{ font-weight:600; }}
tr.exp-nr {{ background:rgba(179,38,30,.03); }}
.exp-note ul {{ margin:2px 0 8px; padding-left:18px; }}
.exp-note li {{ margin-bottom:2px; }}
@media (prefers-color-scheme: dark) {{
  table.ftab td {{ border-color:#2a2a2a; }}
  .f-r {{ color:#ddd; }}
  tr.f-counter {{ background:rgba(46,125,50,.12); }}
}}
svg .ax {{ fill:#5f5f5f; }}
svg .band {{ opacity:1; }}
svg .barlab {{ fill:#3c3c3c; }}
svg .gapnote {{ fill:#7a5c4a; }}
svg .latest {{ fill:#6a1b9a; }}
.chart-empty {{ font-size:12.5px; color:#777; background:#f7f7f7; border-radius:4px; padding:9px 11px; margin:0; }}
.ind-chart {{ border-top:1px solid #eee; padding-top:10px; margin-top:12px; }}
.ind-chart:first-child {{ border-top:0; margin-top:0; padding-top:0; }}
.ind-h {{ font-size:12.5px; font-weight:600; color:#333; }}
.ind-id {{ font-size:10.5px; color:#6b6b6b; font-family:ui-monospace,SFMono-Regular,Menlo,monospace; margin-bottom:2px; }}
@media (prefers-color-scheme: dark) {{
  .ind-chart {{ border-color:#2a2a2a; }}
  .ind-h {{ color:#ddd; }}
  .chart-empty {{ background:#1e1e1e !important; color:#aaa !important; }}
}}
</style></head><body>

<h1>AI Bubble Watch — history</h1>
<div class="sub">{n} recorded run(s) · newest {newest} · bubble-watch {ver}</div>

{headline}

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Score over recorded runs</h3>
  {series}
  <p style="margin:10px 0 0;font-size:12px;color:#777">The shaded bands are the documented phases
  (early below 35, mid 35–55, late 55–75, critical above 75) and the scale is fixed at 0–100, so a
  small move looks small rather than being stretched to fill the chart. These are comparisons of
  measured state, not a forecast.</p>
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Each measurement over time</h3>
  <p class="dt" style="margin:0 0 6px">Every scored measurement, on a fixed 0-100 scale. A line
  breaks wherever a measurement is missing — it is never drawn across a gap or plotted as zero.
  A gap in the credit lines means FRED did not answer, not that credit was calm.</p>
  {ind_charts}
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">All recorded runs</h3>
  <p class="dt" style="margin:0 0 10px">One row per day. The right-hand column shows the largest
  changes in the individual measurements since the previous run.</p>
  <table>
    <thead><tr><th>Date</th><th>Score</th><th>Coverage</th><th>Phase</th><th>Biggest changes</th></tr></thead>
    <tbody>{rows}</tbody>
  </table>
</div>

<p class="disc"><b>This is a state descriptor, not a forecast and not investment advice.</b>
It reports where a set of published measurements sit relative to documented historical reference
points, and it says so plainly when it cannot measure something. Unavailable measurements are
never imputed or defaulted to zero — they are reported as gaps. State extremity carries no
timing information.</p>
</body></html>"##,
        n = points.len(),
        newest = esc(points.last().map(|p| p.date.as_str()).unwrap_or("—")),
        ver = esc(super::VERSION),
        headline = headline,
        series = series,
        ind_charts = ind_charts,
        rows = body_rows
    )
}

/// A minimal index of the served site, written alongside the dashboard so the
/// directory can also be browsed directly as static files.
pub fn render_index() -> String {
    "<!DOCTYPE html><html lang='en'><head><meta charset='utf-8'>\
     <meta name='viewport' content='width=device-width,initial-scale=1'>\
     <title>bubble-watch</title><style>body{font:14px/1.6 -apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,sans-serif;\
     max-width:640px;margin:60px auto;padding:0 24px;color:#1a1a1a}a{color:#1a73e8}</style></head><body>\
     <h1>AI Bubble Watch</h1>\
     <ul><li><a href='dashboard.html'>History and trends</a> — the score over time, one row per day</li>\
     <li><a href='latest.html'>Latest full report</a> — the complete current reading with provenance</li></ul>\
     <p style='color:#777;font-size:12.5px'>A state descriptor, not a forecast and not investment advice.</p>\
     </body></html>"
        .to_string()
}

/// The direction-of-travel card. Three honest states: a delta, a refusal with a
/// reason, or no history yet — never a blank panel.
fn trend_card(r: &Report) -> String {
    let series = sparkline(&r.trend.points, 30);

    let mut rows = String::new();
    for p in r.trend.points.iter().rev().take(14).rev() {
        rows.push_str(&format!(
            "<tr><td class='num'>{date}</td><td class='num'>{comp:.1}</td><td class='num'>{cov:.0}%</td><td>{ph}</td></tr>",
            date = esc(&p.date),
            comp = p.composite,
            cov = p.coverage * 100.0,
            ph = esc(&p.phase)
        ));
    }
    let table = if rows.is_empty() {
        "<p class='spark-empty'>No runs recorded yet.</p>".to_string()
    } else {
        format!(
            "<table style='margin-top:12px'><thead><tr><th>Date</th><th>Score</th><th>Coverage</th><th>Phase</th></tr></thead><tbody>{}</tbody></table>",
            rows
        )
    };

    let body = match &r.trend.delta {
        Some(d) => {
            let mut ind_rows = String::new();
            for i in &d.indicators {
                ind_rows.push_str(&format!(
                    "<tr><td class='id'>{id}</td><td class='num'>{then:.1}</td><td class='num'>{now:.1}</td>\
                     <td class='num stress' style='color:{col}'>{delta:+.1}</td><td style='color:{col}'>{dir}</td></tr>",
                    id = esc(&i.id),
                    then = i.baseline_stress,
                    now = i.current_stress,
                    delta = i.delta,
                    col = dir_color(i.direction.as_str()),
                    dir = esc(i.direction.as_str())
                ));
            }
            let ind_table = if ind_rows.is_empty() {
                "<p class='spark-empty'>No indicator was scored on both runs, so no per-measurement \
                 change can be shown. A missing measurement is never treated as a change of zero.</p>"
                    .to_string()
            } else {
                format!(
                    "<table style='margin-top:12px'><thead><tr><th>Indicator</th><th>Then</th><th>Now</th><th>Change</th><th>Direction</th></tr></thead><tbody>{}</tbody></table>",
                    ind_rows
                )
            };
            let phase_note = if d.phase_changed {
                format!(
                    "<div class='note' style='margin-top:10px'>The phase label changed: <b>{}</b> → <b>{}</b>.</div>",
                    esc(&d.phase_then),
                    esc(&d.phase_now)
                )
            } else {
                String::new()
            };
            // Movement chart: bars from the previous reading to the current one.
            let chart_deltas: Vec<(String, String, f64, f64)> = d
                .indicators
                .iter()
                .map(|i| (i.id.clone(), i.label.clone(), i.delta, i.current_stress))
                .collect();
            let movement = crate::report::charts::movement_chart(&chart_deltas, d.elapsed_days);
            format!(
                "<p style='margin:0 0 4px'>Against the baseline of <b>{date}</b>, <b>{days:.0} day(s)</b> ago: \
                 the score moved <b style='color:{col}'>{delta:+.1}</b> and is now \
                 <b>{dir}</b>. Baseline weighted coverage was {bcov:.0}% against {ccov:.0}% today, within the \
                 comparability tolerance — without that guarantee no comparison would be shown at all.</p>\
                 {phase_note}<div style='margin-top:12px'>{movement}</div>{ind_table}",
                date = esc(&d.baseline_date),
                days = d.elapsed_days,
                col = dir_color(d.direction.as_str()),
                delta = d.composite_delta,
                dir = esc(d.direction.as_str()),
                bcov = d.baseline_coverage * 100.0,
                ccov = r.coverage * 100.0,
                phase_note = phase_note,
                movement = movement,
                ind_table = ind_table
            )
        }
        None => {
            // The plain-English narrative above already explains the refusal in
            // full. Repeating the raw technical reason here produced a wall of
            // near-identical text, so the technical detail is collapsed into a
            // footnote instead of a second paragraph.
            let raw = r.trend.reason.as_deref().unwrap_or("No reason recorded.");
            format!(
                "<details class='tech'><summary>Technical reason ({} ineligible baseline(s) rejected)</summary>\
                 <div class='f-d'>{}</div></details>",
                raw.matches("; ").count() + 1,
                esc(raw)
            )
        }
    };

    let warn = if r.trend.warnings.is_empty() {
        String::new()
    } else {
        let items: String = r
            .trend
            .warnings
            .iter()
            .map(|w| format!("<li>{}</li>", esc(w)))
            .collect();
        format!("<ul style='margin-top:10px'>{}</ul>", items)
    };

    // THE ATTRIBUTION GOES IMMEDIATELY BELOW THE HISTORY TABLE, because that table is the thing
    // a reader will misread. A climbing composite with nothing beside it reads as deteriorating
    // conditions; measured, roughly 99% of that climb in this archive is the MODEL being edited.
    // Placing the caveat anywhere else would let the table be read alone.
    let attribution = match &r.drift {
        Some(d) if !d.epochs.is_empty() => {
            let share = match d.model_share() {
                Some(s) => format!("{:.0}%", s * 100.0),
                None => "n/a".into(),
            };
            let market = if d.market_measurable {
                format!(
                    "Market movement within a single methodology averages <b>{:.2}</b> points.",
                    d.mean_market_range
                )
            } else {
                "No market movement is measurable yet — every methodology so far spans only one \
                 date, so ALL recorded change is model change."
                    .to_string()
            };
            // THE PHASE AND TIMING CROSSING GOES IN THE SAME BLOCK, because it is the consequence
            // a reader acts on. The phase label and the quoted time range are keyed to absolute
            // composite thresholds, so a model edit can move BOTH without the market moving — the
            // archive shows 'early' -> 'mid' at a methodology boundary, which halves the quoted
            // window. Saying only "the composite is model-driven" understates what that did.
            let phase_block = if d.phase_changes_at_model_boundary > 0 {
                format!(
                    "<br><b style='color:#8a5a00'>The PHASE LABEL and the TIMING OVERLAY moved with \
                     it.</b> The phase crossed {} time(s) at a methodology boundary ('{}' to '{}'), \
                     so the same market was reported as '{}' with one time range and '{}' with \
                     another — 'early' quotes 30-72 months, 'mid' quotes 15-42. <b>The window \
                     halved without the market moving.</b> The timing output is already stated to \
                     be not-a-probability and fitted on n=2; this is a different caveat: a model \
                     edit can move it, so two ranges from different methodology versions are not \
                     comparable either.",
                    d.phase_changes_at_model_boundary, d.phase_first, d.phase_last,
                    d.phase_first, d.phase_last
                )
            } else {
                String::new()
            };
            // THE BAND MARGIN goes in the same block. It answers "is the label stable?" directly,
            // against this project's OWN measured drift rather than a guess, which is the honest
            // alternative to retuning thresholds.
            let margin_block = match &r.band_margin {
                Some(m) => format!(
                    "<br><span style='color:#666'>{}</span>",
                    esc(&m.statement())
                ),
                None => String::new(),
            };
            format!(
                "<div style='margin-top:14px;padding:10px 12px;border-left:3px solid #b8860b;\
                 background:#fffbf0;font-size:12.5px;line-height:1.5'>\
                 <b>Most of this history is the MODEL changing, not the market.</b><br>\
                 Across {} methodology version(s), model changes account for <b>{:+.1}</b> points \
                 of movement. {} Model-caused movement is roughly <b>{}</b> of the total.<br>\
                 <span style='color:#666'>A composite from one methodology version and a composite \
                 from another are two DIFFERENT INSTRUMENTS, not one instrument at two times. The \
                 tool refuses that comparison everywhere else; this table is the one place it was \
                 still implied.</span>{}{}</div>",
                d.epochs.len(),
                d.model_change,
                market,
                share,
                phase_block,
                margin_block
            )
            // (band margin is rendered below, outside this match, because it also applies when the
            //  attribution is unavailable — a margin can be measured from the live composite alone)
        }
        _ => String::new(),
    };

    format!(
        "<div class='card'><h3 style='margin-top:0;font-size:15px'>Direction of travel</h3>\
         <p class='lm-b' style='margin:0 0 10px'>{plain}</p>{body}<div style='margin-top:14px'>{spark}</div>{table}{attr}{warn}\
         <p style='margin:10px 0 0;font-size:12px;color:#777'>The trend is context only. It never enters the score \
         above, because the score is what the trend is measured from.</p></div>",
        plain = esc(&r.layman.direction_of_travel),
        body = body,
        spark = series,
        table = table,
        attr = attribution,
        warn = warn
    )
}

pub fn render(r: &Report) -> String {
    let mut rows = String::new();
    for i in &r.indicators {
        rows.push_str(&reading_row(i, r.data_quality.total_weight));
    }

    let mut gaps = String::new();
    for g in &r.data_quality.unavailable {
        gaps.push_str(&format!(
            "<li><b>{}</b> ({}% of intended weight) — {}</li>",
            esc(&g.label),
            format!("{:.0}", g.weight),
            esc(&g.reason)
        ));
    }
    if gaps.is_empty() {
        gaps.push_str("<li>None — all configured indicators produced a reading.</li>");
    }

    let mut src_rows = String::new();
    for s in &r.sources {
        let cls = match s.status.as_str() {
            "ok" => "ok",
            "degraded" => "warn",
            _ => "gap",
        };
        src_rows.push_str(&format!(
            "<tr class='{cls}'><td>{}</td><td>{}</td><td>{}</td><td class='dt'>{}</td></tr>",
            esc(&s.name),
            esc(&s.status),
            format!("{} ok / {} failed", s.ok_count, s.failed_count),
            esc(&s.detail)
        ));
    }

    let mut caveats = String::new();
    for c in &r.caveats {
        caveats.push_str(&format!("<li>{}</li>", esc(c)));
    }

    // Plain-English summary blocks, in reading order.
    let lm_blocks = [
        layman_block("What is stretched", &r.layman.what_is_stretched),
        layman_block("What is calm", &r.layman.what_is_calm),
        layman_block("Getting better or worse", &r.layman.direction_of_travel),
        layman_block("What this cannot see at all", &r.layman.known_blind_spots),
        layman_block(
            "What this could not check today",
            &r.layman.what_we_cannot_measure,
        ),
        layman_block("About timing", &r.layman.about_timing),
    ]
    .join("");

    let analog = match (r.analog.range_months, &r.analog.band) {
        (Some(rg), Some(b)) => format!(
            "Band <b>{}</b> — historically <b>{:.0}–{:.0} months</b> to a peak in the reference analogs. \
             <b>This is not a probability and not a prediction:</b> it is a comparison against {} prior \
             episodes, and state extremity is not timing.",
            esc(b),
            rg[0],
            rg[1],
            r.analog.analogs.len()
        ),
        _ => format!(
            "Not shown. {}",
            esc(r.analog.band_note.as_deref().unwrap_or("insufficient data"))
        ),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>bubble-watch — {date}</title>
<style>
:root {{ color-scheme: light dark; }}
* {{ box-sizing: border-box; }}
body {{ margin:0; padding:28px; font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
  background:#fafafa; color:#1a1a1a; max-width:1240px; }}
@media (prefers-color-scheme: dark) {{ body {{ background:#121212; color:#e8e8e8; }}
  table {{ border-color:#333 !important; }} th {{ background:#1e1e1e !important; }}
  td, th {{ border-color:#2a2a2a !important; }} .card {{ background:#1a1a1a !important; border-color:#2e2e2e !important; }} }}
h1 {{ font-size:20px; margin:0 0 2px; }}
.sub {{ color:#777; font-size:12px; margin-bottom:20px; }}
.card {{ background:#fff; border:1px solid #e2e2e2; border-radius:8px; padding:18px; margin-bottom:18px; }}
.score {{ font-size:54px; font-weight:700; line-height:1; letter-spacing:-1px; }}
.score small {{ font-size:16px; font-weight:400; color:#888; }}
.phase {{ display:inline-block; padding:3px 10px; border-radius:999px; color:#fff; font-weight:600; font-size:12px; text-transform:uppercase; letter-spacing:.5px; }}
table {{ border-collapse:collapse; width:100%; font-size:13px; }}
th {{ text-align:left; background:#f2f2f2; padding:8px; border-bottom:1px solid #ddd; font-size:11px; text-transform:uppercase; letter-spacing:.4px; color:#666; }}
td {{ padding:8px; border-bottom:1px solid #eee; vertical-align:top; }}
td.num {{ text-align:right; font-variant-numeric:tabular-nums; white-space:nowrap; }}
td.id {{ font-weight:600; white-space:nowrap; }}
td .lbl {{ font-weight:400; color:#888; font-size:11px; white-space:normal; }}
td.stress {{ font-weight:700; }}
td.val {{ font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:12px; white-space:nowrap; }}
td.dt {{ color:#555; font-size:12px; }}
.prov {{ color:#999; font-size:11px; }}
tr.gap {{ background:rgba(198,40,40,.06); }}
tr.gap td.stress {{ color:#b3261e; }}
tr.warn td:nth-child(2) {{ color:#a84800; font-weight:600; }}
tr.ok td:nth-child(2) {{ color:#2e7d32; }}
ul {{ margin:6px 0 0; padding-left:20px; }}
li {{ margin-bottom:4px; font-size:13px; }}
.note {{ background:#fff8e1; border-left:3px solid #f9a825; padding:10px 14px; border-radius:4px; font-size:13px; }}
.card.lm {{ border-left:4px solid #1a73e8; }}
/* The blind-spot card is deliberately visually distinct: it is not data quality,
   it is a permanent limit on what the score can mean. */
.card.blind {{ border-left:4px solid #b3261e; background:#fdf5f4; }}
@media (prefers-color-scheme: dark) {{ .card.blind {{ background:#241a1a !important; border-color:#7f3b36 !important; }} }}
.lm-intro {{ font-size:14px; margin:0 0 14px; color:#333; }}
.lm-grid {{ display:grid; gap:12px; grid-template-columns:repeat(auto-fit,minmax(300px,1fr)); }}
.lm-block {{ background:#f5f8fd; border-radius:6px; padding:11px 13px; }}
.lm-h {{ font-weight:700; font-size:12px; text-transform:uppercase; letter-spacing:.5px; color:#1a73e8; margin-bottom:4px; }}
.lm-b {{ font-size:13.5px; line-height:1.55; color:#222; }}
.lm-bottom {{ margin-top:14px; padding:11px 13px; background:#eef3fb; border-radius:6px; font-size:13.5px; line-height:1.55; }}
@media (prefers-color-scheme: dark) {{
  .lm-intro {{ color:#ddd; }}
  .lm-block {{ background:#1e2733; }}
  .lm-b {{ color:#e8e8e8; }}
  .lm-bottom {{ background:#1e2733; }}
}}
.disc {{ color:#777; font-size:12px; border-top:1px solid #e2e2e2; padding-top:12px; }}
/* Falsification panel. Colour-coded by verdict so the AGAINST rows are
   impossible to miss, since they are the ones a reader most needs to see. */
.card.fals {{ border-left:4px solid #2e7d32; }}
table.ftab {{ margin:0; }}
table.ftab td {{ border-bottom:1px solid #eee; padding:9px 8px; vertical-align:top; }}
td.f-mark {{ font-weight:700; font-size:10.5px; letter-spacing:.5px; white-space:nowrap; width:74px; }}
tr.f-counter td.f-mark {{ color:#2e7d32; }}
tr.f-forb td.f-mark {{ color:#ef6c00; }}
tr.f-uninf td.f-mark {{ color:#888888; }}
tr.f-counter {{ background:rgba(46,125,50,.05); }}
.f-q {{ font-weight:600; font-size:13px; }}
.f-r {{ font-size:12.5px; color:#333; margin-top:2px; }}
.f-d {{ font-size:11.5px; color:#777; margin-top:4px; line-height:1.5; }}
.card.expl {{ border-left:4px solid #1a73e8; }}
details.tech {{ margin-top:10px; font-size:12px; }}
details.tech summary {{ cursor:pointer; color:#777; }}
details.tech .f-d {{ margin-top:6px; padding:8px 10px; background:#f7f7f7; border-radius:4px; }}
@media (prefers-color-scheme: dark) {{ details.tech .f-d {{ background:#1e1e1e; }} }}
.expl-stat {{ font-size:36px; font-weight:700; line-height:1; letter-spacing:-1px; margin-bottom:4px; }}
.card.exp {{ border-left:4px solid #6a1b9a; }}
.card.judg {{ border-left:4px solid #5f6368; }}
table.jtab td {{ border-bottom:1px solid #eee; padding:10px 8px; vertical-align:top; }}
.j-claim {{ font-weight:600; font-size:13px; }}
.j-meta {{ font-size:10.5px; text-transform:uppercase; letter-spacing:.4px; color:#888; margin:2px 0 5px; }}
.j-ev, .j-fal {{ font-size:12px; color:#444; margin-top:4px; line-height:1.5; }}
.j-fal {{ color:#7a3e12; }}
tr.j-sup {{ background:rgba(239,108,0,.05); }}
tr.j-aga {{ background:rgba(46,125,50,.06); }}
@media (prefers-color-scheme: dark) {{
  table.jtab td {{ border-color:#2a2a2a; }}
  .j-ev {{ color:#ccc; }} .j-fal {{ color:#e0a878; }}
}}
.nd {{ color:#b3261e; font-size:11px; font-style:italic; }}
.exp-note {{ font-size:11.5px; color:#777; }}
.exp-nr td {{ padding-top:0; border-bottom:1px solid #eee; }}
.exp-nh {{ font-weight:600; }}
tr.exp-nr {{ background:rgba(179,38,30,.03); }}
.exp-note ul {{ margin:2px 0 8px; padding-left:18px; }}
.exp-note li {{ margin-bottom:2px; }}
@media (prefers-color-scheme: dark) {{
  table.ftab td {{ border-color:#2a2a2a; }}
  .f-r {{ color:#ddd; }}
  tr.f-counter {{ background:rgba(46,125,50,.12); }}
}}
.spark-empty {{ font-size:12.5px; color:#777; background:#f7f7f7; border-radius:4px; padding:9px 11px; margin:0; }}
@media (prefers-color-scheme: dark) {{ .spark-empty {{ background:#1e1e1e; color:#aaa; }} }}
</style></head><body>

<h1>AI Bubble Watch</h1>
<div class="sub">generated {gen} · bubble-watch {ver} · coverage {cov:.0}% · confidence {conf}</div>

<div class="card lm">
  <h3 style="margin-top:0;font-size:15px">Plain-English summary</h3>
  <p class="lm-intro">{lm_what}</p>
  <div class="lm-grid">
    {lm_blocks}
  </div>
  <div class="lm-bottom"><b>Bottom line.</b> {lm_bottom}</div>
</div>

<div class="card blind">
  <h3 style="margin-top:0;font-size:15px">What this report cannot see</h3>
  <p style="margin:0;font-size:13.5px;line-height:1.6">{blind}</p>
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">The score</h3>
  <div class="score">{score:.1}<small>/100 stress</small></div>
  <div style="margin:10px 0 4px"><span class="phase" style="background:{pcol};color:{ptxt}">{phase}</span></div>
  <p class="lm-b" style="margin:12px 0 0">{thescore}</p>
  <div class="note">{pdetail}</div>
  <div style="margin-top:16px">{gauge}</div>
  <div style="margin-top:6px;font-size:13px">{analog}</div>
  <div style="margin-top:10px;font-size:12px;color:#777">{analogcaveat}</div>
</div>

{judgcard}

{falscard}

{explcard}

{trendcard}

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Headline</h3>
  <p style="margin:0">{headline}</p>
</div>

{expcard}

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Indicators</h3>
  <table>
    <thead><tr><th>Indicator</th><th>Weight</th><th>Stress</th><th>Contribution</th><th>Value</th><th>Data</th><th>Reading &amp; provenance</th></tr></thead>
    <tbody>{rows}</tbody>
  </table>
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Data quality — coverage {cov:.0}%</h3>
  <p style="margin:0 0 6px;font-size:13px">Unavailable indicators contribute <b>nothing</b>: they are never imputed or defaulted. The composite is renormalized over the {avail:.0} of {total:.0} weight that is actually available.</p>
  <ul>{gaps}</ul>
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Sources</h3>
  <table><thead><tr><th>Source</th><th>Status</th><th>Counts</th><th>Detail</th></tr></thead><tbody>{srcrows}</tbody></table>
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Caveats</h3>
  <ul>{caveats}</ul>
</div>

<p class="disc">{disc}</p>
</body></html>"##,
        date = esc(&r.generated_at),
        gen = esc(&r.generated_at),
        ver = esc(&r.version),
        cov = r.coverage * 100.0,
        conf = esc(&r.confidence),
        lm_what = esc(&r.layman.what_this_is),
        lm_blocks = lm_blocks,
        lm_bottom = esc(&r.layman.bottom_line),
        score = r.composite,
        thescore = esc(&r.layman.the_score),
        phase = esc(&r.phase),
        pcol = phase_color(&r.phase),
        ptxt = phase_text_color(&r.phase),
        pdetail = esc(&r.phase_detail),
        gauge = gauge(r.composite),
        analog = analog,
        analogcaveat = esc(&r.analog.caveat),
        headline = esc(&r.headline),
        trendcard = trend_card(r),
        falscard = falsifiers_card(r),
        judgcard = judgments_card(r),
        explcard = explosiveness_card(r),
        expcard = exposure_card(r),
        blind = esc(&r.layman.known_blind_spots),
        rows = rows,
        avail = r.data_quality.available_weight,
        total = r.data_quality.total_weight,
        gaps = gaps,
        srcrows = src_rows,
        caveats = caveats,
        disc = esc(&r.disclaimer)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_in_untrusted_text() {
        assert_eq!(esc("<script>x</script>"), "&lt;script&gt;x&lt;/script&gt;");
        assert_eq!(esc("a & b"), "a &amp; b");
    }

    #[test]
    fn phase_colors_cover_every_phase() {
        for p in ["early", "mid", "late", "critical", "unknown"] {
            assert!(!phase_color(p).is_empty());
        }
    }

    #[test]
    fn gauge_clamps_out_of_range_scores() {
        // Must not panic or emit NaN coordinates for impossible inputs.
        let g = gauge(-50.0);
        assert!(g.contains("polygon"));
        let g2 = gauge(500.0);
        assert!(g2.contains("polygon"));
        assert!(!g2.contains("NaN"));
    }

    #[test]
    fn sparkline_refuses_to_draw_a_line_from_one_point() {
        // One point cannot make a line; drawing a flat one would imply a
        // measurement of stability that was never taken.
        let one = vec![crate::model::TrendPoint {
            date: "2026-09-17".into(),
            generated_at: "2026-09-17T00:00:00Z".into(),
            composite: 32.0,
            coverage: 1.0,
            phase: "early".into(),
            methodology_version: "1.1".into(),
            stresses: Default::default(),
        }];
        let s = sparkline(&one, 30);
        assert!(
            !s.contains("polyline"),
            "must not draw a line from one point"
        );
        assert!(s.contains("Not enough recorded runs"));
    }

    #[test]
    fn sparkline_handles_a_flat_series_without_dividing_by_zero() {
        let mk = |c: f64| crate::model::TrendPoint {
            date: "2026-09-17".into(),
            generated_at: "2026-09-17T00:00:00Z".into(),
            composite: c,
            coverage: 1.0,
            phase: "early".into(),
            methodology_version: "1.1".into(),
            stresses: Default::default(),
        };
        let s = sparkline(&[mk(30.0), mk(30.0), mk(30.0)], 30);
        assert!(s.contains("polyline"));
        assert!(!s.contains("NaN"), "a constant series must not emit NaN");
        assert!(!s.contains("inf"));
    }

    /// WCAG relative luminance and contrast ratio, so the colour rules are
    /// enforced by a test rather than by eye.
    fn luminance(hex: &str) -> f64 {
        let h = hex.trim_start_matches('#');
        let chan = |i: usize| -> f64 {
            let c = u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * chan(0) + 0.7152 * chan(2) + 0.0722 * chan(4)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (l1, l2) = (luminance(a), luminance(b));
        let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn every_phase_pill_meets_the_contrast_threshold() {
        // Regression guard. The original palette put WHITE text on the amber
        // "mid" pill: 1.97:1, which is unreadable, and on orange "late": 3.08:1.
        // Amber and orange are light colours and need dark text.
        for phase in ["early", "mid", "late", "critical", "unknown"] {
            let bg = phase_color(phase);
            let fg = phase_text_color(phase);
            let ratio = contrast(fg, bg);
            assert!(
                ratio >= 4.5,
                "phase '{}': {} on {} is only {:.2}:1, below the 4.5:1 threshold",
                phase,
                fg,
                bg,
                ratio
            );
        }
    }

    #[test]
    fn phase_colours_are_distinguishable_from_each_other() {
        // A palette is useless if two phases render identically.
        let mut seen: Vec<&str> = Vec::new();
        for p in ["early", "mid", "late", "critical"] {
            let c = phase_color(p);
            assert!(!seen.contains(&c), "phase colour {} is reused", c);
            seen.push(c);
        }
    }

    #[test]
    fn dashboard_never_links_to_a_report_page_that_does_not_exist() {
        // Regression guard: the first version linked every archived date to a
        // per-day page, but pages only existed for dates written by that run —
        // producing 404 links. A dead link is a false claim that a report
        // exists.
        let mk = |d: &str| crate::model::TrendPoint {
            date: d.into(),
            generated_at: format!("{}T00:00:00Z", d),
            composite: 30.0,
            coverage: 1.0,
            phase: "early".into(),
            methodology_version: "1.1".into(),
            stresses: Default::default(),
        };
        let pts = vec![mk("2026-09-10"), mk("2026-09-11"), mk("2026-09-12")];
        // Only 2026-09-11 has a page.
        let have = |d: &str| d == "2026-09-11";
        let h = render_dashboard(&pts, None, 30, &have);

        assert!(
            h.contains("href='2026-09-11.html'"),
            "the existing page must be linked"
        );
        assert!(
            !h.contains("href='2026-09-10.html'"),
            "must not link a missing page"
        );
        assert!(
            !h.contains("href='2026-09-12.html'"),
            "must not link a missing page"
        );
        assert!(
            h.contains("(summary only)"),
            "should say why there is no link"
        );
    }

    #[test]
    fn dashboard_is_self_contained_and_says_so_when_empty() {
        let d = render_dashboard(&[], None, 30, &|_| false);
        assert!(d.starts_with("<!DOCTYPE html>"));
        assert!(!d.contains("<script"), "must not require JavaScript");
        assert!(!d.contains("src=\"http") && !d.contains("href=\"http"));
        assert!(
            d.contains("No runs recorded yet"),
            "an empty dashboard must explain itself rather than render blank"
        );
    }
}
