# bubble-watch — what to add next: consolidated findings and a prioritised plan

**Date:** 2026-09-20 · **Model:** methodology 2.2, composite 41.6/100, 20 indicators, weight 158
**Inputs:** 5 delegated reconnaissance lenses (live endpoint probing), 1 coverage analysis, and
independent re-verification by the orchestrator of the two headline claims.

**Status of the lenses:** L1 timed out at 600s (salvaged — 65 endpoints probed, 36 returned 200);
L2/L4/L5 wrote full reports; L3 wrote raw evidence but not its writeup. Everything below marked
**VERIFIED** was re-fetched by the orchestrator, not taken from a subagent's summary.

---

## 1. The diagnosis: the model has no demand side, and is 96% US-sourced

Grouping the 20 indicators by declared source (measured):

| family | weight | share |
|---|---|---|
| SEC EDGAR (filings/XBRL) | 51 | 32% |
| Yahoo (US equity) | 33 | 21% |
| FRED (US credit) | 18 | 11% |
| EDGAR full-text | 14 | 9% |
| circularity (read filings) | 12 | 8% |
| Fed Z.1 | 11 | 7% |
| Census C30 + EIA-860M | 13 | 8% |
| LMArena | 6 | 4% |

**US-sourced weight = 152/158 = 96%.** `foreign_interest` is *not* an international signal — it
measures foreign ownership of **US** equities.

More important than geography: **every indicator measures cost, financing, price/froth, physical
delivery, or narrative.** None measures whether AI revenue is growing, from where, or at what
margin. `capex_vs_cashflow` and `funding_gap` detect that spending outruns revenue, but cannot
distinguish *demand failure* from *demand outrunning supply* — opposite worlds, read identically.

---

## 2. THE FINDING: the demand side is measurable today, keyless

**VERIFIED by the orchestrator** (fetch repeated independently of the subagent):

```
GET https://openrouter.ai/api/frontend/v1/rankings/models
→ HTTP 200, 328,773 bytes, no key, no header
→ 573 rows: date, model_permaslug, variant,
   total_prompt_tokens, total_completion_tokens, count
→ 7 dates: 2026-09-13 .. 2026-09-19
→ TOTAL: 128,294,580,866,020 tokens = 128.3 trillion over the window
```

This is **the single most direct measurement of demand for AI compute available anywhere**, and it
was not known to the project. It yields three separable quantities:

1. **Throughput** — total tokens/day. Volume up ~24x in 12 months (L2: 5.26e12 → 1.278e14/day).
2. **Open-weight share** — 70.4% of tokens went to open-weight models (L4). This directly measures
   **whether intelligence is commoditising**, which is the moat question `frontier_premium` was
   trying to answer with a price proxy.
3. **Free-tier share** — 9.04% of tokens flowed through the `free` variant. Real usage that
   monetises nothing.

**A second endpoint gives about a year of weekly history:**
`openrouter.ai/api/frontend/v1/rankings/model-rankings-chart` → HTTP 200, 23,935 bytes,
52 weekly points. That is enough to compute a **rate of change**, which is what a stress model needs.

**Why this is the highest-value addition:** it is demand-side, it is keyless, it has history, and it
simultaneously (a) fixes the `frontier_premium` fixture problem and (b) provides the missing
bridge between capex spent and compute actually used.

---

## 3. The other verified capability: `frontier_premium` can be made LIVE

Two independent keyless routes, both confirmed:

| route | endpoint | status | measured |
|---|---|---|---|
| capability gap | LMArena leaderboard parquet on HuggingFace | 200, 589 KB, refreshed **daily** | gap **+32.5 Elo** (2026-09-13) — reproduces the committed fixture **exactly** |
| price premium | `openrouter.ai/api/v1/models` | 200, 737,913 B, 446 models | open-weight median **$0.20/M** vs closed **$1.00/M** = **5.0x** |

`frontier_premium` currently reads `tests/fixtures/arena_frontier_gap.json` — a file extracted once
on 2026-09-19, contributing **2.259 points**. The LMArena route makes it live with no key and no
scraping. **This is a correctness fix, not just an improvement.**

Also found: **LiteLLM's pricing file git history** is a free historical archive of published
inference prices. A like-for-like basket of 200 models across six snapshots: median input price
**$1.00/M (2024-05) → $0.70/M (2026-09) = 14.3%/yr**. Materially slower than the "10x/year"
folklore, and the difference matters for how fast the revenue case erodes.

---

## 4. What the utilization lens found, and why it is not an all-clear

L2's three independent measures all point the same way: OpenRouter throughput up ~24x/yr, ERCOT
grid load **+5.8% YoY**, Epoch's installed capacity **5.5x** (2024-01 → 2025-10). Rented H100
capacity is now **cheap**: vast.ai on-demand min **$1.469/hr** vs interruptible **$0.447/hr**,
against a $1.5–3.5/hr multi-vendor band, well below the ~$8–10/hr of 2023–24.

**The reconciliation is the important part:** volumes compound *while* prices collapse. That is
elastic demand meeting steep deflation — the genuine-buildout reading, not the empty-shell one. If
utilization were the problem, volume would stall as prices fell; instead usage rises ~24x/yr.

**Two caveats that keep it honest:**
- **46.7%** of work usage in Epoch's own survey runs on a **free** plan — real usage, zero revenue.
- Falling prices remain **ambiguous** between genuine deflation and providers cutting prices to fill
  idle capacity. Only realised revenue per token would separate them, and no free source publishes it.

---

## 5. Physical constraint: the grid queue is the real bottleneck

L5 fetched LBNL's "Queued Up" 2026 dataset (**15.5 MB; the landing page is Cloudflare-blocked to
curl and needs a real browser** — a methodology note worth keeping). Two measured reads:

**Median time-to-energise has roughly tripled:** 17.7 months (2005) → 46.1 (2020) → **60.8 months (2025)**.

**The queue's fuel mix flipped:** solar and storage entries are *shrinking* (solar standalone
**−21.5%**, storage **−16.1%**, wind **−18.9%** from 2024→2025) while **gas standalone rose +94.6%**
(69.4 → 240.0 GW). Data centres need firm power, and the queue is re-mixing toward gas to get it.

This is a **supply-constraint** reading: if interconnection takes five years and the queue is
shifting to gas, then announced capacity is partly paper and the binding constraint is energisation,
not capital.

---

## 6. International: one strong series, and the rest is key-gated

**Taiwan export orders** — `service.moea.gov.tw`, free, no key, **511 months (1984→2026-07)**:

| | 2023 | 2024 | 2025 | 2026 (7mo) |
|---|---|---|---|---|
| mean monthly US$mn | 46,878 | 49,199 | 61,977 | **86,005** |

Latest: **2026-07 = 97,939 (+61.9% YoY)**. A leading indicator (orders precede production), and
demand-side. Failure mode: it aggregates all electronics, not AI specifically.

Also live: Korea chip exports (HS 8542, **US$142.8bn** 2025; Comtrade **429s** under repeat calls),
Eurostat EU AI adoption (510 points), and the **Federal Register API for BIS export-control
changes** — which may be the more *direct* signal, since it measures a policy constraint on non-US AI
revenue rather than a proxy for demand.

**The landscape finding:** international data is **heavily key-gated** — Japan e-Stat (403), METI
(403), KOSIS (key), data.go.kr (401), KITA (404) all blocked. Adding international indicators means
committing to free-registration API keys, unlike every US source the tool currently uses.

---

## 7. Recommended sequence

**Tier 1 — do first (keyless, live, closes a stated gap)**

1. **`inference_demand`** — OpenRouter daily token volume + open-weight share + free-tier share.
   Keyless, ~1 year of history via the weekly chart, and it is the demand side the model lacks.
   Weight: high (12–14). *This is the single most valuable addition available.*
2. **Make `frontier_premium` LIVE** via the LMArena parquet (daily, keyless, reproduces the value
   exactly). Removes the only fixture-backed scored indicator. Consider splitting the *price* ratio
   (OpenRouter, 5.0x) from the *capability* gap (LMArena, +32.5 Elo) — they are different questions.
3. **`energisation_delay`** — LBNL median time-to-energise (17.7 → 60.8 months). A supply-constraint
   measure that reads in the *opposite* direction to most of the model, which also serves the
   falsification gap.

**Tier 2 — high value, needs an API key (free registration)**

4. **Taiwan export orders** as `international_demand` — the best non-US series found.
5. **Export-control activity** from the Federal Register API — direct policy constraint, open JSON.
6. **Eurostat EU AI adoption** — demand from the other side; needs no key but is annual.

**Tier 3 — investigate before committing**

7. **Realised revenue per token.** The one measurement that would separate price deflation from
   price-cutting-to-fill-idle-capacity. No free source found; this is the honest remaining gap.
8. **AI-specific credit** (L3's area: neocloud debt, BDC Level-3 marks, securitisation). Raw
   evidence was gathered but not written up — needs a second pass before anything ships.

**Do not build on (verified dead):** Artificial Analysis (401 + JS-rendered), Japan e-Stat/METI
(403), KITA/KOSIS/data.go.kr (key-gated or 404), FRED international export series (no response),
DBnomics search (400).

---

## 8. The methodological warning this exercise produced

The L1 lens **timed out and nearly lost its entire output** to one slow probe — 23 completed API
calls were almost discarded because a 24th hung for 299s. It was only recoverable because the child
happened to write intermediate files. Two rules follow:

- **A reconnaissance agent must write its findings file FIRST and append**, not save writing for the
  end. A partial file is a deliverable; a lost transcript is not.
- **Every network probe needs a hard `--max-time`.** An unbounded fetch is a single point of failure
  for the whole run.

Both are now in the delegation brief template for this project.
