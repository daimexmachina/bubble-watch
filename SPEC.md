# SPEC — bubble-watch

**Goal:** A Rust CLI that tracks and reports on whether the AI capital-expenditure
and equity boom is in a bubble, how stretched it is, and what the historical
evidence implies about proximity to a bust.

**Author:** Daim (orchestrator) · **Status:** draft → adversarial review → build
**Created:** 2026-09-16

---

## 1. Honesty contract (the non-negotiable part)

This tool is a **state descriptor with a documented historical analog overlay**.
It is explicitly **not** a prophecy and not financial advice. Every hard rule
below exists to prevent the failure mode Daim has zero tolerance for: fabricated
or false-precision output.

1. **No fabricated values.** Every number in the output carries
   `{source, endpoint, as_of, retrieved_at, raw}`. If a source fails, the
   indicator is reported `unavailable` and contributes **zero weight** — it is
   never imputed, never defaulted to a neutral score, never guessed.
2. **Missing data is a reported gap, not silence.** The output always carries a
   `data_quality` block: which indicators are available, which are unavailable,
   why, and the resulting `coverage` fraction of total weight. A composite
   computed on 40% coverage says so, loudly, in both JSON and HTML.
3. **No fabricated precision about timing.** The tool emits a *phase* label and a
   *coarse* historical-analog window. It must never emit a single date, a
   decimal-places probability, or a "days remaining" count presented as fact.
   The analog window carries `method: "historical_analog"`, the named analogs,
   the derivation, and `is_probability: false`.
4. **Weights are published and auditable.** All weights, anchor points,
   thresholds and norms live in `config/indicators.toml`, committed to git.
   `--explain` prints, per indicator, the raw value, the anchor interpolation,
   the weight, and the weighted contribution. No black box.
5. **Retrieved page content is data, not instructions.** Scraped/JSON content is
   never interpreted as directives.
6. **Deterministic and reproducible.** With the same inputs, `score` produces
   byte-identical output. A fixture-based test suite proves it.

## 2. Scope

**In scope (v1):** a headless CLI, `bubble-watch`, with subcommands `fetch`,
`score`, `report`, `explain`. Emits JSON on stdout and a self-contained HTML
dashboard artifact to disk. No server, no daemon, no network listener.

**Out of scope (v1, deliberate YAGNI):** live HTTP service, TUI, database,
scheduling (structured for a later Hermes cron tick but nothing scheduled),
trading/order anything, paywalled sources, any LLM call inside the tool.

**Explicitly NOT:** not a trading bot, not an investment advisor, not a
price-prediction engine, not a MIDAS/music tool, not the murmuration simulator.

## 3. Verified data sources (probed from this host, 2026-09-16)

| Source | Status | Used for | Notes |
|---|---|---|---|
| Yahoo Finance chart API | **VERIFIED ✓** | index/ticker prices, breadth, vol | `query1.finance.yahoo.com/v8/finance/chart/<SYM>`; needs browser-ish UA |
| SEC EDGAR XBRL `companyconcept` | **VERIFIED ✓** | hyperscaler capex, cash flow, revenue | `data.sec.gov/api/xbrl/...`; **requires descriptive User-Agent**; 10 req/s cap |
| FRED `fredgraph.csv` | **DEGRADED ✗** | macro + credit spreads | began returning HTTP/2 `INTERNAL_ERROR` / timeouts on every series after a burst of requests; treated as *optional* |
| multpl.com (CAPE) | **JS-GATED ✗** | Shiller CAPE | now requires JavaScript; not scrapable in plain HTTP → declared gap |

**Consequence:** the core must run to completion on Yahoo + EDGAR alone. FRED is
an *enhancement*: when it answers, credit and macro indicators light up and
coverage rises; when it does not, they are reported unavailable and the composite
renormalizes. This is the primary architectural pressure in this spec and the
main thing the adversarial review should attack.

### 3.1 Concrete source recipes (as verified)

- **Yahoo:** `GET /v8/finance/chart/%5EGSPC?range=1y&interval=1d` and
  `?range=1y&interval=1mo`. Symbols: `%5EGSPC` (S&P 500), `RSP`, `SPY`, `QQQ`,
  `%5EVIX`, `TLT`, `HYG`, `SMH`, `%5ETNX`. Response shape:
  `chart.result[0].indicators.quote[0].close[]`, `...adjclose`, `timestamp[]`.
- **EDGAR:** `GET https://data.sec.gov/api/xbrl/companyconcept/CIK{10digit}/us-gaap/{tag}.json`
  → `units.USD[]` of `{start,end,val,form,filed,fy,fp}`.
  **Pitfall found in probing:** tag fallbacks can return *stale* facts
  (naive fallback returned MSFT revenue from 2010 and META from 2018). The
  resolver must pick the candidate tag whose latest quarterly `end` is **most
  recent**, then select the latest-duration fact, not the first match.
  Tags: capex `PaymentsToAcquirePropertyPlantAndEquipment`; CFO
  `NetCashProvidedByUsedInOperatingActivities`; revenue `Revenues` |
  `RevenueFromContractWithCustomerExcludingAssessedTax` | `...IncludingAssessedTax`.

### 3.2 CIK map (v1)

MSFT 0000789019 · GOOGL 0001652044 · AMZN 0001018724 · META 0001326801 ·
ORCL 0001341439 · NVDA 0001045810

## 4. Architecture

Pure-functional scoring core, imperative IO shell. The core is a *pure function*
of `(observations, config) -> Report`, so it is trivially unit-testable and
provably deterministic; all IO lives in `sources/`.

```
src/
  main.rs            CLI entry (clap derive)
  lib.rs             public surface: fetch_all, build_report, render_*
  model.rs           Observation, IndicatorReading, Composite, DataQuality, Report
  score.rs           PURE: anchor interpolation + weighted composite + coverage
  phase.rs           PURE: phase classification + historical-analog window
  config.rs          indicators.toml load + validation (weights sum, anchors monotone)
  http.rs            shared GET: descriptive UA, retry w/ backoff, http1.1, rate limit, timeout
  sources/
    mod.rs           Source trait + fetch_all aggregation
    yahoo.rs         chart API client
    edgar.rs         XBRL client incl. the most-recent-end tag resolver
    fred.rs          optional; marks Unavailable(reason) on failure
  indicators/
    mod.rs           Indicator trait + registry
    *.rs             one file per indicator (see §5)
  report/
    json.rs          canonical JSON
    html.rs          self-contained dashboard (inline CSS/SVG, no CDN, no JS deps)
tests/
  fixtures/*.json    captured real responses (trimmed) for deterministic tests
  score_tests.rs     math + determinism + missing-data behavior
  integration_tests.rs  live-but-tolerant smoke tests
```

**The `Indicator` contract:**

```rust
pub trait Indicator {
    fn id(&self) -> &'static str;
    fn weight(&self, cfg: &Config) -> f64;
    fn evaluate(&self, obs: &Observations, cfg: &Config) -> IndicatorReading;
}
pub enum Reading { Scored { stress: f64, value: f64, unit: String, detail: String },
                   Unavailable { reason: String } }
```

## 5. Indicator set (v1)

Mapped to Capital Economics' eight bubble indicators (CNBC 2026-09-10), plus
credit and fundamental signals. `stress` is 0 = calm, 100 = historical extreme.

| # | id | What it measures | Primitive | Wt |
|---|---|---|---|---|
| 1 | `valuation` | earnings yield vs 10Y (`%5ETNX`) → equity risk premium compression; plus S&P level vs 200d | Yahoo | 14 |
| 2 | `concentration` | cap-weight (SPY) vs equal-weight (RSP) 12m relative; proxy for mega-cap dominance | Yahoo | 12 |
| 3 | `breadth` | RSP/SPY ratio trend + divergence from its own 200d | Yahoo | 10 |
| 4 | `issuance` | net equity issuance proxy: Σ Δ shares-outstanding across mega-caps (EDGAR `dei:EntityCommonStockSharesOutstanding`) + hyperscaler debt growth | EDGAR | 10 |
| 5 | `volatility` | VIX (`%5EVIX`) level + realized vol of SPX | Yahoo | 8 |
| 6 | `credit_hy` | US HY option-adjusted spread (wide = high stress; direction of travel is the signal) | FRED | 12 |
| 7 | `credit_ig` | US IG OAS | FRED | 6 |
| 8 | `capex_vs_cashflow` | hyperscaler capex as % of operating cash flow ("the C-F-D ratio") | EDGAR | 14 |
| 9 | `funding_gap` | capex vs revenue run-rate + debt growth across the cohort | EDGAR | 8 |
| 10 | `leverage` | cohort long-term debt trend vs EBITDA-like (CFO) proxy | EDGAR | 6 |
| 11 | `foreign_interest` | foreign ownership of US equities | — | 0 (declared gap, no free source) |

Weights sum to 100 excluding the declared-gap indicator. All values, anchors and
weights are in `config/indicators.toml`.

## 6. Scoring math (must be exactly this)

**Normalization — published piecewise-linear anchors.** For each indicator the
config declares monotone anchor pairs `[(raw_value, stress), ...]`. The stress is
`interpolate(raw, anchors)`, clamped to `[0,100]`, and extrapolated flat beyond
the outermost anchors. Anchors are chosen from documented historical reference
points, and are *inputs to be reviewed*, not derivations.

*Worked example, `credit_hy`:* anchors
`[(2.0, 5), (2.8, 18), (3.5, 30), (5.0, 50), (7.0, 75), (10.0, 95)]`.
Current HY OAS 2.76% → ~17.6 → **low stress**. That is the correct answer and the
tool must be willing to print it: tight credit spreads in isolation are *not*
currently a bubble signal.

**Composite — weight-renormalized over what is actually available:**

```
coverage = Σ w_i (available) / Σ w_i (all)
composite = Σ (w_i * stress_i)(available) / Σ w_i (available)
```

So an unavailable source never drags the score toward 0 or 50; it shrinks
coverage instead. `confidence` is reported as `low | medium | high` from coverage
(<0.6 / 0.6–0.85 / >0.85).

**Phase classification** (thresholds in config):
`early` <35 · `mid` 35–55 · `late` 55–75 · `critical` >75.

**Historical-analog window.** A committed table maps composite band →
lead time to peak observed in named analogs (dot-com 1999–2001 and telecom
2000–02), expressed as a **range in months**. Output carries
`{"method":"historical_analog","is_probability":false,"analogs":[...],"derivation":"..."}`
and the README must state plainly that state-extremity is not timing.

## 7. CLI

```
bubble-watch fetch [--offline]            # warm data/cache, report source health
bubble-watch score [--json|--pretty]      # composite + phases + coverage
bubble-watch report --out DIR             # writes bubble-<date>.json + .html
bubble-watch explain                      # per-indicator math, contributions, gaps
bubble-watch sources                      # probe each source, print health
```

Global: `--config PATH`, `--offline` (cache only), `--cache-dir PATH`,
`--verbosity`. Exit codes: `0` ok · `2` config error · `3` all sources failed ·
`4` partial coverage below floor.

## 8. Testing strategy

- **Unit (pure core, fixture-driven):** anchor interpolation incl. clamping and
  extrapolation; composite renormalization; coverage math; determinism
  (same fixture twice → identical bytes); every missing-data branch; phase
  boundaries; the EDGAR most-recent-end tag resolver against a stale-tag fixture.
- **Golden file:** `tests/fixtures/golden_report.json` — the canonical report for
  a frozen fixture set; regenerating it is a reviewed, explicit act.
- **Integration (live, tolerant):** hits real Yahoo + EDGAR, asserts HTTP 200,
  parseable shape, and non-empty values — but *skips with a clear message* rather
  than failing the build when offline. Never asserts a specific market value.
- **No test may require FRED.** Its degradation must be provable entirely offline.

## 9. Risks the adversarial review must attack

1. **Proxy validity.** SPY/RSP spread is a *proxy* for concentration, and
   Δshares-outstanding is a *proxy* for net issuance. Both are weak. How weak,
   and does the tool overstate them?
2. **Anchor subjectivity.** The anchors encode the author's priors. Is a
   piecewise-linear, hand-anchored map defensible, or does it launder opinion as
   mathematics?
3. **The timing output.** Even with "is_probability: false", does presenting a
   months-range constitute the financial astrology Daim rejects? Is it culled?
4. **Degradation illusion.** With FRED down, coverage drops and the composite
   renormalizes over the remaining indicators — which are mostly *equity-price*
   indicators. Does that systematically bias the score in one direction?
5. **Survivorship / reference-class choice.** The analogs are two US tech busts.
   Is n=2 a legitimate basis for anything?
6. **Rate limiting.** EDGAR 10 req/s, Yahoo unofficial. Does the retry/backoff
   design actually hold, and does aggressive default concurrency trip blocks?

## 10. Definition of done

- `cargo build --release` clean; `cargo test` green with the fixture suite.
- `bubble-watch score`, `explain`, and `report` all run against live sources and
  produce real output captured in the completion evidence.
- **No number in any output without a source citation.**
- FRED-down path demonstrated: score still produced, gaps reported, coverage and
  confidence both drop.
- `config/indicators.toml` documents every weight and anchor with a rationale.
- README states the honesty contract and the not-a-prophecy disclaimer.
- Git history shows incremental commits; spec and config committed.

---

## 11. v1.1 — direction of travel (run history)

**Problem this fixes.** v1 is a point-in-time scorer with no memory. Its own
config says of credit spreads that *"the informative credit signal is the
DIRECTION OF TRAVEL from a tight base"* — and v1 cannot see direction at all,
because nothing persists between runs. A level with no reference point cannot
distinguish "wide and widening" from "wide and narrowing", which are opposite
signals.

**Design decision 1 — the trend is CONTEXT, not a scored indicator.** It does
NOT enter the composite. Adding it as an eleventh weighted indicator would change
a published, audited number (32.4 on the 2026-09-17 run) for a signal that is
*derived from the composite itself* — the score would partly be a function of its
own past. The composite is therefore byte-identical before and after this
version, and there is a test that asserts exactly that.

**Design decision 2 — comparing runs of unequal coverage is INVALID, and the
tool must refuse to do it.** §6 renormalizes the composite over available weight,
so a run at 100% coverage and a run at 62% coverage are not on the same scale;
subtracting them produces a difference that is partly an artefact of which
sources answered. The existing COMPARABILITY caveat already warns about this in
prose. The trend feature therefore *enforces* it: a baseline run is eligible only
if its weighted coverage is within `coverage_tolerance_pp` of the current run's.
When no eligible baseline exists, **no delta is reported** — the reason is
printed instead. There is no fallback to "the previous run anyway", because that
would manufacture a directional claim out of a definitional difference.

**Design decision 3 — a delta is never presented without its elapsed time.** A
change of +4 over 3 days and a change of +4 over 400 days are different facts.
Every delta carries the actual gap in days, and the baseline's date and coverage.

**Design decision 4 — the archive is append-only, but the series is daily.** The
archive (`data/history/runs.jsonl`, one JSON object per line) is append-only so
nothing is ever destroyed; re-running the tool four times in an afternoon is a
legitimate audit trail. The *series* used for trend computation takes at most one
entry per calendar date — the last one written that day — so intraday re-runs
cannot masquerade as a longer history. This is a selection rule for display, not
a deletion.

**Design decision 5 — a malformed archive line is reported, not swallowed.** A
line that fails to parse becomes a warning naming the line number; the remaining
record still loads. One bad byte must not destroy the history, and must not pass
silently either.

**Scope of the change:**

| Area | Change |
|---|---|
| `src/history.rs` | NEW. Archive load/append, daily series, pure delta computation. |
| `config [trend]` | `min_gap_days`, `coverage_tolerance_pp`, `flat_band`, `sparkline_points`. |
| `model` | `TrendPoint`, `IndicatorTrend`, `Trend`; `Report.trend`; `LaymanSummary.direction_of_travel`. |
| `report::build_with_history` | Builds the trend alongside the report; `build` delegates with an empty history so existing callers are unaffected. |
| CLI | `trend` subcommand; `report --no-record` and `--history-dir`. |
| HTML | Trend card with inline-SVG sparkline and per-indicator direction. |

**Definition of done for v1.1:**
- The composite is unchanged by this feature (test-asserted against the frozen
  fixture; the published 2026-09-17 number must still reproduce).
- No delta is ever emitted against a coverage-mismatched baseline.
- No delta is ever emitted without its elapsed days.
- `cargo test` green; the offline suite proves both suppression paths with no
  network.
- The HTML still loads nothing external and still needs no JavaScript.

---

## 12. v1.2 – v1.8 — corrections, new evidence, and falsification

Methodology `schema_version` moved 1.1 → 1.8 across this stretch. Every change is recorded in
`config/indicators.toml` with its measured justification; this section records the reasoning.

### 12.1 The rule that governs all of it

**A number the source did not supply is never printed.** Three distinct failure modes were found in
the wild and each now has its own handling, because conflating them is how fabrication happens:

| Case | Handling |
|---|---|
| Concept never reported (MSFT purchase obligations) | "not disclosed" |
| Reported but stale (AMZN, 810 days; ORCL debt, 2022) | rejected with the age stated |
| Series exists but the layout changed | hard error, never an empty series |

A stale or exactly-zero balance sheet series is a **wrong tag**, not a fact. This was not hypothetical:
`us-gaap:LongTermDebt` resolves for ORCL to a single 2022 fact worth `0.0`, so the model reported the
most leveraged company in the cohort as carrying **no debt**.

### 12.2 Corrections (the score moved because the model was wrong, not the market)

| Change | Measured effect |
|---|---|
| Leverage: ORCL debt bug + operating leases | stress 18.2 → 34.1 (understated 15.1 points) |
| Concentration/breadth double-counting | r = −0.70 (12m) / −0.93 (6m); weights 22 → 11 |
| Issuance: share-count proxy → reported cash flows | +50.9 → 25.3; four of five cohort members retire stock on net |
| `credit_ig`: 2.5 years → 40 years (Baa less 10Y) | anchors now reference 2000, 2008, 2020 |
| `foreign_interest`: a **false** "unmeasurable" claim | retired; now the highest reading at 92.0 |

That last one deserves emphasis: the config stated *"There is no free, machine-readable series for it
at the required timeliness."* That was false. Fed Z.1 publishes it quarterly since 1945. Declaring
something unmeasurable when it is measurable converts a gap in **effort** into a claim about the
**world**, and a reader cannot tell the difference from outside.

### 12.3 New evidence

- **`backlog_quality`** — RPO ÷ deferred revenue. Contracted backlog is a promise; deferred revenue is
  cash billed. ORCL's backlog exploded 137.8B → 455.3B in one quarter while billed cash went
  10.7B → 13.4B. Deliberately *not* an RPO/revenue measure, which is a Rorschach test.
- **`private_credit_growth`** — the channel BIS Bulletin 120 names as fastest-growing is invisible to
  public index spreads. Live: `credit_hy` 16.4 and `credit_ig` 9.7 (calm) against private credit 44.7.
- **`datacenter_construction`** — the only **non-financial** series in the model. +58.5% YoY.
- **`grid_cancellations`** — 38.7% of announced capacity abandoned. A *level*, not a rate; the sheets
  are cumulative inventories.
- **`narrative_saturation`** — an EDGAR census (not a sample). 81× growth in AI mentions since 2015.

### 12.4 Two things deliberately kept OUT of the composite

**GSADF explosiveness test.** A formal hypothesis test, not a hand-anchored judgement. Averaging it in
would destroy the point of having a method that can disagree. It currently does: composite "mid" while
the test is significant at 1%.

**Falsification tests.** Built by asking the opposite question — what would show the thesis is wrong?
Reported beside the score because a 30 would otherwise mean either "mild bubble" or "strong
counter-evidence". Two of three currently read against.

The reason both exist: **the composite rose 30.6 → 40.5 during development, largely by adding
indicators that scored high.** A researcher looking for confirming evidence finds it. A model that
cannot say "I was wrong" is not an instrument, it is a position.

### 12.5 Definition of done for v1.8

- 272 tests green offline; live assertions opt-in.
- Every new source probe-verified with real values before the indicator was written.
- Every redefinition bumped `schema_version`, and the archive **refuses** cross-version baselines.
- The site renders the falsifiers, the explosiveness test and the exposure table — shipping analysis
  nobody can see is not finished work.

---

## 13. v1.9 — the last two decisions

### 13.1 NVIDIA is excluded from the capex cohort, on a measurement

The circularity gap asserted that "NVIDIA is not in the cohort", which read as an
oversight. It is a decision, and it was tested: NVIDIA is **fabless**, so its capex is
0.005x operating cash flow against a cohort mean of 0.512, and 0.042x revenue against
0.268. Adding it would pull both ratios down ~18% and convert them into a measure of the
supplier rather than the spender. The claim was reworded from a fact into a decision, and
the reasoning is recorded on `COHORT` in the source.

### 13.2 The depreciation "capital subsidy" ships behind a two-stage guard

Five attempts were needed, and the failures are the finding. A naive rate test flags six
of nine companies — including AAPL and AVGO, which show the same drift with no AI capex —
and misses ORCL, the real signal. Two data faults surfaced along the way: EDGAR repeats a
fact once per filing that carries it, and a naive "first versus last period" compared 2017
for one filer against 2008 for another.

Working design requires both stages:

1. `series_health` on gross PP&E first — rejects AMZN (gap + definition swap), META
   (abandoned tag) and GOOGL (stale).
2. A rate test over the trailing four annual periods, deduplicated by date, **ANDed with
   asset growth**. The AND is what suppresses the negative controls.

Result: exactly one of six flags (ORCL, −2.59pt with PP&E 4.28x).

### 13.3 A structural lesson

The first run measured only two filers, because the negative controls are not in the capex
cohort — so the guard's suppression could not be exercised and the rationale claimed more
than the code did. Fixed with a separate `ACCOUNTING_PEERS` set in its own map.

Kept separate rather than folded into `COHORT` deliberately: five other indicators iterate
the cohort, so adding peers there would have silently changed all of them.

**General rule this establishes:** when a new indicator needs *different or additional
data* than the cohort provides, give it its own source set. Widening a shared set is a
quiet, wide-reaching change, and the blast radius is invisible at the call site.

## 14. v2.0 — the frontier premium, declared judgment, and three truthfulness defects

Methodology 2.0 adds one indicator and, more importantly, fixes three places where the tool
described its own data incorrectly. The defects are recorded here because two of them were
live in shipped output.

### 14.1 `frontier_premium` (weight 6; weight total 140 -> 146 at the time)

The one structural feature no prior bubble had: the financed asset is reproducible by anyone.
Measured as the best closed model's arena rating minus the best open one's, over 243 LMArena
snapshots. Compressed from **+135 Elo (2024-02) to +32.5 (2026-09)**, briefly **negative** in
early 2025.

**Anchors descend in the gap** — a narrow gap scores as high stress. This is the only indicator
where the usual direction is inverted, because the capex assumes a wide moat. Stated in the
config rationale and in the output, since an inverted anchor is exactly the kind of thing a
later reader would "fix" into a bug.

Source is parquet, so it is extracted once into a committed fixture and regenerated by a
versioned script. Tested that the script reproduces the committed fixture exactly.

### 14.2 Three truthfulness defects, all found by reading output rather than trusting it

**(a) A frozen fixture reported with a fresh timestamp.** `frontier_premium` reads committed
data and cannot update itself, so without a guard it would report the same gap forever while
`retrieved_at` showed today. Now a 120-day staleness guard converts it into a *reported gap*
naming the vintage and the refresh procedure, and an integration test fails if the shipped
fixture has expired — otherwise the indicator would silently vanish from every future run.

**(b) "No previous run is recorded yet" when 28 were on disk.** `score` never opens the
archive, but it printed a factual claim *about* the archive plus a false remedy ("run the tool
again and a comparison will appear") that its code path could never fulfil. Root cause: an
empty slice is ambiguous between "did not look" and "looked and found nothing". The report now
takes `history_consulted` and describes the *command* instead of asserting a fact about data it
never read.

**(c) One fixed rationale stapled onto every refusal.** The report appended "a change computed
across runs of unequal coverage..." to *every* direction-of-travel refusal. All 31 archived runs
carried coverage 1.0, so no run was ever rejected for coverage — yet every refusal was explained
with a coverage rationale. Real causes were methodology changes, same-day runs and short gaps.
The reason and its rationale were built in two different places and could diverge; the rationale
is now produced locally by the branch that knows the cause.

**General rule:** a message that explains *why* a number is absent must be produced where the
cause is known. Any explanation assembled at a distance from its cause will eventually describe
something that did not happen — which is the same class of error as fabricating the number.

### 14.3 Elapsed time is measured from timestamps, not calendar dates

The gap test used `days_between`, which parses `%Y-%m-%d` — dates only. Two runs either side of
UTC midnight differed by "1 day" while being minutes apart, and this host's archive contained
exactly such a pair (17 minutes apart, 2026-09-19T23:47:36Z and 2026-09-20T00:04:49Z) from which
a direction of travel was reported.

This defeated the guard's own documented purpose: `min_gap_days` exists so that "a re-run minutes
later being presented as a trend" cannot happen. It failed at precisely that case, and only near
midnight — where a scheduled job lands.

Now measured from `generated_at`. Consequences handled explicitly:

* **Unparseable timestamps fall back conservatively** to `date_gap - 1`, floored at zero. Two
  timestamps on adjacent dates can be one second apart, so a one-date difference earns *zero*
  days and the candidate is refused with a reason. Crediting a gap that cannot be demonstrated
  is the same error as scoring a missing indicator.
* **`min_gap_days` changed 1.0 -> 0.9.** Measuring real time makes 1.0 mean a strict 24 hours,
  and the daily timer fires ~0.9977 days apart, so legitimate daily runs would be rejected for
  firing minutes early — trading a nightly blind spot for a permanent one. 0.9 (21.6h) still
  refuses any same-day re-run while tolerating ~2.4h of jitter.
* **`TrendCfg::defaults()` was updated to match**, because its doc comment claims equality with
  the shipped config and that had just become false. A test now asserts they agree field by
  field so they cannot diverge silently.

### 14.4 What the backtests say about prediction (unchanged at 2.0)

Two independent tests — 3 crash peaks, then 13 over 155 years of Shiller data — put every
momentum signal **statistically indistinguishable from chance** (p = 1.000 / 0.903 / 1.000). One
weak real effect: conditional on a 12-month return above 25%, months land within 24 months of a
peak 36.9% of the time versus 17.4% unconditionally. That is a conditional *rate*, not a
predictor, and adversarial tests assert momentum must not beat chance. This is why the tool is a
state descriptor and says so in every report.

## 15. v2.1–v2.2 — circularity: closing the last declared gap

The model shipped with one declared gap (`circularity`, weight 0) and no others. It now has none.

### 15.1 Why the declared gap was wrong to declare

The weight-0 entry argued the topic was unmeasurable because "the measure required is a
RELATIONSHIP between companies while every indicator here is a price or a single-company
aggregate". That reasoning assumed the only possible measure was a numeric ratio.

**A relation is a PAIR, and the pair is disclosed.** Companies report related-party transactions
under ASC 850 and equity-method investments under their own notes. EDGAR full-text search accepts
a `ciks=` parameter, which attributes each hit to the FILER — turning a keyword count into a
DIRECTED EDGE (who discloses a relationship with whom) rather than a bag of mentions. The
mechanism is in `sources/circularity`.

### 15.2 A rubric, not a ratio — and the rubric is printed

No free source publishes "share of counterparty revenue funded by the investor's own capital", so
the honest options were to keep the gap or to **state the scoring rule explicitly and let a reader
disagree with it**. This does the second. Points:

| structure | pts |
|---|---|
| upstream supply commitments scaled to demand the vendor finances | 50 |
| vendor organising third-party capital to fund demand for its own product | 42 |
| supplier guaranteeing AND investing in its customer's infrastructure | 45 |
| datacentre buildout financed on a speculative-grade tenant's credit | 35 |
| vendor sublicensing a long-term lease while remaining liable for it | 38 |
| supplier holding its customer's tradable equity | 28 |
| vendor investing in, contracting with, and lending to the same counterparty | 48 |
| equity issued as consideration for the customer's purchases | 40 |
| a parent absorbing its AI unit's debt | 30 |
| disclosed related-party revenue with an equity stake | 25 |
| related-party supply under common control | 10 |
| upstream supply commitments scaled to demand the vendor finances | 50 |
| read and REFUTED | 0 |
| real, material supply relationship with NO financing element | 0 |

Plus a **bounded scale term** (0/2/5/8 points) by disclosed exposure against the filer's market
cap, **capped below the weakest structure** so the rubric cannot become a size contest. A test
asserts that cap.

**Rank is decided by commitment, not by size.** NVIDIA's announced $500B of third-party capital is
the largest figure in the research and ranks BELOW its signed $279B supply commitment, because it
is "subject to definitive agreements" and because the balance-sheet risk sits with the institutions
rather than with NVIDIA. A test asserts that ordering AND that the qualifier is recorded — dropping
it would make the biggest number in the model read as a committed obligation.

### 15.3 Four measurement defects found in this module, all by testing properties

Every one of these was caught by asserting a PROPERTY, never by asserting an expected number:

1. **A value and its unit described different quantities.** `value = 230` with `unit = "pct of the
   expressible rubric scale"` — a point total labelled as a percentage. Both fields are free-form,
   so nothing failed; the report simply stated two things that cannot both be true.
2. **A scale term out-ranked a structure class** (15 vs 10), which would have made the rubric a
   size contest. The bound test failed on the first attempt and the fix was to lower the scale, not
   to weaken the assertion.
3. **Coverage rose with reading effort.** The scored fraction had the read count in its
   denominator, so every edge read moved points from denominator to numerator. Two wrong fixes were
   tried first: the read count (effort-dependent) and the scanned count (which measures COVERAGE,
   not stress, and buried a 65% reading down to 12%).
4. **A stale coverage literal.** `coverage < 0.10` broke when a weight changed. The bound is now
   derived from the fixture-backed weights.

The correct normalisation — the mean severity per read edge — is **effort-independent**: reading an
average edge leaves it flat (tested to 1e-9, and for thirty average edges at once), a severe edge
raises it, and a refutation lowers it.

### 15.4 What the filings actually disclosed

Twenty-three verified entries, every one with a citation, a magnitude, and a **written falsifier**.
The figures are quoted from filings rather than summed, and that is deliberate — see §15.8:

- **NVDA** — **$500B** of third-party capital organised with Apollo, BlackRock, Blackstone,
  Brookfield, Goldman Sachs and KKR (announced, "subject to definitive agreements"); $279B of
  upstream supply commitments (up $160B in one quarter) and a **$105B**
  guarantee over 20-year OpenAI leases, which NVIDIA classifies as a credit derivative.
- **AMZN** — $8B in Anthropic notes (Level 3) plus a >$100B cloud commitment and a **$20B facility
  drawable only as AMZN hits compute-delivery milestones**; and $38B + $100B + $28.7B with OpenAI.
- **AMD** — a warrant for 160M shares at $0.01, vesting against 6 GW of purchases: **9.8% of AMD**.
- **AMZN / Lambda — REFUTED, and the most instructive case in the set: A PRODUCT NAME MATCHED AS
  A COMPANY.** "Lambda MicroVMs, a new flavor of the popular **AWS Lambda** serverless compute
  service" — Lambda is AMAZON'S OWN PRODUCT. The detector matched a product against the counterparty
  list. Disambiguating this requires **WORLD KNOWLEDGE**, not evidence in the filing: no reading of
  the passage alone reveals that AWS Lambda exists and is unrelated to the GPU neocloud of the same
  name. That is the honest limit of a name-based detector, recorded rather than papered over.
- **META / Anthropic — REFUTED, a LITIGATION CAPTION.** Meta's FY2025 10-K names Anthropic as a
  CO-DEFENDANT: "Carreyrou et al. v. Anthropic PBC, et al." The names co-occur because they share a
  docket category, not a contract. This is the failure mode a name-matching detector is MOST likely
  to hit and LEAST likely to catch by inspection — a caption reads like a corporate pairing to anyone
  skimming.
- **NVDA / Anthropic — REAL BUT NOT A CIRCULARITY, and a distinct category.** NVIDIA's Q3 FY2026
  release: "for the first time, Anthropic will run and scale on NVIDIA infrastructure, initially
  adopting 1 gigawatt of compute capacity". A material, named commitment — with **no stake, facility
  or guarantee** disclosed for Anthropic, while the SAME release discloses all three for OpenAI. A
  `Refuted` label would be wrong here (the detector fired correctly on a real relationship), and
  scoring it would inflate the reading. It is its own category and scores zero.
- **CRWV / Lambda and CRWV / Crusoe — REFUTED, from ONE sentence.** CoreWeave's competition risk
  factor: "We also compete with smaller cloud service providers focused on AI, including Crusoe and
  Lambda." A single clause produced TWO apparent relationships, neither real — the highest
  concentration of false positives found anywhere in this research.
- **AMZN / Databricks — REFUTED, and the subtlest case.** Amazon's disclosure is real:
  "Entered a strategic collaboration with Databricks ... to leverage AWS Trainium chips as the
  preferred AI chip". It is a COMMERCIAL PARTNERSHIP, not a financing structure — no equity, no
  facility, no guarantee, no lease. Contrast the AMZN entries that DO score, from the same issuer:
  an $8B Anthropic stake, a $20B facility, a $100B cloud commitment. The absence of financing terms
  is the distinguishing fact.
- **NVDA / Tesla — REFUTED.** Tesla appears in NVIDIA's 10-K only in the COMPETITION risk
  factor, listed among "companies with internal teams designing SoC products ... such as Tesla,
  Inc." alongside AMD, Broadcom and Intel. A rival list, not a relationship. Recorded because it is
  a SECOND KIND of false positive: Oracle's was a model-integration list, this is a competitor list,
  and a detector that scored mentions would score rivals as counterparties.
- **NVDA / CoreWeave** — NVIDIA holds CoreWeave's **listed equity**, reclassified from
  non-marketable to marketable on listing and marked at FairValueInputsLevel1. Scored LOW (28)
  because NVIDIA states the gain was "not significant": the relationship, not the amount, is the
  finding.
- **SMCI** — a **$600.0M** ten-year obligation for 21 MW, **sublicensed in full to Lambda** while
  SMCI keeps only "a right to seek reimbursement", filed under Item 2.03 as an off-balance-sheet
  arrangement.
- **APLD** — 250 MW of datacentre capacity leased to CoreWeave, where the LANDLORD's 9.250%
  notes are secured on a tenant rated **BB** while the refinanced debt carries **A3**; CoreWeave
  supplies a $50M letter of credit and two springing guaranties as "credit enhancement".
- **CRWV** — $18.4B of OpenAI commitments against revenue that is **67% Microsoft**, which closes
  the loop MSFT → OpenAI → CoreWeave → MSFT.

### 15.5 Refutations, retained on purpose

A rubric that could only rise would be an alarm. Two entries score **zero** and are kept so the
instrument's ability to come back LOWER is visible:

- Oracle names OpenAI only in a **model-integration list**, not as a contract counterparty.
- Oracle's **$638B** of RPO (up from $138B) is attributed to "certain significant cloud contracts"
  with **no counterparty named**. This is the largest figure in the research and it contributes
  nothing. Adding it *lowered* the score, which is the opposite of how a mention count behaves.

### 15.6 Weight history and the bias it accepts

Weight moved **0 → 6.0** (methodology 2.1, when three edges had been read) **→ 12.0** (methodology
2.2, with twelve). 12.0 matches `credit_hy`, the other financing-channel indicator, and sits below
the two 14s measuring headline spending and valuation.

**This is the alarm-drift the model already flags, accepted knowingly.** The composite moved
40.5 → 43.0 across this work largely by adding and raising indicators that score high. The defence
is not that the number is small; it is that every point traces to a specific sentence in a filing
that a reader can check, and that the instrument has twice demonstrated it can move the other way.

### 15.7 The limit that cannot be fixed by reading

The method cannot see a relationship a filer declines to name. Oracle discloses $638B anonymously
and compliantly. No amount of further reading resolves it, because the identity is not in the
filing. Anonymity on one side can sometimes be read from the counterparty's — that is how the
CoreWeave links were established, from the other end of the relationship.

### 15.8 Why there is no headline total

An earlier revision of this document and the README stated a combined exposure figure. **It was
wrong, and the error is instructive enough to record rather than quietly delete.**

The number was written without being derived. Checking it showed the honest total is not a single
number at all, for four reasons:

1. **The set mixes units.** AMD's entry is a share count (9.8% of AMD's shares outstanding) and
   CoreWeave's is a revenue share (67% from Microsoft). Neither is a dollar figure and neither can
   be added to one.
2. **Entries restate the same commitment.** Amazon's $100B cloud expansions appear in two separate
   edges, by counterparty; adding them is correct, but the $38B existing commitment is a different
   figure describing the same relationship and is easy to double-count.
3. **Two entries are REFUTATIONS and must contribute NOTHING.** Oracle's $638B of RPO is the
   largest figure in the research and it is explicitly a LIMIT, not exposure. A naive sum of every
   number in the file reaches roughly $2 trillion and is meaningless; a proper dedup by scored edge
   gives roughly $1.16 trillion — and even that figure is not comparable to the others, because
   $500B of NVIDIA's total is third-party institutional capital that sits on OTHER balance sheets,
   while the $105B guarantee is a contingent exposure of NVIDIA's own.

**The rule this establishes**: state each figure with its source and its nature, and refuse to
aggregate across units, across contingent and committed items, or across scored and refuted
entries. A combined total would be exactly the kind of confident, unauditable number this project
exists to refuse — and it was produced here by the maintainer, in the documentation, which is why
the rule is written down rather than assumed.
