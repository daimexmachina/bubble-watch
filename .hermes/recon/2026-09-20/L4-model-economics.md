# L4 — Model economics & competitive substitution (reconnaissance)

**Lens:** what could CAP revenue growth rather than only inflate cost.
**Retrieved:** 2026-09-20 (UTC), from this host. Every URL below was actually called; status and measured value are recorded as returned.
**House rule honoured:** a source that failed is recorded as a GAP. Nothing here is imputed, defaulted or recalled from memory. Where a number is a *derived* quantity I computed from fetched bytes, it says so.

---

## 0. Headline findings (what changes the model)

1. **`frontier_gap` can be made LIVE with a 589 KB keyless fetch.** LMArena publishes its whole leaderboard as parquet on HuggingFace, refreshed **daily**, and the latest text/overall slice reproduces the committed fixture **exactly** (gap **+32.5 Elo**, 2026-09-13). No scraping, no key, no browser.
2. **`frontier_premium` can be made LIVE, and it exposes a per-token price split by open/closed.** `openrouter.ai/api/v1/models` returns 446 models with pricing; models carrying a `hugging_face_id` (i.e. weights published) have a **median input price of $0.20/M vs $1.00/M for closed — a 5.0x median premium**, measured live.
3. **OpenRouter exposes daily per-model TOKEN VOLUME, publicly and keyless.** `openrouter.ai/api/frontend/v1/rankings/models` returns prompt/completion tokens and request counts per model per day. On 2026-09-19: **127.3 trillion tokens in one day**, of which **70.4% went to open-weight models** and **9.0% flowed through the `free` variant**. This is the single most direct measurement of "is intelligence becoming free" available anywhere, and it is live today.
4. **A free historical archive of published inference prices exists** — the LiteLLM pricing file's git history. A like-for-like basket of **200 models priced in all six snapshots** shows median input price **$1.00/M (2024-05-31) → $0.70/M (2026-09-20)**, i.e. **1.43x cheaper over 2.31 years = 14.3%/yr**. Slower than the "10x/year" folklore, and the difference matters.
5. **Epoch AI is a full live data platform, not a page to scrape.** Model database, ECI capability scores, per-benchmark series and a **price-at-fixed-performance series** all download as plain CSV/ZIP, keyless, CC-BY. Their price insight is the best capability-and-cost series found — with one hard limitation (see §3.4).
6. **Model-release cadence and training-run counts are free and live** (Epoch `ai_models.zip`, arXiv API `totalResults`).
7. **Artificial Analysis is a dead end without a key.** Its API v2 returns 401 and the HTML is JS-rendered. Do not build on it.

---

## 1. OpenRouter — live, and far richer than the fixture assumes

### 1.1 Catalogue (the `frontier_premium` source)

| | |
|---|---|
| **URL** | `https://openrouter.ai/api/v1/models` |
| **Status** | `HTTP 200`, 737,913 bytes, 0.16 s |
| **Retrieved** | 2026-09-20 |
| **Fields** | `id, canonical_slug, hugging_face_id, name, created, architecture, pricing{prompt,completion,input_cache_read,…}, top_provider, context_length, expiration_date, links.details` |

`total_count = 446` models. Measured split and price distribution (**input price, USD per 1M tokens**, derived by ×1e6 on the published per-token string):

| group | n priced | median | mean | p10 | p90 | min | max |
|---|---|---|---|---|---|---|---|
| **open-weight** (`hugging_face_id` present) | 168 | **$0.2000** | $0.3661 | $0.0500 | $0.8000 | $0.0170 | $3.000 |
| **closed** (no `hugging_face_id`) | 249 | **$1.0000** | $3.0928 | $0.1250 | $5.0000 | $0.0250 | $150.000 |
| all | 417 | $0.5000 | $1.9943 | $0.0750 | $3.0000 | $0.0170 | $150.000 |

- **Median open/closed input-price ratio = 5.0x.** This is a *level* measure, not a time series, so it is a cross-section as of the fetch — it says what the market charges today, not how the charge changed.
- **24 of 446 models (5.4%) are priced at exactly $0.00 prompt.** Of the 186 open-weight entries, **18 are free (9.7%)**; of 260 closed, 6. The `:free` suffix convention is real and countable.
- 168/446 (37.7%) of catalogue entries carry an HF repo — i.e. an open weight release exists for them.

**Failure modes (state these in the indicator output):**
- **Mix shift, not price cuts.** The median can fall because cheap models are *added*, not because anything got cheaper. This endpoint has no history and cannot distinguish the two.
- **Promotional and router pricing.** OpenRouter is a reseller/aggregator; `pricing` is the routed price including its own margin, and `overrides` blocks show the same model priced differently by time-of-day (e.g. `deepseek/deepseek-v4.1-flash` is $0.15/$0.60 weekday-off-peak and $0.30/$1.20 in one peak window). A single fetch can land inside a promo.
- **`hugging_face_id` ≠ "open weights".** It is a *link* to an HF repo. A provider may link a repo that is gated, or a base model rather than the served checkpoint. It is the best available free proxy and it is not the same thing.
- **`created` is the OpenRouter listing date**, not the model release date. Using it as a release series measures listing behaviour.
- **One endpoint per model** — `https://openrouter.ai/api/v1/models/{canonical_slug}/endpoints` returns `HTTP 200`, 1,467 bytes, and gives **per-provider** prices for a single model (provider name, quantization, context, `pricing`). Useful for a like-for-like single-model series; one call per model, so not for all 446 on a Pi in one pass.

### 1.2 THE FINDING: public daily token-volume endpoint (new capability)

| | |
|---|---|
| **URL** | `https://openrouter.ai/api/frontend/v1/rankings/models` |
| **Status** | `HTTP 200`, 328,773 bytes |
| **Retrieved** | 2026-09-20 |
| **Auth** | none (keyless, no header needed) |

Returns 573 rows over **7 daily dates, 2026-09-13 → 2026-09-19** (544 rows on the latest day). Per row: `date, model_permaslug, variant, total_prompt_tokens, total_completion_tokens, total_native_tokens_reasoning, total_native_tokens_cached, total_tool_calls, count (requests), …`.

**Measured for 2026-09-19**, with each slug joined to the catalogue's `hugging_face_id` flag (`base-slug` on the trailing `-YYYYMMDD`):

| metric | value |
|---|---|
| total tokens that day | **127,307 bn (127.3 trillion)** |
| open-weight share of tokens | **70.4%** (155 distinct models) |
| closed share of tokens | 29.0% (171 models) |
| unclassified | 0.63% |
| `free`-variant share of tokens | **9.04%** (29 rows) |
| variants present | standard 479, batch 36, free 29 |

Top of the leaderboard, 2026-09-19:

| model | share | class |
|---|---|---|
| `deepseek/deepseek-v4.1-flash-20260910` | 11.45% | open |
| `z-ai/glm-5.3-flash-20260826` | 10.18% | open |
| `openai/gpt-5.6-luna-20260709` | 9.77% | closed |
| `tencent/hy4-preview-20260827` | 9.37% | open |
| `deepseek/deepseek-v4-flash-20260731` | 7.67% | open |
| `xiaomi/mimo-v2.5-20260422` | 5.64% | open |
| `tencent/hy3-20260706` | 3.73% | open |
| `nvidia/nemotron-3-ultra-550b-a55b-20260604` | 3.29% | open (`free` variant) |
| `deepseek/deepseek-v4-flash-20260423` | 3.04% | open |
| `z-ai/glm-5.3-20260816` | 2.33% | open |

Also live and keyless: `https://openrouter.ai/api/frontend/v1/rankings/apps` (`HTTP 200`, 27,873 B) — the same shape aggregated by downstream app, with `total_tokens`, `total_requests`, `rank`.

**Why this matters for the bubble question.** The model's premise is that capex is repaid out of AI revenue. This endpoint measures the *volume* flowing through the paid API channel and the *price class* it flows through. **70% of all tokens and 9% of all tokens on free variants is a direct measurement of revenue-per-token dilution**: if usage migrates to models that are open-weight and near-free, then aggregate token growth can be enormous while billed revenue grows far more slowly. State the direction: **rising open-weight share and rising free-variant share are revenue-headwinds.**

**Failure modes (severe — this is a router, not the market):**
- **OpenRouter is a reseller slice, not the industry.** Most enterprise inference never touches it. Share *within* OpenRouter is not share of the market; it is share of the portion of the market that shops on an aggregator, which biases *toward* price-sensitive and open-weight usage. Do not present 70.4% as "70% of AI usage".
- **Only 7 daily points are served, and the 6 older days are near-empty** (1, 4, 10, 3, 1, 10 rows vs 544). The endpoint appears to publish a **rolling 7-day window in which only the newest day is complete**. A daily cron on this endpoint will therefore build a genuine series, but the *backfill is not available* — the first run is the first observation. Query parameters (`limit`, `days=90`, `start=/end=`, `date=`, `period=month`) were all probed and **return byte-identical output**, so they are ignored. Do not imply 90 days of history.
- **`variant == "free"` is a promotional/rate-limited tier**, not a durable price. It is a mix-shift signal and cannot be read as a unit-price measure.
- **Cache reads are separate.** `total_native_tokens_cached` exists; uncached tokens are the revenue-bearing ones, and the token totals above are not cache-adjusted.

### 1.3 Endpoints probed and NOT available (recorded gaps)

| URL | status | note |
|---|---|---|
| `openrouter.ai/api/v1/models?category=top-weekly` | **400** | ZodError; only these category values are accepted: `programming, roleplay, marketing, marketing/seo, technology, science, translation, legal, finance, health, trivia, academia` |
| `openrouter.ai/api/v1/models?category=programming` | **200**, 43,501 B | 20 models, full pricing — a *curated* subset, not a ranking, and not usage-weighted |
| `openrouter.ai/api/v1/models?category=roleplay` | **200**, 42,328 B | same shape |
| `openrouter.ai/api/frontend/models`, `…/stats/endpoint`, `…/v1/models`, `…/v1/analytics/models` | **404** | do not exist |
| `openrouter.ai/api/v1/analytics/tokens` | **404** | no keyed analytics endpoint under this path |
| `openrouter.ai/rankings` | **200**, 2,357,160 B | HTML; **no embedded token numbers** (0 matches for `totalTokens`/`share`/`tokens` JSON fields) — the data arrives from the frontend JSON endpoint above, so **fetch the JSON, never the page** |

`openrouter.ai/api/frontend/models` returning 404 while `…/v1/rankings/models` returns 200 confirms the frontend API is versioned under `/v1/` and unversioned guesses will 404.

---

## 2. LMArena — `frontier_gap` can be made LIVE, daily, keyless

The fixture `tests/fixtures/arena_frontier_gap.json` says it was *"Extracted once with pyarrow and the RESULT committed"* citing `text/full` parquet. That extraction can be automated with no key and a small payload.

| | |
|---|---|
| **Dataset API** | `https://huggingface.co/api/datasets/lmarena-ai/leaderboard-dataset` → `HTTP 200`, 21,586 B |
| **File tree** | `https://huggingface.co/api/datasets/lmarena-ai/leaderboard-dataset/tree/main/text?recursive=true` → `HTTP 200` |
| **Latest slice** | `https://huggingface.co/datasets/lmarena-ai/leaderboard-dataset/resolve/main/text/latest-00000-of-00001.parquet` → `HTTP 200`, **588,920 bytes** |
| **Full history** | `…/text/full-00000-of-00001.parquet` → `HTTP 200`, **56,502,664 bytes**, 2.3 s |
| **Licence** | `cc-by-4.0` |
| **Freshness** | `lastModified 2026-09-16T03:02:16Z`; commit list shows **daily** uploads (`Update agent … for 2026-09-15` etc.); 40,104 downloads |

Columns: `model_name, organization, license, rating, rating_lower, rating_upper, variance, vote_count, rank, category, leaderboard_publish_date`.

**Live reproduction of the committed fixture** (from the 589 KB `latest` slice, `category == 'overall'`, `vote_count >= 100`):

```
closed  claude-fable-5.1-max   1507.6  license=Proprietary   votes=5783
open    glm-5.3-max            1475.1  license=MIT           votes=10960
LIVE GAP = 1507.6 - 1475.1 = +32.5 Elo      leaderboard_publish_date = 2026-09-13
```

The committed fixture's last point is `{date: 2026-09-13, best_proprietary: 1507.6, best_open: 1475.1, gap: 32.5, …}`. **Exact match on gap and on both model identities**, so the refresh path is validated end to end.

**Verdict: `frontier_gap` should be refreshed from the 589 KB `latest` parquet daily (Pi-friendly), with the 56 MB `full` fetched only when the whole 243-point series must be rebuilt.** `latest` is enough for a new observation; `full` is needed to bootstrap.

**Failure modes:** licences are the leaderboard's own `license` string, and my split (any licence other than `Proprietary`/`Other` = open) is a *definition* — `'Kimi K3 license'`, `'NVIDIA Open Model'`, `'Modified MIT'` and `'CC-BY-NC-4.0'` are not all open weights in any legal sense, and `CC-BY-NC` is explicitly non-commercial. `glm-5.3-max` is MIT and genuinely open; the 2nd-place open model carries a bespoke licence. Report the split you used and the licence strings of the two models being compared.

Alternative: `https://lmarena.ai/leaderboard` → **HTTP 200 but redirects to `arena.ai/leaderboard`, 5,120,935 B of HTML** with no structured payload. **Do not scrape it** — the parquet is smaller, typed and licensed.

---

## 3. Epoch AI — the strongest free capability-and-cost platform found

Everything below is keyless, CC-BY 4.0, and downloads as CSV/ZIP. Note the traps: several plausible URLs 404 while returning a 58 KB Astro **error page** with status-shaped content, so check status codes, not bodies.

### 3.1 Landing pages and the real download paths

| URL | status | bytes | note |
|---|---|---|---|
| `https://epoch.ai/data` | 200 | 102,759 | index; lists every download |
| `https://epoch.ai/data/ai-models.csv` | **404** | 58,250 | **DOES NOT EXIST** — 404 body, not a CSV |
| `https://epoch.ai/data/notable_ai_models.csv` | 200 | **2,254,963** | the one flat CSV link on the site |
| `https://epoch.ai/data/ai_models.zip` | 200 | 3,335,497 | **the real bundle** |
| `https://epoch.ai/data/ai_companies.zip` | 200 | 43,636 | |
| `https://epoch.ai/data/benchmark_data.zip` | 200 | 2,293,470 | |
| `https://epoch.ai/data/ml_hardware.zip` | 200 | 29,498 | |
| `https://epoch.ai/data/gpu_clusters.csv` | 200 | 304,469 | |
| `https://epoch.ai/data/arxiv_trends_monthly.csv.gz` | 200 | 284,612 | 73,606 rows |
| `https://epoch.ai/data/ai-data-centers` | 200 | 746,072 | data-centre directory, per-site pages |

`ai_models.zip` contains: `notable_ai_models.csv` (8,431 rows), **`frontier_ai_models.csv` (1,671 rows)**, `large_scale_ai_models.csv`, **`all_ai_models.csv` (3,617 records)**, README (citation = `Epoch AI, 'Data on AI Models'`).

### 3.2 Capability: Epoch Capabilities Index (ECI) — free, and an independent check on the LMArena gap

`benchmark_data.zip` → `epoch_capabilities_index/eci_scores.csv` (**267 rows, 266 models, 2023-02-24 → 2026-09-03**) with `eci, eci_ci_low, eci_ci_high, date, Organization, Model accessibility, Accessibility group, model_versions`. Accessibility groups count: **Open weights 126, Closed weights 118, Other 22**.

Best closed vs best open per quarter (computed here from that CSV):

| quarter | best closed | best open | gap (ECI) |
|---|---|---|---|
| 2023-03 | GPT-4 (Mar 2023) 125.9 | Cerebras-GPT-13B 82.6 | +43.3 |
| 2024-02 | Claude 3 Opus 126.9 | Gemma 7B 111.8 | +15.1 |
| 2025-01 | o3-mini 140.3 | DeepSeek-R1 139.0 | **+1.4** |
| 2025-08 | GPT-5 150.0 | gpt-oss-120b 140.1 | +9.9 |
| 2026-02 | GPT-5.3 Codex 156.6 | Qwen3.5 397B-A17B 147.0 | +9.6 |
| 2026-04 | GPT-5.5 Pro 162.2 | Kimi K2.6 151.0 | +11.2 |
| 2026-07 | Claude Opus 5 162.3 | Kimi K3 157.6 | **+4.7** |
| 2026-08 | Gemini 3.7 Flash 157.4 | DeepSeek V4 Pro 0813 155.5 | **+2.0** |

Cross-check against the independent ECI-gap chart slice (`/data/charts/open-closed-eci-gap/benchmarked_models.csv`, 643 rows, 396 with a numeric ECI, `HTTP 200`, 146,144 B): same shape, **2026-04 closed GPT-5.5 Pro 159.3 vs open Kimi K2.6 151.6 = +7.7**, and Epoch's own published insight title (fetched 2026-09-20) is **"Open models lag state-of-the-art closed models by 4 months"** (Data Insight, May 29 2026): *"Since January 2026, the most capable open-weight models have lagged frontier closed models by an average of four months in the ECI… The average ECI gap was 8 points, similar to the gap between GPT-5 and GPT-5.5."*

**This is the important disagreement to surface.** LMArena (human preference) says the gap is **+32.5 Elo**; Epoch ECI (benchmark-aggregated) says it is a **4-month lag worth ~2–11 ECI points**. Both are free and live. Two sources measuring "the moat" with different methods that do **not** agree is exactly the L9 multi-method principle this model already claims — and reporting both is more honest than picking one.

### 3.3 Inference price at fixed performance — THE cost series

| | |
|---|---|
| **Page** | `https://epoch.ai/data-insights/llm-inference-price-trends` → `HTTP 200`, 780,071 B |
| **Data (lowest price models)** | `https://epoch.ai/data/charts/llm-inference-price-trends/lowest_price_models_data.csv` → `HTTP 200`, 11,563 B, **119 rows** |
| **Data (all models)** | `…/other_models_data.csv` → `HTTP 200`, 26,624 B, 372 rows |
| **Published insight** | Mar 12 2025 — *"the price to achieve GPT-4's performance on a set of PhD-level science questions fell by 40x per year. The rate of decline varies dramatically depending on the performance milestone, ranging from 9x to 900x per year."* |

Method: for a benchmark and a fixed performance threshold, take the **cheapest model on the market that clears it**, and follow that price over time. Columns: `Benchmark, Threshold model, Performance range, Model Name, Release Date, USD per 1M Tokens, Predicted log price, Benchmark score`.

Measured here from the 119 fetched rows (first → last observation per benchmark/threshold):

| benchmark, threshold | n | first $/M | last $/M | years | x cheaper | %/yr |
|---|---|---|---|---|---|---|
| MMLU ≥43.9 | 9 | 60.00 | 0.070 | 2.87 | 857.1x | 90.5 |
| MMLU ≥86 | 5 | 37.50 | 0.175 | 1.90 | 214.3x | 94.1 |
| GPQA Diamond ≥33 | 6 | 37.50 | 0.122 | 1.75 | 306.1x | 96.2 |
| HumanEval ≥67 | 7 | 37.50 | 0.100 | 1.36 | 375.0x | 98.7 |
| LMSys Arena ELO ≥1186 | 6 | 37.50 | 0.070 | 1.56 | 535.7x | 98.2 |
| MATH 5 ≥23 | 5 | 37.50 | 0.122 | 1.50 | 306.1x | 97.8 |
| MATH-500 ≥44 | 6 | 3.25 | 0.070 | 1.31 | 46.4x | 94.7 |

**THE HARD LIMITATION, and it is disqualifying for a live indicator as it stands: the series STOPS at `2025-02-05` (Gemini 2.0 Flash at $0.175/M).** The newest row in the fetched CSV is `2025-02-05`; nothing from 2026. The "Predicted log price" column is a fitted trend line, not data, and extrapolating it is exactly the imputation this project forbids. Also note the published 9x–900x/yr range is *per-year rates on the fitted trend*, and my endpoint-to-endpoint numbers (90–99%/yr) are a different, cruder quantity: they are first-to-last of the observations, not the fit. **Do not quote either as a current rate.**

So: Epoch gives a **frozen** price-at-fixed-performance series and a **live** model/capability database. It does **not** publish a live inference-price series. That gap is filled by §5.

### 3.4 Cadence, training compute, and training cost

- **Model releases by year** — `https://epoch.ai/data/charts/models-by-release-year/models_by_release_year.csv` → `HTTP 200`, **87 bytes**: `2019:1, 2020:2, 2021:10, 2022:31, 2023:119, 2024:168, 2025:122`. Page note: *"Updated Nov. 24, 2025."* This is a **static snapshot**, not a live series.
- **Live equivalent, recomputed here** from `all_ai_models.csv` (3,617 records) by publication year: 2016:92, 2017:91, 2018:98, 2019:168, 2020:126, 2021:210, 2022:233, 2023:525, **2024:938**, 2025:578, **2026:155 (partial year)**. Different definition, different numbers — the site's chart counts models above a compute threshold; `all_ai_models` counts every tracked model. **Do not mix them in one series.**
- **Open-weights share of newly published models, computed here from `all_ai_models.csv`** (`Open model weights? == yes`): 2020 40.5%, 2021 37.6%, 2022 45.1%, 2023 51.2%, 2024 43.0%, **2025 52.6%**, 2026 38.7% (partial). Direction: **flat-to-slightly-up, not a surge** — worth saying out loud against a narrative of accelerating open-weight proliferation.
- **Training compute cost** — `https://epoch.ai/data/charts/cost-trend-large-scale/large_scale_models_training_cost.csv` → `HTTP 200`, 4,012 B: `Model, Domain, Training compute (FLOP), Training compute cost (2023 USD), Organization, Publication date`, beginning `GNMT 6.6e21 $200k (2016-09-26)`. Constant 2023 dollars, so the series is deflated — usable as a cost-per-training-run trend.
- **Training-run counts** — `frontier_ai_models.csv` (1,671 rows) has `API prices` populated for only **16 rows** (e.g. Claude 3 Opus `$15 / 1M input, $75 / 1M output`; GPT-4 Mar-2023 `$30 / M in (≤8k), $60 / M out`). **This is a hand-curated, sparse field — not a price series.** It is, however, the only free source found that ties a *published API price* to a *named model with a training-compute estimate*, which makes it the right place to anchor "price of the model that costs $X to train".
- **`FLOP/$` column** exists and is populated — e.g. `Claude 2` 7.885e17, `GPT-3 175B` 1.483e17. Note `GPT-4 (Jun 2023)` returns `Infinity` and others `NaN`: **guard for non-finite values before scoring.**
- **GPU frontier lifespan** — `…/gpu-frontier-lifespan/gpu_frontier_lifespan.csv` → `HTTP 200`, 776 B: `NVIDIA V100 2017-06-21 → 2021-12-22 (4.5 yr, 9 models)`, `A100 2020-03-01 → 2024-04-15 (4.1 yr)`, `TPU v4 2021-05-20 → 2024-02-14 (2.7 yr)`. **Directly relevant to the depreciation-subsidy indicator [L4]** — it is an independent, free, measured estimate of *useful* accelerator life.
- **Context windows** — `…/context-windows/context_data.csv` (9,002 B), live.
- **Hyperscaler capex vs cash flow** — `…/hyperscaler-capex-vs-cash-flow/ocf_vs_capex_log_data.csv` (808 B) — Epoch already publishes the series this model computes from EDGAR, so it is a **free independent cross-check** on `capex_vs_cashflow`.
- **AI company revenues** (from `ai_companies.zip`, 68 rows) — `Annualized revenue (USD)`, `Source 1/2/3`, `Confidence`: Anthropic $9.0bn (2025-12-31) → $14.0bn → $19.0bn → $30.0bn → **$47.0bn (2026-05-15)** → **$65.0bn (2026-07-31)**; OpenAI $21.4bn (2025-12-31) → $25.0bn → **$40.0bn (2026-08-13)**; Mistral $1.0bn ARR (2026-09-08, "Confident"); Z.ai $1.6bn ARR. **This is a free revenue series for the companies the capex must be repaid by — the denominator side of the entire thesis — and it is the single most valuable item found in this recon.**
- **AI company usage** (`ai_companies_usage_reports.csv`, 49 rows; only 12 carry `Daily tokens`, newest 2025-10-29 Google Gemini API 10 trillion/day): **too sparse and too stale to be a series. Recorded as a GAP.**

### 3.5 Epoch gaps recorded

- No live inference-price series (§3.3 stops 2025-02-05).
- `/data/ai-models.csv`, `/data/notable-ai-models.csv`, `/data/ai_models.csv` → **404** (Astro error page, 58,250 B). Use the zip.
- `https://raw.githubusercontent.com/epoch-research/epoch-ai/main/README.md` and `https://api.github.com/repos/epoch-research/epoch-ai-data` → **404**. No public GitHub mirror found.
- Epoch's chart data lives at `/data/charts/<topic>/<file>.csv` — discoverable **only** from the insight page HTML, and the sitemap index is `https://epoch.ai/sitemap-index.xml` (200) → `sitemap-data-insights-0.xml` (200, 99 insight pages listed).

---

## 4. Stanford HAI AI Index 2026 — reachable, but a document, not a dataset

| URL | status | note |
|---|---|---|
| `https://hai.stanford.edu/ai-index/2026-ai-index-report` | 200, 414,125 B | landing page |
| `https://hai.stanford.edu/assets/files/ai_index_report_2026.pdf` | **200, 37,885,501 B**, PDF 1.7, 425 pages | **the real report** |
| `https://aiindex.stanford.edu/wp-content/uploads/2026/04/HAI_2026_AI-Index-Report.pdf` | **403** | guessing the path fails |
| `https://hai.stanford.edu/assets/files/hai_ai_index_report_2026.pdf` | **404** | ditto |
| `https://aiindex.stanford.edu/data/` | 200 → **redirects to `hai.stanford.edu/ai-index`** | old domain is dead |

Extracted locally with `pdftotext` (907,440 characters). Content grep results — **`price` appears 2×, and NEITHER is a model price** (a humanoid robot and a scam site). `inference` appears 21×, all about *energy and water*, not price.

What it *does* give, quotable with page provenance:
- *"AI data center power capacity rose to 29.6 GW, comparable to New York state at peak demand"*; GPT-4o inference water use *"1.3 to 1.6 million kiloliters"* annually.
- Inference **energy** per medium-length prompt (Jegham et al. 2025, ~1,000 in / 1,000 out tokens): DeepSeek V3.2 Exp and V3.2 highest at **23 Wh**, GPT-5 (high) 21.9 Wh; Claude 4 Opus and Mistral Medium 3 lowest at 1.6 and 1.5 g CO₂e.
- *"Industry produced over 90% of notable frontier models in 2025"*; SWE-bench Verified 60% → near 100% in one year; organizational adoption 88%.
- *"The U.S.-China AI model performance gap has effectively closed."*

**Verdict: annual, PDF-only, no CSV, no API, no per-token price series. High value as a citation and as an energy-per-inference source; NOT usable as a live indicator. Downloading 37.9 MB on a Pi and parsing 425 pages is a poor trade for a daily run.** A `hai.stanford.edu/ai-index/data` page exists (200, 73,735 B) but exposes **no** CSV/JSON links.

---

## 5. FREE HISTORICAL INFERENCE-PRICE ARCHIVE — found, and it works

This is the answer to "does a free archive of provider pricing exist". It does, via git history on a machine-readable price file maintained by a third party.

| | |
|---|---|
| **File** | `https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json` → `HTTP 200`, **2,822,093 B**, **4,326 entries** |
| **Archive** | GitHub commits API filtered by path: `https://api.github.com/repos/BerriAI/litellm/commits?path=model_prices_and_context_window.json&until=<ISO>&per_page=1` → `HTTP 200` |
| **Then** | `https://raw.githubusercontent.com/BerriAI/litellm/<sha>/model_prices_and_context_window.json` |
| **Fields** | `input_cost_per_token`, `output_cost_per_token`, per provider, keyed by model name |

Five dated snapshots actually fetched (sha + author date as returned):

| requested `until` | resolved commit | entries |
|---|---|---|
| 2024-06-01 | 2024-05-31 (`668c894e9a`) | 410 |
| 2025-02-28 | 2025-02-27 (`ff553fedf8`) | 805 |
| 2025-08-30 | 2025-08-29 (`607f425cc6`) | 1,441 |
| 2026-03-01 | 2026-03-01 (`13e74dd389`) | 2,610 |
| 2026-07-31 | 2026-07-30 (`3e4669dbc5`) | 2,968 |
| live main | 2026-09-20 | 4,326 |

**Like-for-like basket: 200 models with a numeric input price in ALL six snapshots** (this removes new-model mix shift by construction):

| snapshot | 2024-05-31 | 2025-02-27 | 2025-08-29 | 2026-03-01 | 2026-07-30 | 2026-09-20 |
|---|---|---|---|---|---|---|
| median $/1M input | **1.0000** | 0.9995 | 0.8000 | 0.8000 | 0.8000 | **0.7000** |
| mean $/1M input | 4.3260 | 4.0785 | 4.0313 | 4.0838 | 4.0807 | 4.0023 |
| median index | 100.0 | 100.0 | 80.0 | 80.0 | 80.0 | **70.0** |

**Measured: median like-for-like input price fell from $1.00/M to $0.70/M over 2.31 years — a factor of 1.43, i.e. 14.3% per year.** The mean barely moved (4.33 → 4.00) because the frontier end of the basket repriced far less than the middle. **The gap between the mean and the median is the finding: price declines are concentrated in the cheap half of the market, while the expensive frontier holds its price.** That is exactly the shape that lets token volume explode without revenue following.

Per-model examples across the five snapshots (USD per 1M input tokens; `0.000` = the name is absent from that snapshot, which for a *retired* name is itself a survivorship signal):

| model | 2024-06 | 2025-02 | 2025-08 | 2026-03 | 2026-07 |
|---|---|---|---|---|---|
| `gpt-4o` | 5.000 | 2.500 | 2.500 | 2.500 | 2.500 |
| `gpt-3.5-turbo` | 1.500 | 1.500 | 1.500 | 0.500 | 0.500 |
| `claude-3-5-sonnet-20240620` | — | 3.000 | 3.000 | 3.000 | — (dropped) |
| `mistral/mistral-large-latest` | 4.000 | 2.000 | 2.000 | 2.000 | 0.500 |
| `gemini/gemini-1.5-flash` | — | 0.075 | 0.075 | 0.075 | 0.075 |

Also measured: of the **513** model keys present in both the 2025-02-28 and current snapshots, **46 had a changed published price** — i.e. **91% were unchanged**. Within those 46, moves run both ways (down: `mistral/mistral-small` x10.0, `openrouter/gryphe/mythomax-l2-13b` x23.4, `sambanova/Meta-Llama-3.2-1B-Instruct` x10.0; **up**: `openrouter/mistralai/mixtral-8x22b-instruct` x0.33 meaning 3x *more expensive*, `ft:davinci-002` x6 more expensive, `eu.anthropic.claude-3-5-haiku` x3.2 more expensive). **Price is not monotone — a one-directional "costs always fall" indicator would be wrong.**

**Failure modes:**
- **Third-party curation.** LiteLLM's file is maintained for a routing library, not as an economic series. Keys appear, get renamed and disappear; provider-prefixed duplicates (`azure/gpt-4o` vs `gpt-4o` vs `openrouter/openai/gpt-4o`) price the same model differently. The like-for-like basket is the only defensible read.
- **Survivorship, in both directions.** Models that were *discontinued* leave the file, so the basket is biased toward models still on sale — and a surviving model at an unchanged price is not evidence that prices held.
- **It is a published list price, not a transacted price.** Enterprise discounts, committed-use pricing, batch and cache tiers are invisible.
- **GitHub API is unauthenticated** — 60 requests/hour per IP. A full rebuild (5 commits + 5 raw fetches ≈ 10 calls) fits; do not loop it. Unauthenticated raw.githubusercontent fetches are not rate-limited the same way.
- **A 2.8 MB JSON is a real download on a Pi.** Parse streaming and keep only `input_cost_per_token`.

Other price sources probed and **not** available: `https://raw.githubusercontent.com/simonw/llm-prices/main/prices.json` → **404** (the repo README is 200 but the data path is different — not pursued further). Artificial Analysis (below) is keyed.

---

## 6. Artificial Analysis — DEAD END without a key

| URL | status | note |
|---|---|---|
| `https://artificialanalysis.ai/leaderboards/models` | **200, 2,690,422 B** | JS-rendered; **0 matches** for `"price"`, `input_price`, `pricePerMillion`, `USD per 1M` in the HTML |
| `https://artificialanalysis.ai/api/v2/data/llms/models` | **401**, 31 B — `{"error":"API key is required"}` | |
| `…?api_key=demo`, `…?apikey=x` | **401** | no free tier; the parameter name is not the credential |
| `https://artificialanalysis.ai/api/v2/data/llms/models/docs` | 404 | |
| `https://artificialanalysis.ai/robots.txt` | 200 | `User-Agent: * / Allow: /` — crawling is permitted, but the data is not in the HTML |

**The leaderboard page renders its table client-side and embeds no price JSON.** Browser automation would be required, and it still would not produce history. **Recorded as a GAP: no free, keyless, machine-readable price/performance tracker from Artificial Analysis.**

The Wayback Machine does hold snapshots — CDX for `artificialanalysis.ai/leaderboards/models` returns 200s from **2024-04-07** onward — so a *scrape-based historical* series is theoretically reconstructible. But: (a) my attempts to fetch CDX and `id_` replay URLs drew **HTTP 429 (rate limited)** on the `web.archive.org` host, so I could **not** verify a single snapshot's contents and am **not** claiming they contain price tables; (b) the same 429 hit `archive.org/wayback/available`. **Treat Wayback as an unverified possibility, not a source.** Only this much is measured: **CDX returned HTTP 200 with rows for `openrouter.ai/api/v1/models` starting 20230726** (and 20250904, 20250911, 20250912, 20250916 ×2…), so a Wayback time series of the OpenRouter catalogue is *indexed*; the replay fetches were 429'd and the contents are unverified.

---

## 7. Model release cadence & training-run counts — free and live

**arXiv (keyless, official API)** — `http://export.arxiv.org/api/query?search_query=…` (redirects to `https://`, 200). Monthly submission counts via the Atom feed's `<opensearch:totalResults>`, month ranges as `submittedDate:[YYYYMMDDHHMM+TO+YYYYMMDDHHMM]`:

| month | cs.AI | cs.LG |
|---|---|---|
| 2024-01 | 1,958 | 2,450 |
| 2025-01 | 2,484 | 3,000 |
| 2025-07 | 3,477 | 3,413 |
| 2026-01 | 4,207 | 3,735 |
| 2026-07 | 4,558 | 4,093 |
| 2026-08 | 4,978 | 4,016 |
| 2026-09 (partial, to 09-20) | 2,649 | 2,329 |

**cs.AI grew 1,958 → 4,978 per month (2024-01 → 2026-08), i.e. +154% in 31 months.** Direction for the bubble read is genuinely two-sided: research intensity growing faster than commercial deployment is consistent with *both* a real buildout and with a wave of effort chasing a capital pool. Also: `cs.LG` counts were flat-to-down at the end while `cs.AI` kept rising — **the two categories are not the same phenomenon and must not be summed.**

Failure modes: `totalResults` is a **catalogue count, not a count of distinct papers** (cross-lists are double-counted), the last month of any query is **always partial**, and arXiv's API is rate-limited (`export.arxiv.org` returned a TLS handshake timeout once mid-loop; 1.5–2 s between calls is required). Epoch's own `arxiv_trends_monthly.csv.gz` (284,612 B, 73,606 rows, columns `metric, field, period, population, category, numerator, denominator, is_partial, provenance`) is a pre-aggregated alternative and **carries an explicit `is_partial` flag** — prefer it over scraping the API if the categories line up.

**Epoch model database** (live, §3.4) supplies training-run counts with training-compute estimates and `Frontier model` flags — the closest thing to "count of frontier training runs" available free.

---

## 8. HuggingFace — open-weight dissemination is live but weakly quantifiable

| URL | status | note |
|---|---|---|
| `https://huggingface.co/api/models?sort=trendingScore&direction=-1&limit=30` | 200, 16,351 B | keyless |
| `…?sort=downloads&direction=-1&limit=30` | 200, 18,549 B | keyless |
| `…?filter=text-generation&sort=trendingScore&limit=100` | 200, 318,727 B | |
| `https://huggingface.co/api/models?limit=1000&sort=createdAt` | 200 | **1000 rows span ONE DAY** (2026-09-20 15:51 → 22:34 UTC) — HF create-volume is far too large for this to be a useful frontier-cadence signal |

Measured top-25 trending, 2026-09-20 (fields `id, downloads, likes, createdAt, trendingScore`):

```
prism-ml/Ternary-Bonsai-2-27B-gguf        dl=1,908,396 likes=1480 created=2026-09-16 ts=1374
convaiinnovations/laya                    dl=0         likes=1054 created=2026-09-18 ts=1037
deepseek-ai/DeepSeek-V4.1-Flash           dl=496,684   likes=3423 created=2026-09-10 ts=905
Qwen/Qwen3.8-27B                          dl=7,331,932 likes=15858 created=2026-08-05 ts=622
unsloth/Qwen3.8-27B-GGUF                  dl=6,941,478 likes=4427 created=2026-08-13 ts=326
meta-llama/Llama-3.1-8B-Instruct          dl=5,910,102 likes=7767 created=2024-07-18 ts=213
MiniMaxAI/MiniMax-H3                      dl=4,057,444 likes=5524 created=2026-07-28 ts=208
```

Top-100 trending by organisation: Qwen 4, prism-ml 3, DavidAU 3, Comfy-Org 3, dealignai 3, orcarouter 3, deepseek-ai 2, zai-org 2, openbmb 2… Sum of `downloads` across the top 100 = **374,115,984**.

**Why this is weak:** `downloads` counts download *events* on the HF hub, dominated by quantised community re-uploads (`unsloth/*-GGUF`, `ISTA-DASLab/*-GSQ-GGUF`, a `DavidAU/…Heretic-Uncensored…` GGUF). It measures hobbyist distribution intensity, **not enterprise substitution**, and `downloads=0` with `likes=1054` (convaiinnovations/laya) shows likes and downloads are unrelated scales. `createdAt` is upload time, not release time. **Usable as an attention proxy; do not present it as adoption.**

**Related:** HF dataset search for OpenRouter data returns only third-party conversation datasets (no official OpenRouter usage dataset). `lmarena-ai/leaderboard-dataset` (§2) is the one high-quality, licensed, daily-updated HF artefact in this space.

---

## 9. Verdict per existing indicator, and the direction of each signal

| indicator | can it go LIVE? | free source | direction | failure mode to state |
|---|---|---|---|---|
| **`frontier_premium`** (open-vs-closed price) | **YES** | OpenRouter `api/v1/models` (738 KB, 446 models) | open median $0.20/M vs closed $1.00/M = **5.0x** — a *narrowing* ratio is a revenue headwind | reseller price incl. margin; time-of-day `overrides`; `hugging_face_id` is a link not a licence; `created` is a listing date |
| **`frontier_gap`** (capability moat) | **YES, daily** | LMArena `text/latest` parquet (589 KB) + Epoch ECI CSV | LMArena **+32.5 Elo**; Epoch **~4-month lag / 2–11 ECI pts** — the two DISAGREE | human preference vs benchmarks; single best model each side; bespoke licences inflate "open" |
| **NEW: token-volume dilution** | **YES** | OpenRouter `api/frontend/v1/rankings/models` | **70.4% of 127.3tn daily tokens to open-weight; 9.0% free-variant** — both rising = revenue headwind | OpenRouter slice ≠ market and skews price-sensitive; only a 7-day window, no backfill; `free` is promotional |
| **NEW: like-for-like inference price** | **YES, historical** | LiteLLM file git history + GitHub commits API | **$1.00 → $0.70 /M input median over 2.31 yr = 14.3%/yr**, mean flat | list price not transacted; survivorship; key renames; 60 req/h unauth; 91% of models unchanged — *not* monotone |
| **NEW: open-weight share of releases** | **YES** | Epoch `all_ai_models.csv` | 2025 **52.6%**, 2026 38.7% (partial) — flat, not a surge | definition of "open"; partial final year; differs from Epoch's threshold-based chart |
| **NEW: cadence** | **YES** | arXiv API; Epoch `ai_models.zip` | cs.AI 1,958 → 4,978/month (+154% in 31 mo) | cross-listed papers double-counted; final month partial; cs.AI ≠ cs.LG |
| **NEW: AI revenue** | **YES** | Epoch `ai_companies_revenue_reports.csv` | Anthropic $9.0bn → **$65.0bn** run-rate (2025-12-31 → 2026-07-31); OpenAI $21.4bn → $40.0bn | `Annualized run rate` and `ARR` are self-reported and not the same; `Confidence` column must be carried |
| inference cost curve (provider pages) | **no live series** | Epoch price-at-fixed-performance **frozen 2025-02-05** | 90–99%/yr endpoint-to-end over 2021–25 | fitted "Predicted log price" is a trend, not data — extrapolating it is imputation |
| Artificial Analysis | **NO** | — | — | 401 without key; HTML has no prices |
| Stanford HAI AI Index | **NO (annual PDF)** | 37.9 MB PDF, 425 pp | energy/water per inference, adoption 88%, US–China gap closed | no CSV, no API, `price` appears twice and never as a model price |

---

## 10. Recommended build order (cheapest, highest value first)

1. **`frontier_gap` refresh** — one GET of the 589 KB `text/latest` parquet, pyarrow already validated here. Reproduces the fixture exactly. This retires a fixture at near-zero cost.
2. **OpenRouter daily snapshot** — two keyless GETs (catalogue 738 KB + rankings 329 KB). Persist to a workspace JSONL from day one, because **the rankings endpoint has no backfill**; every day not captured is lost forever. Record `open_weight_token_share`, `free_variant_token_share`, `total_tokens`, `median_open_price`, `median_closed_price`.
3. **AI-company revenue series** from `ai_companies.zip` (44 KB) — the revenue side of the thesis, free, with source URLs per observation.
4. **Like-for-like price basket** from the LiteLLM file's git history — a monthly job, not daily (60 req/h unauth limit). Report median *and* mean separately; their divergence is the signal.
5. **arXiv cadence** + Epoch `all_ai_models.csv` — both trivial, both live, both cheap.
6. Cross-check `capex_vs_cashflow` against Epoch's own `ocf_vs_capex_log_data.csv` (808 B) as an independent arithmetic check.

**Do NOT build on:** Artificial Analysis (keyed), the OpenRouter `/rankings` HTML page (no data), `epoch.ai/data/ai-models.csv` (404), the LMArena `arena.ai` HTML (5 MB, no payload), the HAI PDF (37.9 MB/year), Wayback replay (429 from this host, unverified contents).

---

## 12. Additional live Epoch chart slices found (late, high value)

Discovered from insight pages; all `HTTP 200`, all keyless, all CC-BY, all small enough for a Pi.

### 12.1 Independent cross-check on `capex_vs_cashflow`

`https://epoch.ai/data/charts/hyperscaler-capex-trend/stacked_hyperscalers_trend.csv` → **200, 928 B**, columns `Calendar Quarter, MSFT Broad CapEx ($M), AMZN, GOOG, META, ORCL, Total ($M), Exp Trend ($M)` — **the same five-hyperscaler cohort this model reads from EDGAR**, published quarterly by a third party:

| quarter | total cohort broad capex |
|---|---|
| CY2022 Q1 | $37,620M |
| CY2023 Q4 | $46,485M |
| CY2025 Q1 | $82,301M |
| CY2025 Q4 | **$140,648M** |

**3.74x over 3.75 years.** Caveat on the comparison: Epoch's is "Broad CapEx" (includes finance leases and possibly other items) whereas this model's EDGAR ratio uses reported capex; the ratio of the two series is the only valid comparison, and a level mismatch is expected. It is nevertheless a free, independent arithmetic check on a 14-weight indicator.

### 12.2 The frontier moat at the top of the compute scale (directly relevant to `frontier_premium`)

`https://epoch.ai/data/charts/downloadable/model_accessibility_by_compute.csv` → **200, 245 B**:

| training compute band | downloadable (open weights) | non-downloadable | open share |
|---|---|---|---|
| 10²³ – 10²⁴ FLOP | 51 | 103 | **33.1%** |
| 10²⁴ – 10²⁵ FLOP | 30 | 65 | **31.6%** |
| **Over 10²⁵ FLOP** | **3** | **19** | **13.6%** |

**This is the sharpest available measurement of the moat, and it is a different answer from both LMArena and ECI.** At the very top of the compute scale, only **13.6% of models are open-weight — about a quarter of the rate at lower scales.** So: open models match the frontier *in capability* (ECI gap ~4 months / 2–11 points) but **do not exist at the frontier *in training compute*.** The moat has not closed at the top of the ladder; it has closed at the rung below. Direction for the bubble read: **this is the pro-incumbent side of the argument and should be reported alongside the anti-incumbent LMArena gap**, because a 6-weight indicator whose rationale says "the financed asset is reproducible by anyone" is only half true.

### 12.3 Cost of compute falling (the cost side of the same moat)

`https://epoch.ai/data/charts/chip-performance-per-dollar/chip_perf_per_dollar_quarterly_other_merged.csv` → **200, 1,664 B**, `Quarter, Chip, Performance per dollar (relative to an H100 at its 2025 price), Spending in quarter (billion 2025 USD)`. Latest quarter **2025 Q4**: GB300 **2.334**, GB200 1.764, TPU v6e 5.883, Trainium2 4.381, H100/H200 1.000 (the numeraire). So a 2025-Q4 dollar buys **2.3x** the GB200-class throughput of an H100 at its 2025 price, and 5.9x for TPU v6e. **Falling cost per unit of compute is a tailwind for the buildout's unit economics and simultaneously the mechanism that lets prices fall** — the same fact cuts both ways and the output should say so.

### 12.4 Cost of a 1 GW data centre (physical, named, recent)

`https://epoch.ai/data/charts/ai-datacenter-cost-breakdown/one_gw_dc_upfront_capex.csv` → **200, 129 B**: Servers 21,188 · Facility 11,433 · Network 4,925 · Land 172 · Utility works 164 · **Total 37,883 ($M, i.e. ~$37.9bn)**.
`…/one_gw_dc_capex_opex.csv` → **200, 207 B**: the same plus an **annual opex of $907M** (Energy 594, Taxes 143, Maintenance 120, Labor 40, Water 6).

This is a named, source-backed, physical cost for the unit the whole thesis rests on. **Failure mode:** the two files disagree on the total ($37,883M upfront vs $7,607M in the capex/opex file), which means they are **different scopes** (one appears to be compute-equipment-inclusive, the other a narrower facility scope). My read of the difference is an inference, not a fetched fact — the unit and scope labels are not in the CSVs. **Do not quote either total without resolving the scope from the insight page.**

### 12.5 Other Epoch slices located, not analysed (for other lenses)

`open-weights-vs-closed-weights-models/benchmarked_models.csv` (81,374 B) · `open-models-threshold-insight/open_models_compute.csv` (15,789 B, 310 rows, top-10 open model compute by year from mT5-XXL 8.2e22 in 2020.8) · `1e25-models/1e25_models.csv` (9,021 B, models over 1e25 FLOP with `Model accessibility` per model) · `power-usage-trend/training_power_draw.csv` (49,322 B). All fetched `HTTP 200`; contents spot-checked, not fully analysed.

---

## 13. Honest gaps in this recon

- **Wayback replay was never verified.** CDX *indexes* snapshots of `openrouter.ai/api/v1/models` from 20230726 and of `artificialanalysis.ai/leaderboards/models` from 20240407, but every replay/CDX fetch from this host returned **429** and no snapshot body was retrieved. Whether those captures contain usable price data is **unknown**.
- **No free per-model token-volume history beyond 7 days** was found anywhere. The OpenRouter rankings window is the only volume source, and it does not backfill.
- **No free transacted (as opposed to list) inference pricing** was found.
- **Epoch's price-at-fixed-performance series has not been updated since 2025-02-05** — 19 months stale as of this retrieval. I did not find an updated equivalent.
- **`epoch.ai/data/ai-data-centers` (746 KB page, per-site directory, CSV slices for chillers/cooling towers/chip quantities) was located but its files were not downloaded** — out of scope for the model-economics lens, flagged for the infrastructure lens.
- The 2025-02-28 LiteLLM basket comparison reused a sha resolved in an earlier call (`1affd0f178`); the five-snapshot series in §5 used freshly resolved shas (`ff553fedf8` etc.). Both were fetched and parsed; the 46-changed-models figure comes from the earlier pair, the 200-model basket from the later set.
