# L2 — Utilization and real demand for AI compute

**Recon lens:** the missing bridge between capex and revenue. The existing model measures
what is BUILT (capex, construction, grid) and what is PAID (prices, credit, RPO), but not
whether the compute is USED. Low utilization → capex is speculative. Saturated utilization
→ capex is justified and the constraint is supply, not demand. Both readings are informative
and they mean opposite things.

**All values below were fetched live from this host on 2026-09-20 (UTC ~22:28–23:20).**
Every entry states URL, HTTP status, and a real measured value. Nothing here is imputed,
defaulted, or carried over. **Dead ends are reported as dead ends.**

---

## 0. HEADLINE FINDINGS (the four that matter)

1. **Utilization is growing fast and broadly.** OpenRouter measured throughput:
   **5.26e12 tokens/day (2025-09-22) → 1.278e14 tokens/day (2026-09-14), ~24x in 12 months.**
   ERCOT grid load grew **+5.8% YoY (Jan–Aug, mean 56,691→59,075 MW)**. Epoch's installed
   GPU capacity went **813,913 → 4,453,918 H100-equivalents from 2024-01 to 2025-10 (5.5x)**.
   Three independent physical/behavioral measures, all up.

2. **Rented H100 capacity is cheap and abundant.** vast.ai: **on-demand min $1.469/hr vs
   interruptible min $0.447/hr (3.3x)**; median $5.145 vs $4.000. RunPod's own published
   **on-demand H100 SXM: $2.69/hr Community, $3.49/hr Secure**. Multi-vendor H100 on-demand
   sits in a **~$1.5–3.5/hr band**, far below the ~$8–10/hr of 2023-24 — capacity is not
   scarce on the spot/community tier.

3. **Inference prices are collapsing — Epoch measures a median ~50x/year decline**
   (range 9x–900x, performance-matched). The strongest deflation signal in the lens.

4. **The reconciliation: volumes compound WHILE prices fall.** That is the signature of
   elastic demand meeting steep deflation — the genuine-buildout reading, NOT the empty-shell
   reading. If utilization were the problem, volume would stall while prices fell. Instead
   usage is up ~24x/yr and grid load up ~5.8% while unit prices fall ~50x/yr.
   **Two caveats keep this from being a clean all-clear:** (a) 46.7% of work usage in Epoch's
   own survey runs on a FREE plan (§3c) — real usage that monetises nothing; (b) falling
   prices remain **ambiguous** between genuine deflation and providers price-cutting to fill
   idle capacity. Only realised revenue/token would separate them; no free source publishes it.

---

## 1. OPENROUTER — token volume, the utilization series

### 1a. Confirmed working endpoints

| Endpoint | HTTP | Bytes | What it returns |
|---|---|---|---|
| `https://openrouter.ai/api/v1/models` | 200 | 737,913 | 446 models, pricing + context. **No token volume.** |
| `https://openrouter.ai/api/frontend/v1/rankings/models` | 200 | 328,773 | **Daily token volume per model, 7-day window.** 573 rows. |
| `https://openrouter.ai/api/frontend/v1/rankings/model-rankings-chart` | 200 | 23,935 | **52 weekly points, ~1 year of history**, top-10 models/week. |
| `https://openrouter.ai/api/frontend/v1/rankings/apps` | 200 | 27,873 | Token volume by *application* (day/week/month). |
| `https://openrouter.ai/api/frontend/v1/rankings/task-spend` | 200 | 64,456 | Spend share by task category, 30-day window. |
| `https://openrouter.ai/api/frontend/v1/rankings/session-cost` | 200 | 38,742 | Median USD per session by harness/model. |
| `https://openrouter.ai/api/frontend/v1/rankings/programming-language` | 200 | 15,190 | Token volume by programming language. |

These are **frontend** routes (undocumented, no key, no public API tier). Fragile but live.
Route fields: `date, model_permaslug, variant, total_prompt_tokens, total_completion_tokens,
count` — a genuine measured series, not an estimate. The namespace was discovered by grepping
OpenRouter's own Next.js JS chunks for `/api/frontend/...` strings; the `?view=`/`?window=`
query params are ignored (all return the same 52-point weekly series).

### 1b. Measured: OpenRouter total tokens/day (weekly chart, top-10 models summed)

Source: `GET .../rankings/model-rankings-chart` → HTTP 200, retrieved 2026-09-20.
Billions of tokens/day, summed over the 10 models the endpoint exposes per week:

| Week starting | B tok/day | Week starting | B tok/day |
|---|---|---|---|
| 2025-09-22 | 5,258 | 2026-04-06 | 20,991 |
| 2025-10-27 | 5,511 | 2026-05-04 | 25,732 |
| 2025-11-24 | 7,185 | 2026-06-01 | 36,126 |
| 2025-12-22 | 5,665 | 2026-07-06 | 52,620 |
| 2026-01-26 | 8,250 | 2026-08-03 | 69,047 |
| 2026-02-23 | 13,616 | 2026-08-24 | 112,997 |
| 2026-03-30 | 27,006 | 2026-09-07 | 126,763 |
| | | **2026-09-14** | **127,765** |

**Direction: strongly UP — ~24x over 12 months, ~2.4x in the last ~3 months**
(52.6T → 127.8T). Visible flat step Apr-2026 (27.0→21.0T, −22%), then re-acceleration.

### 1c. Measured: top models by token volume, latest daily bucket

Source: `GET .../rankings/models` → HTTP 200. Latest bucket **2026-09-19**. Prompt+completion:

| Model | B tokens (2026-09-19) | Requests |
|---|---|---|
| deepseek/deepseek-v4.1-flash-20260910 | 14,572 | 243,670,574 |
| z-ai/glm-5.3-flash-20260826 | 12,964 | 352,815,355 |
| openai/gpt-5.6-luna-20260709 | 12,438 | 345,446,996 |
| tencent/hy4-preview-20260827 | 11,926 | 104,098,259 |
| deepseek/deepseek-v4-flash-20260731 | 9,766 | 517,876,145 |
| xiaomi/mimo-v2.5-20260422 | 7,174 | 98,052,983 |
| nvidia/nemotron-3-ultra-550b-a55b-20260604 | 4,185 | 31,124,306 |
| google/gemini-3.8-flash-20260902 | 2,140 | 57,748,196 |

**Read:** volume leaders are **cheap fast models**, not frontier. High token counts at low
price/token can coexist with flat *revenue* per unit of compute. Token volume is a utilization
measure, **not** a revenue measure, and the top-8 by volume are dominated by
sub-dollar-per-million-token models.

### 1d. Measured: application token volume (agent traffic)

Source: `GET .../rankings/apps` → HTTP 200, `day` bucket:

| App | T tokens/day | Requests | Category |
|---|---|---|---|
| Hermes Agent | 1.491 | 14,921,299 | personal-agent, cli-agent |
| Claude Code | 0.917 | 10,130,558 | cli-agent |
| Kilo Code | 0.568 | 7,588,613 | cli-agent, ide-extension |
| Cline | 0.395 | 3,750,743 | ide-extension, cli-agent |
| pi | 0.299 | 2,793,624 | cli-agent |
| OpenClaw | 0.163 | 2,263,527 | personal-agent, cli-agent |
| DeepSeek Harness | 0.150 | 1,052,416 | — |
| Codex | 0.128 | 1,670,200 | cli-agent |

**Read:** the top of the volume table is **agentic coding harnesses** — token-hungry by
construction (long context, multi-turn tool loops). Real demand, but the most price-sensitive
and portable demand in the market: an agent harness can switch providers in one config line.
**Failure mode:** agent traffic inflates token volume without proportionally inflating
revenue, and it is trivially portable — a weak moat signal.

### 1e. Measured: task spend share (30-day window)

Source: `GET .../rankings/task-spend` → HTTP 200, `windowDays: 30`:
**General 31.52% · Code 29.70% · Agent 29.27% · Data 9.52%.**

~59% is Code + Agent. Genuine enterprise-adjacent usage, but heavily exposed to one workflow
class — coding is the workload most amenable to cheap open-weight substitution (ties to the
existing `frontier_premium` indicator, open/closed gap compressed ~76%).

---

## 2. GPU RENTAL / SPOT PRICES

### 2a. vast.ai — keyless live marketplace (WORKING)

```
GET https://console.vast.ai/api/v0/bundles/?q=<urlencoded JSON>
```
Query used: `{"gpu_name":{"in":["<NAME>"]},"limit":200,"order":[["dph_total","asc"]]}`.
Fields: `dph_total` (on-demand $/hr), `min_bid` (interruptible/spot $/hr). All rows HTTP 200:

| GPU | n | on-demand min | p10 | median | max | spot min | spot median |
|---|---|---|---|---|---|---|---|
| **H100 SXM** | 52 | **$1.469** | $2.016 | $5.145 | $39.629 | **$0.447** | $4.000 |
| **B200** | 41 | **$6.252** | $7.510 | $32.510 | $75.010 | **$5.938** | $28.500 |
| A100 SXM4 | 64 | $0.376 | $0.602 | $1.003 | $2.109 | $0.133 | $0.800 |
| RTX 4090 | 64 | $0.121 | $0.401 | $0.961 | $2.137 | $0.053 | $0.709 |
| RTX 5090 | 64 | $0.213 | $0.468 | $1.326 | $4.268 | $0.100 | $1.120 |
| L40S | 53 | $0.476 | $0.601 | $1.631 | $6.519 | $0.200 | $1.227 |
| H200 SXM | — | **no offers under this exact GPU-name string** | | | | | |

**On-demand vs spot for H100: $1.469 vs $0.447 at the floor — 3.3x.** Deep spot discount
means idle/interruptible capacity seeks work — the supply-exceeds-demand reading.
**BUT the counter-reading is equally consistent:** spot tiers exist to clear *unreserved*
capacity and a persistent gap is normal in any two-tier market. **A spot discount alone does
not prove oversupply** — what would prove it is the spot price *falling over time*, and this
endpoint returns a current snapshot only. **No history in this API.** A time series must be
built by polling and storing. That is the honest gap.
**B200 floor is 4.3x the H100 floor** with a wide on-demand spread (median $32.51 ≈ 5x floor);
newest silicon remains scarce and unevenly priced.

### 2b. RunPod — published on-demand list prices (WORKING, keyless)

Source: `https://www.runpod.io/pricing` → **HTTP 200** (308,695 bytes). Prices extracted from
the page's **JSON-LD `AggregateOffer` blocks** (not scraped prose), retrieved 2026-09-20:

| GPU | Community Cloud | Secure Cloud |
|---|---|---|
| **H100 SXM** | **$2.69/hr** | **$3.49/hr** |
| H100 PCIe | $1.99/hr | $2.89/hr |
| H100 NVL | $2.59/hr | $3.19/hr |
| **H200** | **$3.59/hr** | **$4.59/hr** |
| **B200** | **$5.98/hr** | **$6.79/hr** |
| **B300** | **$6.94/hr** | **$7.89/hr** |
| A100 SXM | $1.39/hr | $1.59/hr |
| A100 PCIe | $1.19/hr | $1.59/hr |
| L40S | $0.79/hr | $1.09/hr |
| RTX Pro 6000 | $1.69/hr | $2.09/hr |
| RTX 5090 | $0.69/hr | $0.99/hr |
| RTX 4090 | $0.34/hr | $0.74/hr |
| RTX A6000 | $0.33/hr | $0.53/hr |

**Cross-vendor check:** RunPod's published H100 SXM ($2.69 Community) sits **above** vast.ai's
on-demand *floor* ($1.469) but **below** vast.ai's *median* ($5.145) — consistent with vast.ai
being a heterogeneous host marketplace (cheap hobby hosts + premium clusters) while RunPod is
a curated single-brand cloud. The two measure different things; do not average them.

### 2c. DEAD ENDS (reported, not guessed)
| Provider | Endpoint | Result |
|---|---|---|
| Lambda Labs | `https://api.lambdalabs.com/v1/instance-types` | no response; `lambdalabs.com/service/gpu-cloud` → **301** only |
| Nebius | `https://nebius.com/prices` | **HTTP 200** but prices client-rendered — only prose ("H100, RTX PRO 6000, L40S"), no extractable numeric table |
| RunPod GraphQL | `https://api.runpod.io/graphql` | **HTTP 400** — needs a query + API key (the *pricing page* works instead) |

**No historical spot series exists for any provider** — this is a poll-and-accumulate
indicator, not a fetch-a-series indicator.

---

## 3. EPOCH AI — free datasets

**Working: `https://epoch.ai/data` (HTTP 200).** Direct download links in the page HTML
(no login, no key):

| File | HTTP | Notes |
|---|---|---|
| `epoch.ai/data/gpu_clusters.csv` | 200, **304,469 B**, csv | **Downloaded & parsed.** Cluster inventory w/ H100e, MW, owner, date |
| `epoch.ai/data/ai_chip_owners_quarters_by_chip_type.csv` | 200, **80,049 B** | **Downloaded & parsed.** Installed capacity by owner/chip/quarter |
| `epoch.ai/data/ai_chip_sales_chip_types.csv` | 200, 7,115 B | Chip specs + FLOP/s |
| `epoch.ai/data/ai_chip_sales_organizations.csv` | 200, 830 B | Designer→chip-type map |
| `epoch.ai/data/polling/polling_on_ai_usage_jul_2026.csv` | 200, **722,637 B** | **Downloaded & parsed.** Direct AI-usage survey, 3,008 rows |
| `epoch.ai/data/data_centers/data_centers.zip` | 200, zip | Data-centre directory (no Content-Length) |
| `epoch.ai/data/ai_chip_sales.zip`, `ml_hardware.zip`, `ai_models.zip` | 200, zip | sizes unknown (no Content-Length on HEAD) |

### 3a. Installed GPU capacity by quarter — the capacity-built series (MEASURED)

Source: `ai_chip_owners_quarters_by_chip_type.csv` (HTTP 200, 80,049 B, 384 rows),
retrieved 2026-09-20. Column `Compute estimate in H100e (median)`, summed across owners per
quarter. Start-date range **2022-01-01 → 2026-01-01**:

| Quarter | Total H100-equivalents |
|---|---|
| 2024-01 | 813,913 |
| 2024-04 | 1,020,602 |
| 2024-07 | 1,290,956 |
| 2024-10 | 1,881,887 |
| 2025-01 | 2,574,238 |
| 2025-04 | 2,879,884 |
| 2025-07 | 3,796,015 |
| **2025-10** | **4,453,918** |

**5.5x growth from 2024-01 to 2025-10.** Cumulative H100e by owner (across all quarters):
**Google 5,442,222 · Other 3,839,353 · Microsoft 3,558,492 · Amazon 2,498,626 ·
Meta 2,390,082 · Oracle 1,194,238 · China 1,152,892 · CoreWeave 828,624 · xAI 553,713.**

**IMPORTANT CAVEAT — this series ends 2025-10 and is a *median estimate with a 5th/95th
percentile band*, not a measured count.** The 2026-01 rows (14 of them) sum to only 1,190,091
H100e, i.e. the latest quarter is **incomplete in the file** — do NOT read the fall from
2025-10's 4.45M as a decline. It is a coverage artefact.

**gpu_clusters.csv** (HTTP 200, 304,469 B) top record: **`xAI Colossus Memphis Phase 3`**,
Status `Existing`, Certainty `Confirmed`, **275,795.86 H100-equivalents**, primary chip
`NVIDIA H100 SXM5 80GB`, quantity 200,000, owner `xAI`, first operational `2025-07-22`, USA;
note field records "150k H100s and 50k H200s". This gives **built capacity by operator and
date** — the "how much is BUILT" side, not utilization.

### 3b. Epoch inference-price index — the price-per-token trend (MEASURED)

Source: `https://epoch.ai/data-insights/llm-inference-price-trends` → **HTTP 200**,
retrieved 2026-09-20. Authors: Ben Cottier, Ben Snodin, David Owen, Tom Adamczewski (2025).
Directly measured claims quoted from the page:

- *"The inference price of LLMs has fallen dramatically in recent years."*
- Price to reach GPT-4-level performance on PhD-level science questions: **fell 40x per year**.
- Across benchmarks/thresholds: **prices declined between 9x and 900x per year, median 50x/year**.
- Removing pre-January-2024 data *increased* the measured rates.
- Method note (verbatim): *"All prices were a 3:1 weighted average of input and output token
  prices"* — i.e. a **constructed index from published list prices, not transaction prices**.

**Failure mode, confirmed by the source:** this is an **API list-price index**. It can fall
from real cost deflation, from **mix shift** toward cheaper models, or from providers cutting
prices to fill capacity. Epoch's *performance-matched* design — holding the benchmark score
fixed — controls for mix shift, which is why it is the best public series. **But it remains
list prices, not realised revenue per token.** Direction: **down, steeply. Genuinely two-sided.**

Dead end: `epoch.ai/data/ai-model-prices` and `/data/inference-prices` → **HTTP 404**.

### 3c. Epoch polling — direct demand evidence (MEASURED)

Source: `epoch.ai/data/polling/polling_on_ai_usage_jul_2026.csv` (HTTP 200, 722,637 B, 3,008
rows), retrieved 2026-09-20. Full sample (`demographic=full_sample`):

**Frequency of use over last 7 days** (`q4_frequency`):
7 days **21.94%** · 5 days 15.19% · 3 days 17.5% · 2 days 14.39% · 1 day 16.23% · 6 days 5.13%

**AI tenure** (`q10_ai_tenure`): 6 months–2 years **44.93%** · 1–6 months 27.3% ·
2+ years 12.9% · ≤1 month 13.02%

**Share of task time interacting with AI** (`q12_ai_time_share`):
all/almost all 7.07% · most 14.99% · about half 26.72% · a little **41.61%** · none 7.7%

**Subscription used for work** (`q8_employer_ai`):
**Free plan 46.69%** · employer-provided 28.8% · personal paid 12.78% · both 8.37%

**Read:** usage is **frequent and habitual** (21.9% daily users; 57.8% used it 3+ of the last
7 days) — genuine adoption, not trial. **But the monetisation gap is stark: 46.7% of work
usage is on a FREE plan**, and 41.6% of task-time involves only "a little" AI. Heavy usage
exists, but a large share currently generates no revenue. **This is the most direct
demand-side evidence for the capex/revenue bridge in this report, and it cuts BOTH ways.**

---

## 4. ELECTRICITY LOAD — the physical utilization proxy

### 4a. EIA-930 / EIA API — key required, NOT available from this host
| Endpoint | Result |
|---|---|
| `https://api.eia.gov/v2/electricity/rto/region-data/data/?...&facets[respondent][]=ERCO` | **HTTP 403 — `{"error":{"code":"API_KEY_MISSING","message":"No api_key was supplied..."}}`** |
| `https://api.eia.gov/v2/electricity/operating-generator-capacity/data/` | **HTTP 403 — same `API_KEY_MISSING`** |
| `https://www.eia.gov/electricity/gridmonitor/dashboard/electric_overview/US48/US48` | 200, but **HTML SPA shell** (43,526 B), not data |
| `https://www.eia.gov/electricity/gridmonitor/sixMonthFiles/EIA930_2026_Jan_Jun.csv` | 200 but returned **the same 43,526-byte HTML shell** — path wrong, not usable |

**Verified independently: a free EIA key IS required for the v2 API.** Registration is free
but needs a signup step this session cannot complete. **This is the highest-value actionable
gap in the lens** — EIA-930 hourly region load is the cleanest national utilization proxy and
needs one free key to unlock.

### 4b. ERCOT native load — FULLY WORKING, downloaded, parsed (MEASURED)

`https://www.ercot.com/gridinfo/load/load_hist` → HTTP 200 (76,035 B) links annual native-load
archives. Fetched and parsed (HTTP 200 each) in a **stdlib xlsx reader — no openpyxl needed**:

| URL | HTTP | bytes |
|---|---|---|
| `ercot.com/files/docs/2022/02/08/Native_Load_2022.zip` | 200 | 1,093,481 |
| `ercot.com/files/docs/2023/02/09/Native_Load_2023.zip` | 200 | 1,091,030 |
| `ercot.com/files/docs/2024/02/06/Native_Load_2024.zip` | 200 | 1,088,476 |
| `ercot.com/files/docs/2025/02/11/Native_Load_2025.zip` | 200 | 1,085,085 |
| `ercot.com/files/docs/2026/02/10/Native_Load_2026.zip` | 200 | 725,925 (Jan–Aug 2026) |

Each zip holds one `Native_Load_<year>.xlsx`, hourly, 8 ERCOT weather zones + an `ERCOT`
total column. **ERCOT total hourly load, measured:**

| Year | hourly obs | mean MW | peak MW | YoY mean |
|---|---|---|---|---|
| 2022 | 8,760 | 49,074 | 80,038 | — |
| 2023 | 8,760 | 50,748 | 85,464 | +3.4% |
| 2024 | 8,784 | 52,538 | 85,199 | +3.5% |
| 2025 | 8,760 | 55,734 | 83,679 | +6.1% |
| **2026 (Jan–Aug)** | **5,831** | **58,978** | **91,134** | **+5.8%** |

Monthly means, 2026: Jan 53,574 · Feb 49,440 · Mar 51,142 · Apr 52,723 · May 56,364 ·
Jun 65,757 · Jul 69,394 · **Aug 73,270 MW**.

**Monthly YoY (mean load), 2026 vs 2025:** Jan −1.7% · Feb −4.4% · Mar +9.3% · Apr +3.8% ·
May +1.6% · Jun +4.7% · **Jul +7.7% · Aug +10.1%**.

**Read — and this is the important subtlety:** ERCOT load is **growing, accelerating in summer
2026 (Jul +7.7%, Aug +10.1%)**, and the growth rate has risen from ~3.4%/yr (2023–24) to
~6%/yr (2025–26). Texas is the densest data-centre corridor in the US. **But load growth is
confounded:** weather (2026 peak 91,134 MW vs 2025 83,679 MW, +8.9%), general population/
industrial growth, crypto mining, and electrification all contribute. Load alone cannot be
attributed to AI data centres without the sectoral breakdown. **Direction: up and
accelerating. Attribution to AI: plausible, not proven.**

**Method note (a reusable finding):** the `.xlsx` was read with **Python stdlib only** —
`zipfile` + `xml.etree` parsing `xl/sharedStrings.xml` and `xl/worksheets/sheet1.xml` directly.
`openpyxl` is NOT installed on this host and PEP 668 blocks a system install; the stdlib route
works and needs no dependency. **This unblocks the same xlsx-only pattern the project already
uses for Census C30 and EIA-860M.**

### 4c. Large-load interconnection — NOT FOUND
| Target | Result |
|---|---|
| `ercot.com/mp/data-products/data-product-details?id=NP3-198-CD` | HTTP 200 (52,764 B) — load data-product page |
| `ercot.com/gridinfo/planning` | HTTP 200 |
| `ercot.com/gridinfo/interconnection` | **404** |
| `ercot.com/services/rq/large-load` | **404** |
| `ercot.com/files/docs/2024/04/30/LargeLoadInterconnectionStatus.xlsx` | **404** |
| `ercot.com/files/docs/2026/01/01/LargeLoadQueue.xlsx` | **404** |
| `api.pjm.com/api/v1/hrl_load_metered` | **HTTP 401** — key required |
| `dataminer2.pjm.com/feed/hrl_load_metered` | **HTTP 406** / 1,467-byte HTML shell — not an API |

**ERCOT's dedicated large-load / data-centre interconnection queue was NOT found at any
guessed URL, and PJM's Data Miner requires a key (401).** This is the most direct measure of
data centres *actually connecting*, and it sits behind ERCOT's MIS portal and PJM's keyed API.
**Honest gap: NOT obtained. Flag for follow-up with deliberate portal navigation.**

---

## 5. API PRICE PER TOKEN OVER TIME

- **Best free historical series: Epoch's inference-price insight (§3b)** — median ~50x/yr
  decline, range 9x–900x/yr, performance-matched, free, no key.
- **OpenRouter** provides current list prices for 446 models (`pricing.prompt`,
  `pricing.completion`) — e.g. `prism-ml/ternary-bonsai-2-27b` at `$0.000000075`/input token.
  **No price-history endpoint was found** — it is a current snapshot.
- **RunPod** publishes current on-demand list prices (§2b); **vast.ai** publishes live
  market-clearing prices (§2a). Neither has history.
- **Failure mode (dominant — restated):** a falling API list price may reflect (a) real cost
  deflation (elastic-demand reading), (b) mix shift toward cheaper models (Epoch's matched
  design partly controls this), or (c) providers price-cutting to fill idle capacity
  (oversupply reading). **The same falling number supports opposite conclusions.** Only
  *realised revenue per token* would separate them, and no free source publishes it.

---

## 6. DATA-CENTRE VACANCY / PRE-LEASING

| Target | Result |
|---|---|
| `datacenterhawk.com/market-reports` | **HTTP 404** |
| `cbre.com/insights/reports/north-america-data-center-trends-h1-2026` | **HTTP 403** (bot-blocked) |

**Not obtained.** CBRE and JLL gate their data-centre reports behind bot protection and/or
lead-capture forms; datacenterHawk's free report URL 404s. **Vacancy and pre-leasing are the
cleanest occupancy measures that exist and are effectively paywalled.** Confirmed gap — a
future session could try a real browser session with JS, or secondary reporting of their
numbers (clearly labelled as a secondary quote, not a measured fetch).

---

## 7. SYNTHESIS — direction of each signal and its failure mode

| Signal | Measured value (as_of) | Direction | Failure mode |
|---|---|---|---|
| OpenRouter tokens/day | **127.8T (2026-09-14)**, from 5.3T a year earlier | **UP ~24x** | One aggregator; mix skews to cheap agent traffic; not revenue |
| OpenRouter top models | deepseek-v4.1-flash 14.6T tok (2026-09-19) | UP | Volume leaders are the *cheapest* models — token growth ≠ revenue growth |
| Spend concentration | Code+Agent = 59% of spend (30d) | — | Single-workload concentration; most substitutable demand |
| **ERCOT grid load** | **2026 Jan–Aug mean 58,978 MW, +5.8% YoY; Aug +10.1%** | **UP, accelerating** | Weather + population + crypto confounded; not AI-attributable alone |
| Epoch installed GPU capacity | **4,453,918 H100e at 2025-10**, from 813,913 at 2024-01 | **UP 5.5x** | Estimate w/ percentile band; series incomplete after 2025-10 |
| Epoch AI-usage polling | **21.9% daily users; 46.7% of work usage on FREE plan** | UP usage / weak monetisation | Self-reported; free-plan share caps revenue conversion |
| H100 on-demand (vast.ai) | $1.469 min / $5.145 median (2026-09-20) | — | Snapshot only, no history |
| H100 on-demand (RunPod) | **$2.69 Community / $3.49 Secure** (2026-09-20) | — | List price, not cleared price |
| H100 spot (vast.ai) | $0.447 min / $4.000 median (2026-09-20) | — | 3.3x discount normal in two-tier markets; alone ≠ oversupply |
| B200 on-demand | vast.ai $6.252 min; RunPod $5.98 Community | — | Newest silicon scarce; wide dispersion |
| **Inference price index (Epoch)** | **−50x/yr median, range 9x–900x** | **DOWN, steep** | List prices, not transaction; ambiguous deflation vs price-cutting |
| ERCOT large-load queue | **not found (404s); PJM 401** | — | The most direct demand signal; behind portals |
| EIA-930 hourly load | **403 API_KEY_MISSING** | — | Needs one free key; then strongest power proxy |
| DC vacancy/pre-leasing | **404 / 403** | — | Paywalled + bot-gated; confirmed gap |

### The honest bottom line for the lens
**Measured compute USAGE is growing very fast (OpenRouter ~24x/yr; ERCOT load +5.8%/yr and
accelerating; installed GPU capacity 5.5x), while the price of a unit of inference is falling
very fast (Epoch, ~50x/yr median).** Those together are the signature of **elastic demand
meeting steep deflation** — the "real buildout" reading, NOT the "empty shells" reading. If
utilization were the problem you would expect *volume to stall* while prices fell; instead
volume compounds while prices fall.

**Two caveats keep this from being a clean all-clear:**
1. **The free-plan finding (46.7% of work usage monetises nothing) is direct evidence that a
   large share of real usage does not yet convert to revenue** — the capex/revenue bridge is
   real but leaky.
2. **Falling prices remain ambiguous** between deflation and capacity-filling price cuts.

**What still could NOT be measured:** actual utilization *rate* (we have capacity-built from
Epoch and volume-from-one-aggregator from OpenRouter — not the ratio), data-centre occupancy
(paywalled), and grid-load attribution to AI (weather-confounded, needs sectoral breakdown).
**Three of the most direct utilization measures are blocked, and that is the honest state of
this lens.** The measures that DO work point the same way: usage strongly up, unit price
steeply down.

---

## 8. FOLLOW-UPS RANKED BY VALUE-PER-EFFORT

1. **Register one free EIA key** → unlocks EIA-930 hourly region load (ERCO/PJM). Highest-value single action.
2. **Navigate ERCOT MIS / PJM Data Miner portals deliberately** → find the large-load interconnection queue; the most direct "data centres actually connecting" measure (PJM needs a key: 401 above).
3. **Add a poll-and-store job for vast.ai `min_bid`/`dph_total`** → builds the missing spot-price history; no provider API has a series.
4. **Use the stdlib xlsx reader (verified here) on Census C30 / EIA-860M / ERCOT** → removes the `openpyxl` dependency entirely.
5. **Try CBRE/JLL data-centre reports through a real browser session** → vacancy/pre-leasing, currently bot-gated.
6. **Poll RunPod / Lambda / Nebius list prices on a schedule** → cross-vendor on-demand trend; RunPod's page needs no key.
7. **Get the sectoral ERCOT load breakdown** (data-centre vs residential vs industrial) to de-confound §4b.

---

*Recon by L2 (utilization lens). All endpoints fetched live from this host 2026-09-20.
No value in this document is estimated, inferred, or carried over from another date.*
