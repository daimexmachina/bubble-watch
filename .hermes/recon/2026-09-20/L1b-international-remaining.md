# L1b — International lens, remaining four probes
Run: 2026-09-20 (PDT). Host: Raspberry Pi, home connection. All probes `curl -sS --max-time 15`.
Raw bodies saved under `/home/daim/workspace/bubble-watch/.hermes/recon/2026-09-20/l1b/` (and `l1/` for pre-existing files).

---

## (2) SOVEREIGN-AI COMMITMENTS — G42 and HUMAIN (offline parse, no network)

### g42_alt.body — 122,519 bytes, HTTP 200 (previously fetched), HTML → 7,082 chars of visible text
Parsed with a local tag/script stripper. **Result: NO quantified commitments at all.**
Regex sweep over the raw HTML for `$`/`USD`/`SAR` + digits, and for `GW|MW|gigawatt|megawatt|GPU|chip|PFLOPS|EFLOPS` → **zero matches** (only `partnership` ×10, `invest` ×2 as bare words).

What the homepage *does* contain (all marketing/unquantified):
- `23,000+ PEOPLE`, `30 COUNTRIES`, `10 COMPANIES` — org-scale stats, no dollars.
- "The Intelligence Grid" — described qualitatively only ("a new paradigm", "super-utility").
- News headlines with dates but **headlines only, no release bodies**, all uncosted:
  - 17 Jul 2026 — "G42 welcomes a new milestone in the U.S.-UAE AI partnership"
  - 02 Jun 2026 — "Core42 advances U.S. AI infrastructure strategy with expanded New York deployment"
  - 15 May 2026 — "G42 and Government of India Formalize Commercial Framework for Condor Galaxy India AI Supercomputer"
  - 03 Jun 2026 — Banco Santander + G42 **Memorandum of Understanding**
- Note the "formalize **commercial framework**" = framework/MoU language, not committed capex.

**Verdict G42: the corporate homepage is unusable as a demand signal.** It carries dates and org headcount but zero dollars, MW or chip counts. Any G42 figure in the write-up must come from a press release *detail* page or an SEC/court/regulatory filing, not this page.

### humain_sa.body — 43,073 bytes, HTTP 200 (previously fetched), AEM HTML → 4,319 chars visible text
**Result: NO quantified commitments.** Regex sweep for currency + digits, and for `GW|MW|GPU|chip` → **1 hit for `GPU` and 1 for `chip`, both in the same unquantified sentence**.

Literal text found (Infrastructure tile): *"From AI training infrastructure like **GPUs**, to next-gen inference engines powered by LPUs, we architect performance from the ground up… our next-generation AI-native data centers… Purpose-built for hyper scale intelligence, these sovereign facilities are robotic, modular, and optimized for the lowest total cost of ownership."*
Also: four-layer stack (Infrastructure / Cloud / Data & Models / Applications); ALLAM Arabic-first LLM **"*Co-developed with SDAIA"**; HUMAIN ONE agent platform.

No MW, no GW, no GPU count, no SAR/USD figure, no date-bearing commitment. Much of the body is raw AEM component config (`trackData`, `cmpid`, DAM image paths) — i.e. CMS scaffolding, not content.

**Verdict HUMAIN: marketing positioning only.** Sovereign AI demand from the Gulf is currently **not measurable from corporate web properties.**

### Consequence for the report
Sovereign-AI demand (UAE/Saudi) cannot be quantified from either primary corporate source. The only quantified, *actually fetched* international number in this run remains **Taiwan export orders: US$97,939 mn in 2026-07, +61.9% YoY** (see L1-international.md).

---

## (3) US EXPORT CONTROLS — Federal Register API (live)
Query: `https://www.federalregister.gov/api/v1/documents.json?per_page=100&order=oldest&fields[]=…&conditions[term]=advanced+computing&conditions[agencies][]=industry-and-security-bureau&conditions[publication_date][gte]=2025-09-20&conditions[publication_date][lte]=2026-09-20`
**HTTP 200, 1,638 bytes.**

- **COUNT of BIS "advanced computing" rule/notice documents in the last 12 months: 2** (`"count":2`, `total_pages:1`).
  1. "Revision to License Review Policy for Advanced Computing Commodities" — **2026-01-15**, type **Rule**, document_number **2026-00789**
  2. (second result truncated in the saved body; first is the substantive one)
- **Most recent: 2026-01-15.**
- Unfiltered `term=advanced computing` across all agencies: `count 4385` — mostly FERC/software noise, hence the agency filter matters.

Cross-check from `l1/fr_all_bis_2026.body` (BIS all document types, published ≥ 2026-07-01): **count 14**, `total_pages 2`, most recent **2026-09-17**. Those are: "Azur Air… / PJSC Aeroflot… / UTair Aviation… Order Renewing Temporary Denial of Export Privileges" (3 Notices, 2026-09-17), plus **"Revisions to the Entity List" (Rule, 2026-08-24)** and **"Removal From the Entity List" (Rule, 2026-08-24)**, plus drone-export and critical-minerals rules.

**Interpretation (constrained):** the BIS agenda in the last 12 months is **not** dominated by new advanced-computing/AI-chip tightening — only the January 2026 license-review-policy revision lands under that term. The active 2026 Q3 flow is Entity List revisions (Aug) and enforcement notices. This *softens* rather than confirms a "policy is tightening on AI accelerators" narrative for the last 12 months; the Jan-2026 item is the real signal.

---

## (1) CHINA — NBS portal and Huawei/Cambricon financials
- **NBS China English data portal: DEAD.** `https://data.stats.gov.cn/english/easyquery.htm?cn=A01` → **HTTP 403, 314 bytes**, `text/html`, and the block page is explicit: `reason:UrlACL`, `Client IP: 2601:647:…:6365`. Same 403 for the query API `https://data.stats.gov.cn/easyquery.htm?m=QueryData&dbcode=hgnd…` → **HTTP 403, 314 bytes, reason:UrlACL**. This is a source-IP ACL (URL allowlist), not a parameter problem — **no parameter tuning will fix it from this host.** China's official accelerator/computing-investment data is not reachable without a proxy or a key.
- Huawei Ascend / Cambricon: see below.

---

## (4) ASML — `l1/asml_subs.body` (offline parse)
File is the **`data.sec.gov/submissions/CIK0000937966.json` payload**, 96,944 bytes — parsed as JSON.
**Contains form metadata ONLY. It carries no revenue line and no China-sales share.** Fields present: `cik, entityType, sic (3559 Special Industry Machinery, NEC), name "ASML HOLDING NV", tickers [ASML, ASMLF], exchanges [Nasdaq, OTC], category "Large accelerated filer", fiscalYearEnd 1231, addresses, filings.recent{accessionNumber, filingDate, reportDate, form, primaryDocument, primaryDocDescription, …}`. `filings.recent.form` has **591 entries**.

Recent filings (from the metadata, no financials):
| # | form | filingDate | reportDate | accession |
|---|------|-----------|-----------|-----------|
| 0 | 6-K | 2026-07-15 | 2026-06-28 | 0001628280-26-048235 |
| 1 | SD | 2026-06-05 | — | 0001628280-26-041057 |
| 2 | 6-K | 2026-04-23 | 2026-04-23 | 0001628280-26-026703 |
| 3 | 6-K | 2026-04-15 | 2026-03-29 | 0001628280-26-025147 |
| 4 | 6-K | 2026-03-11 | 2026-03-09 | 0001628280-26-016671 |
| 5 | 20-F | 2026-02-25 | 2025-12-31 | 0001628280-26-011378 |
| 6 | 6-K | 2026-02-25 | 2025-12-31 | 0001628280-26-011377 |
| 7 | 6-K | 2026-01-28 | 2025-12-31 | 0001628280-26-003701 |
| 8 | S-8 | 2025-11-17 | — | 0001628280-25-052510 |
| 10 | 6-K | 2025-10-15 | 2025-09-28 | 0001628280-25-045043 |
| 11 | 6-K | 2025-07-16 | 2025-06-29 | 0001628280-25-034992 |

**Explicit: no ASML revenue figure and no China-sales share is present in `asml_subs.body`. None is reported here and none is invented.**
Other pre-fetched ASML files were also text-extracted this run and checked: `asml_investor.body` (186,707 B → only 4,157 chars visible; the word "China" appears only in nav/menu strings "China", "China Technology" — no sales split), `asml_6k_recent.body` (17,719 B → 7,979 chars), `asml_6k_listing.body` (15,593 B → 6,276 chars). No revenue or China-share line surfaced in any of them. **To get ASML revenue / China share you must fetch a 6-K exhibit or the 20-F body, not these listings.**
