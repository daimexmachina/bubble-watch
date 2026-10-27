# China/Asia equity reachability — can the model measure it, and does the story check out?

**Tested:** 2026-10-27, in response to two Reuters-derived articles ("In China, the AI bubble has
already burst", 2026-10-26) supplied by Daim. Every figure below was measured through the tool's own
`Fetcher`, or from a public endpoint probed directly. Nothing here is taken from the articles except
where explicitly marked as their claim.

## The question

The model is **96% US-sourced** (152/158 weight, measured 2026-09-20) and has **no Chinese or Asian
equity coverage at all** — `grep` for china / chinext / star50 / csi300 / hang seng / h-share /
shanghai / shenzhen returns zero matches in `src/sources/` and `config/indicators.toml`. Before
specifying a channel, find out whether the existing keyless path can even reach the data.

## Reachability — measured, 8 + 4 symbols

| symbol | result |
|---|---|
| `%5EIXIC` Nasdaq | **full history** — 251 daily bars |
| `%5EHSI` Hang Seng | **full history** — 245 bars |
| `000001.SS` Shanghai Composite | **full history** — 242 bars |
| `000688.SS` STAR50 | **ONE point only** — at range `1y`, `2y` AND `max` |
| `000300.SS` CSI300 | **ONE point only** — same, all ranges |
| `399006.SZ` / `399102.SZ` ChiNext | **ONE point only** |
| `%5EHSTECH` Hang Seng Tech | 404 not found |
| `sh000300` / `sh000688` (alt formats) | 404 not found |

**Why this matters:** Yahoo will give a *snapshot* of the mainland AI indices but **no history** at any
range, so a drawdown or a trend cannot be computed from it. A snapshot has no memory, and the whole
point of the channel would be the drawdown.

## The route that does work: US-listed vehicles

Yahoo has full history for the US-listed China vehicles, and they need no new transport, no key and no
mainland feed:

```
KWEB   China internet        251 bars
CQQQ   China technology      251 bars
FXI    China large-cap       251 bars
MCHI   China broad           251 bars
```

An alternative mainland feed (Eastmoney `push2his`) returned **400 daily bars once** for both STAR50
and CSI300 — the last values agreed exactly with Yahoo's snapshots — and then **dropped every
subsequent connection**, including on four retries with a Referer. It is not dependable for a daily
job, and hammering it to find out would be the same mistake that starved CourtListener earlier.

## Does the article's claim hold? — corroborated, independently

Measured from US-listed vehicles through the tool's own path, 1-year daily, as of 2026-10-27:

| vehicle | last | peak (date) | from peak | from July-onward high |
|---|---|---|---|---|
| KWEB China internet | 23.13 | 38.80 (2025-10-29) | **−40.4%** | −20.6% |
| CQQQ China tech | 39.49 | 58.89 (2026-01-12) | −32.9% | −30.9% |
| FXI China broad | 32.50 | 40.48 (2026-01-29) | −19.7% | −11.0% |
| SPY US large-cap | 769.66 | 779.09 (2026-10-06) | **−1.2%** | −1.2% |

The articles claim STAR50 −40% and CSI300 −20% from their highs while the US has only wobbled. The
independent US-listed proxies land at **−40.4%** (China internet) and **−19.7%** (China large-cap)
against **−1.2%** for the US. **The claim is corroborated.** Note one discrepancy: the articles frame
the 40% as "less than four months from a July high", but KWEB's peak is a year old and its fall from
the July-onward high is 20.6%; CQQQ is 30.9% off its July high. The magnitude is right; the window is
looser than stated.

## The measure worth building — relative, not absolute

Absolute China drawdown is contaminated: Morgan Stanley attributes part of the slide to **trust taxes,
a weakening economy and global volatility**, not to AI repricing. The fix is the trick `breadth` and
`concentration` already use — divide by the broad market, which cancels the macro and policy component:

| ratio | now | peak (date) | from ratio peak |
|---|---|---|---|
| CQQQ / FXI | 1.2151 | 1.8053 (2026-06-30) | **−32.7%** |
| KWEB / FXI | 0.7117 | 0.9658 (2025-10-31) | −26.3% |

**China tech has de-rated ~33% relative to the broad Chinese market since June.** That is a real,
AI-specific repricing with the macro stripped out, and it is the version of this story the model
cannot currently express: its market-pricing indicators are all US aggregates.

## Recommendation

Buildable at **weight 0**, using `KWEB`/`CQQQ` against `FXI` — no new source module, no key, the
`yahoo.rs` path already in place. It would be the model's first non-US market-pricing measure.

Before it carries weight it needs the same pre-registered lead/lag gate as the other unscored
channels, and the honest prior is poor: the question is whether a Chinese tech de-rating LEADS
AI-economy fragility anywhere the model scores, and one year of daily data cannot answer that. Expect
UNPROVEN, declared, and visible — which is still better than the present state, where the model says
nothing about a market that has fallen 40%.

## Caveats, stated

1. US-listed ETFs are not the mainland indices: KWEB/CQQQ/FXI carry ADR, FX and US-sentiment exposure,
   and their peaks do not coincide with STAR50's July high.
2. One year of history only — no 2021-22 China tech bust in the window, which would be the natural
   comparison.
3. `SPY −1.2%` is a point-in-time reading; the articles' "Nasdaq −6% from a record" could not be
   checked from a cached series and was not asserted here.
