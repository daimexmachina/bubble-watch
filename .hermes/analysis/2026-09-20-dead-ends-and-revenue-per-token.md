# bubble-watch — dead-end register and the revenue-per-token spike

**Date:** 2026-09-20 · Companion to `v1.3-demand-side-plan.md` and `v1.3-execution-log.md`.

The second-most valuable output of reconnaissance is knowing which doors are permanently
closed. Without this list, the next session pays for the same search again.

---

## Task 7 — dead ends (verified, with status codes)

### Key-gated: reachable in principle, blocked without registration

| source | status | note |
|---|---|---|
| Japan e-Stat API | **403** | 200 only with a valid `appId`; a placeholder returns no data |
| METI machinery orders | **403** | `meti.go.jp/english/statistics/tyo/kikaijutyu/` |
| Korea KOSIS open API | 200 but **63 bytes** | service key required |
| Korea `data.go.kr` nitemtrade | **401** | service key required |
| EIA keyed API v2 | key required | the xlsx path is used instead, keyless |

**Implication:** an international indicator needs a free-registration key. This is different from
every US source the tool currently uses, all of which are keyless.

### Gone or moved during this session

| source | then | now |
|---|---|---|
| **Taiwan MOEA export orders** (`service.moea.gov.tw/EE520/Investigate/InvestigateData.aspx`) | 200, 33,883 B, 511 months | **302 → `/gmweb/htmNotFoundPage.htm`** |

Worked at 15:30 and was gone by 17:00. Every path variant tried — `InvestigateData.aspx`,
`InvestigateCSV.aspx`, `InvestigateBA.aspx`, `opendata/all.csv`, and the `/gmweb/` prefix — now
302-redirects to a not-found page. The captured data survives locally and is real (511 months,
1984-01 → 2026-07, latest US$97,939mn at +61.9% YoY), but **a source that unstable is not a basis
for a live indicator.** Blocked, not built on the capture.

### Permanently unavailable

| source | status | note |
|---|---|---|
| China NBS portal | **403 `UrlACL`** | a source-IP allowlist, not a parameter problem. No parameter tuning fixes it |
| Huawei / Cambricon as SEC filers | 0 matches | checked against `company_tickers.json` (799,073 B) |
| Artificial Analysis | **401** | API v2 needs a key; HTML is JS-rendered |
| FRED international export series | no response | all six `XTEXVA01*` variants returned 0 bytes |
| DBnomics series search | **400** | search endpoint rejects; only explicit series paths work |
| Japan customs direct CSV | **404** | the `suii/html/data/` path structure changed |
| KITA / K-stat | **404** | `stat.kita.net` |
| Korea MOTIE press releases | **404** | `english.motie.go.kr` |
| Japan BoJ stat-search | **404** | `stat-search.boj.or.jp` |
| Eurostat semiconductor import `ds-…` | **404** | dataset id wrong; `isoc_eb_ai` and `naio_10_cp1700` DO work |
| Sovereign AI (G42, HUMAIN, QIA) | 200 but **no quantified commitments** | zero currency/MW/GPU figures in the pages. Not measurable from corporate sites |

### Reachable but fragile — use with the UA caveat recorded

| source | behaviour |
|---|---|
| **LBNL queue workbook** | **403 without a browser User-Agent**, 200 (15.5 MB) with one. The landing page `emp.lbl.gov/queues` is Cloudflare-blocked **regardless**. This briefly made the source look dead |
| Comtrade API | **429** under repeated calls; needs backoff |
| OpenRouter frontend routes | undocumented; no versioning; may change without notice |
| FRED anonymous CSV | flaky from this host; recovers on its own |

---

## Task 8 — the revenue-per-token spike

**Question:** the one measurement that would separate *genuine deflation* from *price-cutting to
fill idle capacity*, which the utilisation lens explicitly left unresolved.

### Result: no free absolute revenue-per-token exists. But the RATIO does, and it is informative.

`https://openrouter.ai/api/frontend/v1/rankings/task-spend` → **HTTP 200, 64,521 B**, keyless.
It returns, for the **same 30-day window**, both `spend` and `tokens`, each broken down by task
category — but as **SHARES of the total, not absolute dollars**. So it cannot produce a
dollar-per-token level.

What it CAN produce, measured 2026-09-20:

| category | spend share | token share | ratio | reading |
|---|---|---|---|---|
| **code** | 29.6% | 37.2% | **0.80** | usage-heavy — monetises below its volume |
| **agent** | 29.3% | 34.9% | **0.84** | usage-heavy |
| general | 31.5% | 22.0% | **1.43** | revenue-rich |
| data | 9.5% | 5.9% | **1.60** | revenue-rich |

**Finding: the fastest-growing workloads monetise worst.** Code and agent traffic — exactly the
high-volume, agentic use the industry is betting on — take a *larger share of tokens than of spend*.
General chat, the mature category, is the revenue-rich one.

This is a genuine partial answer to the spike's question: it does not give $/token, but it shows
that **spend is not keeping pace with usage in the growing categories**, which is the same
phenomenon the deflation question was pointing at. It is a share ratio, so it is scale-free and
cannot be confused with revenue.

### Not built, deliberately

No indicator was added from this. A share ratio over one aggregator's 30-day window, with no
history endpoint found, cannot support a scored level — and inventing a level from it would be
exactly the fabrication the project forbids. **Recorded as a finding and a candidate**, not shipped.

Cheaper next step if wanted: check whether the endpoint supports a historical window parameter. If
it does, the ratio becomes a time series and could be scored on its own rate of change.
