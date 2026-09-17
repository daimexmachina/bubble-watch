# bubble-watch

A Rust CLI that tracks and assesses whether the AI capital-expenditure and equity boom is a bubble —
how stretched the measurable indicators are, and what the historical record says about such states.

It produces a composite **bubble-stress score** (0–100) from documented indicators with published
weights, and a self-contained HTML dashboard.

---

## Read this first: what this tool is and is not

**It is a state descriptor.** It measures where a set of published indicators sit relative to
documented historical reference points, and it is explicit about what it cannot measure.

**It is not a forecast.** It cannot tell you if or when a market reversal will occur, and it is not
investment advice. The "historical analog" output is a coarse comparison against two prior episodes
and is labelled `is_probability: false` in the JSON because it is not a probability.

**It will not guess.** If a source is unavailable, the indicator is reported as a **gap** and
contributes **nothing**. It is never imputed, never defaulted to a neutral midpoint, and never
scraped from an aggregator that might be stale. A degraded run reports lower coverage — it does not
quietly pretend to have measured something.

A **calm reading is not evidence of safety.** Bubbles are generally identifiable only in hindsight.
Tight credit spreads and healthy breadth are consistent both with "no bubble" and with "the
complacent phase of one".

---

## Install and run

```bash
cargo build --release

./target/release/bubble-watch sources              # probe each data source, report health
./target/release/bubble-watch score                # composite + phases + coverage
./target/release/bubble-watch score --json         # machine-readable
./target/release/bubble-watch explain              # per-indicator math, weights, anchors, provenance
./target/release/bubble-watch report --out out     # writes <name>.json and <name>.html
./target/release/bubble-watch --offline score      # cache only; never touches the network
```

Exit codes: `0` ok · `2` config error · `3` no usable data · `4` partial coverage.

Tests:

```bash
cargo test                                         # 64 tests, no network required
BUBBLE_WATCH_LIVE=1 cargo test -- --nocapture      # adds live source assertions
```

The networked test asserts **shape, never a market value**, so a moving market cannot fail the
suite.

---

## The indicators

Everything lives in `config/indicators.toml`, committed so the model's opinions are auditable.
Change a weight or an anchor and you have changed the model's opinion — commit it and say why.

| Indicator | Weight | What it measures | Source |
|---|---|---|---|
| `capex_vs_cashflow` | 14 | Cohort TTM capex ÷ TTM operating cash flow | SEC EDGAR |
| `valuation_stretch` | 14 | S&P 500 deviation from its 5-year log-linear trend | Yahoo |
| `concentration` | 12 | Cap-weight (SPY) vs equal-weight (RSP) 12-month return gap | Yahoo |
| `credit_hy` | 12 | US high-yield option-adjusted spread | FRED *(optional)* |
| `breadth` | 10 | RSP/SPY ratio vs its own 200-day average | Yahoo |
| `issuance` | 10 | Trailing share-count change across the cohort | SEC EDGAR |
| `volatility` | 8 | VIX level | Yahoo |
| `funding_gap` | 8 | Cohort TTM capex ÷ TTM revenue | SEC EDGAR |
| `credit_ig` | 6 | US investment-grade option-adjusted spread | FRED *(optional)* |
| `leverage` | 6 | Cohort long-term debt ÷ TTM operating cash flow | SEC EDGAR |
| `foreign_interest` | 0 | Foreign ownership of US equities | **declared gap** |

Mapped to the eight bubble indicators published by Capital Economics (CNBC, 2026-09-10) plus the
credit and fundamental signals that dominate the current debate.

### Scoring math

Stress is a piecewise-linear interpolation over anchors declared in the config
(`[[raw_value, stress], ...]`), clamped flat beyond the outermost anchor so no single input can
dominate:

```
coverage  = Σ weight(available) / Σ weight(all)
composite = Σ (weight × stress)(available) / Σ weight(available)
```

Renormalizing over *available* weight means a dead source **shrinks coverage** rather than dragging
the score toward an arbitrary midpoint. The reported `confidence` falls out of coverage.

**The renormalization has a cost, and the tool states it:** runs with different coverage are not
comparable at the decimal level. When coverage is imperfect the report emits an explicit
`COMPARABILITY` caveat, and when the credit indicators are missing it emits an `EQUITY-PRICE BIAS`
caveat, because what remains is the sentiment half of the picture rather than the financing half.

---

## Data sources and their real behaviour

All of the following was established by probing the live endpoints from this host, not assumed.

| Source | Status | Notes |
|---|---|---|
| Yahoo Finance chart API | works | Needs a browser-like UA. **Returns HTTP 429 under load** — the client throttles to ~3 req/s with backoff. |
| SEC EDGAR XBRL `companyconcept` | works | **Requires a descriptive User-Agent.** Rate limit ~10 req/s. |
| FRED `fredgraph.csv` | **optional, often down** | Began refusing this host entirely (HTTP/2 `INTERNAL_ERROR`, then read timeouts) after a burst of requests. May recover. Also serves only a ~3-year trailing window regardless of `cosd`/`coed`. |
| multpl.com (Shiller CAPE) | **not usable** | Now JavaScript-gated; cannot be scraped over plain HTTP. |

The tool is designed to run end-to-end on **Yahoo + EDGAR alone**; FRED only ever adds coverage.

### Three traps found the hard way, each now handled

1. **Stale XBRL tags.** `Revenues` for MSFT returns 2010 data — the company migrated to
   `RevenueFromContractWithCustomerExcludingAssessedTax` years ago and kept publishing the old
   history. A naive "first tag that answers" fallback silently returns a 16-year-old number. The
   resolver fetches **every** candidate tag and keeps the one whose most recent fact is newest.

2. **Missing XBRL tags.** `dei:EntityCommonStockSharesOutstanding` 404s for GOOGL and META, which
   report `us-gaap:CommonStockSharesOutstanding` instead. Resolving one tag silently dropped two of
   five filers. Both taxonomies are now resolved together.

3. **A 404 is not an outage.** EDGAR returns 404 for any concept a filer does not use. Treating that
   as a host failure tripped the circuit breaker and skipped every remaining EDGAR request. A 404 is
   now a distinct, non-retried, breaker-neutral outcome meaning "absent".

**Optional sources cannot stall the run.** A per-host circuit breaker means a dead FRED costs seconds
(one attempt, then the remaining series fail fast) rather than minutes of sequential timeouts.

---

## Proxies, stated plainly

Several indicators are proxies for the ideal measure. Each one names its own limitation in its output
row, and the report carries a `PROXIES IN USE` caveat:

- **Valuation** is *price stretch versus its own trend*, not an equity risk premium — no free EPS
  series exists. The 10-year yield is reported as context but deliberately not scored.
- **Concentration** and **breadth** use cap-weight versus equal-weight returns, not a free-float
  market-cap share or an advance/decline line.
- **Issuance** uses reported share counts — net of buybacks, and blind to IPOs outside the cohort, so
  it understates a genuine supply wave. Treated as a lower bound.
- **Leverage** uses reported balance-sheet debt only, so it excludes the large off-balance-sheet
  lease and purchase commitments. True leverage is higher than the ratio suggests.

---

## Project layout

```
src/
  score.rs          pure: anchor interpolation, weighted composite, coverage
  phase.rs          pure: phase classification + historical-analog overlay
  config.rs         config load + validation (rejects inconsistent configs)
  model.rs          types; every scored value carries Provenance
  http.rs           shared client: UA, throttle, retry, circuit breaker, cache
  sources/          yahoo.rs, edgar.rs, fred.rs  (the only modules that do IO)
  indicators/       market.rs, credit.rs, fundamentals.rs
  report/           json.rs, html.rs
config/indicators.toml   all weights, anchors, thresholds + their rationale
tests/                   fixture-driven, offline; live assertions are opt-in
```

The scoring core is a pure function of `(observations, config) -> Report`: no clock, no randomness,
no IO. Identical inputs produce byte-identical output, and a test proves it.

---

## Extending it

- **Change the model's opinion:** edit `config/indicators.toml`. The loader rejects non-monotone
  anchors, out-of-range stress values, descending phase thresholds and duplicate ids, so a typo
  cannot silently change the score.
- **Add an indicator:** implement `Indicator` in `src/indicators/`, register it in `find()`, and add
  it to `IMPLEMENTED`. A weighted indicator in the config with no implementation is reported as an
  explicit *bug*, not a data gap — never as a silent omission.
- **Schedule it:** the CLI is stateless and exits non-zero on degraded data, so a cron job can call
  `bubble-watch report` and alert on coverage or phase changes.

## License

MIT.
