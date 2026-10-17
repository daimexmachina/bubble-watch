# Verification — independent recomputation of the 2026-10-17 builder / withdrawal lead-lag gate

**Verified:** 2026-10-17, independently of the gate note, from the repo's own cache. No network call was made.
**Subject:** `.hermes/analysis/2026-10-17-lead-lag-gate-builder-and-withdrawal-channels.md` (commit `2dd801f`).
**Verdict: the FAIL SURVIVES verification** — see §6. One wording correction and one reproducibility gap are
recorded, neither of which moves the verdict.

How this note is organised, against the four things a verification is asked for: **§1–§4 recompute (a)–(d);
§5 states every agreement and every difference with the gate note and names the cause; §6 is the explicit
verdict statement; §7 records what was not touched; §8 is the residual uncertainty** and what would change the
verdict. Appendix A is the exact script, and its printed output below it was regenerated from the committed
file to prove the appendix is not decorative.

```
base commit (the gate note's own commit) : 2dd801f85e6311e7f91ed5e520ad2f947ec0a06d
                                          "docs(gate): pre-registered lead/lag gate for the two new
                                           channels — FAILS, both stay unscored"
working tree at verification time          : clean (git status --short empty)
shuffle seed                              : 20261017   (numpy.random.default_rng, PCG64)
shuffles per null                         : 2000
environment                               : Python 3.13.5, numpy 2.2.4, scipy 1.17.1
**second recomputation (§9, added later)  : 6532015 (this note's own commit); Python 3.13.5 stdlib only,
                                            pure-Python Pearson, own Mersenne-Twister shuffle seeds
                                            20261017 / 1 / 7 / 12345 / 987654321 and 5 x 2000 replicate
                                            seeds 555000+k.** §9 states where its numbers differ.
```

**Exact commands run** (from the repository root; `verify.py` is reproduced verbatim in Appendix A, so the
run is re-derivable from this note plus the cache):

```
cd /home/daim/workspace/bubble-watch
git rev-parse HEAD          # -> 2dd801f85e6311e7f91ed5e520ad2f947ec0a06d
git status --short          # empty
python3 verify.py           # prints (a)-(d) for both price fields
git log --oneline -1 -- .hermes/analysis/2026-10-17-verification-of-the-builder-withdrawal-gate.md
```

---

## 1. Input, and how it was read

The input is the tool's own cache, `data/cache/`, read exactly the way the tool reads it:

- the URL for each series is the one `src/sources/mod.rs` (`YAHOO_SPECS`) builds —
  `https://query1.finance.yahoo.com/v8/finance/chart/{SYM}?range=1y&interval=1d` for SPY, CRWV, NBIS, IREN,
  APLD, WULF, CORZ;
- the filename is the one `src/http.rs::cache_path()` computes — FNV-1a 64 of the URL, `{hash:016x}.body`.
  Verified by hand for all seven: e.g. SPY → `3ba3cef87ebbeeb0.body`;
- **the cached bodies are the whole input.** Nothing was fetched.

Result: **251 aligned trading days, 2025-10-17 → 2026-10-16**, identical to the gate note's stated window.
The cache entries for the seven series were last written 2026-10-17 13:47 (the note is committed 11:33; the
13:39 report run reads the same values, see §5.2), so the note's vintage and mine agree in content.

**Method choices, stated because the gate note does not state them** (these are the choices a reviewer should
attack first):

| element | choice made here | why / what it changes |
|---|---|---|
| Predictor value at date *t* | equal-weighted mean over the six builders of `(P_b(t) / max P_b(0..t) − 1) × 100` | "each builder's own 52-week high, exactly as the indicator computes it" (`src/indicators/fundamentals.rs:2805-2808`). With 251 points available a **trailing-252-day** high never truncates, so the expanding-window high and the rolling-52-week high are *the same window* here — the convention cannot move the answer. |
| Price field for the drawdown | `adjclose` in the primary run | This is what the indicator uses (`src/sources/yahoo.rs:39-47` takes `adjclose` first, `quote.close` only as a fallback). **The gate note used `quote.close`** — see §5.1. For the six builders the two fields are bit-identical (they pay no dividends), so this choice does not touch the predictor at all. |
| Target | forward **simple** close-to-close return, `SPY(t+N)/SPY(t) − 1` | log returns give r within ±0.003 (§5.3); the sign and ordering do not change. |
| Alignment | predictor at *t*'s close against the return realised from *t*'s close to *t+N*'s close — no lookahead | alternatives (a one-day gap, a one-day shift of the predictor) move r by ≤0.015 and never toward the pass condition (§5.3). |
| Overlapping correlation | all pairs with a full forward window: n = 246 / 241 / 231 / 211 | the gate note does not print these n, but its four r values reproduce at exactly these counts and at no other window definition I tried (§5.1). |
| Non-overlapping grid | blocks at `offset + 20k`; a block counts only if all 20 forward days fit inside the series | gives n = 12, 12, 12, 11, 11 — the same n the gate note reports, which is what confirms the convention. |
| Shuffle block length | 20 trading days, contiguous, order permuted, block contents untouched | preserves the predictor's own persistence. |
| Trailing partial block | 251 = 12×20 + 11; the trailing 11-day chunk is **shuffled with the rest** in the primary run | the gate note does not say. Holding the remainder fixed instead moves the null up by 0.015–0.022 and puts the note's own p90/p95 outside my values by 3–5 Monte-Carlo standard errors — so the note's "shuffle everything" form is the one consistent with its quantiles (§5.4). |
| Statistic | `max abs(r)` over horizons 5/10/20/40 | as pre-registered. |
| p-value | fraction of the 2000 null statistics `>= observed` | strict `>` and the `(k+1)/(N+1)` correction change p by ≤0.0002. |
| Power | Fisher-z closed form, two-sided α = 0.05, power 0.80: `n = ((z_{0.975}+z_{0.8})/atanh(r))² + 3` | cross-checked by a 10,000-trial Monte-Carlo simulation (§4). |

**The second channel is not testable from this repo, and I verified that it is not.** The GDELT `withdrawn`
series has no cache entry: the computed filename for
`…/doc?query=%22data+center%22+withdrawn&mode=timelinevol&timespan=12m&format=json` is `b06c445a66899c96.body`
and it does not exist in `data/cache/` (the moratorium volume and the "AI bubble" tone series *are* cached, so
this is not a path bug). The tool's own report for 2026-10-17 states the reason in its own words:
*"no GDELT news series for data-centre withdrawals was retrieved…"*, and names the endpoint
`https://api.gdeltproject.org/api/v2/doc/doc+"data center" withdrawn`. **This is a genuine gap and no
workaround was attempted** — see §6.2.

**Cross-check that the input is really the indicator's input.** Recomputing the drawdown on the last cached
date from these bodies gives **−56.7004 %** equal-weighted, per name CRWV −56.13, NBIS −42.28, IREN −64.68,
APLD −63.65, WULF −59.70, CORZ −53.77. The tool's own 2026-10-17 report (`data/history/reports/2026-10-17.html`)
reads **−56.7 %** with "CoreWeave −56 %, Nebius −42 %, IREN −65 %, Applied Digital −64 %, TeraWulf −60 %,
Core Scientific −54 %". Match. My predictor is the indicator's predictor, not a lookalike.

---

## 2. (a) Overlapping windows — recomputed

Predictor vs forward N-day SPY return, all dates with a full forward window. Primary field `adjclose`; the
gate note's field `close` in the second column, for §5.1.

| horizon | n | r (`adjclose`, primary) | r (`close`) | gate note |
|---|---|---|---|---|
| 5 d | 246 | **−0.1096** | −0.1058 | −0.106 |
| 10 d | 241 | **−0.1440** | −0.1387 | −0.139 |
| 20 d | 231 | **−0.1772** | −0.1699 | −0.170 |
| 40 d | 211 | **−0.2122** | −0.2143 | −0.214 |
| **statistic `max abs(r)`** | | **0.2122** | **0.2143** | 0.214 |

Negative at every horizon and growing with horizon, in the pre-registered direction — the gate note's
description is accurate. Two independent implementations agree to ten decimal places at every horizon
(hand-written pure-Python Pearson and `numpy`; `scipy.stats.pearsonr` agrees again on the 20-day case,
r = −0.1772388746, two-sided p = 0.0069, 95 % CI −0.2995 … −0.0493). Alignment spot-checked by hand on five
dates: the 52-week-high dates printed for each name per date are internally consistent (e.g. on 2026-03-13
the highs sit in Oct-2025/Jan-2026/Feb-2026, and the forward 20-day SPY return 657.1572 → 682.6418 = +3.878 %
is the actual close-to-close move over that window).

Note the CI above: it is narrow enough to look like a finding, and it is the wrong interval — it assumes 231
independent pairs. §3 and §4 show why.

---

## 3. (b) Grid-offset stability — recomputed, and it does not survive

Non-overlapping blocks at horizon 20 d, blocking grid shifted 4 trading days at a time:

| offset | n | r, `adjclose` (primary) | r, `close` | gate note |
|---|---|---|---|---|
| 0 | 12 | **+0.0469** | +0.0651 | +0.065 |
| 4 | 12 | **−0.3355** | −0.3318 | −0.332 |
| 8 | 12 | **−0.1202** | −0.1115 | −0.112 |
| 12 | 11 | **−0.2557** | −0.2487 | −0.249 |
| 16 | 11 | **+0.0756** | +0.0758 | +0.076 |
| **spread** | | **−0.3355 … +0.0756 (width 0.411)** | −0.3318 … +0.0758 (0.408) | −0.332 … +0.076 |

**Positive in 2 of the 5 offsets, under both fields.** The sign flips on an arbitrary grid choice, exactly as
the gate note reports. This is not an artefact of my alignment: the offsets are shifted by 4 days and the
same 20-day horizon is used throughout.

Two things this arm can and cannot show, stated so a reviewer does not have to find them:

- **The five grids are not five independent samples.** Any two offsets share all but |Δ| days of every block
  (offsets 0 and 4 share 16 of each 20 days; 0 and 16 share 4), so "positive in 2 of 5" describes **one**
  instability, not five confirmations that there is no relationship. What the arm does establish is exactly
  what the pre-registered condition asks — move an arbitrary choice and the sign moves with it — and it
  establishes it with a **spread (0.41) roughly twice the size of the statistic being judged (0.21)**. Had the
  author picked offset 4, the same data would have reported r = −0.33 at 20 d and looked like a real signal.
- **Offset 0 is not a privileged grid.** It is only the grid at which the longest prefix fits inside the
  series; if anything the choice that flatters the hypothesis most is offset 4, not offset 0. There is no
  "natural" grid here — that is the point of the condition.

---

## 4. (c) Block-shuffle null and (d) power — recomputed

**Null, primary (`adjclose`), seed 20261017, 2000 shuffles of the predictor in contiguous 20-day blocks,
statistic `max abs(r)` over the four horizons:**

```
null p50 = 0.2524   p90 = 0.4374   p95 = 0.4857   mean = 0.2672
observed max|r| = 0.2122
p = P(null >= observed) = 0.6355
```

**The same run on the gate note's field (`close`), which is what §5.1 compares:**

```
null p50 = 0.2539   p90 = 0.4367   p95 = 0.4860   mean = 0.2674
observed max|r| = 0.2143
p = P(null >= observed) = 0.6265
```

The gate note's central claim reproduces: **the observed statistic sits below the median of the null
distribution** (0.212 vs a null p50 of 0.252), and it clears neither p90 nor p95. On these numbers, 62.7 % of
shuffles of the *same predictor* produce a larger `max|r|` than the real one.

**On the statistic's wording, since the hypothesis is directional.** `max abs(r)` discards the sign, so in
principle a pass could be manufactured by a *positive* correlation — the opposite of the pre-registered
direction. That cannot happen here: the maximum sits at the 40-day horizon and it is negative under both
fields (−0.2122 `adjclose`, −0.2143 `close`), so the largest magnitude is also the one in the pre-registered
direction. The two arms of the pass condition therefore agree rather than conflict: the magnitude does not
reach the null p90, *and* the sign does not survive the grid. "Below the null median" is a statement about
`max abs(r)` compared against the null distribution of `max abs(r)` — the same quantity on both sides, which
is the pre-registered statistic; it is not a claim about a signed correlation.

Two checks that the null is doing what it claims:

- **The null is not sensitive to the block length in any way that matters.** Re-running it with 10-, 20-, 40-
  and 60-day blocks (seed 20261017, 2,000 shuffles each) gives p = 0.477 / 0.627 / 0.638 / 0.418 on the
  `close` field and 0.477 / 0.636 / 0.645 / 0.423 on `adjclose`. The pre-registered 20-day block length is
  *not* the friendliest to the hypothesis in the sense that matters — every length rejects it — but one honest
  caveat: at 60 days there are only 5 chunks and the null is coarse, with p50 = 0.196, *below* the observed
  0.214. The p-value there is still 0.42. 20 days is also not matched to the longest horizon (40 d), so the
  null does not fully randomise the target's own overlapping structure; matching the block to 40 d would be
  the stricter test, and it gives p = 0.638. **The pre-registered null is used as registered; these variants
  are reported as sensitivity only, not substituted.**

- **An i.i.d. shuffle would have manufactured a finding here.** Shuffling the predictor point-by-point instead
  of in blocks gives p50 = 0.0827, p90 = 0.1412, p95 = 0.1605, and **p = 0.0055** — i.e. the identical data
  would have "passed" a 1 % test against the wrong null. The gate note's decision to use a block shuffle, and
  its warning that an i.i.d. shuffle is anti-conservative, is **confirmed by direct computation**: the block
  null's p50 is ~3× the i.i.d. null's p50.
- **The overlapping pairs are nowhere near independent.** The predictor's lag-1 autocorrelation is **+0.934**;
  the forward 20-day SPY return's is **+0.947** [§9.4: +0.9467 reproduces only on the unadjusted `close`
  field], with 91 % of consecutive pairs sharing days [§9.4: consecutive 20-day windows share 20 of 21
  dates, i.e. 95 %, not 91 %]. So the 231
  overlapping pairs carry roughly 12 blocks of independent information, not 231. That is the reason the
  scipy CI in §2 is the wrong interval and the reason the empirical null is the right instrument.

**Power (d)** — recomputed by two routes:

| true r | n for 80 % power, α = 0.05 (Fisher z) | simulated power at that n (10,000 trials) |
|---|---|---|
| 0.20 | 193.97 → **194** | 0.803 |
| 0.30 | 84.93 → **85** | 0.805 |
| 0.40 | 46.73 → **47** | 0.815 |

**Independent 20-day blocks available: 251 // 20 = 12.** That count is an *upper* bound on the independent
sample, not a second estimate of it: it is the number of non-overlapping 20-day blocks — the same unit the
grid arm already uses — and §4's persistence check (predictor lag-1 autocorrelation +0.934, forward 20-day
return autocorrelation +0.947) is the reason the overlapping arm's 231 pairs do not add independent
information on top of it. The gate note's claim "the gate needs ~85, it has 12" is correct, as is its
arithmetic for what would be needed: 85 blocks × 20 days = 1,700 trading days ≈ **6.8 years** at 251
days/year; 47 blocks × 20 days = 940 days ≈ **3.7 years** for a large r = 0.40. Both figures in the note
check out.

---

## 5. Where my numbers agree, where they differ, and why

### 5.1 The one systematic difference: `close` vs `adjclose`, and it accounts for all of it

Every one of the gate note's thirteen point statistics (four overlapping r, five grid r, plus its
implied n values) reproduces to within **±0.0005** once the **unadjusted `quote.close`** array is used for SPY
instead of `adjclose`. Under `adjclose` — the field the indicator itself reads — my numbers differ by up to
0.0072. Nothing else was needed: no alignment tweak, no off-by-one, no window change, no seed.

| quantity | gate note | mine, `close` | difference | mine, `adjclose` | difference |
|---|---|---|---|---|---|
| r(5 d) | −0.106 | −0.1058 | +0.0002 | −0.1096 | −0.0036 |
| r(10 d) | −0.139 | −0.1387 | +0.0003 | −0.1440 | −0.0050 |
| r(20 d) | −0.170 | −0.1699 | +0.0001 | −0.1772 | −0.0072 |
| r(40 d) | −0.214 | −0.2143 | −0.0003 | −0.2122 | +0.0018 |
| grid offset 0 | +0.065 | +0.0651 | +0.0001 | +0.0469 | −0.0181 |
| grid offset 4 | −0.332 | −0.3318 | +0.0002 | −0.3355 | −0.0035 |
| grid offset 8 | −0.112 | −0.1115 | +0.0005 | −0.1202 | −0.0082 |
| grid offset 12 | −0.249 | −0.2487 | +0.0003 | −0.2557 | −0.0067 |
| grid offset 16 | +0.076 | +0.0758 | −0.0002 | +0.0756 | −0.0004 |

**Cause, and its materiality.** The gate note says the predictor is measured "exactly as the indicator
computes it" — true, because the six builders pay no dividends and their `close` and `adjclose` arrays are
*bit-identical* in the cached bodies (verified: max |close − adjclose| = 0.000000 for CRWV, NBIS, IREN, APLD,
WULF, CORZ; 7.36 for SPY). The difference is entirely in the **target**: SPY's dividend adjustment over the
window, worth 1.24 pp of total return (14.88 % on `close` vs 16.12 % on `adjclose`). So the gate note's
"SPY return" is an unadjusted price return, while the tool's own convention for a SPY series is the adjusted
one. **Effect on the verdict: none.** It moves r by at most 0.007 — a fifth of the gap to the null p90 — and
every direction of travel is preserved. This is recorded because it is the only thing that had to be
identified for an exact reproduction, and because a reader re-running the gate naively on the cache (as the
indicator would) will not get the gate note's exact decimals.

### 5.2 Where the numbers agree

- The window (251 days, 2025-10-17 → 2026-10-16) and the n values per horizon and per grid offset.
- The sign pattern: negative at all four overlapping horizons, growing with horizon.
- The grid offsets: sign flips, positive in 2 of 5, spread ≈ 0.41.
- The statistic `max |r|` ≈ 0.214 and, decisively, its position **below the null median**.
- The pass condition is not met on either arm: `max|r|` is well below the null p90 (0.214 vs 0.437), and the
  sign is not stable across non-overlapping grids.
- The power shortfall (12 blocks vs 85 needed) and the derived "≈ 7 years / ≈ 3.5 years".
- The live readings used to justify the channels as *measurements*: −56.7 % for `leveraged_builder_stress`
  (recomputed to 4 dp, with all six per-name values matching), and the scored `credit_hy` at 3.23 % (verified
  from the rendered 2026-10-17 report; as-of 2026-10-15, one day behind the builders).
- The sibling claim quoted in the gate note — the sentiment gate's "peak at lag +7, p = 0.36" — matches
  `.hermes/analysis/2026-09-26-sentiment-lead-lag.md` (peak |r| 0.221, "fraction of shuffles ≥ observed" 0.356).

### 5.3 Conventions I tested that do NOT explain anything (all still FAIL)

| variant | r(5 d) / r(20 d) | does it change the verdict? |
|---|---|---|
| predictor at *t*, return *t → t+N* (mine) | −0.1096 / −0.1772 | — |
| predictor at *t*, return *t+1 → t+1+N* (one-day gap) | −0.1203 / −0.1811 | no |
| log returns | −0.1093 / −0.1768 | no |
| log-space predictor | −0.1248 / −0.1896 | no |
| high computed excluding day *t* | −0.1077 / −0.1740 | no |
| predictor at *t* vs return *t−1 → t+N−1* | −0.0947 / −0.1723 | no |
| Spearman rank correlation instead of Pearson | −0.0651 / **−0.2159** | no — the rank statistic is *larger* at 20 d and still far below the null p90 |
| leave one builder out (any of the six) | max abs(r) 0.129 … 0.290 | no — the largest, dropping NBIS, is 0.290, still below the null p90 of 0.437 |
| only 4 of 6 builders (IREN, APLD, WULF, CORZ) | max abs(r) 0.305 at 40 d | no |

Every convention tested lands in the same place: **no evidence of a lead, and not enough data to have found
one if it existed.**

### 5.4 The null quantiles: consistent with the gate note, within Monte-Carlo error, seed unknowable

The gate note does not record its seed, so its null p50/p90/p95 **cannot be reproduced exactly** by
construction. Mine (seed 20261017, `close` field, the convention the point statistics identify) against its:

| | mine (seed 20261017) | gate note | difference | MC standard error | difference in se |
|---|---|---|---|---|---|
| p50 | 0.2539 | 0.254 | −0.0001 | 0.0027 | 0.0 |
| p90 | 0.4367 | 0.430 | +0.0067 | 0.0047 | 1.4 (≈1.0 for a difference of two runs) |
| p95 | 0.4860 | 0.477 | +0.0090 | 0.0054 | 1.7 (≈1.2) |
| p | 0.6265 | 0.641 | −0.0145 | 0.0083 | 1.7 (≈1.2) |

The MC standard errors are estimated from **20 independent replicates of 2,000 shuffles** each. Every
difference in the table is within ~1.2 standard errors of a difference between two independent Monte-Carlo
runs, so **the gate note's null is consistent with mine and no seed was reverse-engineered to force
agreement.** For completeness, alternative seeds give p50 ∈ [0.245, 0.255] and p ∈ [0.61, 0.65] — the same
band.

The **chunking convention**, unlike the seed, is decidable from the quantiles. Holding the trailing 11-day
chunk fixed in place instead of shuffling it with the rest gives (seed 20261017, `close`) p50 = 0.2687,
p90 = 0.4561, p95 = 0.5060, p = 0.6440. Against the gate note that is +0.0147 / +0.0261 / +0.0290 / +0.0030 —
about 1.3 se on the median but **3.0–4.8 se on p90, p95 and p**. The note's quantiles are therefore
consistent with the convention I used (the trailing chunk shuffled with the rest) and are **not** consistent
with the hold-the-remainder-fixed variant, at the precision 2,000 shuffles buys. Either way, the observed
0.214 is below the null median under both of those conventions, p is 0.63 either way, and both reject the pass
condition by a wide margin. (The one variant in §4 where the null median dips below the observed statistic —
the coarse 5-chunk 60-day block null — still gives p = 0.42.)

---

## 6. Verdict on the gate result

### 6.1 Does the FAIL survive verification? **YES — it survives.**

Every number in the gate note is reproducible from the repo's own cache, and one of them (the exact price
field) had to be identified to make the reproduction exact. Nothing about the verification moves the result
toward the pass condition:

1. **The statistic is below the null median under every convention I tried** — including every block length
   from 10 to 60 days except the coarse 5-chunk 60-day null, which still returns p = 0.42. 0.2122 (`adjclose`)
   or 0.2143 (`close`) against a null p50 of 0.2524 / 0.2539, p90 of 0.4374 / 0.4367. The pass condition
   required *above p90*. It is not close.
2. **The sign is not stable.** Non-overlapping grids at 20 d: positive at offsets 0 and 16, negative at 4, 8
   and 12, spread 0.41. The pre-registered pass condition required sign stability. It fails.
3. **The sample could not have decided it.** 12 independent 20-day blocks against ~85 needed — and the
   block-shuffle null is the *conservative* choice; the same data would look "significant" (p = 0.0055) against
   the i.i.d. null the note explicitly rejects. The note's discipline here is what keeps a null result from
   being written up as a discovery, and it is verified by direct computation.
4. **Robustness across every alternative I tried** (§5.3): alignment, log vs simple returns, predictor
   definition, rank vs Pearson, and cohort composition. No variant reaches the pass condition, and no variant
   even reaches the null median.

**Nothing may acquire weight on this evidence, and my verification does not propose otherwise.** No weight,
anchor, composite, coverage fraction or scored-set change was made — see §7. Both channels stay UNSCORED.

### 6.2 One correction to the gate note's wording (not to its verdict)

The gate note's verdict says *"Both channels stay unscored. Two independent reasons, either sufficient"*. The
two reasons are both about `leveraged_builder_stress`; the second channel was never tested, and the note says
so in its data section. My reading of the record is that the two channels fail for **different** reasons and
should be labelled differently:

| channel | status | why |
|---|---|---|
| `leveraged_builder_stress` | **TESTED, FAILS** | the gate ran; the statistic is below the null median and the sign flips on the grid. |
| `buildout_opposition_news` | **NOT TESTED — cannot be evaluated** | the GDELT `withdrawn` series was never cached (verified: no cache body for that query; the tool's own report records "no GDELT news series for data-centre withdrawals was retrieved"). There is nothing to compute a correlation against. |

The *outcome* is the same — neither is scored, and neither should be — but "FAILS" is a measurement and
"UNTESTED" is an absence of one, and a later reader who skims the verdict would be wrong to conclude the
withdrawal channel was tested and found wanting. **Recommendation: re-label that channel's line as
NOT TESTED.** This is a documentation correction only; no behaviour changes, and it is proposed here rather
than made, because the gate note is the pre-registered artefact.

---

## 7. What I did not touch, and the reproducibility gap

`git diff --stat` is empty at the end of this run apart from this note, and `git status --short` is empty once
it is committed: **`config/indicators.toml`, all weights, the composite, the coverage fraction and the scored
set are unchanged.** No indicator was added, removed, rescored or given anchors. The only file this
verification writes is this note.

**The reproducibility gap worth recording.** `data/cache/` is git-ignored (`.gitignore` line 3), and the
cached bodies are the gate's entire input, so "reproducible from the repo state at a commit" is true only
while the working tree's cache happens to hold the same bodies. A refresh overwrites them in place. The gate
note does not record which cache vintage it read. In this instance the point statistics identified the exact
field and the reproduction is exact, so the vintage did not matter — but that was luck, not design.

**Proposal (not made, per the rules of this task):** record, in a gate note, the FNV-1a-64 cache filenames
plus the first/last date and a hash of the input series actually used. It is three lines of text and it turns
"reproducible while nothing changes" into "reproducible from the note". The same applies to the shuffle seed,
which the gate note omits and which this note had to record for its own null.

**Also not verifiable from the repo:** the gate note is headed "Registered 2026-10-17, BEFORE any result was
computed", but the registration and the result live in the *same* commit (`2dd801f`), so `git log` cannot
corroborate the ordering. This is a process observation with no bearing on the verdict; the note's discipline
is visible in what it does (it publishes the seductive overlapping-window result *before* dismantling it),
which is not something a post-hoc write-up would be likely to do.

---

## 8. Residual uncertainty, and what would change the verdict

| # | uncertainty | size / nature | does it threaten the FAIL? |
|---|---|---|---|
| 1 | Shuffle seed unknown in the gate note | bounded by MC error: ±0.005 on p50, ±0.008 on p, at 2,000 shuffles | No. The observed statistic is 0.04 below the null median; no seed moves that. |
| 2 | Shuffle chunking convention (trailing 11-day remainder shuffled or held) | moves the null by +0.015 … +0.022; the note's p90/p95 are 3–5 se from the "held" variant, so the note's convention is pinned to "shuffled with the rest" | No. The variant that could matter moves the null *away* from the observed statistic. |
| 3 | `close` vs `adjclose` for SPY | ≤0.007 change in r, 1.24 pp of SPY return | No. A fifth of the gap to p90. |
| 4 | Cache vintage not recorded; cache not versioned | unknown, but the 11:33 and 13:47 vintages agree in content on this date | No, on this evidence — and it is the reason §7 proposes recording the input. |
| 5 | Overlapping windows make the nominal n (231) roughly 12 independent blocks | structural, not fixable by convention | No — it *causes* the failure, and it is the second reason the gate fails. |
| 6 | SPY is dominated by non-AI names; equities are a proxy for credit; drawdown ≠ level; 251 days is one market episode; the cohort is a judgement | design limits the gate note states itself | No, but they bound what the result can mean. It is a statement about *this* test, not a proof that builder stress never leads anything. |

**What would change the verdict — and none of it is a reason to score the channels now:**

- **~7 years of daily history** (≈1,700 trading days) to reach 85 independent 20-day blocks — or ~3.7 years if
  the true effect is a large r = 0.40 and the same test is used. At 20-day horizons this gate cannot pass
  before roughly 2033 on the current design.
- **A re-registered target and/or statistic before it is run**: a narrower AI-exposed basket instead of SPY, or
  a credit-spread series closer to the source. The gate's own pre-registration rules require this to be
  written down *before* the result, with the old result kept alongside — a re-run that simply swaps SPY for a
  target that happens to correlate better is exactly the move the registration exists to prevent.
- **A contemporaneous or lagged question, honestly asked.** Everything here tests *lead*. The predictor's
  lag-1 autocorrelation is 0.934 and equities are priced continuously, so the prior is strong that this series
  is contemporaneous-to-lagging information about a market move that has already happened — the same mechanism
  the media gate found. If the channel is ever shown to carry information, my expectation from these numbers
  is that it will be as a *coincident* measurement, which is what it already is: a visible reading with no
  weight.

---

## 9. Second independent recomputation — a separate run on a separate code path

Added after §1–§8 were committed (`6532015`) by a **second run of the same verification lane**. It exists
because §1's "two independent implementations" are two functions inside *one* script by *one* author. What
follows is a different script written independently, sharing with §1–§8 only the cache bytes and the indicator
definition read out of `src/indicators/fundamentals.rs`:

| element | §1–§8 script | this run |
|---|---|---|
| correlation | `numpy` primary, hand-written Pearson as the check | **pure-Python Pearson only** — numpy is not used anywhere in the core |
| cache path | FNV-1a-64 helper | own FNV-1a-64 implementation; all seven filenames printed and checked against the tool's own `src/http.rs::cache_path` |
| cache body | direct `json.load` | defensive unwrap (gzip / `body` / `content` wrapper) before parsing |
| shuffle RNG | `numpy.random.default_rng` (PCG64), seed 20261017 | `random.Random` (**Mersenne Twister**) — a different stream by construction |
| chunking | list-of-`arange` + `rng.permutation` | own chunk builder + `rng.shuffle` on the chunk order |
| power check | 10,000 simulated trials | 20,000 simulated trials, own gauss stream |

**Base commit for this section: `6532015`**, which was `HEAD` when it ran. The only difference between the gate
note's tree (`2dd801f`) and that one is this note — `git diff --stat 2dd801f 6532015` is *1 file changed, 649
insertions* — so the code, the config and the cached input are the same objects §1–§8 verified. Cache vintage at
run time: all seven bodies `mtime = 2026-10-17 13:47`, same as §1. `data/cache/` is git-ignored (§7), so the
vintage remains a working-tree fact.

Exact commands run (repository root):

```
git rev-parse HEAD            # -> 65320153f426fd0be60f0d1cf31675eb39f7bc61
git diff --stat 2dd801f HEAD  # -> 1 file changed, 649 insertions(+)  (this note)
python3 indep_verify.py       # §9.1-§9.3; script and full output in Appendix B
python3 probe_ac.py           # §9.4, the autocorrelation candidates
python3 probe_power.py        # §9.2, power simulation, seed 31337, 20,000 trials
git status --short            # only this note (see §9.5 on the one untracked directory)
```

No network call; nothing written except this note.

### 9.1 (a) and (b): every printed point statistic reproduces to the last digit

`adjclose` (the field the indicator itself reads) is the primary; the `close` column is the gate note's field.

| horizon | mine, `adjclose` | note §2, `adjclose` | mine, `close` | note §2, `close` | gate note |
|---|---|---|---|---|---|
| 5 d, n = 246 | −0.1095930933 | −0.1096 | −0.1057568039 | −0.1058 | −0.106 |
| 10 d, n = 241 | −0.1440361517 | −0.1440 | −0.1387229749 | −0.1387 | −0.139 |
| 20 d, n = 231 | −0.1772388746 | −0.1772 | −0.1699154262 | −0.1699 | −0.170 |
| 40 d, n = 211 | −0.2121735180 | −0.2122 | −0.2142519335 | −0.2143 | −0.214 |
| statistic `max abs(r)` | **0.2121735180** | 0.2122 | **0.2142519335** | 0.2143 | 0.214 |

| offset (20 d) | n | mine, `adjclose` | note §3, `adjclose` | mine, `close` | note §3, `close` | gate note |
|---|---|---|---|---|---|---|
| 0 | 12 | +0.0469 | +0.0469 | +0.0651 | +0.0651 | +0.065 |
| 4 | 12 | −0.3355 | −0.3355 | −0.3318 | −0.3318 | −0.332 |
| 8 | 12 | −0.1202 | −0.1202 | −0.1115 | −0.1115 | −0.112 |
| 12 | 11 | −0.2557 | −0.2557 | −0.2487 | −0.2487 | −0.249 |
| 16 | 11 | +0.0756 | +0.0756 | +0.0758 | +0.0758 | +0.076 |

All **18** point statistics and both `max abs(r)` values agree with §2/§3 to every printed digit (10 dp for the
correlations, 4 dp for the grid — the grid table's shorter display is the only reason the columns look alike
rather than identical). My run also reproduces the n values (246/241/231/211 and 12/12/12/11/11), the window
(251 days, 2025-10-17 → 2026-10-16) and the per-symbol check that `close` and `adjclose` are bit-identical for
all six builders (`max|close − adjclose| = 0.000000`) while SPY's differ by up to 7.355835.

### 9.2 (c) the null, and (d) power: agreement within Monte-Carlo error

| run | field | p50 | p90 | p95 | mean | p |
|---|---|---|---|---|---|---|
| mine, MT seed 20261017, 2000 | `adjclose` | 0.2555 | 0.4359 | 0.4841 | 0.2667 | 0.6325 |
| note §4, PCG64 seed 20261017, 2000 | `adjclose` | 0.2524 | 0.4374 | 0.4857 | 0.2672 | 0.6355 |
| mine, MT seed 20261017, 2000 | `close` | 0.2547 | 0.4359 | 0.4828 | 0.2671 | 0.6265 |
| note §4, PCG64 seed 20261017, 2000 | `close` | 0.2539 | 0.4367 | 0.4860 | 0.2674 | 0.6265 |
| **gate note (seed unrecorded)** | `close` | 0.254 | 0.430 | 0.477 | — | 0.641 |

The largest disagreement with §4 is **0.0031 on p50** (`adjclose`) and **0.0030 on p** — a different RNG stream,
not a different method. Monte-Carlo standard errors, measured here from **5 x 2000 independent replicates**:
0.0013–0.0014 (p50), 0.0054–0.0056 (p90), 0.0062–0.0070 (p95), 0.0078–0.0101 (p). Every row above is inside
~1.5 of those standard errors of every other row, so the three implementations (mine, §4's, and by implication
the gate note's) are one estimate, not three.

My five separate seeds put the `close`-field null at p50 ∈ [0.2473, 0.2584], p90 ∈ [0.4283, 0.4469],
p95 ∈ [0.4769, 0.4999], p ∈ [0.6165, 0.6430] — the gate note's four numbers (0.254 / 0.430 / 0.477 / 0.641)
all fall inside those ranges. **That is the strongest thing this section can say about the gate note: its
unrecorded seed did not matter, because it could not have.**

Sensitivity variants, mine against §4's (p, `close` field):

| variant | mine | note §4 | agrees? |
|---|---|---|---|
| block width 10 d | 0.4700 | 0.477 | yes, within MC error |
| block width 20 d (pre-registered) | 0.6265 | 0.627 | yes |
| block width 40 d | 0.6475 | 0.638 | yes |
| block width 60 d (5 chunks, coarse) | 0.4190 | 0.418 | yes |
| trailing 11-day chunk held fixed | 0.6310 | 0.644 | yes (and both are *higher* than the shuffled-remainder null, which is §1's chunking inference) |
| i.i.d. point shuffle | **0.0050** | 0.0055 | yes — the anti-conservative null is confirmed by an independent implementation |

(d) Power: identical closed form (n = 193.97 → **194**, 84.93 → **85**, 46.73 → **47**; Fisher z, α = 0.05
two-sided, power 0.80), and 251 // 20 = **12** blocks. My 20,000-trial simulation (own gauss stream, seed
31337) gives power **0.8111 ± 0.0028 / 0.8112 ± 0.0028 / 0.8236 ± 0.0027** at those n, against §4's 0.803 /
0.805 / 0.815 at 10,000 trials — differences of 0.008 / 0.006 / 0.009, about 1.5–2 Monte-Carlo standard errors
of a difference between two runs. (Appendix B's own 5,000-trial run inside `indep_verify.py` — a third RNG
stream, seed 4242 — gives 0.802 / 0.821 / 0.831: the same band, and the spread *between my own two runs*
(0.802 vs 0.811 at n = 194) is the size of the gap to §4, which is what Monte-Carlo error of this size looks
like rather than a method difference.) The conclusion §4 draws from it is unaffected: the design has 12 independent
20-day blocks where ~85 are needed.

### 9.3 A third route: the tool's own output, and alignment by hand

- **The predictor is the indicator's predictor.** Recomputing the last cached date (2026-10-16) from the bodies
  gives, per name, CRWV −56.13, NBIS −42.28, IREN −64.68, APLD −63.65, WULF −59.70, CORZ −53.77 and an
  equal-weighted **−56.7004 %**. The tool's own rendered report `data/history/reports/2026-10-17.html` says
  "CoreWeave −56 %, Nebius −42 %, IREN −65 %, Applied Digital −64 %, TeraWulf −60 %, Core Scientific −54 %" —
  every name agrees at the report's precision. The indicator's code (`fundamentals.rs:2793-2808`) uses the
  **expanding** maximum over the series it holds (`high = vals.iter().fold(f64::MIN, f64::max)`), and with 251
  cached points that is the same window as a trailing 52-week high, as §1 states.
- **Alignment by hand.** Forward-20-day SPY on 2026-03-13 = 657.157166 → 682.641785 = **+3.8780 %**, and the
  52-week-high dates on that day are CRWV 2025-10-29, NBIS 2025-10-31, IREN 2025-11-05, APLD 2026-01-27,
  WULF 2026-02-25, CORZ 2025-11-03 — dates inside the trailing year, none of them at or after the observation
  day, so no lookahead. Same check on 2026-01-05 (+0.2632 %) and 2026-09-15 (+2.9776 %): consistent.
- The two secondary readings the gate note leans on also check out: `credit_hy` reads **3.23** "as of
  2026-10-15" in the 2026-10-17 report, and the sibling sentiment gate's numbers in
  `.hermes/analysis/2026-09-26-sentiment-lead-lag.md` are peak lag k = +7, r = +0.221, "fraction of shuffles ≥
  observed" = **0.356** — matching the gate note's "peak at lag +7, p = 0.36".

### 9.4 Claims in §1–§8 that I could NOT reproduce exactly

Reported rather than papered over, per the rules of this task. **None of them touches (a)–(d) or the verdict**,
and I have not silently adopted either value.

| # | claim | mine | cause, as far as I could take it | materiality |
|---|---|---|---|---|
| 1 | predictor lag-1 autocorrelation **+0.934** (§4) | **+0.9556** (identical on both price fields) | **Not identified.** ~20 candidate definitions were computed; none returns 0.934. Closest: 0.9387 for the drawdown of the equal-weighted **level index** — a *different* series from the indicator's mean-of-per-name-drawdowns — and 0.9481 for the rank version. Table below. | none: the claim is used only to say "the series is highly persistent"; my value is *higher*, and neither the block null nor the 12-block count depends on it |
| 2 | "91 % of consecutive pairs sharing days" (§4) | consecutive 20-day windows share 20 of 21 dates = **95.2 %** | wording; no "sharing days" convention I can construct gives 91 % (231/251 = 92.0 % is the share of dates with a full forward window — the nearest figure, and a different quantity) | none |
| 3 | forward 20-day SPY return autocorrelation **+0.947** (§4) | +0.9457 (`adjclose`), **+0.9467** (`close`) | reproduces only on the **unadjusted** field — consistent with §5.1's finding that §1's target was `quote.close` | none |
| 4 | MC standard errors (p50 0.0027, p90 0.0047, p95 0.0054, p 0.0083, from 20 x 2000) | 0.0013–0.0014, 0.0054–0.0056, 0.0062–0.0070, 0.0078–0.0101 (from 5 x 2000) | different replicate counts and RNG stream; an se estimated from 5 replicates is itself ±~35 %, so these are the same size | none |
| 5 | simulated power 0.803 / 0.805 / 0.815 (10,000 trials) | 0.8111 / 0.8112 / 0.8236 ± 0.0028 (20,000 trials) | different RNG stream and trial count; differences ≈1.5–2 se of a difference between two runs | none — both bracket the 0.80 design target and the closed-form n is identical |

The autocorrelation candidates, all computed on the cached 251-day series (the indicator's definition is the
first row):

| predictor's lag-1 autocorrelation | value |
|---|---|
| **equal-weighted expanding drawdown (the indicator's own definition)** | **+0.9556** |
| same, unadjusted `close` field (identical series for the six builders) | +0.9556 |
| log-space drawdown | +0.9508 |
| linear trend removed | +0.9745 |
| first differences of the predictor | +0.0594 |
| rank (Spearman) lag-1 | +0.9481 |
| high computed excluding day *t* | +0.9517 |
| truncated to the first 211 / 231 / 241 / 246 / 250 points | +0.9425 / +0.9494 / +0.9515 / +0.9529 / +0.9550 |
| mean of the six per-name autocorrelations | +0.9504 |
| drawdown of the equal-weighted **level index** (a different series) | +0.9387 |
| predictor's 20-day lag instead of lag-1 | +0.3604 |
| autocorrelation of the 12 non-overlapping 20-day block means | +0.4087 |
| autocorrelation of the daily changes in the forward 20-day return | −0.0044 |
| **§4's stated value** | **+0.934 — reproduced by none of the above** |

### 9.5 Verdict after the second route, and what it did not touch

**The FAIL survives the second recomputation, unchanged.** It is not an artefact of the first script: 18 point
statistics reproduce to the last printed digit, the null agrees within Monte-Carlo error across two RNG
families, five seeds and four block widths, the two secondary readings in the gate note check out against the
tool's own report and the sibling gate, and the single number that does not reproduce (the predictor's lag-1
autocorrelation) is *larger* in my run and is used by neither the pass condition nor the verdict. Nothing here
is closer to the pass condition than §2–§4 already were.

`config/indicators.toml`, all weights, the composite, the coverage fraction and the scored set are unchanged —
`git status --short` before this commit shows only this note plus one untracked `.hermes/recon/2026-09-26/`
directory that an **unrelated recon pass was writing concurrently** (mtime 2026-10-17 16:16, i.e. during this
run) and that no part of the §1–§9 recomputation reads or writes. This commit stages exactly one path.

---

## Appendix A — the exact script

Run from the repository root as `python3 verify.py`; prints (a)–(d) for both price fields. `data/cache/` must
hold the seven cached Yahoo chart bodies (see §1). No network access, no writes.

```python
#!/usr/bin/env python3
"""verification.py - independent recomputation of the 2026-10-17 builder/withdrawal
lead-lag gate numbers, from the repo's own cache only (no network)."""
import datetime
import json
import math
import os

import numpy as np

SEED = 20261017
NSHUF = 2000
HORIZONS = (5, 10, 20, 40)
BUILDERS = ("CRWV", "NBIS", "IREN", "APLD", "WULF", "CORZ")
CACHE = os.path.join("data", "cache")


def fnv1a64(s):
    h = 0xCBF29CE484222325
    for b in s.encode():
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return "%016x" % h


def series(symbol):
    """{date: (close, adjclose)} from the cached Yahoo chart body."""
    url = ("https://query1.finance.yahoo.com/v8/finance/chart/%s?range=1y&interval=1d"
           % symbol)
    path = os.path.join(CACHE, fnv1a64(url) + ".body")
    raw = json.load(open(path))
    r = raw["chart"]["result"][0]
    q = r["indicators"]["quote"][0]
    adj = r["indicators"]["adjclose"][0]["adjclose"]
    out = {}
    for i, t in enumerate(r["timestamp"]):
        if adj[i] is None or q["close"][i] is None:
            continue
        d = datetime.datetime.fromtimestamp(t, datetime.timezone.utc).strftime("%Y-%m-%d")
        out[d] = (q["close"][i], adj[i])
    return out


def predictor(bars, dates, field):
    """Equal-weighted drawdown from each builder's own 52-week high, in percent.

    field 0 = raw close, field 1 = adjclose.  The 1-year cache holds 251 points, so
    the expanding high from the series start IS the trailing-52-week high for every
    date (a 252-point window never truncates) - the two conventions coincide.
    """
    p = np.zeros(len(dates))
    for t, d in enumerate(dates):
        per = [(bars[b][d][field] / max(bars[b][x][field] for x in dates[:t + 1]) - 1.0)
               * 100.0 for b in BUILDERS]
        p[t] = sum(per) / len(per)
    return p


def pearson_py(x, y):
    """Deliberately hand-written, as the independent route against numpy/scipy."""
    n = len(x)
    mx = sum(x) / n
    my = sum(y) / n
    sxy = sum((a - mx) * (b - my) for a, b in zip(x, y))
    sxx = sum((a - mx) ** 2 for a in x)
    syy = sum((b - my) ** 2 for b in y)
    return sxy / math.sqrt(sxx * syy)


def pearson_np(x, y):
    x = np.asarray(x); y = np.asarray(y)
    x = x - x.mean(); y = y - y.mean()
    return float((x * y).sum() / math.sqrt((x * x).sum() * (y * y).sum()))


def fwd(spy, i, h):
    return spy[i + h] / spy[i] - 1.0


def overlap_r(pred, spy, h):
    n = len(pred)
    return pearson_np(pred[:n - h], spy[h:] / spy[:n - h] - 1.0)


def grid_r(pred, spy, h, off):
    xs, ys = [], []
    k = 0
    while off + k * h + h <= len(pred) - 1:
        i = off + k * h
        xs.append(pred[i]); ys.append(fwd(spy, i, h)); k += 1
    return len(xs), pearson_np(xs, ys)


def max_abs_r(pred, spy):
    return max(abs(overlap_r(pred, spy, h)) for h in HORIZONS)


def block_null(pred, spy, runs, rng):
    """Shuffles of the predictor in contiguous 20-day chunks.

    251 points = 12 full 20-day chunks + a trailing 11-day chunk; the trailing
    chunk is shuffled with the rest (see the note for the alternative convention:
    stop the chunk list at 12 and append the trailing days unchanged).
    """
    T = len(pred)
    segs = [np.arange(k * 20, k * 20 + 20) for k in range(T // 20)]
    if (T // 20) * 20 < T:
        segs.append(np.arange((T // 20) * 20, T))
    m = len(segs)
    out = np.empty(runs)
    for i in range(runs):
        perm = rng.permutation(m)
        sh = np.concatenate([pred[segs[j]] for j in perm])
        out[i] = max(abs(pearson_np(sh[:T - h], spy[h:] / spy[:T - h] - 1.0))
                     for h in HORIZONS)
    return out


def main():
    bars = {b: series(b) for b in BUILDERS}
    bars["SPY"] = series("SPY")
    dates = sorted(bars["SPY"])
    T = len(dates)
    print("aligned calendar: %s -> %s   T = %d trading days" % (dates[0], dates[-1], T))

    for field, fname in ((1, "adjclose"), (0, "close")):
        pred = predictor(bars, dates, field)
        spy = np.array([bars["SPY"][d][field] for d in dates])
        print("\n================ predictor field: %s ================" % fname)
        print("mean drawdown on %s = %.4f%%" % (dates[-1], pred[-1]))
        if fname == "adjclose":
            print("per name:", ", ".join(
                "%s %.2f" % (b, (bars[b][dates[-1]][1] /
                                 max(bars[b][d][1] for d in dates) - 1.0) * 100.0)
                for b in BUILDERS))

        print("\n(a) overlapping windows, predictor(t) vs SPY t->t+N:")
        for h in HORIZONS:
            n = T - h
            x = list(pred[:n]); y = [fwd(spy, t, h) for t in range(n)]
            r_np = pearson_np(pred[:n], spy[h:] / spy[:n] - 1.0)
            r_py = pearson_py(x, y)
            print("    h=%2d  n=%3d  r=%+.4f   (pure-python %.10f, numpy %.10f)"
                  % (h, n, r_np, r_py, r_np))
        obs = max_abs_r(pred, spy)
        print("    observed statistic max|r| = %.4f" % obs)

        print("\n(b) non-overlapping grid offsets at 20d:")
        vals = []
        for off in (0, 4, 8, 12, 16):
            n, r = grid_r(pred, spy, 20, off)
            vals.append(r)
            print("    offset %2d  n=%2d  r=%+.4f" % (off, n, r))
        print("    spread %+.4f .. %+.4f (width %.4f), positive in %d of 5"
              % (min(vals), max(vals), max(vals) - min(vals), sum(1 for v in vals if v > 0)))

        print("\n(c) block-shuffle null, seed %d, %d shuffles:" % (SEED, NSHUF))
        null = block_null(pred, spy, NSHUF, np.random.default_rng(SEED))
        q = np.quantile(null, [0.50, 0.90, 0.95])
        print("    null p50=%.4f  p90=%.4f  p95=%.4f  mean=%.4f"
              % (q[0], q[1], q[2], null.mean()))
        print("    p = P(null >= observed) = %.4f" % float((null >= obs).mean()))

    print("\n(d) power, alpha = 0.05 two-sided, 80% power (Fisher z):")
    za, zb = 1.959963985, 0.841621234
    for r in (0.20, 0.30, 0.40):
        n = ((za + zb) / math.atanh(r)) ** 2 + 3
        print("    true r = %.2f -> n = %.2f  (%d rounded up)" % (r, n, math.ceil(n)))
    print("    independent 20-day blocks in %d days: %d" % (T, T // 20))
    print("    85 blocks x 20 days = %d trading days ~ %.1f years"
          % (85 * 20, 85 * 20 / T))


if __name__ == "__main__":
    main()
```

Output of that exact run on the verified commit (`adjclose` primary; `close` for the gate note's field):

```
aligned calendar: 2025-10-17 -> 2026-10-16   T = 251 trading days

================ predictor field: adjclose ================
mean drawdown on 2026-10-16 = -56.7004%
per name: CRWV -56.13, NBIS -42.28, IREN -64.68, APLD -63.65, WULF -59.70, CORZ -53.77

(a) overlapping windows, predictor(t) vs SPY t->t+N:
    h= 5  n=246  r=-0.1096   (pure-python -0.1095930933, numpy -0.1095930933)
    h=10  n=241  r=-0.1440   (pure-python -0.1440361517, numpy -0.1440361517)
    h=20  n=231  r=-0.1772   (pure-python -0.1772388746, numpy -0.1772388746)
    h=40  n=211  r=-0.2122   (pure-python -0.2121735180, numpy -0.2121735180)
    observed statistic max|r| = 0.2122

(b) non-overlapping grid offsets at 20d:
    offset  0  n=12  r=+0.0469
    offset  4  n=12  r=-0.3355
    offset  8  n=12  r=-0.1202
    offset 12  n=11  r=-0.2557
    offset 16  n=11  r=+0.0756
    spread -0.3355 .. +0.0756 (width 0.4111), positive in 2 of 5

(c) block-shuffle null, seed 20261017, 2000 shuffles:
    null p50=0.2524  p90=0.4374  p95=0.4857  mean=0.2672
    p = P(null >= observed) = 0.6355

================ predictor field: close ================
mean drawdown on 2026-10-16 = -56.7004%

(a) overlapping windows, predictor(t) vs SPY t->t+N:
    h= 5  n=246  r=-0.1058   (pure-python -0.1057568039, numpy -0.1057568039)
    h=10  n=241  r=-0.1387   (pure-python -0.1387229749, numpy -0.1387229749)
    h=20  n=231  r=-0.1699   (pure-python -0.1699154262, numpy -0.1699154262)
    h=40  n=211  r=-0.2143   (pure-python -0.2142519335, numpy -0.2142519335)
    observed statistic max|r| = 0.2143

(b) non-overlapping grid offsets at 20d:
    offset  0  n=12  r=+0.0651
    offset  4  n=12  r=-0.3318
    offset  8  n=12  r=-0.1115
    offset 12  n=11  r=-0.2487
    offset 16  n=11  r=+0.0758
    spread -0.3318 .. +0.0758 (width 0.4076), positive in 2 of 5

(c) block-shuffle null, seed 20261017, 2000 shuffles:
    null p50=0.2539  p90=0.4367  p95=0.4860  mean=0.2674
    p = P(null >= observed) = 0.6265

(d) power, alpha = 0.05 two-sided, 80% power (Fisher z):
    true r = 0.20 -> n = 193.97  (194 rounded up)
    true r = 0.30 -> n = 84.93  (85 rounded up)
    true r = 0.40 -> n = 46.73  (47 rounded up)
    independent 20-day blocks in 251 days: 12
    85 blocks x 20 days = 1700 trading days ~ 6.8 years
```

---

## Appendix B — the second recomputation's script and output

Run from the repository root as `python3 indep_verify.py` (§9.1–§9.3). Stdlib only in the computation — no
`numpy`, no `scipy` — and its own RNG stream. It reads `data/cache/` and writes nothing.

```python
#!/usr/bin/env python3
"""INDEPENDENT recomputation of the 2026-10-17 builder lead-lag gate numbers.

Deliberately a different code path from the note's Appendix A script:
 - pure-Python Pearson (no numpy in the core),
 - own FNV-1a-64 cache-path implementation,
 - own RNG stream (random.Random / Mersenne Twister, not numpy PCG64),
 - defensive cache-body unwrapping.
No network. Read-only.
"""
import datetime
import gzip
import json
import math
import os
import random
import time

CACHE = "/home/daim/workspace/bubble-watch/data/cache"
URL = "https://query1.finance.yahoo.com/v8/finance/chart/%s?range=1y&interval=1d"
BUILDERS = ("CRWV", "NBIS", "IREN", "APLD", "WULF", "CORZ")
HORIZONS = (5, 10, 20, 40)
T0 = time.time()


def fnv1a64(s):
    h = 0xCBF29CE484222325
    for b in s.encode("utf-8"):
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return "%016x" % h


def unwrap(obj):
    while isinstance(obj, dict) and "chart" not in obj:
        moved = False
        for k in ("body", "content", "response", "data"):
            if k in obj and isinstance(obj[k], str):
                try:
                    obj = json.loads(obj[k])
                    moved = True
                    break
                except Exception:
                    pass
        if not moved:
            return obj
    return obj


def load(sym):
    path = os.path.join(CACHE, fnv1a64(URL % sym) + ".body")
    raw = open(path, "rb").read()
    if raw[:2] == b"\x1f\x8b":
        raw = gzip.decompress(raw)
    obj = unwrap(json.loads(raw.decode("utf-8", "replace")))
    r = obj["chart"]["result"][0]
    q = r["indicators"]["quote"][0]
    adj = r["indicators"]["adjclose"][0]["adjclose"]
    out = {}
    for i, t in enumerate(r["timestamp"]):
        if adj[i] is None or q["close"][i] is None:
            continue
        d = datetime.datetime.fromtimestamp(t, datetime.timezone.utc).strftime("%Y-%m-%d")
        out[d] = (q["close"][i], adj[i])
    return path, out


def pearson(x, y):
    n = len(x)
    mx = sum(x) / n
    my = sum(y) / n
    sxy = sxx = syy = 0.0
    for a, b in zip(x, y):
        da = a - mx
        db = b - my
        sxy += da * db
        sxx += da * da
        syy += db * db
    return sxy / math.sqrt(sxx * syy)


def predict(bars, dates, field):
    p = []
    for t, d in enumerate(dates):
        tot = 0.0
        for b in BUILDERS:
            hi = bars[b][dates[0]][field]
            for x in dates[: t + 1]:
                v = bars[b][x][field]
                if v > hi:
                    hi = v
            tot += (bars[b][d][field] / hi - 1.0) * 100.0
        p.append(tot / len(BUILDERS))
    return p


def overlap(pred, spy, h):
    n = len(pred) - h
    return pearson(pred[:n], [spy[i + h] / spy[i] - 1.0 for i in range(n)])


def grid(pred, spy, h, off):
    xs, ys = [], []
    k = 0
    while off + k * h + h <= len(pred) - 1:
        i = off + k * h
        xs.append(pred[i])
        ys.append(spy[i + h] / spy[i] - 1.0)
        k += 1
    return len(xs), pearson(xs, ys)


def maxabs(pred, spy):
    return max(abs(overlap(pred, spy, h)) for h in HORIZONS)


def chunks(n, w):
    segs = [list(range(k * w, k * w + w)) for k in range(n // w)]
    if (n // w) * w < n:
        segs.append(list(range((n // w) * w, n)))
    return segs


def null_block(pred, spy, runs, seed, hold_last=False, width=20):
    T = len(pred)
    segs = chunks(T, width)
    if hold_last and (T // width) * width < T:
        tail = segs.pop()
    else:
        tail = None
    rng = random.Random(seed)
    out = []
    for _ in range(runs):
        order = list(range(len(segs)))
        rng.shuffle(order)
        sh = [v for j in order for v in (pred[i] for i in segs[j])]
        if tail is not None:
            sh += [pred[i] for i in tail]
        out.append(max(abs(overlap(sh, spy, h)) for h in HORIZONS))
    return out


def null_iid(pred, spy, runs, seed):
    rng = random.Random(seed)
    out = []
    for _ in range(runs):
        sh = pred[:]
        rng.shuffle(sh)
        out.append(max(abs(overlap(sh, spy, h)) for h in HORIZONS))
    return out


def quant(v, q):
    s = sorted(v)
    i = q * (len(s) - 1)
    lo = int(math.floor(i))
    hi = int(math.ceil(i))
    if lo == hi:
        return s[lo]
    return s[lo] + (s[hi] - s[lo]) * (i - lo)


def report_null(name, v, obs):
    p = sum(1 for x in v if x >= obs) / len(v)
    print("    %-26s p50=%.4f p90=%.4f p95=%.4f mean=%.4f  p=%.4f"
          % (name, quant(v, 0.50), quant(v, 0.90), quant(v, 0.95), sum(v) / len(v), p))
    return p


def main():
    bars = {}
    for s in BUILDERS + ("SPY",):
        path, d = load(s)
        bars[s] = d
        print("cache %-4s %s  n=%d  %s .. %s  mtime=%s"
              % (s, os.path.basename(path), len(d), min(d), max(d),
                 datetime.datetime.fromtimestamp(os.path.getmtime(path)).strftime("%Y-%m-%d %H:%M")))

    dates = sorted(bars["SPY"])
    T = len(dates)
    print("\naligned on SPY calendar: %s -> %s  T=%d" % (dates[0], dates[-1], T))
    for s in BUILDERS:
        missing = [d for d in dates if d not in bars[s]]
        extra = [d for d in bars[s] if d not in bars["SPY"]]
        print("  %-4s missing_vs_SPY=%d extra_vs_SPY=%d" % (s, len(missing), len(extra)))

    # field identity check
    for s in BUILDERS + ("SPY",):
        mx = max(abs(bars[s][d][0] - bars[s][d][1]) for d in dates if d in bars[s])
        print("  max|close-adjclose| %-4s = %.6f" % (s, mx))

    for field, fname in ((1, "adjclose"), (0, "close")):
        pred = predict(bars, dates, field)
        spy = [bars["SPY"][d][field] for d in dates]
        print("\n================ field: %s ================" % fname)
        print("last-date EW drawdown = %.4f%%" % pred[-1])
        if field == 1:
            print("per name: " + ", ".join(
                "%s %.2f" % (b, (bars[b][dates[-1]][1] / max(bars[b][d][1] for d in dates) - 1.0) * 100.0)
                for b in BUILDERS))
        print("(a) overlapping:")
        for h in HORIZONS:
            print("    h=%2d n=%3d r=%+.10f" % (h, T - h, overlap(pred, spy, h)))
        obs = maxabs(pred, spy)
        print("    max|r| = %.10f" % obs)
        print("(b) grid offsets at 20d:")
        vs = []
        for off in (0, 4, 8, 12, 16):
            n, r = grid(pred, spy, 20, off)
            vs.append(r)
            print("    off=%2d n=%2d r=%+.4f" % (off, n, r))
        print("    spread %+.4f .. %+.4f width=%.4f positive_in=%d"
              % (min(vs), max(vs), max(vs) - min(vs), sum(1 for v in vs if v > 0)))
        print("(c) block null (own MT stream, trailing chunk shuffled):")
        report_null("seed 20261017", null_block(pred, spy, 2000, 20261017), obs)
        for sd in (1, 7, 12345, 987654321):
            report_null("seed %d" % sd, null_block(pred, spy, 2000, sd), obs)
        report_null("held-remainder", null_block(pred, spy, 2000, 20261017, hold_last=True), obs)
        report_null("iid shuffle", null_iid(pred, spy, 2000, 20261017), obs)
        report_null("block width 10", null_block(pred, spy, 2000, 20261017, width=10), obs)
        report_null("block width 40", null_block(pred, spy, 2000, 20261017, width=40), obs)
        report_null("block width 60", null_block(pred, spy, 2000, 20261017, width=60), obs)
        # MC standard error from 5 replicates of 2000
        reps = [null_block(pred, spy, 2000, 555000 + k) for k in range(5)]
        for nm, f in (("p50", lambda v: quant(v, 0.50)), ("p90", lambda v: quant(v, 0.90)),
                      ("p95", lambda v: quant(v, 0.95))):
            vals = [f(r) for r in reps]
            m = sum(vals) / len(vals)
            sd = math.sqrt(sum((x - m) ** 2 for x in vals) / (len(vals) - 1))
            print("    MC se over 5x2000 %s = %.4f" % (nm, sd))
        ps = [sum(1 for x in r if x >= obs) / len(r) for r in reps]
        m = sum(ps) / len(ps)
        sd = math.sqrt(sum((x - m) ** 2 for x in ps) / (len(ps) - 1))
        print("    MC se over 5x2000 p  = %.4f" % sd)

    # (d) power
    print("\n(d) power, Fisher z, alpha=0.05 two-sided, power 0.80")
    za, zb = 1.959963985, 0.841621234
    for r in (0.20, 0.30, 0.40):
        n = ((za + zb) / math.atanh(r)) ** 2 + 3
        print("    true r=%.2f -> n=%.4f (ceil %d)" % (r, n, math.ceil(n)))
    print("    independent 20d blocks: %d // 20 = %d" % (T, T // 20))
    # simulated power (own gauss stream, 5000 trials)
    for r in (0.20, 0.30, 0.40):
        n = math.ceil(((za + zb) / math.atanh(r)) ** 2 + 3)
        rng = random.Random(4242)
        hit = 0
        trials = 5000
        crit = 1.959963985
        for _ in range(trials):
            xs = [rng.gauss(0, 1) for _ in range(n)]
            ys = [r * x + math.sqrt(1 - r * r) * rng.gauss(0, 1) for x in xs]
            rr = pearson(xs, ys)
            if abs(rr) * math.sqrt((n - 2) / (1 - rr * rr)) > crit:
                hit += 1
        print("    simulated power at n=%d, 5000 trials: %.3f" % (n, hit / trials))

    # persistence
    pred = predict(bars, dates, 1)
    spy = [bars["SPY"][d][1] for d in dates]
    ac1 = lambda v: pearson(v[:-1], v[1:])
    f20 = [spy[i + 20] / spy[i] - 1.0 for i in range(len(spy) - 20)]
    print("\nlag-1 autocorr predictor = %.4f ; forward-20d SPY return = %.4f" % (ac1(pred), ac1(f20)))

    # hand alignment spot-check
    print("\nhand alignment check (adjclose, 20d forward SPY):")
    for d in ("2026-03-13", "2026-01-05", "2026-09-15"):
        if d not in dates:
            print("   %s not a trading date in the cache" % d)
            continue
        i = dates.index(d)
        if i + 20 >= T:
            print("   %s has no full 20d forward window" % d)
            continue
        highs = []
        for b in BUILDERS:
            hi = max(bars[b][x][1] for x in dates[: i + 1])
            hd = [x for x in dates[: i + 1] if bars[b][x][1] == hi][0]
            highs.append("%s high %s" % (b, hd))
        print("   %s: fwd20 = %.6f -> %.6f = %+.4f%%  |  %s"
              % (d, spy[i], spy[i + 20], (spy[i + 20] / spy[i] - 1) * 100, "; ".join(highs)))
    print("\nelapsed %.1fs" % (time.time() - T0))


main()
```

Output of that run (`6532015`, cache vintage 2026-10-17 13:47):

```
cache CRWV ae2a7b3ef87f9332.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47
cache NBIS 91e2fbac503b418a.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47
cache IREN bc6ede7c9833e7d2.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47
cache APLD d70284fdd08343cd.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47
cache WULF f7beaae64ea91020.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47
cache CORZ b436574b7f311308.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47
cache SPY  3ba3cef87ebbeeb0.body  n=251  2025-10-17 .. 2026-10-16  mtime=2026-10-17 13:47

aligned on SPY calendar: 2025-10-17 -> 2026-10-16  T=251
  CRWV missing_vs_SPY=0 extra_vs_SPY=0
  NBIS missing_vs_SPY=0 extra_vs_SPY=0
  IREN missing_vs_SPY=0 extra_vs_SPY=0
  APLD missing_vs_SPY=0 extra_vs_SPY=0
  WULF missing_vs_SPY=0 extra_vs_SPY=0
  CORZ missing_vs_SPY=0 extra_vs_SPY=0
  max|close-adjclose| CRWV = 0.000000
  max|close-adjclose| NBIS = 0.000000
  max|close-adjclose| IREN = 0.000000
  max|close-adjclose| APLD = 0.000000
  max|close-adjclose| WULF = 0.000000
  max|close-adjclose| CORZ = 0.000000
  max|close-adjclose| SPY  = 7.355835

================ field: adjclose ================
last-date EW drawdown = -56.7004%
per name: CRWV -56.13, NBIS -42.28, IREN -64.68, APLD -63.65, WULF -59.70, CORZ -53.77
(a) overlapping:
    h= 5 n=246 r=-0.1095930933
    h=10 n=241 r=-0.1440361517
    h=20 n=231 r=-0.1772388746
    h=40 n=211 r=-0.2121735180
    max|r| = 0.2121735180
(b) grid offsets at 20d:
    off= 0 n=12 r=+0.0469
    off= 4 n=12 r=-0.3355
    off= 8 n=12 r=-0.1202
    off=12 n=11 r=-0.2557
    off=16 n=11 r=+0.0756
    spread -0.3355 .. +0.0756 width=0.4111 positive_in=2
(c) block null (own MT stream, trailing chunk shuffled):
    seed 20261017              p50=0.2555 p90=0.4359 p95=0.4841 mean=0.2667  p=0.6325
    seed 1                     p50=0.2575 p90=0.4319 p95=0.4834 mean=0.2693  p=0.6430
    seed 7                     p50=0.2504 p90=0.4460 p95=0.4999 mean=0.2675  p=0.6265
    seed 12345                 p50=0.2523 p90=0.4283 p95=0.4769 mean=0.2635  p=0.6270
    seed 987654321             p50=0.2465 p90=0.4433 p95=0.4913 mean=0.2647  p=0.6175
    held-remainder             p50=0.2656 p90=0.4557 p95=0.5034 mean=0.2773  p=0.6385
    iid shuffle                p50=0.0814 p90=0.1400 p95=0.1587 mean=0.0873  p=0.0050
    block width 10             p50=0.2051 p90=0.3520 p95=0.4065 mean=0.2194  p=0.4750
    block width 40             p50=0.2645 p90=0.4599 p95=0.5202 mean=0.2799  p=0.6530
    block width 60             p50=0.1971 p90=0.3989 p95=0.4199 mean=0.2273  p=0.4220
    MC se over 5x2000 p50 = 0.0013
    MC se over 5x2000 p90 = 0.0056
    MC se over 5x2000 p95 = 0.0070
    MC se over 5x2000 p  = 0.0101

================ field: close ================
last-date EW drawdown = -56.7004%
(a) overlapping:
    h= 5 n=246 r=-0.1057568039
    h=10 n=241 r=-0.1387229749
    h=20 n=231 r=-0.1699154262
    h=40 n=211 r=-0.2142519335
    max|r| = 0.2142519335
(b) grid offsets at 20d:
    off= 0 n=12 r=+0.0651
    off= 4 n=12 r=-0.3318
    off= 8 n=12 r=-0.1115
    off=12 n=11 r=-0.2487
    off=16 n=11 r=+0.0758
    spread -0.3318 .. +0.0758 width=0.4076 positive_in=2
(c) block null (own MT stream, trailing chunk shuffled):
    seed 20261017              p50=0.2547 p90=0.4359 p95=0.4828 mean=0.2671  p=0.6265
    seed 1                     p50=0.2584 p90=0.4325 p95=0.4792 mean=0.2697  p=0.6390
    seed 7                     p50=0.2512 p90=0.4469 p95=0.4984 mean=0.2678  p=0.6200
    seed 12345                 p50=0.2526 p90=0.4290 p95=0.4777 mean=0.2640  p=0.6280
    seed 987654321             p50=0.2473 p90=0.4429 p95=0.4932 mean=0.2650  p=0.6165
    held-remainder             p50=0.2662 p90=0.4564 p95=0.5033 mean=0.2774  p=0.6310
    iid shuffle                p50=0.0810 p90=0.1401 p95=0.1591 mean=0.0873  p=0.0050
    block width 10             p50=0.2084 p90=0.3503 p95=0.4053 mean=0.2198  p=0.4700
    block width 40             p50=0.2643 p90=0.4578 p95=0.5174 mean=0.2801  p=0.6475
    block width 60             p50=0.1985 p90=0.4024 p95=0.4198 mean=0.2276  p=0.4190
    MC se over 5x2000 p50 = 0.0014
    MC se over 5x2000 p90 = 0.0054
    MC se over 5x2000 p95 = 0.0062
    MC se over 5x2000 p  = 0.0078

(d) power, Fisher z, alpha=0.05 two-sided, power 0.80
    true r=0.20 -> n=193.9680 (ceil 194)
    true r=0.30 -> n=84.9278 (ceil 85)
    true r=0.40 -> n=46.7316 (ceil 47)
    independent 20d blocks: 251 // 20 = 12
    simulated power at n=194, 5000 trials: 0.802
    simulated power at n=85, 5000 trials: 0.821
    simulated power at n=47, 5000 trials: 0.831

lag-1 autocorr predictor = 0.9556 ; forward-20d SPY return = 0.9457

hand alignment check (adjclose, 20d forward SPY):
   2026-03-13: fwd20 = 657.157166 -> 682.641785 = +3.8780%  |  CRWV high 2025-10-29; NBIS high 2025-10-31; IREN high 2025-11-05; APLD high 2026-01-27; WULF high 2026-02-25; CORZ high 2025-11-03
   2026-01-05: fwd20 = 682.390076 -> 684.186035 = +0.2632%  |  CRWV high 2025-10-29; NBIS high 2025-10-31; IREN high 2025-11-05; APLD high 2025-10-20; WULF high 2025-11-03; CORZ high 2025-11-03
   2026-09-15: fwd20 = 755.513916 -> 778.010010 = +2.9776%  |  CRWV high 2025-10-29; NBIS high 2026-06-18; IREN high 2025-11-05; APLD high 2026-05-28; WULF high 2026-06-18; CORZ high 2026-06-18

elapsed 17.7s
```
