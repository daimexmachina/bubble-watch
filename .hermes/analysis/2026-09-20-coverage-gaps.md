# bubble-watch — coverage analysis of the 20-indicator model

**Date:** 2026-09-20 · **Model:** methodology 2.2, composite 41.6/100, 20 indicators, weight 158
**Purpose:** establish what families of risk the model actually measures, before adding anything.
**Method:** parsed `config/indicators.toml` and grouped by declared `source`; counted weight share.

---

## 1. The model is 96% US-sourced — measured

| source family | count | weight | share |
|---|---|---|---|
| SEC EDGAR (filings/XBRL) | 6 | 51 | 32% |
| Yahoo (US equity prices) | 4 | 33 | 21% |
| FRED (US credit spreads) | 2 | 18 | 11% |
| EDGAR full-text | 2 | 14 | 9% |
| `sec-edgar (read filings)` — circularity | 1 | 12 | 8% |
| Fed Z.1 | 2 | 11 | 7% |
| Census C30 | 1 | 7 | 4% |
| EIA-860M | 1 | 6 | 4% |
| LMArena (global) | 1 | 6 | 4% |
| **TOTAL** | **20** | **158** | |

**US-sourced weight: 152/158 = 96%. Non-US: 6/158 = 4%** — and the 4% is a capability
benchmark, not a demand or competition signal.

**`foreign_interest` is not an international signal.** It measures *foreign ownership of US
equities* (Fed Z.1 FL263064105.Q) — a US asset viewed from the foreign side. It tells you who
holds the claim, not where the revenue is earned.

---

## 2. There is no revenue-side coverage at all

Every indicator measures one of: **cost** (capex, funding gap, depreciation schedule),
**financing** (credit spreads, issuance, leverage, private credit), **price/froth**
(valuation, volatility, concentration, breadth, primary-market supply),
**physical delivery** (construction, grid), or **narrative** (filing census, circularity).

**Nothing measures whether AI revenue is growing, from where, or at what margin.**
`funding_gap` and `capex_vs_cashflow` are *ratios of spend to existing* revenue — they detect
that spending outruns revenue, but they cannot distinguish:

- spend is outrunning revenue because **revenue is not arriving** (demand failure), from
- spend is outrunning revenue because **demand is outrunning supply** (healthy, capacity-constrained).

Those are opposite worlds and the model currently reads them identically. **This is the single
largest structural gap: the model is entirely cost-and-financing side, with no demand side.**

---

## 3. Where competition could cap the revenue (the question asked)

If US AI revenue depends on global demand and on holding a frontier monopoly, then two things
matter that are currently invisible:

1. **Non-US demand and where it flows.** Korean and Taiwanese export orders, Japanese equipment
   billings, Chinese domestic-accelerator production, and sovereign programmes (UAE/Saudi/Qatar)
   are all leading indicators of who is buying compute and from whom. A shift in these would show
   revenue moving away from US hyperscalers *before* it appears in their filings.
2. **Frontier monopoly erosion.** `frontier_premium` is the only indicator aimed at this, at
   weight 6, and it measures a **price ratio of model access** — not capability-adjusted
   performance, and not substitution actually occurring.

---

## 4. Two correctness issues found while measuring

**(a) `frontier_premium` is scored from a fixture, not a live feed.** Its provenance resolves to
`tests/fixtures/arena_frontier_gap.json`, a 57KB file extracted **once** on 2026-09-19, `as_of`
2026-09-13. It contributes **2.259 points** to the composite. The config labels it `LMArena
(fixture)`, so this is disclosed rather than hidden — but a scored indicator that cannot update is
a stale-by-design input, and the tool's own contract says a stale source should degrade to a gap.
Either make it live or treat it as the one legitimately-fixtured indicator *with a staleness
ceiling* (the code does check staleness — verify the threshold is short enough).

**(b) `foreign_interest` scores on a percentile since 1945.** Its own rationale discloses that Z.1
holdings include **valuation changes**, so a rising series partly reflects US equities
outperforming rather than foreign investors buying more. At weight 5 with stress 92.0 (the highest
single stress in the model) this is the most extreme reading in the report and the most weakly
attributed to its own stated cause.

---

## 5. Where the model stands against its own plan

From `.hermes/plans/v1.2-development-plan.md`:

| phase | intent | status |
|---|---|---|
| 1 — integrity | fix ORCL zero-debt, add leases, depreciation test, GSADF, exposure table | **done** (all live) |
| 2 — financing architecture | private credit, off-balance-sheet, maturity timing, credit history | **partly** (private_credit_growth live; off-balance-sheet and timing NOT) |
| 3 — falsification | "the tool currently cannot show the bubble thesis is wrong" | **3 falsifiers exist, 1 yields counter-evidence** — improved but thin |
| 4 — backlog | C30, EIA-860M, short interest, options, GDELT, GitHub, job postings | **C30 + EIA-860M done**; rest open |

---

## 6. Candidate directions, ranked by structural value

Not yet verified against live endpoints — the recon lenses are probing these now.

1. **Revenue/demand side (largest gap).** Anything that shows AI revenue growth *direction* —
   inference token volume, API spend, cloud AI segment revenue growth — as a rate of change.
2. **Utilization of the installed base.** Whether compute that has been built is actually used.
   Low utilization ⇒ speculative capex; saturation ⇒ the constraint is supply, not demand.
3. **International demand and competition.** Korea/Taiwan/Japan leading indicators; Chinese
   domestic supply; sovereign programmes. Directly addresses the 96%-US concentration.
4. **AI-specific credit and financing structure.** Where the debt sits, who holds it, and at what
   mark (BDC Level-3 holdings are a measurement of self-marking).
5. **A second falsification channel.** Only one of three falsifiers currently produces
   counter-evidence; the model still mostly accumulates confirming evidence.
