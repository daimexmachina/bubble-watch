# L1 — Non-US demand and competition (salvaged)

**Status: lens TIMED OUT at 600s after 23 API calls.** The timeout was a single slow probe
(`python3 l1probe.py batch7.txt` ran 299s before being killed). Everything below is from **live
fetches that completed** before the timeout, recovered from `l1/` and `raw/`. Nothing here is
reconstructed or inferred.

**Probe ledger:** 65 endpoints attempted, **36 returned HTTP 200**. Full ledger: `l1-results.jsonl`.

---

## THE HEADLINE FIND — Taiwan export orders

`GET https://service.moea.gov.tw/EE520/Investigate/InvestigateData.aspx` → **HTTP 200, 33,883 bytes**

Taiwan Ministry of Economic Affairs **外銷訂單金額** (export orders, US$ mn). This is a classic
leading indicator: orders are placed *before* production and shipment, so it leads actual trade.

- **511 months of history, 1984-01 → 2026-07** — 42 years, far deeper than most candidates.
- Monthly cadence, published with a short lag.
- CSV, no key, no auth.

**Measured values (US$ mn/month):**

| month | value | YoY |
|---|---|---|
| 2025-09 | 70,341 | +30.6% |
| 2025-12 | 76,198 | +43.8% |
| 2026-01 | 76,907 | +60.1% |
| 2026-03 | 91,125 | +65.9% |
| 2026-06 | 95,262 | +59.4% |
| **2026-07** | **97,939** | **+61.9%** |

Annual mean monthly: 2023 **46,878** → 2024 **49,199** → 2025 **61,977** → 2026 **86,005** (7 months).

**Direction:** HIGHER = more demand for the hardware that AI compute runs on. This is a *demand-side*
signal, which is the model's largest structural gap. It reads **against** the bubble thesis while
it accelerates — and a deceleration would be an early warning the model currently cannot see.
**Failure mode:** Taiwan export orders aggregate ALL electronics plus other sectors; they are not
AI-specific, and they measure orders for hardware, not AI revenue itself.

---

## Korea semiconductor exports (Comtrade)

`GET https://comtradeapi.un.org/public/v1/preview/C/A/HS?reporterCode=410&...&cmdCode=8542`
→ **HTTP 200, 83,022 bytes** (note: two sibling calls returned **429** — this API rate-limits)

- HS code **8542** (electronic integrated circuits), reporter Korea, annual.
- **2025 world total: US$142.8bn.** Sum across all partners: 285.7bn (double-counts both flows —
  use the world row only).
- Annual cadence. Free, no key, but **429s under repeated calls** — needs backoff.

**Direction:** higher = Korean chip exports rising, i.e. demand. **Failure mode:** Korea exports
to the world including China; it does not isolate AI accelerators.

---

## Eurostat — EU AI adoption (a demand signal from the other side)

`GET https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/isoc_eb_ai`
→ **HTTP 200, 19,489 bytes** — "Artificial intelligence by size class of enterprise", **510 data
points**, dataset updated **2026-06-15**.

`GET .../data/naio_10_cp1700` → **HTTP 200, 255,479 bytes** (ICT investment, EU).

**Direction:** share of EU enterprises USING AI. Rising = real adoption; stalling = the demand
story is weaker outside the US. **Failure mode:** enterprise *survey*, annual, and it measures
adoption breadth rather than spend or intensity.

---

## Other endpoints verified reachable (HTTP 200)

| what | endpoint | size |
|---|---|---|
| ASML SEC submissions | `data.sec.gov/submissions/CIK0000937966.json` | 96,944 |
| ASML investor results page | `asml.com/en/investors/financial-results` | 186,707 |
| Federal Register API — BIS entity list | `federalregister.gov/api/v1/documents.json?conditions[term]=...` | 3,668 |
| Federal Register API — advanced computing rules | same, advanced-computing term | 3,530 |
| EU AI Act dashboard | `digital-strategy.ec.europa.eu/en/policies/european-approach-artificial-intelligence` | 94,281 |
| Japan customs advance release index | `customs.go.jp/toukei/shinbun/happyou_e.htm` | 111,310 |
| SEAJ (Japan semi equipment) statistics | `seaj.or.jp/statistics/index.html` | 21,442 |
| G42 (UAE) | `g42.ai` | 122,519 |
| HUMAIN (Saudi) | `humain.ai` | 43,073 |
| OECD SDMX dataflow | `sdmx.oecd.org/public/rest/dataflow/OECD.SDD.STES` | 50,969 |
| Bank of Korea ECOS | `ecos.bok.or.kr/api/StatisticSearch/sample/...` | 4,905 |
| Taiwan open-data catalogue | `data.gov.tw/api/v2/rest/dataset/6845` | 2,297 |

**The Federal Register API is the cleanest of these** — a documented JSON API for US export-control
rule changes (BIS entity list, advanced-computing rules). A tightening of export controls is a
*direct* constraint on non-US AI revenue potential, and it is fully machine-readable.

---

## Confirmed DEAD ENDS (verified, not assumed)

| what | endpoint | status |
|---|---|---|
| Japan e-Stat API | `api-e-stat.go.jp/.../getStatsList` | **403** without a key; 200 with a placeholder key but no data |
| METI machinery orders | `meti.go.jp/english/statistics/tyo/kikaijutyu/index.html` | **403** |
| Japan customs CSV/PDF direct | `customs.go.jp/toukei/suii/html/data/*.csv` | **404** (path structure changed) |
| KITA / K-stat | `stat.kita.net/...` | **404** |
| KOSIS open API | `kosis.kr/openapi/...` | 200 but **63 bytes** = key required |
| Korea data.go.kr nitemtrade | `apis.data.go.kr/1220000/nitemtrade/...` | **401** = key required |
| Korea MOTIE press releases | `english.motie.go.kr/...` | **404** |
| Japan BoJ stat-search | `stat-search.boj.or.jp/...` | **404** |
| Eurostat semiconductor import (ds-…) | `eurostat/api/.../data/ds-…` | **404** |
| DBnomics series search | `api.db.nomics.world/v22/series?q=...` | **400** on search; 404 on `KOR`; but `Eurostat/isoc_eb_ai` returns 200 |
| FRED international export series | `fredgraph.csv?id=XTEXVA01KRM667S` etc. | **no response / 0 bytes** (all six variants) |
| Comtrade Taiwan & 2025 Korea (repeat calls) | `comtradeapi.un.org/...` | **429 rate-limited** |

---

## Assessment

**One genuinely strong candidate: Taiwan export orders.** 511 months of history, monthly, free,
CSV, and it is a *demand-side* series — exactly the structural gap identified in the coverage
analysis. It is the best international signal found in this lens.

**The rest are secondary**, and two findings are worth more than the endpoint list:

1. **The international data landscape is heavily key-gated.** Japan e-Stat, METI, Korean KOSIS,
   data.go.kr and the Comtrade repeat calls were all blocked by missing keys or rate limits. An
   international indicator will need an API-key strategy (free registration) rather than
   anonymous access, unlike the US sources the tool currently relies on.
2. **Export controls are the exception** — the Federal Register API is genuinely open JSON, and
   it is arguably the more *direct* signal anyway: it measures a policy constraint on non-US AI
   revenue rather than a proxy for demand.

**Not yet probed** (the lens died before reaching them): China domestic accelerators, NBS
computing investment, sovereign-programme substance, ASML content parsing.
