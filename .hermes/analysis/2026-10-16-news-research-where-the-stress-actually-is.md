# News research brief — 2026-10-16: where the AI-economy stress actually is, and what this tool is not measuring

**Compiled:** 2026-10-16 evening PDT, from live web research. Every figure below is sourced and
was retrieved today; nothing here is inferred or reconstructed from memory.

**Why this brief exists.** Daim asked repeatedly why the composite (39.9/100, "mid") does not move on
bad AI news. This research gives a concrete answer: **the stress is concentrated in two channels the
tool does not measure at all**, and in one channel it *deliberately* measured the wrong layer. The
score is not conservative — it is partly blind, and the blind spots are exactly where the action is.

---

## 1. The credit channel — the leading edge, and the biggest blind spot

**What is happening.**
- CoreWeave: **>$51bn of debt and obligations**. Its 9.75% notes due 2031 hit a **yield to maturity of
  15.78%**; bond prices fell to 78 before recovering to 83. *(CNBC 2026-10-15; Bloomberg)*
- CoreWeave 5-year **CDS peaked >1,140 bps**, last ~1,038 bps — **+16% week-over-week, +164% year-over-year**.
  S&P Global's Gavan Nolan: **1,000 bps and above is generally regarded as credit distress.** *(CNBC)*
- Evercore ISI's trading desk: the market is *"basically saying 'no mas' to AI debt without operating
  cash flow support."*
- Bonds funding several different data centres have tumbled — one investor called it a **"Darwin moment."**
  *(FT, via Axios)*
- S&P **caps data-centre ABS at single-A** and declines to rate them higher. Moody's has issued an opinion
  that threatens to derail off-balance-sheet data-centre deals. *(GlobalCapital; IFR)*
- CMBS investors growing cautious as AI-linked data-centre debt spreads widen. *(Bloomberg 2026-08-06)*

**Why it matters more than equity.** Axios, citing Bloomberg Intelligence's Robert Schiffman:
**"Credit markets are where some of the AI boom's weaknesses can show up first."** In every prior large
investment boom that later broke — railroads, housing — **the bonds warned first.** Bondholders ask
"will I get paid back"; equity holders ask "how high will this go." The tool currently has **no
AI-specific credit measurement whatsoever.**

**The specific trap in this tool.** `credit_hy` uses **BAA10Y**, which reads **1.44% — the 2nd percentile
of its own history since 1986, i.e. near the tightest spreads on record.** That is true of broad
investment-grade credit. It is the *opposite* of true for AI-specific credit. **The aggregate says calm
while the specific is in distress**, and the tool scores only the aggregate.

---

## 2. Local opposition — the tool measured the wrong layer, and knew it

**What is happening.**
- **$260 billion** of planned US data-centre projects were called off in 2026. *(Heatmap Pro, via QZ
  2026-10-16)*
- Cancellations are **accelerating**: in dollar terms, projects killed **July–September 2026 exceeded
  every data-centre project cancelled in all of 2025 combined.**
- Compare the prior baseline: **$18bn blocked + $46bn delayed = $64bn** across May 2024 – March 2025.
  *(Data Center Watch)* **The figure roughly quadrupled.**
- **142 activist groups across 24 states**; **55% Republican / 45% Democratic** officials opposing —
  bipartisan, and it cuts against the usual ideological lines. *(Data Center Watch)*
- The mechanism is **local**: most projects died when a developer could not obtain a local permit or a
  town/county government acted against it. A supportive White House has limited control, because
  permitting is local.

**The defect this exposes.** `opposition_pressure` counts **court filings** (`"data center" moratorium /
lawsuit / injunction` docket counts) and reports filings **fell ~17% year-over-year**. Meanwhile local
opposition cancelled a **quarter of a trillion dollars** of projects.

The tool's own detail text already says the quiet part:

> *"Legistar (needs a commercial licence, HTTP 500), the Socrata open-data catalog (0 datasets), and FERC
> eLibrary (no public endpoint) … This is therefore a LOWER BOUND on opposition."*

So the tool **knowingly measured the litigation layer and declared the local layer unmeasurable** — and
the local layer turned out to be where the money dies. A falling court-filing count is not evidence that
opposition is easing; it is evidence that opposition moved to a layer the tool cannot see.

**A live example.** In late June 2026 residents of three Illinois towns turned out against Karis
Critical's rezoning request at Plum Farms, and **the developer withdrew.** *(Capitol News Illinois, via
QZ)* Most such fights are won only because the developer chose to leave — reversible, and not visible in
any docket.

---

## 3. The neocloud economics — the leverage is worse than the headlines suggest

From BlockWest Research's Q2 2026 neocloud scorecard (data as of 2026-10-07):

- CoreWeave, Nebius and Nscale hold about **$247bn of contracted revenue ≈ 19× their combined annualised
  revenue of ~$12.9bn.**
- **CoreWeave Q2 2026:** revenue $2.575bn (+112% YoY) — but **net interest expense $640m, consuming 25% of
  revenue and 42% of adjusted EBITDA**; **total debt $35.1bn, up 64% in six months**; cash capex $6.4bn
  (**2.5× revenue**); cash $5.5bn.
- **Nebius** spent **9.7× revenue** on capex in Q2.
- **Customer concentration:** three customers were **72%** of CoreWeave's Q2 revenue; a single Anthropic
  contract (up to ~$44.6bn) is **43%** of Nscale's contracted value.
- **Oracle** as the scaled comparison: cloud infrastructure revenue $7.4bn, **RPO $664bn**, capex $28.5bn
  in the quarter, **debt $125.3bn**.
- Despite growing backlogs, the listed names fell in Q3 2026: **CoreWeave −12.5%, Nebius −14.6%,
  Applied Digital −34.7%** (June 30 → Sept 30).

**Gartner (2026-10-14):** more than **80% of neocloud providers will cease operating independently by
2030.** *"All of them want to be the full-stack. They can't all be,"* said Will Sommer.

---

## 4. The revenue gap — the fundamental arithmetic

**Gartner's analysis (2026-10-14, via Bloomberg):**
- AI data-centre owners must generate **$8.5 trillion of cumulative revenue through 2030** to cover capex,
  financing costs and a 12% return on invested capital.
- Gartner forecasts **no more than $3.6 trillion** in attributable enterprise and consumer AI revenue over
  the same period — **less than half the required amount.**
- **More than $3.5 trillion of AI infrastructure spending is at risk of impairment by 2030.**

Also of note: **IMF Global Financial Stability Report, October 2026**, contains a background analysis
titled *"Rising Leverage in the AI [sector]"*; the **FSB** published a report on vulnerabilities in private
credit; the **BIS** published on financing the AI infrastructure boom. Leveraged loans backing data centres
reached **$16.5bn this year, against $2bn in 2025.**

---

## 5. Compute pricing — the revenue side is turning

- Live H100 rental listings ranged **$0.72 to $15.14 per hour within a single 24-hour window** — a 21×
  spread across 24 marketplaces, 141,000+ observations since December 2025. *(Data Center Knowledge,
  2026-05-13)
- GPU rental markets are showing **early price compression**; H100 oversupply risk against Blackwell
  scarcity. *(Data Center Knowledge; SemiAnalysis H100 1-year rental price index)*
- **JPMorgan:** token prices and H100 prices are **falling together**.

This is the **revenue line** of the AI economy, and the tool measures none of it.

---

## 6. Actual failures — the signal Daim has been pointing at

- **Character.AI filed for bankruptcy** on 2026-10-13, five years after founding. *(TechCrunch)*
- **Momentum Cloud AI Ltd went into liquidation** on 2026-09-30. *(K2 Partners)*
- **Relay** (AI workflow automation) shut down entirely. *(TechCrunch, Aug 2026)*
- **~42% of AI initiatives are ultimately abandoned** by their corporate parents. *(S&P Global Market
  Intelligence)*
- TechCrunch maintains a running **"AI graveyard"** of projects and startups that failed.
- **ailayoffs.org** publishes a source-cited register of **243** AI layoffs.

---

## 7. Demand-side and macro wobbles

- **Anthropic:** Q3 2026 revenue **$16.8bn, +45–46%** — but the *slowdown in growth* was read as
  "the AI trade's latest red flag" *(MarketWatch)*; Reuters ran *"Anthropic's $2trln dream faces
  deflation trap."*
- **OpenAI paused its UK data-centre project** over regulation and cost. *(Reuters 2026-04-09)*
- **Oracle:** $18bn of data-centre debt under pressure; credit-downgrade risk as AI spending outpaces
  cash flow. *(Reuters; Bloomberg)*
- **Fortune (2026-10-07):** AI is heading into a **cash-flow-negative inflection point**, and spending is
  expected to slow in 2027.
- **Macro driver:** the Iran war pushed energy prices up, feeding inflation, driving rates higher —
  squeezing exactly the weaker borrowers that need to refinance.
- iShares Semiconductor ETF on an **eighth straight decline**; AMD and Nvidia selling off.

---

## What this means for bubble-watch — the actionable part

The composite reads 39.9/100 "mid" and barely moves because **it is measuring the calm aggregates and
missing the stressed specifics.** Ranked by leverage:

| # | Blind spot | Evidence it matters | Cost to measure |
|---|---|---|---|
| 1 | **AI-specific credit** | CoreWeave CDS 1,038bps, yields 15.78%; S&P caps data-centre ABS at single-A; credit warns first (railroads, housing) | Proxy via the **public borrowers'** equity — CRWV, NBIS, IREN, APLD, WULF, CORZ — free (Yahoo) |
| 2 | **Local/permitting opposition** | **$260bn cancelled in 2026**, ~4× the 2024–25 rate; 142 groups; bipartisan | **GDELT news queries** — free, keyless, and the GDELT source already exists |
| 3 | **Neocloud leverage** | $247bn contracted vs $12.9bn revenue; interest = 42% of CoreWeave EBITDA; 80% of neoclouds gone by 2030 (Gartner) | Balance-sheet data; quarterly |
| 4 | **Compute pricing** | H100 $0.72–$15.14/hr; token and H100 prices falling together | Publishes exist; needs a source |

**The single most misleading line in the current report** is the credit reading: *"credit spread BAA10Y
at 1.44%, the 2nd percentile of its own history since 1986. Spreads are near the TIGHTEST levels on
record."* That is accurate about broad IG credit and **actively misleading** about the AI economy, whose
credit is simultaneously in distress. The tool presents a calm number where a specific, measurable
distress exists — the same class of defect as the silent query subset, in a different channel.

**Two channels are free and buildable immediately**, using infrastructure that already exists:
1. **GDELT cancellation/opposition volume** — the `gdelt.rs` source already fetches news timeline data,
   keyless. A query on data-centre cancellation/withdrawal would have caught the $64bn → $260bn escalation.
2. **Debt-funded builder basket** — the `yahoo.rs` source already fetches daily prices. Adding the
   borrowers (CRWV, NBIS, IREN, APLD, WULF, CORZ) measures the stress directly, daily, at no cost.

Neither requires a key, a paid feed, or a new transport.

**Caveat on direction, stated before any of this is scored:** Gartner's revenue gap and the credit
warnings are *bearish* evidence; Daim's thesis that failures signal a healthy clearing is a *different*
claim that this research does not settle. Both should be built as **declared, unscored channels first** —
measured and rendered every day, with the direction stated — exactly as `gdelt_tone` and
`public_attention` already are. Weight follows only if a pre-registered gate passes.
