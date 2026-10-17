# Pre-registered lead/lag gate — `leveraged_builder_stress` and `buildout_opposition_news`

**Registered:** 2026-10-17, BEFORE any result was computed.
**Status of both channels: UNSCORED. The gate FAILS. Neither acquires weight.**

This document exists so that the test is fixed before the history accumulates, and so that a later
reader cannot quietly lower the bar to make a promising-looking series scoreable. The sibling sentiment
channel (`gdelt_tone`) was put through the same discipline on 2026-09-26 and failed (peak at lag +7,
p = 0.36) — its failure is the reason the gate is written down rather than remembered.

---

## The gate

An indicator may carry weight only if it **LEADS** the thing it claims to warn about. A bubble
indicator that lags price is worse than useless: it adds weight to a trailing number.

| element | specification |
|---|---|
| **Hypothesis (directional, stated in advance)** | More stress in the debt-funded AI builders precedes WEAKER subsequent broad-market returns. So the correlation between builder drawdown and forward return must be **negative**. |
| **Predictor** | `leveraged_builder_stress` — equal-weighted drawdown from each builder's own 52-week high, exactly as the indicator computes it. |
| **Target** | Forward N-day SPY return, N ∈ {5, 10, 20, 40} trading days. |
| **Statistic** | `max |r|` over the four horizons. |
| **Null** | 2,000 shuffles of the predictor in contiguous 20-day BLOCKS, so the predictor keeps its own persistence. An i.i.d. shuffle would destroy that and produce an anti-conservative null — a mistake I made earlier the same day and caught only by re-running. |
| **Pass condition** | observed `max |r|` above the null **p90**, AND the sign stable across non-overlapping block-grid offsets. |
| **Power requirement** | at least ~85 independent observations for a true r = 0.30 at 80% power, α = 0.05. |

---

## First pass, 2026-10-17

**Data:** 251 aligned trading days (2025-10-17 → 2026-10-16), the six builders plus SPY, from the
tool's own cache. The GDELT `withdrawn` series could not be evaluated — it was never cached, because
my own validation probing had already rate-limited GDELT when the run went out.

### 1. Overlapping windows — the seductive answer

```
horizon   5d: r = -0.106
horizon  10d: r = -0.139
horizon  20d: r = -0.170
horizon  40d: r = -0.214
```

Negative at every horizon, growing with horizon — exactly the pre-registered direction. Taken alone
this looks like a signal.

### 2. Grid-offset stability — it does not survive

Non-overlapping blocks, horizon 20d, shifting the block grid by 4 days at a time:

```
offset  0: n=12  r = +0.065
offset  4: n=12  r = -0.332
offset  8: n=12  r = -0.112
offset 12: n=11  r = -0.249
offset 16: n=11  r = +0.076

spread: -0.332 .. +0.076     positive in 2 of 5 offsets
```

**The offset is arbitrary. The sign flips.** There is no stable relationship.

### 3. Empirical null — the observed statistic is weaker than chance

```
observed max|r| = 0.214
null p50 = 0.254   p90 = 0.430   p95 = 0.477
p = 0.641
```

The observed statistic sits **below the median of the null distribution**. Shuffled versions of the
same predictor produce a larger `max|r|` more often than not. This is not a near-miss requiring more
data; it is no evidence of a relationship at all.

### 4. Power — the sample could not have decided this anyway

```
to detect true r = 0.20 at 80% power  ->  n ≈ 194 independent observations
to detect true r = 0.30 at 80% power  ->  n ≈  85
to detect true r = 0.40 at 80% power  ->  n ≈  47

independent 20-day blocks available:  251 / 20 = 12
```

**The gate needs ~85. It has 12.**

---

## Verdict

**Both channels stay unscored.** Two independent reasons, either sufficient:

1. The point estimate is indistinguishable from noise — and specifically *weaker than* the null
   median, with a sign that flips on an arbitrary grid choice.
2. The sample is roughly **one seventh** of the size needed to detect even a strong effect.

This is not a result that flips with more days. At 20-day horizons it needs roughly **85 blocks ≈ 7
years** of daily history, or ~3.5 years if the true effect is a large r = 0.40. Stating that plainly is
the point: nobody should wait for this gate expecting it to pass next month.

## What the channels are still for

They remain valuable as **measurements**, not as inputs, and that is now visible where it matters:

- `leveraged_builder_stress` reads **−56.7%** while the scored `credit_hy` reads a calm **3.23%**. That
  contradiction is the finding, and it is now rendered on the dashboard in its own
  "Reported but NOT counted" card and in the full report at `/latest.html`.
- A reader can act on a contradiction they can see. Nobody can act on a channel that is invisible or on
  one that has been quietly given weight to make a narrative look scored.

## What would change the verdict

- **More history.** ~7 years of daily data for the builders channel.
- **A better target or predictor.** The target here is SPY, which is dominated by non-AI names; a
  narrower AI-exposed target might behave differently. Any such change must be re-registered BEFORE it
  is run, and the old result kept alongside it.
- **A credit-spread series rather than equity drawdowns**, which is closer to the source. No free
  keyless source exists for individual CDS, which is why the equity proxy was used and stated.

**None of these is a reason to score the channels now.**
