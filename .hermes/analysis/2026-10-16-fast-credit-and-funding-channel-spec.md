# Spec: a FAST, verifiable credit-and-funding channel — research and gate design

**Status: research complete, awaiting review before build.** Decision taken 2026-10-16 after Daim
chose path (A) — add high-frequency *verifiable* series, each gated — and delegated the choice of
which series ("your call").

**Chosen family: credit & funding spreads.** Not arbitrary. The model's own config already calls
credit "the single most important non-equity signal… credit reprices faster and with covenants, and
the debt-funded buildout means any disappointment transmits through credit first." It is also the
only fast channel demonstrably moving right now (credit_hy stress 18.0 → 25.7 in three weeks).

## 1. What was measured (not assumed)

All eight candidates probed keylessly against `fredgraph.csv` on 2026-10-16:

| series | what it is | last obs | rows | history via this transport |
|---|---|---|---|---|
| BAMLH0A3HYC | ICE BofA **CCC & lower** OAS | 2026-10-15 | 795 | **3-year rolling window** |
| BAMLC0A4CBBB | ICE BofA **BBB** OAS | 2026-10-15 | 795 | 3-year rolling window |
| BAMLH0A0HYM2 | ICE BofA HY OAS (already `credit_hy`) | 2026-10-15 | 795 | 3-year rolling window |
| DCPF1M | 1-month financial commercial paper | **2026-10-05** | 7763 | full, from 1997 |
| SOFR | secured overnight financing rate | 2026-10-15 | 2228 | since inception 2018 |
| IORB | interest on reserve balances | **2026-10-16** | 1906 | since inception 2021 |
| STLFSI4 | St. Louis Fed financial stress | 2026-10-09 | 1711 | full, **weekly** |
| NFCI | Chicago Fed financial conditions | 2026-10-09 | 2910 | full, **weekly** |

**FINDING 1 — the ICE BofA credit indices are licence-windowed to ~3 years.** `fredgraph.csv`
returns full history for non-ICE series (CP from 1997, NFCI from 1971) but a hard rolling window for
ICE indices: every ICE call returned exactly 795 rows starting 2023-10-17, and an explicit
`cosd=1997-01-01` (or `cosd`+`coed`) is **ignored**. The `.txt` "Table Data" endpoint returns HTML,
not observations. So any new ICE-based indicator anchored on this transport can only be anchored to
a recent distribution.

**FINDING 2 — a pre-existing reproducibility gap, flagged not fixed.** `credit_hy`'s rationale cites
"HY OAS has had a ~3.0% median" over the post-2009 record, but the tool's own transport can only
hold **3 years** of that series. The stated anchor therefore cannot be reproduced from the data the
tool actually retrieves. This does not change the current reading; it means the anchor is inherited
from outside the tool. Resolving it needs a free FRED API key (`api_key is not set` today), which
would also unlock `observation_start` for the full ICE history.

**FINDING 3 — the funding channel is entirely absent from the model.** There is no money-market
indicator at all. Measured SOFR−IORB (a standard funding-pressure gauge: secured overnight funding
versus the administered rate) over 1301 observations, 2021-07-29 → 2026-10-15:

```
last   -0.010     (calm: SOFR 3.89 below IORB 3.90)
median -0.090     sd 0.055
p95    +0.030     p99  +0.120
max    +0.320 on 2025-10-31;  next widest 2025-11-03, 2025-11-28, 2025-12-01, 2025-12-31
days > +0.10: 24 (1.8%)      days > +0.02: 79 (6.1%)
```

The widest readings sit in a single cluster (late Oct–Dec 2025), so the channel does register real
episodes rather than noise.

## 2. The gate: what claim is and is NOT being made

This is the part that decides whether the work is legitimate.

A credit-tail or funding indicator must **NOT** be justified as "it leads the crash". The project
already tested the leading-indicator hypothesis twice and rejected it both times: the financier
lead/lag gate returned *no* (all 10 pairs peaked at **k=0, same-day**, with controls peaking at k=0
at *higher* correlation), and GDELT tone is fetched-and-reported but deliberately unscored for the
same reason. Claiming these new series lead would be the unproven-leading-indicator failure the
config explicitly names.

**The claims actually being made, each separately falsifiable:**

- **(i) A MISSING CHANNEL, not a leading one.** Funding stress transmits through money markets,
  which the model cannot currently see at all. Falsified if the candidate is highly correlated with
  `credit_hy`/`credit_ig` — i.e. redundant, adding no distinct information.
- **(ii) TAIL SENSITIVITY within an episode.** A crash is a *tail* event; the index can widen
  modestly while the weakest borrowers blow out. Falsified if CCC−HY dispersion does not widen
  materially more than the index in the same episodes.
- **(iii) NOT REDUNDANT WITH HISTORY.** Falsified if adding the candidate leaves the composite
  byte-identical across the archive (in which case it is decoration).

**Explicitly NOT claimed:** that any of these predicts or times a reversal. The report must say so.

## 3. Proposed candidates (ranked)

**C1 — `funding_stress` (recommended first).** SOFR − IORB, daily, current to 2026-10-16 (the most
current series of all eight probed). Full available history (2021+). Genuinely new channel.
*Anchoring limitation:* the series starts 2021, so it contains no GFC or COVID reference point. It
would be anchored on its own observed distribution (p99 ≈ +0.12, max +0.32) **declared as such**,
with the short window stated in the rationale rather than dressed up as a full-cycle calibration.

**C2 — `credit_tail`.** CCC−HY dispersion (BAMLH0A3HYC vs BAMLH0A0HYM2). Directly measures the tail
claim (ii). *Anchoring limitation:* the 3-year window means the anchor covers only recent episodes;
must be declared.

**C3 — `cp_funding`.** Commercial paper spread (DCPF1M vs a T-bill). Has the **longest history of any
candidate** (1997+, so it contains the GFC and COVID) but its last observation was **2026-10-05, 11
days stale**, so it is not a fast indicator on current evidence. Worth a separate probe of why.

**Rejected or deferred:** STLFSI4 and NFCI are weekly, so they cannot be the *fast* channel; both are
composite indices that would also overlap the existing indicators.

## 4. What is needed to proceed

1. **Review of the anchoring approach in C1/C2.** This is a methodology question: adding an
   indicator changes the total weight, which orphans cross-version comparison and needs a
   `schema_version` bump (precedent: 175 → 183 → 2.5). Anchors set what the score MEANS, so this is an
   owner decision, not a loop's.
2. **A decision on the free FRED API key.** Without it, every ICE-based credit indicator is capped
   at a 3-year anchor (Finding 1), and `credit_hy`'s existing anchor stays unreproducible
   (Finding 2). With it, both are fixable. No key value should be pasted into chat — the project's
   convention is that credentials are entered by the owner, never handled by the loop.

## 5. Not concluded

- **Not concluded that funding stress leads anything.** It is proposed as a missing channel, and
  the k=0 evidence from the financier gate is the reason the leading claim is not made.
- **Not concluded that C1 is the best of the three.** It is ranked first on currency (updated to
  2026-10-16), completeness (no windowing), and novelty (no overlapping indicator) — not on a
  demonstrated forecasting edge, which has not been measured.
- **Not concluded that the 3-year ICE window is permanent.** It may be a transport default rather
  than a licence term; the API-key path (item 2) would settle it, and that has not been tried
  because no key is configured.
- **Not concluded that these readings change the headline.** Today's funding spread is −0.010, i.e.
  calm. If added, it would most likely move the composite only slightly — the value is an early
  channel, not a larger number.
