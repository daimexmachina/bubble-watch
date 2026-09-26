# Does AI news sentiment LEAD the market, or follow it?

**Task 7 of the v1.4 plan — the gate on scoring sentiment.** Measured 2026-09-26.

## Decision, stated first

**Sentiment stays UNSCORED. Permanently, on this evidence.**

The pre-registered rule was: if tone leads price at a peak meaningfully above the noise
floor it is eligible for scoring; if it follows, or shows no distinguishable peak, it stays
documented context forever. It does not lead.

## What was tested

| element | detail |
|---|---|
| Tone series | GDELT `timelinetone` for `"AI bubble"`, 169 daily points, 2026-03-31 → 2026-09-25 |
| Price series | `SPY`, `^GSPC`, `RSP`, `^VIX` daily bars from the tool's own cache |
| Overlap | 117 common days → 116 daily log returns |
| Method | Pearson correlation of tone(t) against return(t+k) for k = −10 … +10 |
| Reading | k < 0 = tone LEADS price; k > 0 = tone FOLLOWS price |

## Result: no lead, and the apparent peaks do not survive a proper null

Every index peaked at **positive k** — i.e. tone *follows* price:

| index | peak lag | r | sign meaning |
|---|---|---|---|
| SPY | **k = +7** | +0.221 | tone turns hostile ~7 trading days *after* a price rise |
| ^GSPC | **k = +7** | +0.245 | same |
| ^VIX | **k = +7** | −0.255 | same |
| RSP | **k = +8** | +0.194 | same |

No index showed its maximum at a negative lag. A leading indicator would peak there.

## The methodological point that decides it

A naive test compares the peak |r| against a nominal threshold of `1.96/√n ≈ 0.18` at
n = 116. By that standard the peak (|r| = 0.221) clears the bar and one might score it.

**That test is wrong, and this is the trap worth recording.** We searched 21 lags and took
the **maximum**, so the peak is a maximum of 21 draws and its expected size under the null
is far larger than a single-lag threshold suggests. The correct null is **empirical**:
shuffle the tone series against returns 2,000 times and take the distribution of the *peak*
|r| each time.

| | value |
|---|---|
| observed peak \|r\| | **0.221** |
| null peak \|r\|, p90 | 0.264 |
| null peak \|r\|, p95 | 0.285 |
| null peak \|r\|, p99 | 0.328 |
| fraction of shuffles ≥ observed | **0.356** |

**So the observed peak is smaller than 90% of pure-chance peaks, and p ≈ 0.36.** It is not
merely insignificant — it is *unremarkable*. Searching 21 lags and reporting the best one,
against a single-lag threshold, would have manufactured a finding here.

Two supporting checks:

- **Serial correlation:** tone's lag-1 autocorrelation is **+0.079**, so the series is
  roughly independent and the effective sample is close to the nominal n. This is the one
  condition that *favours* a finding, and it still does not produce one.
- **Sign consistency:** the four independent price series agree on the lag and the
  direction, which looks like coherence. But they are highly correlated with each other, so
  this is **one observation measured four times**, not four confirmations. Recorded here so
  that the agreement is not later mistaken for independent replication.

## Why this is the right outcome

The temptation was real: tone hit the **1.2th percentile of six months** on 2026-09-25, the
most hostile reading in the window, at the same time as the news you flagged. A model that
scored it would have shown a dramatic spike exactly when a human noticed one — which is
precisely how a lagging attention series gets dressed up as a leading risk signal.

The honest reading: **tone follows price, and even that weak follow is indistinguishable
from chance at n≈116.** Scoring it would add noise with a narrative attached.

## What is shipped instead

- **`opposition_pressure` (scored, weight 8)** — filed court dockets, a recorded event
  rather than an opinion.
- **GDELT volume and tone (fetched, reported, unscored)** — shown as context, labelled as
  context, exactly as `trend` is.
- The tone series is retained in `Observations` so the correlation can be **re-run as the
  history lengthens**. At n≈116 the power to detect a real lead of r≈0.2 is poor; this is a
  verdict on the evidence available, not a permanent claim about the world.

## Limits, stated

- **Window length.** 116 returns is short. A genuine lead of |r| ≈ 0.2 would be detectible
  only sometimes at this n. This test cannot prove tone is useless; it can only say the
  evidence does not support scoring it, which is the decision rule.
- **One tone query.** `"AI bubble"` was chosen as the most direct phrasing. A different
  query might behave differently, but each additional query adds another multiple-comparison
  problem, so widening the search would require a stricter threshold, not a looser one.
- **Returns, not levels.** Deliberately: a correlation of tone against price *levels* would
  be dominated by any common trend and would show a large spurious r.
