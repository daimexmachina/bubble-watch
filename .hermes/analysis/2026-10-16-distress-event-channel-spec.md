# Spec: a distress-event channel — the events are capturable, but NOT where I first looked

**Status: research complete, awaiting the build decision.** Prompted by Daim: Oracle is one example,
not the pattern. He cited **Character.ai filing Chapter 11** and **Firmus pulling its IPO**, and asked
for these to be captured *as they happen* and factored into the score.

He is right that they should be. What follows is where they are actually observable.

## 1. Both examples verified from primary sources

**Character Technologies, Inc. (Character.ai)** — Chapter 11, Case **26-11603 (BLS)**, U.S. Bankruptcy
Court for the District of **Delaware**, **petition date 2026-10-12**; Notice of Commencement 2026-10-15.
First-day declaration: burns **$8–9 million per month to run operations** (the cited "$14mm/month on
compute" is not the court figure — this is total operations, per the CEO's declaration under penalty of
perjury); the estate is described as a *"melting ice cube"*; Disney is stalking horse at **$20m** for a
non-exclusive perpetual licence of certain IP, with a **$7.5m** up-front payment pre-petition ($2m of it
a refundable deposit); absent a qualifying bid **the platform will be wound down**.

**Firmus Grid** — withdrew its **ASX** listing application on **2026-10-09**, having sought ~**US$5.5bn**
at a ~**US$30.6bn** valuation. Nvidia-backed, with Blackstone, Coatue and Jane Street as shareholders.
UniSuper, one of Australia's largest pension funds, declined to participate, citing valuation *and*
concern that Firmus *"would have to go further into debt to fund its growth plans"*.

## 2. THE DATA-AVAILABILITY FINDING — the two examples are the two hard classes

Measured 2026-10-16. Both candidate sources reachable, keyless:

| | EDGAR full-text | **CourtListener** |
|---|---|---|
| `"Character Technologies"` | **0 hits** | **24 dockets**, incl. **2026-10-12, D. Del. Bankruptcy** |
| AI-related Chapter 11 | **3** (`"Item 1.03" "artificial intelligence"`), **4**, **20** | **622** |
| coverage | SEC registrants only | filed dockets, incl. **private** companies |
| non-US | none | none |

**EDGAR CANNOT SEE THE CASE THAT PROMPTED THIS.** Character.ai is a private company and not an SEC
registrant (`"Character Technologies"` returns **zero** EDGAR hits). Nor can it see Firmus, which
withdrew on the ASX and made no US filing.

**CourtListener CAN.** Its first result for `"Character Technologies"` is the Chapter 11 petition
itself, dated the day it was filed. The same source is **already wired into this model** for
`opposition_pressure`, and the config already justifies scoring exactly this class of data:

> *"IT COUNTS FILED COURT DOCUMENTS, NOT NEWS COVERAGE. … A docket is a recorded event and can be scored;
> salience is a transmission channel and is not."*

So the mechanism the requested feature needs **already exists in the codebase**, pointed at a different
query. That is the build: extend the court-document channel with insolvency queries.

## 3. Why this is NOT the media channel, restated

(a) `velocity` and (c) the media gate already showed that *commentary* about events lags the events. An
event is different: a Chapter 11 petition is filed ON the day, is a matter of public record, is
citable, and is not a sample. This is the same reasoning that made `narrative_saturation` defensible as
a filing census and `opposition_pressure` defensible as a docket count.

## 4. Proposed indicator (NOT built)

A `distress_events` count over tracked insolvency queries — e.g. Chapter 11 petitions by AI-exposed
operating companies, and the US analogue of an IPO withdrawal (SEC form **RW**, 250 filings YTD, which
is a *filing* event and so EDGAR-visible). Counted as a **census of filed documents**, scored on a
year-over-year change basis like `opposition_pressure`, with anchors measured from the count's own
history rather than chosen.

**DECLARED LIMITS, each already measured:**
- **US only.** Firmus is invisible; a non-US insolvency is a declared blind spot, not a zero.
- **RECAP is crowd-sourced.** CourtListener's bankruptcy coverage is incomplete and sometimes delayed,
  so any count is a **FLOOR, not a census**. The indicator must say "at least N", never "N".
- **Sparse by construction.** AI-specific bankruptcy is rare (622 AI-related Chapter 11 dockets across
  the full RECAP history, not per year). A rare-event channel will read near zero most months and jump
  when something happens — which is correct behaviour for a distress measure, and means its variance is
  lumpy rather than smooth.
- **Not yet gated.** Before scoring it needs the same treatment as every other predictor: a lead/lag
  gate against a suitable target, with a shuffle null, on **differenced** series (see the near-miss
  recorded in the media gate doc).

## 5. Status

Research complete. The events ARE capturable as they happen, and the model already has the transport
and the justification — but the flagship example sits in a class (private US companies) that the
obvious source (EDGAR) cannot see, and in a class (non-US) that neither can. Building it means
extending `courtlistener.rs` with insolvency queries and gating the result; that is the next step.
