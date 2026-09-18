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
Cheap credit and healthy breadth are consistent both with "no bubble" and with "the quiet phase of
one".

### Credit: level vs direction (read this before trusting the credit rows)

The credit-spread anchors are **not inverted** — a wide spread scores as high stress, a tight one as
low. That is a deliberate choice against a plausible alternative.

The alternative reading is that *tight* spreads are themselves the bubble signal, because the market
is charging nothing for the risk. Credit did tighten into the dot-com peak, so that reading has real
precedent. It is not implemented because it fails a practical test: HY OAS has a **~3.0% median**
over the available record and is rarely wide, so a complacency-scored level would label most of the
last decade a bubble — a near-constant that carries **no timing information**.

Measured effect of the alternative on the composite: **32.4 → 44.2**, i.e. **EARLY → LATE**. A
full-phase swing resting on a definition rather than on data. So the level is scored as cost-of-credit
and **direction of travel is the signal to watch**: widening from a tight base is the early warning,
not the level. If you prefer the other reading, mirror the two anchor stress values in
`config/indicators.toml` — the arithmetic is symmetric, and the trade-off is documented in the config.

---

## Install and run

```bash
cargo build --release

./target/release/bubble-watch sources              # probe each data source, report health
./target/release/bubble-watch score                # composite + phases + coverage
./target/release/bubble-watch score --json         # machine-readable
./target/release/bubble-watch explain              # per-indicator math, weights, anchors, provenance
./target/release/bubble-watch report --out out     # writes <name>.json and <name>.html
./target/release/bubble-watch trend                # direction of travel vs the last comparable run
./target/release/bubble-watch site --out site      # self-contained site: dashboard + per-day reports
./target/release/bubble-watch --offline score      # cache only; never touches the network
```

Exit codes: `0` ok · `2` config error · `3` no usable data · `4` partial coverage.

Tests:

```bash
cargo test                                         # 198 tests, no network required
BUBBLE_WATCH_LIVE=1 cargo test -- --nocapture      # adds live source assertions
```

The networked test asserts **shape, never a market value**, so a moving market cannot fail the
suite.

---

## Direction of travel (why level alone is not enough)

The credit rationale above concludes that the informative signal is the **direction of travel**, not
the level. A point-in-time scorer cannot see direction, because nothing persists between runs. So
each run is appended to `data/history/runs.jsonl` and the tool reports a delta.

**The trend is context, not an eleventh indicator.** It is derived *from* the composite, so scoring
it inside the composite would make the score partly a function of its own past. The composite is
byte-identical with and without history, and a test asserts exactly that.

Three rules are enforced rather than merely documented:

1. **Unequal coverage is not comparable, so the comparison is refused.** Because the composite is
   renormalized over available weight (see the math above), a run at 100% coverage and one at 62%
   are not on the same scale — subtracting them mixes a real market move with the effect of which
   sources answered. A baseline is eligible only if its coverage is within
   `coverage_tolerance_pp` (default 5pp). With no eligible baseline, **no delta is emitted** and the
   reason is printed. There is deliberately no "use the previous run anyway" fallback.
2. **No delta is ever shown without its elapsed time.** +4 over 3 days and +4 over 400 days are
   different facts.
3. **A gap is never treated as a zero.** Zero is a legitimate stress reading, so an unavailable
   indicator is *absent* from the archived stresses and omitted from the per-indicator delta rather
   than being compared against a number.

The archive is append-only, so nothing is destroyed; the *displayed* series keeps at most one point
per calendar date, so re-running the tool three times in an afternoon is an audit trail rather than
three days of history. A malformed archive line is reported with its line number and skipped — it
never passes silently, and never destroys the rest of the history.

Tuning lives in the `[trend]` block of `config/indicators.toml`.

---

## Running it daily, and serving it on the LAN

```bash
scripts/daily-refresh.sh      # live fetch, record the run, regenerate the site
scripts/serve.sh              # static server on :8770
```

Both are wired to user-level systemd units (copies in `deploy/`):

```bash
mkdir -p ~/.config/systemd/user
cp deploy/*.service deploy/*.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now bubble-watch-site.service
systemctl --user enable --now bubble-watch-refresh.timer
systemctl --user list-timers bubble-watch-refresh.timer
```

The timer fires at **13:30 local** — after the US market close, so the day's final prices and spreads
are settled — with `Persistent=true` so a day missed to downtime is caught up on boot, and a
randomized delay to avoid piling load onto the Pi.

**Why systemd and not an agent cron turn:** the job is deterministic. Running it under systemd costs
zero model tokens and cannot hallucinate a number, which is the whole point of this tool's design.

The server serves exactly one directory. Path traversal is refused, directory listings are disabled,
no uploads and no CGI; it is **not** exposed to the internet. The site is plain HTML with no
JavaScript and no external resources, so it renders with no network access at all — a report that
needs the network to display breaks exactly when a market event makes it most interesting.

Layout of the generated site:

| Path | Contents |
|---|---|
| `index.html` | landing page |
| `dashboard.html` | the score over time, one row per recorded day, plus biggest per-indicator moves |
| `latest.html` | the newest full report |
| `<date>.html` | the full report for each archived date |
| `series.csv` | the series as plain text, usable without a browser and diffable in git |

A date with no archived report page is rendered as plain text marked *(summary only)*, never as a
link — a dead link would be a false claim that a report exists.

---

## The indicators

Everything lives in `config/indicators.toml`, committed so the model's opinions are auditable.
Change a weight or an anchor and you have changed the model's opinion — commit it and say why.

| Indicator | Weight | What it measures | Source |
|---|---|---|---|
| `capex_vs_cashflow` | 14 | Cohort TTM capex ÷ TTM operating cash flow | SEC EDGAR |
| `valuation_stretch` | 14 | S&P 500 deviation from its 5-year log-linear trend | Yahoo |
| `credit_hy` | 12 | US high-yield option-adjusted spread | FRED *(key)* |
| `issuance` | 10 | Net equity issuance ÷ operating cash flow, from reported cash flows | SEC EDGAR |
| `primary_market_supply` | 9 | SEC S-1 registration count, trailing year | EDGAR full-text |
| `funding_gap` | 8 | Cohort TTM capex ÷ TTM revenue | SEC EDGAR |
| `volatility` | 8 | VIX level | Yahoo |
| `backlog_quality` | 7 | RPO ÷ deferred revenue — is backlog converting to billed cash? | SEC EDGAR |
| `datacenter_construction` | 7 | Data-center construction spending, YoY | Census C30 |
| `concentration` | 6 | Cap-weight vs equal-weight 12-month return gap | Yahoo |
| `credit_ig` | 6 | Baa yield less 10Y Treasury (40 years of history) | FRED *(key)* |
| `leverage` | 6 | Long-term debt **plus operating leases** ÷ operating cash flow | SEC EDGAR |
| `private_credit_growth` | 6 | Private-credit lending, YoY (whole-economy channel) | Fed Z.1 |
| `grid_cancellations` | 6 | Cancelled/postponed generating capacity as share of announced | EIA-860M |
| `breadth` | 5 | Equal-weight/cap-weight ratio vs its 200-day average | Yahoo |
| `foreign_interest` | 5 | Rest-of-world US equity holdings, percentile of own history | Fed Z.1 |
| `narrative_saturation` | 5 | 10-K filings mentioning AI, annualised YoY | EDGAR full-text |
| `circularity` | 0 | **declared gap** — see below | — |

Weight totals 134 across the 17 scored indicators. `circularity` is carried at weight 0 as an
explicitly acknowledged blind spot: no free source relates an equity investment to the revenue it
generates, so it is declared rather than silently dropped.

### Two indicators are deliberately NOT in the composite

- **GSADF explosiveness test** (`src/gsadf.rs`) — a formal hypothesis test on the price series, per
  Phillips-Shi-Yu. A different *kind* of evidence from a hand-anchored judgement, so averaging it in
  would destroy the value of having two methods that can disagree. The live run currently shows
  **composite "mid" while the test is significant at 1%**, and the report presents both without
  reconciling them.
- **Falsification tests** (`src/falsifiers.rs`) — measurements chosen to show the thesis is *wrong*.
  Averaging "evidence for" and "evidence against" into one number would merge opposite meanings, so
  they are reported beside the score. Currently **2 of 3 read against** the bubble thesis.

Both exist because a model that can only accumulate confirming evidence is not an instrument. The
composite rose 30.6 → 40.5 during development, largely by *adding indicators that scored high* —
which is exactly the bias these two panels are for.

### Per-company exposure

`src/exposure.rs` ranks who is most exposed and who is tested first, from debt, operating leases,
unconditional purchase obligations and RPO relative to cash generation. CONTEXT ONLY — company-level
analysis never enters a market-level score. Every absent value renders as **"not disclosed"**, never
as `0.00`: MSFT reports no purchase-obligation concept at all, while AMZN discloses one whose latest
figure is 810 days old, and those are different facts.

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
| FRED `fredgraph.csv` | **optional, intermittently flaky** | Failure is **transient and time-varying**, not series-specific: measured from this host it served `DGS10` in 0.13 s, then refused *every* series minutes later, then recovered. Treated as optional; failures become reported gaps. |
| FRED `api.stlouisfed.org` (keyed) | key optional, host fine | A **different host** which answered 5/5 in ~0.15 s while the CSV host was timing out. A free key routes through it and additionally returns full history. |
| multpl.com (Shiller CAPE) | **not usable** | Now JavaScript-gated; cannot be scraped over plain HTTP. |
| Fed Z.1 Financial Accounts | works | 8MB zip of 307 CSVs, no key. **FRED does NOT mirror these series** — the `FL…` ids 400 as "does not exist", and FRED's own "private credit" search returns BIS *total credit*, a different concept. Read directly. |
| Census C30 construction | works | xlsx only; no CSV and no keyless API (EITS returns "Missing Key"), and FRED has the aggregate but not the data-center line item. |
| EIA-860M generator inventory | works | xlsx only (14MB); `api.eia.gov/v2` needs a key and none is configured. Sheets located by name. |
| EDGAR full-text search | works | A **census** of filings, not a sample, which is why it is usable for hype when news/search measures are not. Needs a descriptive UA. |

The tool is designed to run end-to-end on **Yahoo + EDGAR alone**; FRED only ever adds coverage.

### FRED key — how to get one

1. Register: **https://fredaccount.stlouisfed.org/login/secure/** → *Register* (free, no institutional affiliation).
2. Request: **https://fredaccount.stlouisfed.org/apikeys** — this page **redirects to login unless you already hold a session**, so sign in first.
3. The key is **exactly 32 lowercase alphanumeric characters**; the API enforces this and rejects anything else. FRED asks that each application use its own key.
4. Put it in `~/.hermes/.env` as `FRED_API_KEY=...` and pass it via the environment — never in the repo.

With a key set, requests go through the keyed API v2 host and fall back to the anonymous CSV; coverage rises from 82% to 100% by enabling `credit_hy` and `credit_ig`.

**Scheduled outage:** FRED / ALFRED / FRED Account are down **2026-09-19, 08:30–10:00 CT** for maintenance. Key retrieval will fail in that window — expected, not a fault.

**Keys are never logged.** A key in a query string is masked centrally before any URL is logged, stored as provenance, or embedded in an error string, and a test asserts a dummy key reaches no output path.

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
  history.rs        pure trend logic + the append-only run archive
  gsadf.rs          pure GSADF explosiveness test (Phillips-Shi-Yu)
  falsifiers.rs     pure: measurements that could show the thesis is WRONG
  exposure.rs       pure: per-company exposure ranking (context only)
  config.rs         config load + validation (rejects inconsistent configs)
  model.rs          types; every scored value carries Provenance
  http.rs           shared client: UA, throttle, retry, circuit breaker, cache
  sources/          the only modules that do IO: yahoo.rs, edgar.rs, fred.rs,
                    fulltext.rs, z1.rs, census.rs, eia.rs
  indicators/       market.rs, credit.rs, fundamentals.rs
  report/           json.rs, html.rs (per-run report + multi-run dashboard), layman.rs
config/indicators.toml   all weights, anchors, thresholds + their rationale,
                         plus a cited BIBLIOGRAPHY block [L1]-[L9]
scripts/                 serve.sh, daily-refresh.sh
deploy/                  systemd units for the timer and the LAN server
data/history/runs.jsonl  append-only archive of recorded runs (git-ignored)
site/                    generated site (git-ignored)
tests/                   fixture-driven, offline; live assertions are opt-in
```

The scoring core is a pure function of `(observations, config) -> Report`: no clock, no randomness,
no IO. Identical inputs produce byte-identical output, and a test proves it. Trend computation is
pure in the same way — the caller loads the archive and passes the timestamp in, so the whole
scoring path stays deterministic and testable offline.

---

## Extending it

- **Change the model's opinion:** edit `config/indicators.toml`. The loader rejects non-monotone
  anchors, out-of-range stress values, descending phase thresholds, duplicate ids and nonsensical
  trend settings, so a typo cannot silently change the score.
- **Add an indicator:** implement `Indicator` in `src/indicators/`, register it in `find()`, and add
  it to `IMPLEMENTED`. A weighted indicator in the config with no implementation is reported as an
  explicit *bug*, not a data gap — never as a silent omission. Nothing else needs changing: the
  trend picks up per-indicator deltas automatically, and an indicator absent from an older archived
  run is omitted from that comparison rather than compared against a zero.
- **Schedule it differently:** `scripts/daily-refresh.sh` is a plain script; point any scheduler at
  it. It propagates the tool's exit codes, so a degraded run shows up as a failed unit rather than a
  silent success.

## License

MIT.
