# Does the FINANCIER side lead the AI/tech complex?

**The lead/lag gate for the counterparty channel.** Measured 2026-09-26.
Method is identical to Task 7's tone gate (`.hermes/analysis/2026-09-26-sentiment-lead-lag.md`),
so the two verdicts are directly comparable.

## Decision, stated first

**The financier channel does NOT lead. It moves at the SAME TIME — k = 0 — and that is the
decisive answer.**

It is therefore **not eligible for scoring**, and unlike the tone series this is not a
marginal call: there is no lag structure at all to argue about. The strongest negative lag
is 90–99% weaker than the same-day correlation in every pair tested.

## Why this gate could finally be run

The earlier attempt was blocked on a source, not a design question. Resolved as follows:

| attempt | result |
|---|---|
| Yahoo `query1` and `query2`, every symbol | **HTTP 429**, still 429 after a 75 s backoff at 1 req/2 s |
| Stooq | HTTP 200 but a **JS proof-of-work challenge page**, not CSV |
| **Nasdaq `api.nasdaq.com/api/quote/{sym}/historical`** | **HTTP 200, keyless** — the workaround |
| Control probes, same minute | FRED 200, EDGAR 200 → so the Yahoo failure was IP-level rate-limiting |

Ten series fetched from Nasdaq, 1,944 daily bars each (1,452 for OWL, which listed later),
covering 2019-01-01 → 2026-09-26. Ledger: `2026-09-26-bars/ledger.jsonl`.

## The test

- **A** = financier/counterparty log returns, **B** = tech/market log returns
- Pearson `corr(A(t), B(t+k))` for **k = −20 … +20**; **k < 0 means A LEADS B**
- Null is **empirical** — 2,000 shuffles, distribution of the **peak |r|** — because taking
  the maximum over 41 lags inflates the peak under the null. A single-lag threshold would
  manufacture a finding, which is exactly how the tone series would have been wrongly passed.

## Result: every pair peaks at k = 0. Nothing leads.

| pair | peak lag | r | P(chance peak ≥ observed) | verdict |
|---|---|---|---|---|
| OWL → SPY | **+0** | +0.562 | 0.000 | not eligible |
| OWL → QQQ | **+0** | +0.519 | 0.000 | not eligible |
| OWL → NVDA | **+0** | +0.390 | 0.000 | not eligible |
| DLR → SPY | **+0** | +0.532 | 0.000 | not eligible |
| EQIX → QQQ | **+0** | +0.529 | 0.000 | not eligible |
| ARCC → SPY | **+0** | +0.573 | 0.000 | not eligible |
| BX → SPY | **+0** | +0.727 | 0.000 | not eligible |
| NVDA → SPY *(control)* | **+0** | +0.699 | 0.000 | not eligible |
| MSFT → SPY *(control)* | **+0** | +0.756 | 0.000 | not eligible |
| ORCL → SPY *(control)* | **+0** | +0.534 | 0.000 | not eligible |

**Read the controls carefully, because they are the interpretation.** The tech controls peak
at k = 0 with *higher* correlations than the financiers. The financier series are simply
**more volatile beta of the same market factor**, not a distinct leading signal. The p = 0.000
is not evidence of a lead — it is evidence of co-movement, which is not what was being tested.

## Robustness: the k = 0 peak is NOT a plateau artefact

The obvious objection is that picking the max off a flat curve would land on k = 0 by
accident. The curves are the opposite of flat:

```
OWL -> SPY    k :  -10   -9   -8   -7   -6   -5   -4   -3   -2   -1   +0   +1   +2  ...
              r : +0.06 +0.05 -0.00 -0.04 +0.02 +0.00 -0.02 -0.03 -0.03 +0.03 +0.56 +0.01 +0.03
```

A **single spike at k = 0** and noise everywhere else. Verified for OWL/SPY, DLR/SPY,
ARCC/SPY and BX/SPY: **no negative lag reaches even 90% of the peak**, and the strongest
negative lag anywhere is |r| = 0.06–0.17 against peaks of 0.53–0.73.

Two further checks, because a daily test alone would be a thin basis for a permanent verdict:

| check | result |
|---|---|
| **Weekly** frequency (non-overlapping 5-day returns, k = −8…+8) | peak still **k = 0** for all six pairs (n = 290–388) |
| **Drawdown levels** (rolling 252-day, not returns) | peak **k = 0**; k = −1 sits 0.6 pp lower — but see the caveat below |

So the conclusion holds at daily and weekly frequency, and at drawdown level it holds for any
lead of **more than one day**: **the counterparty complex and the tech complex reprice
together.**

**One honest qualification on the drawdown leg.** Its k = −1 is only 0.6 pp below its k = 0
(OWL→SPY: 0.318 vs 0.324), and with n = 1,452 that gap is about **0.26 standard errors** — i.e.
**indistinguishable from noise**. So the drawdown result cannot separate a same-day move from a
one-day lead; what it does establish is the absence of a lead *longer* than a day, which is the
claim that matters here and is consistent with the returns legs. Do not quote the drawdown leg as
positive evidence of a same-day peak — quote it as the absence of a multi-day lead. This
correction was made after re-reading the raw output; the earlier text described the 0.6 pp gap as
merely "1–2 pp weaker", which overstated what that leg shows.

## Replication: a second, independent implementation agrees

The headline numbers were produced twice, by two implementations sharing no code path for the
statistics or the shuffling (pure-Python `statistics.fmean` + `random.shuffle` versus numpy +
`RNG.permutation`). Both peak at **k = 0** for every pair they cover, and the correlations agree
to the precision the first run printed:

| pair | peak lag | r (impl 1) | r (impl 2) | n |
|---|---|---|---|---|
| OWL → SPY | +0 / +0 | +0.562 | +0.562 | 1451 |
| OWL → QQQ | +0 / +0 | +0.519 | +0.519 | 1451 |
| OWL → NVDA | +0 / +0 | +0.390 | +0.390 | 1451 |
| DLR → SPY | +0 / +0 | +0.532 | +0.532 | 1943 |
| EQIX → QQQ | +0 / +0 | +0.529 | +0.529 | 1943 |
| ARCC → SPY | +0 / +0 | +0.573 | +0.573 | 1943 |

Also agreeing: `n` exactly, and the null's p90 to 0.001 or better on every shared pair.

Two caveats, stated rather than glossed. The second implementation covered **6 of the 10 pairs**
before it was terminated — it was the slow version superseded by the vectorized one — and the
shuffle nulls are **not identical by construction**, since the two runs draw different permutations
from differently-seeded generators. So this is corroboration of the point estimate and the lag
structure, not a bit-for-bit reproduction. The published artifact remains
`2026-09-26-bars/gate_results.json`, from the vectorized implementation, which covers all ten pairs.

## What this means for the user's story

The story (Oracle force majeure → Project Jupiter → Blue Owl/Stack) reads as though the
financing side is *ahead* of the technology side — "the financing side of the AI buildout is
starting to ask much harder questions than the demand side."

**This test does not support a timing claim.** What it shows is that when the market
reprices this complex, the lessors, the private-credit funds and the tech names reprice
**at the same time**. An indicator built on counterparty prices would therefore add
**no lead** to what the model already reads from the equity indices — it would be the same
information, noisier, with a narrative attached.

The relevant confirmation, from the same data: **OWL closed at $9.32 on 2026-09-25 against
$18.44 on 2025-09-02 — roughly halved.** The counterparty distress is real and large. It is
simply not *early*.

## Decision recorded

- **`public_attention` stays the model's only weight-0 observed channel.** The financier
  series is NOT added, at any weight. Adding it unscored would be defensible but pointless:
  unlike Wikipedia pageviews it answers no question the model cannot already answer from
  indices it already holds, and unlike the tone series it has no independent content.
- **The blind spot stays declared.** `leverage`'s output text already states that it excludes
  off-balance-sheet vehicles, and `backlog_quality`'s states it cannot see counterparty credit
  quality. Those declarations are now backed by a measured reason rather than an assumption.
- **If this is ever revisited, the test is pre-registered**: the same k = −20…+20 curve with
  an empirical shuffle null, at least two frequencies, and a control series that must not
  beat the candidate.

## Limits, stated

- **The gate tests TIMING, not INFORMATION.** A counterparty series that leads nothing could
  still be worth *reporting* as context — it is simply not scoreable, and it is not a signal
  about what comes next. This document says nothing about whether OWL's 50% drawdown is
  *informative about the level* of stress; only about whether it arrives earlier.
- **Daily bars cannot resolve sub-day dynamics.** If the financiers lead by hours rather than
  days, this test would not see it — and a lead of hours is not useful to a daily indicator.
- **OWL has the shortest history** (1,452 bars, listed 2021) so its pairs span 2021–2026,
  a market that rose almost throughout. The controls are affected the same way, so the
  comparison is fair, but a verdict drawn on a single regime is weaker than one drawn across
  a full cycle.
- **Beta was not removed.** The financier series correlate with SPY at 0.39–0.73, so a
  market-neutralised version might behave differently. That is a *different* test (does the
  idiosyncratic component lead?) and it is noted as the natural follow-up rather than
  implied by this result.
- The Nasdaq endpoint is undocumented. It answered reliably today at 1 request/second, but
  it is not a contract and could change or gate without notice. Re-probe before building.
