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

fn phase_color(phase: &str) -> &'static str {
    match phase {
        "early" => "#2e7d32",
        "mid" => "#f9a825",
        "late" => "#ef6c00",
        "critical" => "#c62828",
        _ => "#616161",
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
             <text x='{mx:.1}' y='60' font-size='10' fill='#9e9e9e' text-anchor='middle'>{lbl}</text>",
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
  background:#fafafa; color:#1a1a1a; }}
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
tr.gap td.stress {{ color:#c62828; }}
tr.warn td:nth-child(2) {{ color:#ef6c00; font-weight:600; }}
tr.ok td:nth-child(2) {{ color:#2e7d32; }}
ul {{ margin:6px 0 0; padding-left:20px; }}
li {{ margin-bottom:4px; font-size:13px; }}
.note {{ background:#fff8e1; border-left:3px solid #f9a825; padding:10px 14px; border-radius:4px; font-size:13px; }}
.disc {{ color:#777; font-size:12px; border-top:1px solid #e2e2e2; padding-top:12px; }}
</style></head><body>

<h1>AI Bubble Watch</h1>
<div class="sub">generated {gen} · bubble-watch {ver} · coverage {cov:.0}% · confidence {conf}</div>

<div class="card">
  <div class="score">{score:.1}<small>/100 stress</small></div>
  <div style="margin:10px 0 4px"><span class="phase" style="background:{pcol}">{phase}</span></div>
  <div class="note">{pdetail}</div>
  <div style="margin-top:16px">{gauge}</div>
  <div style="margin-top:6px;font-size:13px">{analog}</div>
  <div style="margin-top:10px;font-size:12px;color:#777">{analogcaveat}</div>
</div>

<div class="card">
  <h3 style="margin-top:0;font-size:15px">Headline</h3>
  <p style="margin:0">{headline}</p>
</div>

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
        score = r.composite,
        phase = esc(&r.phase),
        pcol = phase_color(&r.phase),
        pdetail = esc(&r.phase_detail),
        gauge = gauge(r.composite),
        analog = analog,
        analogcaveat = esc(&r.analog.caveat),
        headline = esc(&r.headline),
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
}
