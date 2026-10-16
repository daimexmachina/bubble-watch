# Spec: a FAST, verifiable credit-and-funding channel — research and gate design

**Status: BUILD COMPLETE (2026-10-16, commit f7db427). C1 shipped as `funding_stress`, weight 7.0,
schema_version 2.5 -> 2.6. See section 6 for what was verified and section 7 for what the build
changed relative to this spec.** Decision taken 2026-10-16 after Daim chose path (A) — add
high-frequency *verifiable* series, each gated — and delegated the choice of which series ("your
call"), then said "Build it!".

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

**FINDING 1 — the ICE BofA credit indices are licence-windowed to ~3 years, and it is FRED's own
restriction, not the transport.** `fredgraph.csv` returns full history for non-ICE series but a hard
rolling window for ICE indices: every call returned exactly 795 rows starting 2023-10-17, and an
explicit `cosd=1997-01-01` (or `cosd`+`coed`) is **ignored**. The `.txt` "Table Data" endpoint returns
HTML, not observations.

**CORRECTION (measured with the key, 2026-10-16): a valid key does NOT lift this.** With `FRED_API_KEY`
loaded from `~/.hermes/.env` — confirmed working, HTTP 200 — and `observation_start=1900-01-01`:

```
series         type   status     rows  first        last
BAMLH0A0HYM2   ICE    200         794  2023-10-17   2026-10-15
BAMLH0A3HYC    ICE    200         794  2023-10-17   2026-10-15
BAMLC0A4CBBB   ICE    200         794  2023-10-17   2026-10-15
DGS10          non    200      16,902  1962-01-02   2026-10-14
DCPF1M         non    200       7,763  1997-01-02   2026-10-05
NFCI           non    200       2,910  1971-01-08   2026-10-09
```

The key plainly unlocks deep history in general — `DGS10` comes back to **1962** — while the ICE
indices stay pinned at 794 rows. So the cap is enforced **server-side by FRED for licensing**, and it
is permanent, not a transport default. Any new ICE-based indicator is therefore anchored to a recent
distribution, and that must be declared.

**FINDING 2 — a pre-existing reproducibility gap, flagged not fixed, and NOT fixable with a key.**
`credit_hy`'s rationale cites "HY OAS has had a ~3.0% median" over the post-2009 record, but the ICE
series only ever returns 3 years (Finding 1), so the stated anchor cannot be reproduced from the data
the tool retrieves. **CORRECTION: an earlier version of this spec said a free FRED API key "would fix
it". That was wrong** — the key is already configured and working, and the series is capped regardless.
Reproducing that anchor needs a *different source* for long HY OAS history, not a credential.

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

**C1 — `funding_stress` (recommended first), REVISED to SOFR − EFFR.** The first draft proposed
SOFR − IORB. Measurement changed that: **IORB only begins 2021-07-29**, so SOFR − IORB cannot see the
September 2019 repo spike or March 2020, and carries no genuine funding-crisis reference point at all.
Substituting **EFFR** (from 2000) extends the spread back to 2018-04 and puts two documented events
inside the window. Measured over 2,132 observations, 2018-04-03 → 2026-10-15:

```
last   +0.010     (calm)
median -0.010     p95 +0.070     p99 +0.180
max    +2.950 on 2019-09-17      <- the September 2019 repo-market seizure
        +0.750 on 2019-01-02     +0.600 on 2018-12-31     +0.450 on 2019-09-30
        +0.360 on 2025-10-31     +0.310 on 2019-04-30
2020-03-17 +0.290   2023-03-15  0.000
```

That ladder is anchorable in the project's sense: a **real** funding crisis at the top (+2.95), the
year-end and quarter-end turns as known seasonality, the 2025-10 cluster, and calm near zero. Daily
and current (SOFR to 2026-10-15, EFFR to 2026-10-14). *Anchoring limitation, DECLARED:* the window
starts 2018, so it still contains no GFC point; that must be stated in the rationale rather than
implied away. `IORB` remains the most current series available (2026-10-16) but is unusable here for
the reason above.

**C2 — `credit_tail`.** CCC−HY dispersion (BAMLH0A3HYC vs BAMLH0A0HYM2). Directly measures the tail
claim (ii). *Anchoring limitation:* the 3-year ICE window (Finding 1) means the anchor covers only
recent episodes; must be declared.

**C3 — `cp_funding`.** Commercial paper spread vs a T-bill. Has the **longest history of any
candidate** (DCPN30 and DCPF1M both from 1997, so a real GFC and COVID anchor) — **but CORRECTION:
it is WEEKLY in practice, not daily.** DCPF1M's last ten observations are mostly empty, with real
values only on 2026-09-25 and 2026-10-05; the "11 days stale" noted earlier is that weekly cadence,
not a stalled feed. `DCPN30` is the most current of the family (2026-10-14). So C3 trades speed for
anchor depth: it is the best-anchored funding series and the worst as a *fast* one. Worth its own
probe rather than being ranked as if it were daily.

**Rejected or deferred:** STLFSI4 and NFCI are weekly, so they cannot be the *fast* channel; both are
also composite indices that would overlap the existing indicators. `DCPB1M` returns HTTP 400 (no
such series).

## 4. What is needed to proceed

1. **Review of the anchoring approach in C1/C2.** This is a methodology question: adding an
   indicator changes the total weight, which orphans cross-version comparison and needs a
   `schema_version` bump (precedent: 175 → 183 → 2.5). Anchors set what the score MEANS, so this is an
   owner decision, not a loop's.
2. **No longer a key decision — the key already exists and works.** Daim was right: `FRED_API_KEY`
   is configured in `~/.hermes/.env` (32 chars) and returns HTTP 200. An earlier draft of this spec
   claimed no key was configured, quoting a stale comment in `src/sources/fred.rs` ("api_key is not
   set") instead of checking — that comment is out of date and should be corrected in the source.
   **Measured consequence: the key does NOT lift the ICE cap** (Finding 1: 200 OK, still 794 rows,
   with `observation_start=1900-01-01`). So the open question is no longer "enter a key" but
   **"source long HY OAS history from somewhere other than FRED"**, for two separate purposes: a
   reproducible anchor for the existing `credit_hy` (Finding 2), and a longer window for C2 if the
   tail indicator is built.
3. **Correction required in the source.** `src/sources/fred.rs` documents `400 api_key is not set` as
   observed behaviour. That is no longer true and it misleads a reader into thinking the keyed
   transport is unavailable — it is the transport the tool is already using. Should be updated with
   the date it was verified.

## 5. Not concluded

- **Not concluded that funding stress leads anything.** It is proposed as a missing channel, and
  the k=0 evidence from the financier gate is the reason the leading claim is not made.
- **Not concluded that C1 is the best of the three.** It is ranked first on currency (updated to
  2026-10-16), completeness (no windowing), and novelty (no overlapping indicator) — not on a
  demonstrated forecasting edge, which has not been measured.
- **RESOLVED, was "not concluded that the 3-year ICE window is permanent".** It was left open as
  possibly a transport default. Measured on 2026-10-16 with a working key and
  `observation_start=1900-01-01`: the ICE indices still return exactly 794 rows while `DGS10` returns
  16,902 from 1962. So the cap is **server-side, for licensing, and permanent** — it is not fixable
  by credential or parameter. Long HY OAS history must come from another source entirely.
- **Not concluded that these readings change the headline.** Today's funding spread is −0.010, i.e.
  calm. If added, it would most likely move the composite only slightly — the value is an early
  channel, not a larger number.

## 6. What was verified after the build (2026-10-16, `f7db427`)

| check | result |
|---|---|
| Live reading | `funding_stress` weight 7, **stress 17.1**, contribution 0.63 |
| Arithmetic ties | 7 × 17.1 / 190 = 0.63 — matches the table |
| Coverage | 100%, 190 of 190 weight, 0 indicators unavailable |
| Composite | 39.8 (was 40.8 before, on 183 weight) |
| Offline degradation | SOFR uncached → GAP naming the series **and both transports tried**, `api_key` redacted, composite still computed at 96% |
| Tests | 429 green, 0 warnings |

The composite fell 40.8 → 39.8 because the new indicator currently reads calm (17.1) and therefore
pulls the average down. That is the honest consequence of measuring a channel that is quiet, and it is
**not** a reason to weight it differently.

**Falsification: the pairing guard took three attempts, and the first two proved nothing.**

- Attempt 1 edited an anchor that `cargo fmt` had moved. The `assert count == 1` caught it, so
  *nothing was tested* — but without that assertion the suite would have reported green and the
  falsification would have silently retired.
- Attempt 2 replaced a **value** inside a date-keyed map while the **key** still governed which
  observations paired. The buggy behaviour was unreachable, so the test correctly still passed. **A
  mutation that does not change behaviour proves nothing**, even when the bytes changed.
- Attempt 3 replaced the pairing *decision* and failed as required, reporting `got 1.1` against the
  expected `0.10`.
- The disjoint-calendar guard failed correctly on the first attempt (fabricating `0.0` would read as a
  calm funding market).
- The shipped-config direction guard was falsified by inverting the ladder in the TOML — raw still
  ascending, stress descending — and failed with a named reason.

## 7. What the build changed relative to this spec

- **C1 shipped as designed** (SOFR − EFFR), after the IORB → EFFR revision in section 3.
- **`IndicatorCfg` gained `fred_series_b`** — a spread cannot be one series id, and hardcoding the two
  legs inside the indicator would put the measurement in the code instead of the config. The spread is
  always `fred_series` MINUS `fred_series_b`, so direction is stated rather than inferred.
- **A global "anchors must ascend in stress" guard was CONSIDERED AND REJECTED.** Measured: 5 of 25
  shipped indicators are legitimately descending in stress, so the rule would have broken five correct
  indicators while appearing to be a safety check. A targeted assertion on the shipped
  `funding_stress` ladder replaced it — the indicator's unit test cannot catch a config-side inversion
  because it carries its own copy of the ladder.
- **C2 and C3 remain unbuilt**, as ranked. C3's "11 days stale" was resolved as *weekly cadence*, not a
  stalled feed, which removes it from contention as a fast channel.
- **The `credit_hy` anchor remains unreproducible** (Finding 2) and is now known to be unfixable from
  FRED. It is still flagged, not fixed.

