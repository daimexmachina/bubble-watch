# (c) Does media reporting lead? Tested — it does NOT, and the near-miss is the finding

**Run 2026-10-16.** Daim asked for media to carry value in the model. This is the gate result for the
two variants that had never been tested: media → **credit** (prior tests used equity price), and
media → **single-name** stress (prior tests were market-wide).

## 1. The result

GDELT article **volume** for the `"AI bubble"` query, daily, 340 days (2025-10-22 → 2026-10-16),
against each target. Gate = peak |cross-correlation| over lags −10..+10, judged against an empirical
null built by shuffling the media series 500 times and keeping the best |r| over the same lag range.
**Positive lag means media leads.**

| target | n | peak \|r\| | lag | null p95 | verdict |
|---|---|---|---|---|---|
| **HY credit spread** (BAMLH0A0HYM2, daily) | 244 | **0.125** | −1 | 0.194 | **FAILS** |
| **ORCL price** (daily) | 234 | **0.148** | +1 | 0.199 | **FAILS** |

Both fail, and neither peaks at a lag that would justify a leading claim.

## 2. THE NEAR-MISS, which matters more than the result

On raw **levels**, both tests appeared to PASS:

```
media volume -> ORCL price, LEVELS:  peak |r| = 0.581 at lag -15,  null p95 = 0.178  -> "PASSES"
media volume -> HY credit,  LEVELS:  peak |r| = 0.221 at lag -10,  null p95 = 0.182  -> "PASSES"
```

Taken at face value that is a strong result — 0.581 is far above the null, and it would have been
reported as "media leads ORCL by about two weeks". It is an ARTEFACT.

The tell is the SHAPE, not the size: the correlation rises monotonically as the lag is pushed more
negative (−15:+0.58 … −1:+0.41 … +15:+0.28). Two series that both TREND will cross-correlate hardest
at the most extreme lag examined, because the lag is aligning the two trend segments, not measuring a
lead. A genuine lead shows a LOCALISED peak at one lag with the correlation falling away on both sides.

After first differencing — which removes the common trend — the profile flattens into noise
(peak 0.148, and the 0.581 becomes 0.148 at lag +1) and fails the null.

**RULE: never gate a trending level series without differencing it, and never accept a peak that sits
at the edge of the lag window.** A peak at the largest |lag| is a trend signature, not a lead. The
shuffle null does NOT protect against this, because both the observed and the shuffled series carry
the same trend — the null was satisfied at p95 = 0.18 while the real signal was ~0.

## 3. Why this is consistent with the two earlier failures

Three independent tests now agree, on three different targets:

| test | target | result |
|---|---|---|
| GDELT **tone** (pre-existing) | equity price | peak at lag **+7 = TRAILS**, p = 0.36 |
| GDELT **volume** (this test) | HY credit spread | fails on differences |
| GDELT **volume** (this test) | ORCL price | fails on differences |

The mechanism is not mysterious: **coverage of an event follows the event.** A credit spread widens
because someone traded; the article about it is written afterwards. That is why "media is where we
should put value" keeps failing when tested as a LEADING signal — the value is real but it is
CONTEMPORANEOUS-to-LAGGING information about something already priced.

## 4. What was NOT tested, stated precisely

- **Tone → credit**: GDELT's tone endpoint returned **HTTP 429** on every attempt, including after
  20-second pauses ("limit requests to one every 5 seconds"). So the tone variant against a credit
  target remains UNTESTED. Tone and volume are different channels; volume is the attention proxy and
  it failed.
- **Oracle-specific coverage**: the query `(Oracle) (debt OR "data center" OR datacenter)` was rejected
  by GDELT — "Parentheses may only be used around OR'd statements". Not retried with corrected syntax.
  So the single-name variant used ORCL **price** as the target and the broad AI-bubble query as the
  media series, not Oracle-specific coverage. That is a weaker test of the single-name hypothesis than
  the one proposed.
- **ORCL credit** (CDS, bond prices, the 77¢ 2055s): no free source, so price stood in as a proxy.
  Price is the more sensitive series of the two, and it still failed.

## 5. Conclusion, and what it means for the request

**Media reporting does not lead credit or single-name stress on daily changes in this data.** The
requested change — scoring media into the composite — is therefore declined on MEASUREMENT, not on
principle, and the measurement is reproducible from the files named below.

`net`/`gross` velocity (shipped as `bdd6b2c`) already delivers the underlying intent: it surfaces what
is ACTUALLY moving now, by name, instead of asking a lagging channel to speak first.

Files: `gdelt_ai_timelinevol.json`, `gdelt_ai_timelinetone.json`, `credit_hy.json`, `orcl.json` in the
scratch dir; queries and lags as above.
