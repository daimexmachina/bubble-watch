# L1b — International lens: four remaining probes
Run 2026-09-20 (PDT) · Raspberry Pi / home connection · every request `curl -sS --max-time 15`
Raw bodies: `/home/daim/workspace/bubble-watch/.hermes/recon/2026-09-20/l1b/` (this run) and `…/l1/` (pre-existing).
**Every number below was extracted from a file actually downloaded in this run or a listed prior run. Provenance, HTTP status and byte count are given for each. Nothing is estimated unless labelled "computed".**

---

## (2) SOVEREIGN-AI COMMITMENTS — G42 & HUMAIN (offline parse, zero network)

Parsed locally with a tag/script stripper; then regex-swept for currency+digits and for `GW|MW|gigawatt|megawatt|GPU|chip|PFLOPS|EFLOPS`.

| file | bytes | source | visible text after stripping |
|---|---|---|---|
| `l1/g42_alt.body` | 122,519 | g42.ai (prior run, HTTP 200) | 7,082 chars |
| `l1/humain_sa.body` | 43,073 | humain.ai (prior run, HTTP 200) | 4,319 chars |

### G42 → **no quantified commitment whatsoever**
- Currency sweep (`$`, `USD`, `SAR` + digits): **0 matches.** Power/chip sweep (`GW|MW|GPU|chip|PFLOPS`): **0 matches.** Only bare words: `partnership` ×10, `invest` ×2.
- All that exists are unquantified org stats: **`23,000+ PEOPLE`**, **`30 COUNTRIES`**, **`10 COMPANIES`**, "the Intelligence Grid" described as a "paradigm"/"super-utility" with no figure attached.
- Dated news **headlines only, no release bodies**, all uncosted:
  - 17 Jul 2026 — "G42 welcomes a new milestone in the U.S.-UAE AI partnership"
  - 08 Jul 2026 — G42 / UAE Team Emirates-XRG / MET "Helmetverse" season two
  - 03 Jun 2026 — Banco Santander + G42 sign a **Memorandum of Understanding**
  - 02 Jun 2026 — "Core42 advances U.S. AI infrastructure strategy with expanded New York deployment"
  - 15 May 2026 — "G42 and Government of India **Formalize Commercial Framework** for Condor Galaxy India AI Supercomputer"
- Note the framing verbs: *MoU*, *formalize a commercial framework* — instrument types that commit no dollars.

### HUMAIN → **no quantified commitment whatsoever**
- Currency sweep: **0 matches.** Power/chip sweep: **1× `GPU`** and **1× `chip`**, both inside one unquantified sentence.
- The literal sentence: *"From AI training infrastructure like **GPUs**, to next-gen inference engines powered by **LPUs**, we architect performance from the ground up… our next-generation AI-native data centers… Purpose-built for hyper scale intelligence, these sovereign facilities are robotic, modular, and optimized for the lowest total cost of ownership."*
- Also present: a four-layer stack (Infrastructure / Cloud / Data & Models / Applications); `ALLAM`, "Arabic-first LLM, *Co-developed with SDAIA"; `HUMAIN ONE` agent platform. **No MW, no GW, no GPU count, no SAR/USD figure, no dated commitment.**
- Large parts of the body are raw AEM CMS scaffolding (`trackData`, `cmpid`, `/content/dam/PIF-Humain/...`) — template config, not content.

**Verdict for both: marketing/positioning only.** Sovereign AI demand (UAE, Saudi) is **not measurable from the corporate web properties.** Any Gulf figure in the report must come from a press-release detail page, a sovereign-fund disclosure, or a filing — none of which is in hand.

---

## (3) US EXPORT CONTROLS — Federal Register API (live, this run)

URL: `https://www.federalregister.gov/api/v1/documents.json?per_page=100&order=oldest&fields[]=title&fields[]=publication_date&fields[]=type&fields[]=agencies&fields[]=document_number&conditions[term]=advanced+computing&conditions[agencies][]=industry-and-security-bureau&conditions[publication_date][gte]=2025-09-20&conditions[publication_date][lte]=2026-09-20`
**HTTP 200 · 1,638 bytes** → saved `l1b/fr_query1.body`

- **COUNT = 2** (`"count":2`, `"total_pages":1`). Full result set:
  1. **2026-01-15 · Rule · "Revision to License Review Policy for Advanced Computing Commodities" · doc 2026-00789**
  2. 2026-07-14 · Rule · "Enhanced Favorable Treatment for the United Arab Emirates Under the Export Administration Regulations" · doc 2026-14132
- **Most recent document date = 2026-01-15** for the substantive advanced-computing item; **2026-07-14** if the UAE-easing rule counts as an "advanced-computing" hit.
- Sanity check: unfiltered `term=advanced computing` returns `count 4385` (dominated by FERC / software-market documents) — the agency filter is doing real work.

Cross-check — BIS **all document types**, published ≥ 2026-07-01 (`l1/fr_all_bis_2026.body`, 3,668 B, prior run):
- **count 14**, `total_pages 2`, **most recent 2026-09-17**.
- Most recent items: three TDO renewals ("Azur Air…", "PJSC Aeroflot…", "UTair Aviation…", all Notices, 2026-09-17); **"Revisions to the Entity List" (Rule, 2026-08-24)**; **"Removal From the Entity List" (Rule, 2026-08-24)**; drone-export and critical-minerals rules.

**Interpretation (constrained to what was fetched):** the last 12 months of BIS output are **not** dominated by new advanced-computing/AI-accelerator tightening — exactly **one** substantive item lands under that term (Jan 2026 license-review-policy revision). 2026 Q3 activity is Entity List churn plus enforcement notices. This **softens**, rather than confirms, a "policy tightening on AI accelerators over the last year" narrative.

---

## (1) CHINA — official data and domestic accelerator financials

### 1a. NBS of China portal → **DEAD from this host (source-IP ACL)**
| probe | status | bytes | detail |
|---|---|---|---|
| `data.stats.gov.cn/easyquery.htm?m=QueryData&dbcode=hgnd…` | **403** | 314 | `reason:UrlACL`, `Client IP: 2601:647:…:6365` |
| `data.stats.gov.cn/english/easyquery.htm?cn=A01` | **403** | 314 | `reason:UrlACL` |
| `www.stats.gov.cn/english/` | 200 | 24,957 | main site loads; the data-query endpoint does not |

The block page names the reason explicitly (`UrlACL`), so this is an **allowlist on this client IP, not a parameter problem**. No parameter tuning fixes it. China's official computing-investment series is unreachable without a proxy or key. **Do not retry.**

### 1b. Huawei Ascend / Cambricon via SEC-like or newsroom routes
- **They are not SEC filers — confirmed, not assumed.** `https://www.sec.gov/files/company_tickers.json` → **HTTP 200, 799,073 bytes**, 10,438 tickers, **0 name matches** for Cambricon / Huawei (also no SMIC / Hua Hong). Their financials are not obtainable from EDGAR.
- `cambricon.com` → HTTP 200, 46,468 B, **product marketing only** (MLU370/270/220 SKUs, NeuWare, MagicMind); **0 currency or revenue hits.**
- `cambricon.com/index.php?m=content&…catid=9` (IR) → **HTTP 302**, 265 B (redirect, no data).
- `huawei.com/en/news` → HTTP 200, 105,267 B, **navigation chrome only**; `huawei.com/en/news/2026` → **HTTP 404**, 199,066 B.
- Shanghai Stock Exchange announcement APIs (`query.sse.com.cn`) → HTTP 200 but **empty**: one returned `"total":0` (588 B), another 130 B. Not usable for filings retrieval here.

### 1c. What *is* reachable — real, quantified China AI numbers

**Cambricon (寒武纪, SSE 688256) H1-2026 report** — `infonotice.sylapp.cn/LC_STIBNotTextAnnounce/2026/08/08/839469586760.PDF` → **HTTP 200, 127,501 bytes, 5-page PDF**; text extracted to `l1b/cn_cambricon_h1.txt` (7,615 B). *Provenance caveat: third-party mirror of the exchange filing, not fetched from sse.com.cn directly. Its figures corroborate independent coverage of the same filing.*
- **营业收入 (revenue) H1-2026: CNY 5,995,573,619.61** vs CNY 2,880,643,471.09 → **+108.13% YoY**
- **归母净利润 (net profit): CNY 2,310,912,080.33** vs 1,038,082,568.57 → **+122.61%**
- Non-recurring-adjusted net profit: CNY 2,165,555,778.52 → +137.30%
- 经营活动现金流量净额 (operating cash flow): **CNY 311,314,642.46** vs 911,150,321.73 → **−65.83%**
- Total assets CNY 18,199,270,965.06 (+35.43% vs year-end); net assets CNY 13,555,416,904.42; basic EPS CNY 3.68 (+119.05%); R&D = 11.72% of revenue (down 7.09 pp); **report is unaudited** (未经审计).
- Computed: CNY 5.996 bn ≈ **US$0.84 bn** at 7.10; revenue **doubled while operating cash flow fell two-thirds** — the demand is real, the cash conversion is not.

**Huawei FY2025 annual report** — `huawei.com/en/annual-report/2025` → **HTTP 200, 140,428 bytes** (HTML) → `l1b/cn_huawei_ar2025.txt` (29,545 B). Real financials:
- **BY SEGMENT (CNY million, 2025 vs 2024):** ICT Infrastructure **375,014** vs 365,424 (+2.6%) | Consumer 344,473 vs 339,006 (+1.6%) | Cloud Computing 32,161 vs 33,325 (**−3.5%**) | Digital Power 77,312 vs 68,607 (+12.7%) | Intelligent Automotive Solution 45,018 vs 26,158 (**+72.1%**) | Other 6,963 vs 29,552 (−76.4%) | **Total 880,941 vs 862,072 (+2.2%)**
- **BY REGION (CNY million):** **China 616,249** vs 615,264 (**+0.2%**) | EMEA 161,356 vs 148,355 (+8.8%) | Asia Pacific 50,113 vs 43,306 (+15.7%) | Americas 37,184 vs 36,301 (+2.4%) | Other 16,039 vs 18,846 (−14.9%)
- Total **R&D CNY 192.3 bn = 21.8% of revenue**; decade R&D > CNY 1.382 trn; 114,000 R&D staff (53.7%); 165,000 patents.
- Cloud computing revenue including other segments: **CNY 72,075 million.**
- Ascend is quantified only as an ecosystem stat, not revenue: **"more than 3.8 million Kunpeng developers and 4 million Ascend developers."** **Huawei does not disclose Ascend chip revenue** — it remains unmeasurable from primary sources.
- Computed: total revenue CNY 880.9 bn ≈ **US$126 bn** at the report's own USD1.00 = CNY6.99 closing rate; **China +0.2%** while total grew 2.2% — i.e. growth came entirely from outside China.

---

## (4) ASML — and what the pre-fetched file actually contains

### 4a. `l1/asml_subs.body` → **form metadata ONLY (as the brief suspected)**
96,944 bytes, valid JSON from `data.sec.gov/submissions/CIK0000937966.json`. Keys: `cik, entityType, sic=3559, name "ASML HOLDING NV", tickers [ASML, ASMLF], exchanges [Nasdaq, OTC], category "Large accelerated filer", fiscalYearEnd 1231, addresses, filings.recent{accessionNumber, filingDate, reportDate, form, primaryDocument, primaryDocDescription,…}`. `filings.recent.form` holds **591 entries**. Recent forms: 6-K 2026-07-15 (rep. 2026-06-28), SD 2026-06-05, 6-K 2026-04-23, 6-K 2026-04-15 (rep. 2026-03-29), 6-K 2026-03-11 (rep. 2026-03-09), **20-F 2026-02-25 (FY2025)**, 6-K 2026-01-28 (rep. 2025-12-31), S-8 2025-11-17, 6-K 2025-10-15 (rep. 2025-09-28), 6-K 2025-07-16 (rep. 2025-06-29).

**No revenue figure and no China-sales share exists in `asml_subs.body`.** Also checked this run (text-extracted): `asml_investor.body` (186,707 B → 4,157 chars visible; "China" appears only in nav strings), `asml_6k_recent.body`, `asml_6k_listing.body` — **no financials in any of them.**

### 4b. I then fetched the actual ASML filings — the China share *is* obtainable
- **Q2 2026 press release** `…/000162828026048235/pressreleasefinancialresul.htm` → **HTTP 200, 43,854 B** → `l1b/asml_q2_2026_pr.txt`
  - **Q2 2026 total net sales €9.3 billion**, gross margin **54.0%**, **net income €2.9 billion**
  - Q3 2026 guidance €11.0–12.0 bn, GM 55–57%; **FY2026 raised to €43–45 bn, GM 54–56%**
  - **"China" appears 0 times** in this release — no geographic split given.
- **Q4/FY2025 press release** `…/000162828026003701/pressreleasefinancialresul.htm` → **HTTP 200, 63,699 B**
  - **Q4 2025 net sales €9.7 bn**, GM 52.2%, net income €2.8 bn; **net bookings €13.2 bn of which €7.4 bn EUV**
  - **FY2025 total net sales €32,667 million**, GM 52.8%; FY2024 €28,263 m; Q4 units 94 vs Q3 66
  - **"China" also 0 times.**
- **Statutory interim report H1-2026** `…/000162828026048235/statutoryinterimreport20.htm` → **HTTP 200, 76,246 B** → `l1b/asml_stat_int.txt` (62,834 chars). **This is where the China share lives:**

  **Total net sales by geographic region (€ million), H1-2025 → H1-2026:**
  | region | H1-2025 | H1-2026 |
  |---|---|---|
  | Japan | 492.9 | 440.3 |
  | South Korea | 4,413.6 | **7,085.5** |
  | Singapore | 194.3 | 221.8 |
  | Taiwan | 4,361.9 | 5,114.7 |
  | **China** | **3,712.3** | **2,883.3** |
  | Rest of Asia | 1.2 | 1.1 |
  | EMEA | 314.2 | 407.1 |
  | United States | 1,942.8 | 1,939.6 |
  | **Total** | **15,433.2** | **18,093.4** |

  Computed: **China share 24.1% → 15.9% of total net sales; China −22.3% YoY while total +17.2% YoY.** South Korea nearly +61% — memory/HBM demand replacing China.
  Also in the same filing: net system sales H1-2026 **241 units / €11,336.5 m → 274 units / €12,844.2 m**; by end-market H1-2026 **Logic 180 units / €6,442.7 m** vs **Memory 94 units / €6,401.5 m** — memory is now essentially half of system revenue.

**ASML bottom line for the report:** China is a *declining* share of ASML revenue (−22% YoY, 15.9%), and the company still raised FY2026 guidance to €43–45 bn — i.e. ASML's current up-cycle is **not** China-dependent. That is a real, sourced counterweight to a "China demand is the swing factor for the AI capex cycle" framing.

---

## BONUS (fetched while here, directly on-topic and quantified)

**Rhodium Group, "Examining China's AI Financing", Sept 2026** — `rhg.com/wp-content/uploads/2026/09/Examining-Chinas-AI-Financing.pdf` → **HTTP 200, 628,193 bytes, 22-page PDF**; extracted to `l1b/rhg_china_ai.txt` (58,568 B). This resolves probe (1) for China *domestic accelerator/computing investment* better than any official portal:
- **"We expect China's AI capex to double this year to 932 billion yuan ($139 billion) and to top 1.2 trillion yuan ($193 billion) in 2027."** Method: survey of 13 Chinese listed firms (hyperscalers, telecoms, labs, independent DC operators); **+103% y/y in 2026** (from +33% in 2025), 2027 +39%.
- **China's AI capex remains far below estimated US data-centre investment of ~$800 billion this year.**
- Alibaba + Tencent + Baidu capex **CNY 208 billion in H1 2026**, above start-of-year expectations.
- **Free cash flow of those three fell from CNY +170 bn (2025) to −16 bn in H1 2026**; Tencent booked ~CNY 51 bn of AI-related prepayments inside Q2 operating cash flow vs Q2 capex of CNY 59 bn.
- For contrast, US hyperscaler FCF: **Microsoft/Amazon/Alphabet/Meta/Oracle $191 bn → $14 bn in H1 2026**, with debt raised $90 bn → $163 bn — "the bond market and private credit [are] central to the sustainability of the US AI boom."
- **China's AI models' combined ARR ≈ $10.7 bn ≈ 10% of OpenAI + Anthropic combined**; Zhipu MaaS ARR $74 m (Jan) → $1.6 bn (Aug), 20×; ByteDance largest at **$4 bn ARR (Jul)**, Alibaba **$2.4 bn (Aug)**.
- Valuation-to-ARR: OpenAI 34×, Anthropic 21×, Zhipu 46×, Moonshot 50×, **DeepSeek 163×**; Moonshot valuation $4 bn (end-2025) → $50 bn (Aug 2026).
- Chip-supply line: *"This year, domestic chip supplies from SMIC have risen, and access to NVIDIA H200 chips appears to be easing"* — and H200 shipments from August "will further support H2 capex, though further US-China tensions could close off access to chips and remote access loopholes later in the year."
- Note: **"Ascend" and "Cambricon" appear 0 times in this 22-page report** (Huawei ×11, SMIC ×1, Nvidia ×1) — even the best available analyst source does not quantify Huawei's accelerator business. Confirmatory of the 1b finding.
- Corroborating secondary source for the ARR claim: CNBC 2026-09-17 and SCMP (`scmp.com/tech/big-tech/article/3367991/…`) both fetched cleanly this run (SCMP `/tech` HTTP 200, 3,398,793 B) and carry the same ByteDance $4 bn / Alibaba $2.4 bn / $10.7 bn figures.

---

## Open items (honest gaps)
- China official computing-investment series: **unreachable** (NBS IP ACL). Substitute used: Rhodium Group PDF.
- Huawei **Ascend-specific revenue: does not exist in any source reached**; Huawei discloses only developer counts. Treat as unmeasurable.
- Gulf sovereign demand (G42/HUMAIN): **unmeasurable from corporate web properties**; needs filings or press-release detail pages.
- ASML China **full-year 2025** split not extracted (only H1-2026/H1-2025 from the interim report); the 20-F body would carry it.
- Cambricon filing arrived via a third-party mirror, not sse.com.cn directly (SSE APIs returned empty).

---

## ORCHESTRATOR VERIFICATION (2026-09-20) — and two findings the lens missed

Every headline below was re-derived by the orchestrator from the **files the lens fetched**, not
from its summary. Where I could, I re-fetched independently.

### Independently re-fetched: export controls
`GET federalregister.gov/api/v1/documents.json?conditions[term]="advanced computing"
 &conditions[agencies][]=industry-and-security-bureau&conditions[publication_date][gte]=2025-09-20`
→ **HTTP 200, 5,546 bytes, `count: 2`** — confirmed.

| date | type | title |
|---|---|---|
| 2026-07-14 | Rule | Enhanced Favorable Treatment for the **United Arab Emirates** under the EAR |
| 2026-01-15 | Rule | Revision to License Review Policy for **Advanced Computing Commodities** |

**Reading: loosening, not tightening.** One rule *eases* UAE access; the other revises license
review. Two documents in twelve months is a *quiet* policy period — this **softens** the
"export controls are constraining AI revenue" narrative rather than supporting it.

### Verified from ASML's statutory interim report (H1-2026, `asml_stat_int.txt`)

**Geographic net sales (€m), H1-2025 → H1-2026:**

| region | H1-2025 | H1-2026 | change |
|---|---|---|---|
| South Korea | 4,413.6 | **7,085.5** | **+60.5%** |
| Taiwan | 4,361.9 | 5,114.7 | +17.3% |
| **China** | 3,712.3 | **2,883.3** | **−22.3%** |
| United States | 1,942.8 | 1,939.6 | −0.2% |
| EMEA | 314.2 | 407.1 | +29.6% |
| Japan | 492.9 | 440.3 | −10.7% |

**China is shrinking fast (−22.3%) while Korea (+60.5%) and Taiwan (+17.3%) surge.** Total net
sales rose 15,433.2 → 18,093.4 (+17.2%). So the growth is **entirely non-China, and concentrated in
the two countries that physically make AI hardware** — the same two countries whose export orders
and chip exports the international lens independently found accelerating.

### FINDING THE LENS MISSED #1 — the end-use mix flipped to memory

Also in the statutory report, and more material than the China share:

| end use | H1-2025 | H1-2026 | value | units | value/unit |
|---|---|---|---|---|---|
| **Logic** | €7,157.6m | €6,442.7m | **−10.0%** | 157→180 (**+14.6%**) | €45.6m → **€35.8m** |
| **Memory** | €4,178.9m | €6,401.5m | **+53.2%** | 84→94 (+11.9%) | €49.7m → **€68.1m** |

**Logic revenue FELL 10% while logic unit volume ROSE 14.6%** — average selling price down ~22%.
Memory revenue rose 53%. Interpretation: the marginal litography spend is moving toward **memory**
(HBM for AI accelerators), while **leading-edge logic is buying more machines at lower average
price**. A price-per-unit decline in logic is a *deflationary* signal for the compute stack, and it
is the kind of thing the current model's `frontier_premium` proxy cannot see.

### FINDING THE LENS MISSED #2 — ASML's own AI valuation marks

The same filing discloses its **Mistral AI** equity investment, carried at fair value using a
venture-capital method with **key unobservable inputs (Level 3)**: business expansion rate
**~60%**, peer revenue multiples **7.1x–61.4x**, VC target return **30%–50%**. ASML states these
"could result in a significant remeasurement of the carrying amount within the next financial
period." That is a *primary-source description of how AI private marks are set* — directly relevant
to the L3 financing lens, and it is in an audited-adjacent filing rather than a news article.

### Verified from the Cambricon filing (`cn_cambricon_h1.txt`, HKEX PDF)
- Revenue **CNY 5,995,573,619.61** (≈CNY 5.996bn), **+108.13% YoY**
- Net profit **CNY 2,310,912,080.33** (≈CNY 2.311bn)
- Operating cash flow **−65.83%**
Revenue doubling with profit positive but **operating cash flow falling 66%** is a receivables /
working-capital signal worth watching.

### China access — confirmed dead from this host
NBS portal: **HTTP 403, `reason: UrlACL`** — a **source-IP ACL**, not a parameter problem. No
parameter tuning fixes it; needs a proxy or key. Huawei and Cambricon are **not SEC filers**
(checked against `company_tickers.json`, 799,073 B, 0 matches), so US filing infrastructure cannot
reach them. Rhodium Group's "Examining China's AI Financing" PDF **is** fetchable — China AI capex
**CNY 932bn (~$139bn) in 2026, +103%**, with Alibaba/Tencent/Baidu H1 capex CNY 208bn against free
cash flow of +170bn (i.e. **−16bn net**).

### Sovereign AI — the clean negative
**No quantified commitments** (currency, MW, or GPU counts) in either the G42 or HUMAIN pages.
Sovereign demand is **not measurable** from corporate sites; it would need press, filings of the
contracting counterparties, or government procurement records.

### Assessment of this lens
- **Sovereign AI: dead end** for corporate sites (clean negative, recorded).
- **China: unreachable officially**, partly reachable via HKEX/PDF and research houses.
- **Export controls: reachable and cheap**, and the finding is *loosening* — which argues **against**
  the constraint narrative and is therefore useful as a falsification input.
- **ASML: the richest single source found in this lens**, and it yielded two things the lens did not
  notice (end-use mix flip; Level-3 AI marks) because it stopped at the China-share question.
