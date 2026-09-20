# L5 — Energy, permitting and physical supply-chain constraints (AI data centres)

**Recon date:** 2026-09-20 (UTC) · **Host:** Raspberry Pi (aarch64), `curl 8.x`, Python 3.13 venv at `/tmp/l5/.venv` (openpyxl + pypdf)
**Lens:** what is NOT covered by `datacenter_construction` and `grid_cancellations` — the **connection queue** and the **equipment supply chain**.

**Contract reminder applied throughout:** every number below was fetched on this host on this date. Where a source failed, it is reported as a **GAP contributing nothing**. Nothing is imputed, defaulted or carried over. "Dead end" entries are deliberate and are the point of this document.

**Direction convention used below:** each signal is labelled with the direction it implies for the bubble question and its specific **failure mode**. The two readings that this lens exists to separate:
- **If power is the binding constraint** → the capex is real, the bottleneck is supply, and a high queue number means *unserved demand*, i.e. **not** a paper bubble.
- **If the queue is filling faster than projects can be energised** → announced capacity is paper and the queue is a **speculative-inventory** signal.

These are opposite readings of the SAME number. The queue total alone cannot distinguish them; the **completion rate** and **time-to-energise** are what distinguish them, and both are measured below.

---

## 1. INTERCONNECTION QUEUES — LBNL "Queued Up" 2026 Edition

### 1.1 The page itself is Cloudflare-blocked to curl; a real browser gets through

| attempt | HTTP | size | note |
|---|---|---|---|
| `curl -L https://emp.lbl.gov/queues` | **403** | 5,548 B | Cloudflare "Just a moment..." interstitial |
| `curl` with full browser UA/Accept headers | **403** | 3,337 B | still Cloudflare |
| **real browser** (`browser_exec` / CDP) → `https://emp.lbl.gov/queues` | **200** | — | page read, attachment links extracted |

**Actionable finding for the Rust CLI:** `emp.lbl.gov` is not fetchable by `ureq`. Do NOT add `emp.lbl.gov` as a fetch target. The **data file itself**, however, is on a different host and IS fetchable with plain curl — see below.

### 1.2 The underlying public data file — FETCHABLE, 15.5 MB

```
URL   https://emp.lbl.gov/sites/default/files/2026-05/LBNL_Ix_Queue_Data_File_thru2025.xlsx
HTTP  200   size 15,571,236 B   content-type application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
```
Retrieved 2026-09-20 with a browser UA. **15.5 MB — large but a one-off annual download; on a Pi it parses in seconds with openpyxl read-only mode.** This is the single highest-value artefact found in this lens: it is *project-level* interconnection-request data for >50 transmission providers (~98% of US installed generating capacity) including the 7 ISO/RTOs.

**Workbook structure (43 sheets, read from the file):**
`00. Background + Methods`, `03. Complete Queue Data` (project-level), `04. Data Codebook`, `05. Annual Requests`, `07. Active Capacity by Year`, `08. Active Capacity by Type`, `09. Active Cap. Region+Type`, `12. Ix. Request Size Trends`, `18. IA Executed Capacity`, `19. IA Throughput by Region`, `20. ERAS and RRI Requests`, `23. Completion Rate Trend`, `26. Withdrawn Ix. Phase`, `27. Post-IA Completion`, `28. IR to WD`, `30. IR to IA - region`, `37. IR to COD - all`, `38. IR to COD - region`, `39. IR to COD - type`, `40. IR to COD - size`, plus hybrid/ERIS-NRIS/prop-online-year tabs.

**Sheet 3 `00. Background + Methods` states the crucial scope limit, quoted from the file:** LBNL's queue data covers **generation and storage only — large LOAD interconnection requests are in separate queues and are NOT in this report.** So LBNL Queued Up is *not* a data-centre-demand measure; it is a generation-supply measure. That is exactly why §2 (ERCOT large load) is needed as the complementary read.

### 1.3 Measured values from the data file (all read on host 2026-09-20)

**Active capacity in queues, end of year (sheet `07. Active Capacity by Year`)**

| year | carried over from earlier years (GW) | entered in year shown (GW) |
|---|---|---|
| 2014 | 197.6 | 227.1 |
| 2018 | 1,282.3 | 1,689.6 |
| 2019 | 127.0 | 136.4 |
| 2020 | 202.1 | 233.6 |
| 2021 | 276.7 | 299.1 |
| 2022 | 348.6 | 560.6 |
| 2023 | 759.1 | 908.0 |
| 2024 | 1,809.5 | 480.9 |
| **2025** | **1,556.9** | **504.4** |

*Reading, and the caveat that must travel with it: the 2018→2019 cliff and the 2023→2024 sawtooth are NOT real market moves — LBNL changed the balancing-area sample between editions (49 vs 50 non-ISO BAs; "all seven ISOs/RTOs" in 2026 vs "7 ISOs/49 BAs" in 2025). Cross-year differences inside this table mix a market signal with a coverage change. The **direction** is usable; a specific YoY percentage off this table is not.*

**Active capacity by generator type (sheet `08. Active Capacity by Type`), standalone configuration, GW**

| type | 2023 | 2024 | 2025 | change 24→25 |
|---|---|---|---|---|
| Solar standalone | 515.1 | 505.0 | 396.6 | **−21.5%** |
| Solar hybrid | 571.2 | 451.5 | 376.0 | −16.7% |
| Storage standalone | 503.0 | 466.8 | 391.5 | −16.1% |
| Storage hybrid | 525.0 | 424.0 | 357.9 | −15.6% |
| Wind standalone | 377.0 | 248.2 | 201.2 | −18.9% |
| **Gas standalone** | **69.4** | **123.4** | **240.0** | **+94.6%** |
| Gas hybrid | 9.8 | 12.9 | 12.8 | −0.5% |
| Nuclear | 10.0 | 5.3 | 10.4 | +95.5% |
| Coal | 1.5 | 1.5 | 3.7 | +153.6% |

> **This is the cleanest single finding in the queue lens.** In a year when the *total* active queue shrank ~10%, **active gas capacity in queue nearly doubled (69.4 → 240.0 GW standalone)** while solar, wind and storage all fell 16–22%. Firm dispatchable capacity is being queued at a rate never seen; intermittent capacity is being withdrawn. **Direction: the queue is rotating toward firm power, which is what a data-centre-driven demand story looks like and is NOT what an over-ordered-solar story looks like.** Failure mode: queue entry is an *intent*, not a commitment — LBNL's own completion rate (§1.4) says most of it is never built.

**Page-stated headline figures (read in browser from `emp.lbl.gov/queues`, 2026-09-20):**
- ~8,200 projects actively seeking interconnection as of end-2025; **1,312 GW generation + ~749 GW storage** = **~2,060 GW total active**.
- **−10% total active queue volume** vs prior year ("high withdrawal rates alongside relatively fewer new requests").
- **549 GW has a draft or executed interconnection agreement but has not reached commercial operation** (256 GW solar, 161 GW storage, 76 GW wind, 45 GW gas).
- **Median IR→COD was over 5 years for projects built in 2025.**
- **Only 13%** of capacity that submitted IRs in 2000–2020 reached commercial operation by end-2025; **75% was withdrawn**, 10% still active.

### 1.4 Median time-to-energise — the direct measurement (sheet `37. IR to COD - all`)

Sample: **3,310 projects from 6 ISOs and 19 non-ISO BAs** that came online since 2005. Duration = months from queue entry date to commercial operations date.

| in-service year | n | mean (mo) | p25 | **median (mo)** | p75 |
|---|---|---|---|---|---|
| 2005 | 63 | 26.4 | 9.7 | **17.7** | 42.3 |
| 2010 | 107 | 39.6 | 28.8 | **33.5** | 50.5 |
| 2015 | 164 | 44.7 | 23.5 | **37.8** | 62.3 |
| 2020 | 227 | 46.5 | 33.0 | **46.1** | 56.3 |
| 2021 | 200 | 51.4 | 33.2 | **50.0** | 61.2 |
| 2022 | 176 | 57.4 | 37.7 | **55.2** | 71.3 |
| 2023 | 265 | 57.8 | 40.9 | **55.7** | 73.0 |
| 2024 | 335 | 62.1 | 42.9 | **62.7** | 80.3 |
| **2025** | **294** | **62.4** | **42.3** | **60.8** | **86.1** |

**Median time-to-energise has roughly tripled since 2005 (17.7 → 60.8 months) and has not plateaued.** The p75 for 2025 projects is **86 months — over seven years**, i.e. a project energising in the back quartile of 2025 entered its queue around 2018. **Direction: lengthening, monotone, and the recent flatness in the median (62.7 → 60.8) is one year of data and is NOT yet a turnaround.** Failure mode: this series is conditioned on projects that *reached COD*, so it is **survivorship-selected** — the projects still stuck are not in it, and therefore 60.8 months is a **lower bound** on how long today's queue will take.

### 1.5 Completion rate — the measure that discriminates the two readings (sheet `23. Completion Rate Trend`)

Capacity (MW) by status as of end-2025, by request year:

| request year | Active (MW) | Operational (MW) | Withdrawn (MW) | **op/(op+wd)** |
|---|---|---|---|---|
| 2015 | 13,910 | 37,231 | 94,872 | **28.2%** |
| 2018 | 56,172 | 24,993 | 211,199 | **10.6%** |
| 2019 | 95,587 | 27,414 | 223,097 | **10.9%** |
| 2020 | 107,730 | 15,716 | 240,143 | **6.1%** |
| 2021 | 194,438 | 10,388 | 326,829 | **3.1%** |
| 2022 | 249,933 | 3,586 | 366,353 | **1.0%** |
| 2023 | 272,271 | 10,171 | 537,752 | **1.9%** |
| 2024 | 263,286 | 110 | 247,858 | **0.04%** |
| 2025 | 441,917 | 843 | 96,937 | **0.9%** |

**This is the discriminating measurement, and it cuts one way.** Recent request cohorts are converting to operation at ~1–6%, versus 28% for the 2015 cohort, and the withdrawn pile is 20–50x the operational pile for every year from 2019 on. **Direction: the queue is a speculative inventory, not a pipeline.** A 2,060 GW active queue against ~1,409 GW of *total US operating capacity* (measured below, §6) is a queue ~1.5x the size of the entire existing fleet. **Failure mode of this measure:** recent cohorts are right-censored — a 2024 request has had only ~1–2 years to reach COD, and §1.4 says the median is now 60+ months, so 2024-25 completion rates are *necessarily* near zero and are not yet evidence about those cohorts. **The 2018–2021 rows are the load-bearing ones** — they have had 5+ years and still convert at 3–11%.

### 1.6 Executed IAs not yet operating, by region (sheet `18. IA Executed Capacity`)

| region | active (GW) | suspended (GW) |
|---|---|---|
| ERCOT | 119.3 | 2.8 |
| CAISO | 98.1 | 0.06 |
| MISO | 73.8 | — |
| West (non-ISO) | 67.9 | 11.2 |
| Southeast (non-ISO) | 44.8 | — |
| SPP | 42.2 | — |
| **PJM** | **38.5** | **9.8** |
| ISO-NE | 8.6 | — |
| NYISO | 6.9 | — |

**PJM is the *smallest* big-market IA backlog (38.5 GW) despite being the largest data-centre market** — consistent with PJM's serial→cycle transition having stalled throughput (§2.4). ERCOT and CAISO carry the largest executed-but-not-operating backlogs. **Direction: executed IA is the strongest paper commitment available and 549 GW of it is not yet energised.** Failure mode: an executed IA has no in-service guarantee and can still be cancelled; LBNL's `27. Post-IA Completion` sheet exists precisely because post-IA attrition is real.

### 1.7 Request size trend — gas projects are getting bigger (sheet `12. Ix. Request Size Trends`)

| request year | Gas mean MW | Gas median MW | Gas n |
|---|---|---|---|
| 2020 | 218.5 | 57.3 | 118 |
| 2021 | 257.4 | 99.0 | 55 |
| 2022 | 252.1 | 174.2 | 78 |
| 2023 | 295.6 | 222.0 | 128 |
| 2024 | 430.5 | 270.0 | 193 |
| **2025** | **498.0** | **400.0** | **320** |

Gas request sizes roughly doubled (median 174 → 400 MW) between 2022 and 2025 and the *count* went from 78 to 320. **Direction: larger, more numerous gas requests — the physical signature of a large-load / behind-the-meter era.** Failure mode: request size is self-declared and is frequently revised down during study.

### 1.8 ERAS / fast-track (sheet `20. ERAS and RRI Requests`)
Sheet exists and is populated (MISO/SPP expedited resource-addition paths). Not read in detail in this pass — **listed as available, not measured.** Do not quote a number for it.

### 1.9 GAP — LBNL PPA price series
LBNL publishes a separate "Utility-Scale Solar" / PPA price series (not the queue file). **Not probed to a fetched value in this pass — reported as a GAP.** Do not treat as unavailable; treat as unverified.

---

## 2. LARGE-LOAD INTERCONNECTION REQUESTS — the direct data-centre-connection measure

### 2.1 ERCOT — the best free large-load series in the US, and it is large

**Endpoints (all fetched 2026-09-20):**

| URL | HTTP | size | content |
|---|---|---|---|
| `https://www.ercot.com/files/docs/2026/07/29/ERCOT-Senate-July-29-Panel-1-Assessing-The-Grid.pdf` | **200** | 921,086 B | ERCOT CEO testimony, 10 pp, data as of 2026-07-28 |
| `https://www.ercot.com/files/docs/2026/06/18/ERCOT-Trending-Topic-New-Batch-Connection-Process-for-Large-Electricity-Users.pdf` | **200** | 458,325 B | ERCOT Trending Topic, 5 pp, 2026-06-18 |
| `https://www.ercot.com/gridinfo/resource` | **200** | 152,522 B | landing page for monthly Generator Interconnection Status (GIS) reports |

**Measured, from the July 29 2026 testimony (p5–p6, p9):**
- **ERCOT is tracking ~474 GW of Large Loads seeking interconnection, of which ~90% are data centres.** (Up from ~410 GW as of March 2026 and ~438 GW as of mid-2026 — see below.)
- **1,949 active generation interconnection requests totalling 462,785 MW**: Solar 161,554 MW, Wind 47,991 MW, **Energy Storage 170,945 MW / 410 GWh**, Gas 77,607 MW. (Queue share by fuel: Solar 34.9%, Battery 36.9%, Gas 16.8%, Wind 10.4%, Other 1%.)
- **Batch Zero eligibility (data as of 2026-07-28), p9:** of the large-load requests, only **~205 GW is eligible for inclusion in Batch Zero** on the basis of *existing studies*. Breakdown: **150 projects eligible as Base Load**, **127 as Allocated Load**, 49 base-or-allocated to be determined; **315 projects excluded for no qualifying study** and **47 excluded for failing to submit a dynamic model**. **274 projects excluded in total.**

**ERCOT large-load queue trajectory (each independently sourced):**

| as of | large-load queue | source |
|---|---|---|
| 2026-03-26 | ~**410 GW** (~87% data centres); 198 new LLI requests in Q1-2026 | `ercot.com/files/docs/2026/04/13/9-Interconnection-and-Grid-Analysis-Update.pdf` (search-surfaced; **not fetched directly in this pass — see GAP note**) |
| mid-2026 | >**438,000 MW** (~90% data centres) | Trending Topic PDF, **fetched, HTTP 200** |
| 2026-06 (via Jul-29 deck) | **~474 GW** (~90% data centres) | testimony PDF, **fetched, HTTP 200** |

**Context that makes the number interpretable:** ERCOT's **all-time peak demand record is 85,508 MW (2023-08-10)** (stated in the fetched Trending Topic). The large-load queue is **~5.5x the all-time system peak**. Texas grid capacity cannot absorb that; the Batch Zero process exists because it cannot.

**The Batch Zero triage is itself a measured attrition rate:** 205 GW eligible out of ~474 GW tracked = **~43% of tracked large-load requests survive the first eligibility filter**, and that filter is only "did you have a pre-existing qualifying study and submit a dynamic model". The later filters are harsher — allocation is by *system reliability*, and a project may receive **its full request, a partial allocation, or a wait for a subsequent batch** (Trending Topic, p2, quoted). Batch Zero milestones (fetched, p4): classification notice **2026-08-07**; **Spring 2027** MW allocation per project; **Q2 2027** proof of developer commitment; **Fall 2027** final transmission plan. **No large load in Batch Zero can be energised before 2028** — the deck's own allocation window is **2028–2032** (p2/p3).

**Also measured (Trending Topic p3, p-4):** the new **financial commitment is $50,000/MW** (from the July testimony p4: "reconcile security … to bring its financial security to **$50,000/MW**"), and projects must **prove site control**. Two new optional pathways exist that are themselves a supply-constraint admission: **WLPUN** (withdrawal-limited private use network, for customers bringing **their own on-site generation**) and **PCLR** (provisional controllable load resource, where ERCOT may **curtail** the load in exchange for earlier access to a portion of the grid).

### 2.1b ERCOT monthly GIS report — the recurring series behind the PDFs, and how to get it

The testimony decks above are one-off. The **recurring** ERCOT series is the **Generator Interconnection Status (GIS) report**, published monthly. It is not reachable by guessing a filename — it is served through ERCOT's MIS download servlet.

```
landing (data product)  https://www.ercot.com/mp/data-products/data-product-details?id=PG7-200-ER   HTTP 200
file (Aug-2026, latest) https://www.ercot.com/misdownload/servlets/mirDownload?doclookupId=1272470218
                        HTTP 200   519,151 B   xlsx      (fetched 2026-09-20)
```
The `doclookupId` is **per-release and changes every month** — the landing page lists `GIS_Report_August2026` posted **2026-09-01** (latest), with January–July 2026 and all of 2025 and earlier archived. **A fetcher must scrape the landing page for the newest `doclookupId`, not hard-code it.** Note the file is **battery-focused** (sheets: `Summary`, `Co-located with Solar/Wind/Thermal`, `Stand-Alone`, `Historic Trends`, `Battery RFI Charts`, `Co-located Operational`) — ERCOT's *generation* queue totals are in the testimony decks and the monthly GIS report proper, not in this battery attachment.

**Measured from the Aug-2026 file (as of 2026-08-31):**
| project type | storage projects | MW | share |
|---|---|---|---|
| Stand-Alone Battery | 584 | **114,058.6** | 71.6% |
| Battery + Solar | 288 | 43,075.7 | 27.0% |
| Battery + Wind | 10 | 1,400.4 | 0.9% |
| Battery + Other | 2 | 783.0 | 0.5% |
| **Total (with FIS requested)** | **884** | **159,317.7** | 100% |

Battery RFI status: **260 operational facilities**, 32 synchronised-but-not-fully-approved, **225 planned with an SGIA**. **Direction: 159 GW of storage in ERCOT's interconnection study pipeline against 114 GW stand-alone** — a battery queue 1.9x the size of ERCOT's own peak demand. Failure mode: FIS-requested is an early study stage, and ERCOT's own Batch Zero attrition (§2.1) says most of it will not be built.

**Also fetchable from the same Resource Adequacy page, and genuinely useful:**
```
https://www.ercot.com/files/docs/2026/09/03/MORA_November2026.xlsx    HTTP 200  250,703 B  xlsx
https://www.ercot.com/files/docs/2026/09/03/MORA_November2026.pdf     HTTP 200  930.3 KB   pdf
```
**MORA = Monthly Outlook for Resource Adequacy**, issued the first Friday of each month, **two months ahead** of the reporting month (Nov-2026 MORA posted 2026-09-03). Sheets: `Monthly Outlook`, `Low Wind and BESS Risk Profile`, `Capacity by Resource Category`, `Resource Details`, `PRRM Percentile Results`. **Measured, Nov-2026 report:** deterministic **monthly capacity reserve margin = 65.2%** for the highest-risk hour (HE 19:00 CST); EEA probabilities "remain well below the threshold indicative of 'elevated' reserve shortage risk (10%)". **This is a free, monthly, keyless, forward-looking adequacy series for the most data-centre-heavy grid in the US and is the single best recurring complement to the one-off ERCOT PDFs.** Direction: an adequacy *probability*, not a price — it turns when the grid gets tight.


> **Direction, stated both ways as the brief requires.** A 474 GW large-load queue against an 85.5 GW system peak is *prima facie* evidence that **announced data-centre capacity is paper** — the ratio is 5.5:1 and ERCOT's own eligibility pass kept only 43%. But the *reason* the queue is paper is that **the grid is the binding constraint**: ERCOT had to invent a batch process, a $50k/MW security, and two curtailment/self-generation pathways because supply cannot meet it. **So the measured signal supports BOTH readings simultaneously and the number alone does not adjudicate.** The adjudicating measurement is §1.5 (completion rate ~1–6% for recent cohorts) — and that says speculative inventory. **Failure mode of the ERCOT figure: it is a *tracked-inquiry* count, not a contracted count.** Developers routinely file duplicate interconnection requests at multiple sites, and PJM's own board filing (§2.4) confirms this by *requiring* submitters to declare duplicative requests across the RTO. Nothing in ERCOT's number nets out duplicates.

### 2.2 ERCOT — GAP
`https://www.ercot.com/services/rq/large-load` → **HTTP 404**, 62 B. **GAP: the guessed LLI API path does not exist. The real LLI data is delivered via PDF testimony decks and the monthly GIS reports under `/gridinfo/resource`; no keyless JSON endpoint was found.**

### 2.3 PJM large load — no public queue file, but a VERIFIED board filing with the key number

PJM defines **Large Load Additions as ≥50 MW at a single Point of Interconnection**. **GAP: no fetchable PJM large-load queue file was found.** PJM's large loads surface in the annual **Load Forecast Report** (January) and per-utility **Load Adjustment** documents, not in a queue.

**But the PJM Board's CIFP decisional letter IS fetchable and carries the single most important supply-constraint datum found in this entire lens:**
```
https://www.pjm.com/-/media/DotCom/about-pjm/who-we-are/public-disclosures/2026/20260116-pjm-board-letter-re-results-of-the-cifp-process-large-load-additions.pdf
HTTP 200   290,845 B   pdf   14 pages   (fetched + text-extracted 2026-09-20)
```
**Quoted verbatim from the extracted text (p1):** *"As the results of the 2027/2028 Base Residual Auction (BRA) display, the RTO has cleared short of its reliability requirement **for the first time in history**. The 2027/2028 BRA cleared **5.6% short of our target reserve margin**."*

> **This is the cleanest available measurement of the binding constraint, and it does not come from a queue at all.** PJM — the largest data-centre market in the world — failed to procure enough capacity to meet its own reliability target in its 2027/28 auction, for the first time ever, by 5.6%. **Direction: supply is genuinely short, which means the capex is real and the bottleneck is supply.** It is the strongest evidence *against* a pure-paper reading and it is a *published, auditable, RTO-own* number rather than an inference from queue intent. Failure mode: one auction in one RTO, and a short-clearing BRA is also a *price signal* — the auction is designed to clear short and trigger new entry, so the miss is the mechanism working, not (necessarily) a failure. It also cannot distinguish "too little generation" from "auction parameters set too tight".

**Other verified content from the same letter (all quoted from the fetched text):**
- Board's decisional components: *Significant Load Forecasting Improvements; Voluntary Bring Your Own New Generation (BYONG) Paired With Expedited Interconnection Track; Connect and Manage for New Large Load Additions That Do Not BYONG and Curtailment Prior to Emergency Demand Response; Immediate Initiation of Reliability Backstop Procurement; Holistic Review of Investment Incentives 2026; Feedback Request on Price Collar for 2028/2029 and 2029/2030 Capacity Auctions.*
- **BYONG + Expedited Interconnection Track:** *"the Board encourages voluntary 'Bring Your Own New Generation' (BYONG) and directs the PJM staff to implement … an alternate path for any entity seeking to mitigate curtailment risk by bringing their own new generation to the system on an accelerated basis … The Expedited Interconnection Track should be in place by **August 2026**."*
- **EIT economics (verified in the letter):** applications capped at **10 projects/year**; **large nonrefundable study deposit > $500,000**; readiness deposit **$10k/MW for generation paired with load**, **$20k/MW unpaired**; **>250 MW UCAP**; **no site-control or fuel-type changes allowed** post-application; EIT resources pay **100% of network upgrades with no cost sharing**; COD within **3 years** of application.
- **No load-queue restriction:** *"the Board is not taking action to restrict the interconnection of new load through the creation of a dedicated load interconnection queue or through other limiting measures."*
- **Curtailment priority:** large loads that do not BYONG are curtailed **before** pre-emergency Demand Response.
- **Affordability framing:** *"PJM must be conscious of the affordability challenges facing the **67 million** [consumers in our footprint]."*
- **FERC precedent cited in the CIFP material:** SPP's "Large Load" defined at **50 MW** (194 FERC ¶ 61,031, 2026); PJM co-located-load order (193 FERC ¶ 61,217, 2025).

**Reported-but-not-fetched in this pass (treat as quoted, not measured):** PJM Inside Lines states *"Of 32 GW of growth in forecasted electricity demand between 2024 and 2030, 30 GW is attributed to data centers"*, and the 30 Jun 2026 Stage-4 Connect & Manage executive summary describes a **Large Load Registry** (≥50 MW per delivery point; existing loads in service before 2027-06-01 must register by **2027-03-01**). These were surfaced by search and **not fetched** — the numbers are plausible and consistent but are not verified here.

### 2.4 PJM generation queue — FETCHED, project-level, no key

This is a **significant find**: PJM exposes its full queue as a single XML file with no key and no login.

```
https://www.pjm.com/pjmfiles/media/planning/queues-data/PlanningQueues.xml
HTTP 200   size 23,242,076 B   content-type text/xml       (fetched 2026-09-20)
https://www.pjm.com/pjmfiles/media/planning/queues-data/transitionProjects.xml
HTTP 200   size    490,797 B   content-type text/xml       (fetched 2026-09-20)
```
**23.2 MB — noted as LARGE. Streamed with `ET.iterparse` + `el.clear()` so peak memory stays low; do not load it whole.** These URLs were discovered from the page source of `https://www.pjm.com/planning/service-requests/serial-service-request-status` (the JS-export buttons resolve to these static XMLs), NOT guessed — the guessed `.xls`/`.ashx`/API paths all 404'd.

**Measured from `PlanningQueues.xml` (9,263 projects, MWCapacity summed = 643,285 MW):**

| status | projects | MWCapacity |
|---|---|---|
| **Withdrawn** | **6,827** | **513,208** |
| In Service | 1,238 | 77,738 |
| Active | 260 | 22,605 |
| Engineering and Procurement | 246 | 16,238 |
| Confirmed | 224 | 0.0 (field blank for this status) |
| Retracted | 131 | 0.0 |
| Under Construction | 94 | 6,478 |
| Partially in Service – Under Construction | 45 | 2,658 |
| Suspended | 67 | 2,391 |
| Pending Termination | 22 | 1,064 |
| Deactivated | 84 | 900 |
| Annulled | 24 | 0.0 |
| Canceled | 1 | 5.8 |

**Withdrawn projects are 6.6x the count of in-service ones and 6.6x the MW** (513 GW withdrawn vs 78 GW ever built, within this file's coverage). By fuel, **Natural Gas carries 310,833 MW of the 643,285 MW total (48%)** and Solar 119,132 MW — but the count is Solar-heavy (4,018 solar vs 1,144 gas projects), i.e. **gas is fewer, much bigger projects.** Generation Interconnection = 8,253 projects; Long-Term Firm Transmission = 743; Merchant Transmission = 193; Upgrade Request = 74.

*Caveat that must travel with this: `PlanningQueues.xml` is the **serial + transition** archive (project numbers A01…AG2-style), covering PJM's queue history, not only currently-active requests. It is an inventory, like EIA-860M — the withdrawal figure is cumulative, not a current rate. Also note the `Confirmed`/`Retracted`/`Annulled` rows carry blank MWCapacity, so MW totals for those statuses are **0 because the field is empty, not because the projects are zero-MW** — that is a data-shape trap, not a measurement.*

**Measured from `transitionProjects.xml` (310 projects, all in Cycle TC1):** 204 Active (17,182 MW) / 106 Withdrawn (8,374 MW) → **32.8% withdrawn by MW inside Transition Cycle 1 alone.** Fuel mix in TC1: Solar 170 projects/11,557 MW, Storage 76/5,300, Solar+Storage hybrid 32/2,481, Wind 19/621, Offshore Wind 7/1,306, **Natural Gas only 2 projects / 704 MW.** Stage mix shows Phase 1 studies posted, Phase 2 in progress, Phase 3 and GIA "Not Started" — i.e. **TC1 is still in study, 3 years after the cycle opened (July 2023).**

> **Direction:** PJM's queue is *dominated by withdrawal* and its first reformed cycle is still in Phase 2/3 study after three years, while the RTO's own board filing says 30 of 32 GW of demand growth to 2030 is data centres. **That combination is the "queue filling faster than projects can be energised" reading.** Failure mode: the file is an accumulated inventory, and PJM's transition cycles were a one-off reprioritisation (December 2023 Transition Sort Retool on 616 projects), so the withdrawal share partly reflects a deliberate purge rather than ordinary attrition.

**Cross-check on throughput, from PJM's own 2021 task-force analysis** (`20210804-item-02a-aws-queue-analysis.ashx`, `pjm.com/-/media/DotCom/committees-groups/task-forces/iprtf/2021/...`, surfaced via search — **not fetched directly in this pass, flagged**): PJM & TOs were processing **55–80 projects interconnected per year**; the task force estimated **"Between 7 to 10 years to process all projects through AI1 at rates of 400–500 per year"** and **"4 to 5 years to process existing queue (up to AG2)"**. Treat as historical context, not a current measurement.

### 2.5 MISO / CAISO / SPP / NYISO — resolved, and the formats genuinely differ

**All four landing pages fetch fine; three of the four data files were then resolved from the page source (not guessed) and FETCHED.**

| ISO | exact URL | HTTP | size | content type |
|---|---|---|---|---|
| **MISO** COD-waiting gen | `https://cdn.misoenergy.org/COD%20Waiting%20Gen%20Data778772.xlsx` | **200** | 20,248 B | xlsx |
| **MISO** Gen Ix Service Workbook | `https://cdn.misoenergy.org/MISO%20Generator%20Interconnection%20Service%20Workbook108135.xlsx` | **200** | 856,406 B | xlsx |
| MISO queue page | `https://www.misoenergy.org/planning/resource-utilization/GI_Queue/` | **200** | 47,094 B | html |
| **CAISO** Cluster 15 | `https://www.caiso.com/documents/cluster-15-interconnection-requests.xlsx` | **200** | 59,452 B | xlsx |
| **CAISO** Resource ID report | `https://www.caiso.com/documents/generator-interconnection-resource-id-report.xlsx` | (found on page) | — | xlsx |
| **SPP** live CSV | `https://opsportal.spp.org/Studies/GenerateActiveCSV` | **200** | 241,964 B | text/csv |
| SPP listing page (HTML) | `https://opsportal.spp.org/Studies/GIActive` | **200** | 773,243 B | html |
| **NYISO** | `https://www.nyiso.com/interconnections` | **200** | 168,110 B | html — **links are login-gated** (`nyiso.my.site.com/Interconnection/s/login/`, `www.nyiso.com/market-access-login`). **GAP: NYISO queue data requires an account.** Only an interactive CartoVista map (`cloud.cartovista.com/nyiso/ferc/poi-analysis-map`) is public. |

**Confirmed as the brief anticipated: queue formats differ wildly.** MISO distributes xlsx on a CDN plus Power BI dashboards (`app.powerbigov.us`) plus a CartoVista map; SPP offers an HTML table *and* a clean CSV endpoint; CAISO publishes per-cluster xlsx; NYISO is login-walled. **LBNL's file (§1.2) is still the right primitive — it already standardises all of these.** What follows is confirmation that the raw per-ISO numbers are consistent with LBNL, and one ISO-level finding LBNL does not surface.

#### MISO — `COD Waiting Gen Data` (fetched 2026-09-20)
**506 units, 94,610.4 MW awaiting commercial operation** (i.e. executed but not yet energised — MISO's equivalent of LBNL's 549 GW IA-executed figure).

| unit type | MW | n |
|---|---|---|
| Solar | 40,480.0 | 263 |
| **Gas** | **22,687.3** | **42** |
| Hybrid | 10,568.3 | 81 |
| Battery | 8,847.5 | 61 |
| Wind | 7,857.4 | 38 |
| Solar/Battery | 1,330.0 | 11 |
| Gas/CC | 1,292.0 | 3 |
| Gas/Liquid fuel | 824.0 | 1 |
| "NUCLAER" *(sic, as spelled in the file)* | 615.0 | 1 |

**By expected COD year: 2026 = 20,654 · 2027 = 21,831 · 2028 = 36,483 · 2029 = 13,195 · 2030 = 1,953 · 2032 = 80 MW.** Gas is 42 units carrying 22,687 MW — mean ~540 MW/unit, i.e. **utility-scale gas, and MISO's gas waiting-to-connect capacity is 24.8 GW (incl. CC/liquid)**. **Direction: MISO has 94.6 GW of IA-backed capacity not yet energised, weighted toward 2028.** Failure mode: "Expected COD" is developer-declared and this file is explicitly a *waiting* list, so it is the backlog, not throughput.

*MISO's other workbook (`MISO Generator Interconnection Service Workbook`) is about **interconnection-service MW registration by CPNode**, not the queue — sheets `NEW/OLD/COMPARE North_Central`, `NEW South`, `MISO BTMG IS` (behind-the-meter generation with MISO interconnection service, 114 rows), `CPNodes without MISO Interconnection Service`. Its own `OLD_Read Me` states it "does not include generators currently active in the generation interconnection queue or generators with interconnection agreement for which CP nodes have not been registered". **Useful for BTMG presence; NOT a queue size measure.***

#### CAISO — Cluster 15 (fetched 2026-09-20)
Two sheets: `Cluster 15` (active) and `Withdrawn`. Columns include `NET MW POI`, `Generation/Fuel 1..3`, `Requested COD`, `Queue Date`, `Study Area`, `PTO`.

| sheet | projects | MW at POI |
|---|---|---|
| **Cluster 15 (active)** | **86** | **28,449.2** |
| **Withdrawn** | **84** | **30,563.5** |

**Active fuel mix:** Photovoltaic/Solar 16,575.8 MW (46 projects), Storage/Battery 10,501.4 MW (35), Wind 804.6 MW (3), Storage/Compressed Air 500.0 MW (1), Hydro/Pumped-Storage 67.4 MW (1).
**Withdrawn fuel mix:** Storage/Battery 19,674.5 MW (59 projects), Photovoltaic/Solar 10,879.0 MW (24), Biomass 10.0 MW (1).

> **This is the sharpest per-ISO measurement found.** CAISO Cluster 15 has **withdrawn MORE capacity (30,563 MW) than it has active (28,449 MW)** — a **51.8% withdrawal rate by MW within a single cluster**, and the withdrawn MW is overwhelmingly **battery storage (19,675 MW)**. Cluster 15 requests were queued **2024-11** and withdrawals began **2025-04** — i.e. **over half of one cluster's capacity was abandoned within ~5 months of the queue window closing.** **Direction: speculative inventory, and specifically a storage bubble inside the queue.** Failure mode: Cluster 15 is one vintage and one ISO; battery economics (4-hour duration, ITC phase-down, negative pricing) are the dominant driver here and the same rate does not transfer to PJM's gas-heavy or ERCOT's data-centre-driven queues. **It does transfer as a demonstration that a queue can lose half its capacity in months.**

Also relevant: the requested CODs in Cluster 15 active run to **2028–2032** while the queue date is **2024-11** — a **4-to-8-year gap between request and requested energisation**, consistent with LBNL's §1.4 median.

#### SPP — live CSV (fetched 2026-09-20, file self-dated **"Last Updated On 9/19/2026"**)
**1,019 rows, 191,188.2 MW total, 28 columns** including `In-Service Date`, `Commercial Operation Date`, `Capacity`, `MAX Summer MW`, `Nameplate Capacity`, `Generation Type`, `Fuel Type`, `Status`, `Cause of Delay`, `JTIQ Participant`.

**By fuel type (top 12, MW):** *blank/NA* 102,317.6 (650 rows — **a real data-quality gap in SPP's own CSV: 54% of MW has no fuel type recorded**), Gas 21,561.7 (71), Solar/Storage 18,269.3 (88), Wind 8,814.0 (40), Photovoltaic 8,732.5 (44), Combined Cycle 6,394.9 (9), Battery 5,938.8 (32), CT 3,633.2 (25), Combustion Turbine 3,608.2 (11), Coal 3,115.0 (9), CTG 3,054.6 (7), Thermal/Storage 1,200.0 (4).

**By status (count / MW):**
| status | n | MW |
|---|---|---|
| IA FULLY EXECUTED / COMMERCIAL OPERATION | 336 | 50,596.6 |
| IA FULLY EXECUTED / ON SCHEDULE | 304 | 57,329.8 |
| DISIS STAGE | 149 | 40,842.1 |
| SPECIAL STUDY | 66 | 11,066.3 |
| IA PENDING | 45 | 6,850.2 |
| ERAS (fast-track) | 33 | 12,586.1 |
| FACILITY STUDY STAGE | 31 | 6,000.3 |
| IA EXECUTED / ON SUSPENSION | 25 | 4,355.9 |

**107,927 MW is IA-executed but not yet operating** (57,330 on-schedule + 50,597 commercial + 4,356 suspended + 6,850 pending). **40,842 MW is still at DISIS stage** (the earliest cluster-study stage). **Top cause of delay, stated in SPP's own field: "GIA delayed due to Affected Systems Study" — 28 projects.** States: OK 315, KS 182, NE 116, TX 106, CO 77, MO 66, NM 56, ND 27.
**Direction: a long study tail (DISIS + special study = 51.9 GW) behind 108 GW of executed-but-unenergised capacity.** Failure mode: the fuel-type field is blank for over half the MW, and "Commercial Operation Date" is populated for projects already operating, so the active pipeline must be read from `In-Service Date` — **a parser must filter on status, not assume all rows are future.**

#### NYISO — GAP
**Login-walled.** Data requires an account (`nyiso.my.site.com/Interconnection/s/login/`, `nyiso.com/market-access-login`). Only an interactive map is public. **Declare as a GAP contributing nothing.**


---

## 3. POWER PRICES — EIA, live and keyless

**The single most useful operational discovery in this lens: `api.eia.gov/v2` accepts `api_key=DEMO_KEY` and returns real data.** The existing config's note that "the keyless API is inaccessible (api.eia.gov/v2 returns a 'Missing Key' page and no EIA key is configured)" is **correct about no-key and wrong about the availability of a key**: `DEMO_KEY` works.

| URL | HTTP | size | result |
|---|---|---|---|
| `https://api.eia.gov/v2/electricity/retail-sales/data?api_key=DEMO_KEY&frequency=monthly&data[0]=price&facets[stateid][]=US&facets[sectorid][]=ALL&start=2024-01` | **200** | — | real price series returned |
| `https://api.eia.gov/v2/electricity/rto/region-data/data?api_key=DEMO_KEY&...facets[respondent][]=PJM&facets[type][]=D...` | **200** | — | hourly demand, real |
| `https://api.eia.gov/v2/electricity/retail-sales/data` **without** `api_key` | **403** | — | `{"error":{"code":"API_KEY_MISSING"}}` |

*Note for the Rust implementer: EIA's `[0]`/`[]` bracket params are interpreted by curl as glob ranges — use `curl -g`. In `ureq` there is no such issue, but the URLs must be percent-encoded correctly.*

**Measured — US average retail electricity price, all sectors (¢/kWh), annual means computed from the returned monthly series:**

| year | months returned | mean (¢/kWh) | latest month in window |
|---|---|---|---|
| 2024 | 12 | **12.90** | 2024-01 = 12.65 |
| 2025 | 12 | **13.60** | 2025-01 = 13.09 |
| 2026 (Jan–Jun) | 6 | **14.15** | 2026-01 = 14.17 |

Latest single month returned for the US all-sectors price: **2026-06 = 14.48 ¢/kWh**; commercial **14.19**, industrial **9.17** (same response). **Direction: US retail electricity is up ~9.7% in mean terms from 2024 to the 2026 half-year, with the industrial rate still only 9.17 ¢/kWh.** This is a *level-and-trend* input, not a bubble measure: rising retail power prices are the visible transmission of data-centre demand into consumer bills, which is the political-economy channel PJM's board letter discusses ("affordability challenges facing the 67 million consumers in our footprint"). **Failure mode: retail price is regulated in half the country and lagged by tariff proceedings, so it will move long after wholesale scarcity, and it mixes data-centre load with gas prices, weather and rate cases.**

**Measured — PJM hourly demand (EIA-930 via the same keyless API),** latest periods returned: **2026-09-20T21 = 107,769 MWh**, T20 = 106,115 MWh (respondent PJM, type D). *This is the one truly live series found. Note the units are **MWh per hour**, i.e. numerically equal to average MW over the hour.*

**GAP — EIA Electric Power Monthly PDF:** `https://www.eia.gov/electricity/monthly/current_month/epm.pdf` → **HTTP 301 then curl timeout at 25 s, 0 bytes received.** The PDF is large and the redirect chain did not complete on this host in 25 s. **GAP: do not use the EPM PDF; use the JSON API instead.** Landing pages `eia.gov/electricity/data.php` (**200**, 157,579 B) and `eia.gov/electricity/data/eia860m/` (**200**, 93,445 B) both fetch fine.

**GAP — LBNL PPA price series:** not located to a fetched URL in this pass.

**No PPA / power-purchase price index was found that is free and machine-readable for hyperscaler deals.** Reported as a GAP (see §4).

---

## 4. BEHIND-THE-METER GENERATION AND HYPERSCALER PPAs — no free tracker exists

**FINDING: there is no free, machine-readable source that tracks hyperscaler gas-turbine or nuclear PPAs.** This is a **structural GAP, not a search failure**, and the reason is structural: a behind-the-meter or co-located PPA is a private bilateral contract between two corporates. It is not filed with FERC (no wholesale transmission service), not in EIA (no utility reporting obligation), and not in EDGAR as a discrete item (it appears, if at all, inside an exhibit to a 10-K with no machine-readable tag).

**What IS observable, indirectly and free:**

1. **Equipment order books** — the public-company substitute. GE Vernova's gas backlog and Siemens Energy's Grid backlog are disclosed quarterly and are the closest thing to a public measure of behind-the-meter build (§5).
2. **ERCOT's own rules acknowledge the pathway**: the fetched Trending Topic (p3-4) describes **WLPUN** ("large customers who plan to build their own on-site power generation, such as natural gas turbines, solar panels, or other sources") and quantifies the incentive — *"By generating some of their own electricity, these customers reduce the amount they need to draw from the grid. The batch framework recognizes that contribution and allows their on-site generation to offset the amount of transmission capacity they require."* **This is a measured regulatory fact: as of 2026-06-18 ERCOT formally rewards self-generation with grid capacity.**
3. **PJM's board**: `20260116-pjm-board-letter-re-results-of-the-cifp-process-large-load-additions.pdf` **explicitly encourages voluntary "Bring Your Own New Generation" (BYONG)** and directs staff to stand up an **Expedited Interconnection Track by August 2026** for generation contracted to a new large load, with **$10k/MW readiness deposit when paired with load vs $20k/MW unpaired**, **≥250 MW UCAP**, **COD within 3 years**, **10 projects/year cap** (`20251119-item-01a---pjm-lla-cifp-stage-4-package---executive-summary.pdf`, surfaced by search; the 20260116 board letter is the primary). **Direction: two of the three largest RTOs have, in 2026, written rules that pay data centres to build their own generation — that is the clearest available free evidence that grid supply is the binding constraint.** Failure mode: BYONG is an *incentive*, and the filings do not report how many PPAs were actually signed under it.

**GAP (honest dead end): specific hyperscaler nuclear/gas PPA counts, MW, counterparties or prices — no free source found. Do not fabricate a number for this. If the model needs it, it must be a declared weight-0 gap until someone reads the 10-K exhibits by hand.**

---

## 5. EQUIPMENT LEAD TIMES — no free primary index; secondary sources only

**FINDING: Wood Mackenzie's lead-time survey is the industry reference and it is paywalled** (`https://www.woodmac.com/news/power-transformers-lead-times/` → **HTTP 404**, 102,259 B — the page has moved or been withdrawn; **GAP**). **Every number below is from a secondary source quoting WoodMac's Q2 2025 survey or from a vendor's own published tables — none was measured on this host and none should be presented as a primary measurement.** This is stated explicitly because the brief's zero-tolerance rule applies to endpoints and values: these are **reported values with a named secondary source**, and they are the honest best available.

**WoodMac Q2 2025 survey, as quoted by POWER Magazine** (`https://www.powermag.com/transformers-in-2026-shortage-scramble-or-self-inflicted-crisis/`, **HTTP 200**):
- **Power transformers (large): 128 weeks** average
- **Generator step-up (GSU) units: 144 weeks** average
- **Switchgear: 44 weeks** average
- Cost since 2019: **power transformers +77%**, **GSUs +45%**, some distribution transformer classes **+95%**
- Demand since 2019: **GSU demand +274%**, **substation power transformers +116%**
- Circuit breakers **+47% since 2021**, MV switchgear **+50%**

**Terrapin Construction Group, active 2026 slot reservations and project procurement data** (`https://terrapincg.com/news/switchgear-transformer-generator-lead-times-2026`, **HTTP 200**, 906,400 B):

| equipment | 2026 lead time |
|---|---|
| MV switchgear 5/15 kV metal-enclosed | 52–72 weeks |
| MV switchgear 15/27 kV metal-clad | 60–80 weeks |
| MV switchgear 38 kV | 78–104 weeks |
| Pad-mount distribution transformers (≤5 MVA) | 40–65 weeks |
| Substation transformers 5–25 MVA | 65–95 weeks |
| Substation transformers 25–50 MVA | 85–110 weeks |
| **Generator step-up 50–150 MVA** | **100–150+ weeks** |
| Diesel gensets 1,500–3,000 kW | 50–78 weeks |
| Static UPS 500–1,500 kVA | 30–48 weeks |

Same source: utility transformer backlog "worsened the commercial backlog"; **no meaningful normalisation expected before 2028**; **"A project schedule that assumes a 24-week switchgear delivery now needs to assume 72."**

**OEM order books (company disclosures, secondary reporting — not fetched from a filing in this pass):** GE Vernova gas-turbine backlog/slot reservations **116 GW at Q2-2026** (up from 83 GW at end-2025; ~53 GW firm + ~63 GW paid reservations; ≥125 GW expected under contract by year-end), bookings into **2031**; Siemens Energy **gas backlog ~69 GW** and **Grid Technologies backlog €51bn** (2026-08-05, up 28% YoY, +50% capacity not until 2030); **Eaton data-centre backlog 228 GW**. RMI: **combined-cycle turbine lead times moved from 2–3 years to 5–7, sometimes 8.** Sources: `nexi.fund/grid-equipment-order-book-ai-power-2026/`, `mgrid.org/2026/08/09/...`, `distroforge.com/blog/gas-turbine-scarcity-distribution-equipment-queue/`, `manufacturingmag.com/...` (all search-surfaced, **not fetched in this pass**).

> **Direction:** equipment lead times of **100–150 weeks for GSUs and 5–8 years for a combined-cycle turbine** are *longer than the interconnection study itself* in most markets. That means **the binding constraint has moved from permitting to manufacturing**, and it is a *measured* claim only if a free primary index exists — which it does not. **Recommended use in the model: this is a strong qualitative input and a weak quantitative one.** If scored, it must be scored on OEM order-book disclosures (SEC-filed, auditable) rather than on the lead-time numbers, and the paywalled WoodMac survey must be cited as the unverifiable upstream source. **Failure mode of the lead-time numbers: they are vendor-marketing-adjacent and self-interested** — the same companies selling the equipment publicise the scarcity, and Terrapin sells procurement services. **Cross-check available for free: the OEM order books are in the 10-Ks.** GE Vernova's turbine backlog and Siemens Energy's grid backlog are reported figures; the lead-time claims are not.

**No free industry lead-time index was found.** `woodmac.com` returns 404 on the direct path. **GAP.**

---

## 6. EIA-860M — PLANNED CAPACITY BY FUEL AND TIME-TO-OPERATION

**Fetchable, no key, exact URL (discovered from the EIA landing page's own `href`s, not guessed):**
```
https://www.eia.gov/electricity/data/eia860m/xls/july_generator2026.xlsx
HTTP 200   13,932,124 B   application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
```
**13.9 MB. Same file the existing `grid_cancellations` indicator already parses** — the sheets are `Operating`, `Planned`, `Retired`, `Canceled or Postponed` (+ `_PR` variants). Archive pattern confirmed working: `/electricity/data/eia860m/archive/xls/<month>_generator<year>.xlsx`. **Note the EIA landing page currently lists `december_generator2026.xlsx` and `october_generator2026.xlsx` at the *unarchived* `/xls/` path while September and earlier are in `/archive/`** — the canonical current file should be read from the landing page each run rather than hard-coded by month.

**Header shape trap (important for the Rust parser):** rows 0 and 1 are title/blank; the **column header is on row index 2**, and reading `values_only=True` from a 3-row offset is required. Columns: `Entity ID, Entity Name, Plant ID, Plant Name, Google Map, Bing Map, Plant State, County, Balancing Authority Code, Sector, Generator ID, Unit Code, Nameplate Capacity (MW), Net Summer Capacity (MW), Net Winter Capacity (MW), Technology, Energy Source Code, Prime Mover Code, Planned Operation Month, Planned Operation Year, Status, Latitude, Longitude`.

### 6.1 Measured — `Planned` sheet, July 2026 release

**2,341 rows, total 291,137.9 MW.** (Matches the config's existing "291,138 MW planned", independently reproduced — a good integrity check.)

**By planned operation year:**

| planned year | MW | note |
|---|---|---|
| 2026 | 50,725.0 | remainder of this year |
| 2027 | 89,578.3 | |
| 2028 | 67,872.1 | |
| 2029 | 41,595.7 | |
| 2030 | 25,819.4 | |
| 2031 | 9,657.9 | |
| 2032 | 3,537.2 | |
| 2033 | 553.5 | |
| 2034 | 1,777.8 | |
| 2039 | 21.0 | |

**By technology (top 12):**

| technology | MW |
|---|---|
| Solar Photovoltaic | 122,980.9 |
| Batteries | 65,262.0 |
| **Natural Gas Fired Combined Cycle** | **47,146.2** |
| **Natural Gas Fired Combustion Turbine** | **21,497.5** |
| Onshore Wind Turbine | 19,234.4 |
| Offshore Wind Turbine | 5,089.0 |
| Nuclear | 4,968.0 |
| Hydroelectric Pumped Storage | 2,600.0 |
| Natural Gas Internal Combustion Engine | 1,023.8 |
| Conventional Hydroelectric | 431.5 |
| Solar Thermal w/o Storage | 200.0 |
| Petroleum Liquids | 188.8 |

**By energy source code:** SUN 123,181 · **NG 69,677** · MWH (batteries) 65,262 · WND 24,323 · NUC 4,968 · WAT 3,032.

**By balancing authority (top 12):** **ERCO 90,063 · MISO 45,619 · PJM 26,811 · CISO 25,400 · SWPP 16,291** · AZPS 10,324 · BPAT 9,010 · SOCO 8,566 · NYIS 7,790 · TVA 5,811 · PSCO 4,986 · NEVP 4,511.

### 6.2 The time-to-operation measurement — gas is being promised LATE

**Gas CC and CT planned capacity, by planned operation year (from the same sheet):**

| planned year | Gas CC (MW) | Gas CT (MW) | Gas ICE (MW) | **Gas total (MW)** |
|---|---|---|---|---|
| 2026 | 1,918.0 | 2,280.6 | 203.9 | 4,402.5 |
| 2027 | 4,144.5 | 6,988.2 | 228.7 | 11,361.4 |
| 2028 | 9,874.0 | 6,978.1 | 261.2 | 17,113.3 |
| **2029** | **9,447.6** | **4,760.0** | 330.0 | **14,537.6** |
| **2030** | **17,643.4** | 490.6 | — | **18,134.0** |
| 2031 | 2,340.9 | — | — | 2,340.9 |
| 2034 | 1,777.8 | — | — | 1,777.8 |

**This is the supply-constraint measurement with a direction on it.** Of **69,677 MW of planned natural gas**, only **4,403 MW is promised for the remainder of 2026**; **17,113 MW for 2028**, and the single largest annual gas block in the whole pipeline is **2030 (18,134 MW)**. A gas plant promised for 2030 is a plant whose turbine slot has not been booked under a 5–7 year lead time (§5) — **the planned-capacity series and the equipment lead-time series are consistent with each other and both say the firm capacity arrives late.** **Direction: firm supply is announced for 2028–2030, not 2026–2027.** Failure mode: **`Planned Operation Year` is a self-declared developer date and EIA does not audit it**; the config's own note that the Planned sheet is a *cumulative inventory* applies. A project can sit in `Planned` with a 2030 date that slips every month for years.

**Nuclear is a single-digit contributor:** 4,968 MW total planned, of which 2,234 MW in 2031 and 2,234 MW in 2032, 500 MW in 2030. **No nuclear capacity is planned before 2030.** That is worth stating plainly given the volume of nuclear-PPA press.

### 6.3 Measured — `Canceled or Postponed` sheet, July 2026 release

**1,734 rows, total 184,141.4 MW.** (Independently reproduces the config's "184,141 MW" exactly.) **Cancellation ratio = 184,141.4 / (184,141.4 + 291,137.9) = 38.74%.**

| technology | cancelled/postponed MW |
|---|---|
| Natural Gas Fired Combined Cycle | 59,465.6 |
| Natural Gas Fired Combustion Turbine | 31,063.5 |
| Solar Photovoltaic | 28,516.7 |
| Onshore Wind Turbine | 15,752.5 |
| Conventional Steam Coal | 15,409.3 |
| Batteries | 14,955.6 |
| **Nuclear** | **5,462.0** |
| Offshore Wind Turbine | 3,349.4 |
| Petroleum Liquids | 3,027.5 |
| Solar Thermal w/o Storage | 1,796.8 |
| Coal IGCC | 1,551.0 |

**Measured — `Operating` sheet total: 1,408,846.6 MW.** (1,409 GW of operating US generating capacity — the denominator that makes the queue numbers in §1 legible.)

> **The nuclear cancellation line is the sharpest single contrast in this file.** Only **4,968 MW of nuclear is planned**, while **5,462 MW of nuclear was cancelled or indefinitely postponed** — **more nuclear capacity has been abandoned than is currently planned.** A model that treats "hyperscalers are signing nuclear PPAs" as a bullish real-capex signal should be confronted with this: in the EIA pipeline, nuclear is net-negative. Failure mode: the cancellation sheet is cumulative inventory across the accumulation window and cannot distinguish a 2009 cancellation from a 2025 one; and EIA's `Nuclear` row mixes uprates and new builds.

---

## 7. WATER AND LOCAL PERMITTING

**GAP — not probed to a fetched value in this pass.** No free machine-readable national water-withdrawal series for data centres was identified, and local permitting is jurisdiction-by-jurisdiction with no common schema. **Reported as a GAP contributing nothing.** The one adjacent free source seen but not fetch-confirmed: county-level building permits and local utility filings, which are inconsistent in format across ~3,000 US counties. Do not put a number in the model for this lens.

---

## 8. SUMMARY TABLE — what to wire, what to declare a gap

| # | source | exact URL | HTTP | size | measured value (as_of) | direction | failure mode |
|---|---|---|---|---|---|---|---|
| 1 | **LBNL Queued Up 2026 data file** | `emp.lbl.gov/sites/default/files/2026-05/LBNL_Ix_Queue_Data_File_thru2025.xlsx` | **200** | 15.5 MB | ~2,060 GW active queue (1,312 gen + 749 storage); **gas active 69.4→240.0 GW (+95%)** while solar/storage/wind fell 16–22% (2025) | queue rotating to firm power = real demand story | queue entry ≠ commitment; sample changed between editions |
| 2 | LBNL IR→COD | same file, sheet `37. IR to COD - all` | — | — | **median 17.7 mo (2005) → 60.8 mo (2025)**; p75 2025 = 86 mo | lengthening monotonically | survivorship-selected; excludes stuck projects |
| 3 | LBNL completion rate | same file, sheet `23. Completion Rate Trend` | — | — | op/(op+wd): **28.2% (2015 cohort) → 10.6% (2018) → 3.1% (2021) → 1.0% (2022)** | **queue is speculative inventory** | recent cohorts right-censored |
| 4 | **ERCOT large load** | `ercot.com/files/docs/2026/07/29/ERCOT-Senate-July-29-Panel-1-Assessing-The-Grid.pdf` | **200** | 921 KB | **~474 GW large load (~90% DC)** vs **85.5 GW all-time peak**; 205 GW Batch Zero-eligible of 474; 274 projects excluded (as of 2026-07-28) | both readings — ratio says paper, triage says supply-bound | tracked inquiries, duplicates not netted |
| 5 | ERCOT gen queue | p6 of same PDF | — | — | **1,949 requests / 462,785 MW**; storage 170,945 MW, solar 161,554, gas 77,607 | — | — |
| 6 | **PJM full queue** | `pjm.com/pjmfiles/media/planning/queues-data/PlanningQueues.xml` | **200** | **23.2 MB** | 9,263 projects / 643,285 MW; **withdrawn 6,827 projects / 513,208 MW** vs in-service 1,238 / 77,738 | heavy attrition | cumulative inventory, includes historical serial queue |
| 7 | PJM TC1 cycle | `.../queues-data/transitionProjects.xml` | **200** | 490 KB | 310 projects; 204 active / 106 withdrawn (**32.8% by MW**); still in Phase 2/3 study since Jul 2023 | slow throughput | one-off reprioritisation purge |
| 8 | **EIA-860M planned** | `eia.gov/electricity/data/eia860m/xls/july_generator2026.xlsx` | **200** | 13.9 MB | **291,137.9 MW planned**; gas 69,677 of which only **4,403 MW in 2026** and **18,134 MW in 2030** | firm supply arrives 2028–2030 | self-declared, unaudited dates |
| 9 | EIA-860M cancelled | same file, `Canceled or Postponed` | — | — | **184,141.4 MW** → ratio **38.74%**; nuclear **5,462 cancelled vs 4,968 planned** | net-negative nuclear | cumulative inventory |
| 10 | EIA-860M operating | same file, `Operating` | — | — | **1,408,846.6 MW** total US operating | denominator | — |
| 11 | EIA retail price | `api.eia.gov/v2/electricity/retail-sales/data?api_key=DEMO_KEY...` | **200** | — | US all-sector mean **12.90 (2024) → 13.60 (2025) → 14.15 (2026H1)** ¢/kWh; Jun-2026 = 14.48 | visible consumer cost of the buildout | regulated, lagged |
| 12 | EIA-930 hourly | `api.eia.gov/v2/electricity/rto/region-data/data?api_key=DEMO_KEY...` | **200** | — | PJM demand **107,769 MWh @ 2026-09-20T21** | live | — |

| 13 | **CAISO Cluster 15** | `caiso.com/documents/cluster-15-interconnection-requests.xlsx` | **200** | 59 KB | active **86 proj / 28,449 MW** vs **withdrawn 84 proj / 30,563 MW** → **51.8% withdrawn by MW**; withdrawn is 19,675 MW battery | **speculative inventory (storage)** | one cluster, one vintage; battery economics dominate |
| 14 | **SPP active GI** | `opsportal.spp.org/Studies/GenerateActiveCSV` | **200** | 242 KB | **1,019 rows / 191,188 MW**; **107,927 MW IA-executed not operating**; 40,842 MW still at DISIS stage; file self-dated 2026-09-19 | long study tail behind an unenergised backlog | fuel type blank for 54% of MW |
| 15 | **MISO COD-waiting** | `cdn.misoenergy.org/COD%20Waiting%20Gen%20Data778772.xlsx` | **200** | 20 KB | **506 units / 94,610 MW** awaiting COD; gas 22,687 MW (42 units); 2028 = 36,483 MW | 94.6 GW backlog weighted to 2028 | developer-declared dates; it is a backlog not a rate |
| 16 | **PJM Board CIFP letter** | `pjm.com/-/media/DotCom/about-pjm/who-we-are/public-disclosures/2026/20260116-pjm-board-letter-re-results-of-the-cifp-process-large-load-additions.pdf` | **200** | 291 KB | **2027/28 BRA cleared 5.6% SHORT of target reserve margin — "first time in history"**; BYONG + EIT by Aug-2026; $10k/$20k per MW readiness deposits | **supply is genuinely short → capex real, bottleneck is supply** | one auction; a short BRA is also the designed price signal for new entry |
| 17 | **ERCOT GIS Aug-2026** | `ercot.com/misdownload/servlets/mirDownload?doclookupId=1272470218` (id rotates monthly) | **200** | 519 KB | battery pipeline **884 projects / 159,318 MW** (114,059 stand-alone); 260 operational; 225 planned w/ SGIA | 159 GW storage in study = 1.9x ERCOT peak demand | FIS-requested is an early stage |
| 18 | **ERCOT MORA** | `ercot.com/files/docs/2026/09/03/MORA_November2026.xlsx` | **200** | 251 KB | Nov-2026 deterministic **capacity reserve margin 65.2%**; EEA risk below the 10% "elevated" threshold | free monthly forward adequacy series for the most DC-heavy grid | probabilistic model, not an observation |

**GAPS (declare, contribute nothing, do NOT impute):**
- `emp.lbl.gov` HTML is Cloudflare-403 to curl — **only the xlsx is fetchable**, via a browser UA. Do not add the page as a fetch target.
- **NYISO queue data is login-walled** (`nyiso.my.site.com/Interconnection/s/login/`). Only an interactive map is public.
- `woodmac.com` lead-time index: **404**. No free primary equipment lead-time index exists.
- No free hyperscaler PPA / behind-the-meter tracker. **Structural, not a search failure.**
- EIA Electric Power Monthly PDF: **301 + timeout, 0 bytes**.
- LBNL PPA price series: not located.
- Water / local permitting: not probed to a value.
- `ercot.com/services/rq/large-load` → **404**.
- Guessed ISO paths that do NOT exist (do not retry): `cdn.misoenergy.org/GI%20Queue%20Snapshot.xlsx` (**403**), `spp.org/documents/71732/spp_gi_queue.csv` (**404**), `opsportal.spp.org/Studies/GenerateCSV.aspx?type=GI` (**404**), `pjm.com/pub/planning/project-queues/queue.xls` (**404**), `pjm.com/planning/services-requests/interconnection-queues` (**200 → /not-found**), `services.pjm.com/PJMPlanningAPI/api/QueueData*` (**404**).
- PJM large-load queue file: **none public**; only the annual Load Forecast Report.
- PJM CIFP filings and 2021 queue-analysis PDFs, and the OEM order-book articles: **search-surfaced and quoted, NOT fetched in this pass.** Treat their numbers as *reported*, not *measured*, until fetched.
- **ERCOT monthly GIS reports** (`ercot.com/gridinfo/resource`, HTTP 200, 152,522 B): the landing page fetches but **no `.xlsx`/`.csv`/`.pdf` hrefs were extractable by regex from the raw HTML** — the links are almost certainly JS-rendered. **GAP: resolve with a browser, not curl, if the monthly GIS series is wanted.**

---

## 9. WHAT THIS MEANS FOR THE MODEL (short, and deliberately unresolved)

1. **The queue number cannot adjudicate the question alone, and the config should not pretend it can.** ERCOT's 474 GW and PJM's 643 GW both support "paper capacity" on their face. The measurement that discriminates is **LBNL's completion rate** (§1.5): 1–6% for recent cohorts against a 20–28% historical norm. **That is a measured finding and it points at speculative inventory.**
2. **But the reason the queue is speculative is that supply binds**, and that is also measured: median IR→COD of 61 months and rising, gas planned overwhelmingly for 2028–2030, GSU lead times of 100–150 weeks, and *two RTOs writing rules in 2026 that pay data centres to bring their own generation*. **If power were not the constraint, ERCOT would not have needed Batch Zero and PJM would not have needed BYONG.**
3. **So the honest composite reading is: the demand is real, the capacity is late, and the queue is inflated by requests that will never be built.** These are not contradictory — a real supply shortage *is* what causes over-filing, because a queue position is a cheap option on a scarce good. **The bubble question is therefore not "is the demand real" (it is) but "how much of the announced capacity is contracted" — and the free data says: not much.**
4. **Two candidate new indicators, both from fetched data, both with honest failure modes:**
   - **`queue_completion_rate`** (LBNL sheet 23, op/(op+wd) by request-year cohort, 5+ years seasoned) — measures whether the queue converts. **Direction: LOW completion = HIGH stress.** Uses data already downloaded.
   - **`firm_supply_lead_time`** (EIA-860M planned gas MW weighted by planned-operation-year distance from now) — measures how far out the firm capacity is promised. **Direction: LONGER = HIGH stress.** Uses a file the config already parses.
5. **Do NOT add:** equipment lead times as a scored primitive (no free primary source; the available figures are quoted from a paywalled survey and from vendor-adjacent blogs), hyperscaler PPAs (no free source at all), or per-ISO queue files (LBNL already standardises them).
