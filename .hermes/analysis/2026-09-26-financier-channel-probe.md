# The financier channel: probe result, and a defect found on the way

**Initiated 2026-09-26** after the user flagged the Oracle force-majeure story
(Morgan Stanley note, Project Jupiter, Blue Owl/Stack Infrastructure).

The question asked was: *should the model track counterparty-level credit events inside
off-balance-sheet data-centre structures?* The probe answered a different and more
urgent question instead.

---

## 1. What the story says (verified by reading it, not from the headline)

| fact | value |
|---|---|
| Oracle sent a **force majeure notice** to Stack Infrastructure (a Blue Owl unit) | 2026-09-25 |
| Project Jupiter — 1,400-acre campus, New Mexico | power + permit setbacks |
| Notice could defer **full rent payments up to 3 years** if Jupiter misses its 2028 delivery | Bisnow |
| Blue Owl equity / bank debt consortium | **$3B / $18B (~20 banks)** |
| Those bonds have traded **below 90c**, implying **≥$1.8B** paper loss to sellers | WSJ |
| Oracle **2055 bonds at 77c**; 5-year CDS at an all-time high | PitchBook / Bloomberg |
| Oracle shares | **−7% on the week** |
| Root cause named | **local opposition blocked gas turbines and diesel generators**, forcing an energy-strategy pivot |

Two secondary figures, cited but **NOT verified at source** and therefore not used anywhere
in the model: a Brookings estimate of **$10.3T** total AI build-out cost through 2032, and
**~$300B** of AI debt held off balance sheet.

## 2. What the model already saw, independently

Measured live, `--offline`, composite **40.20**, coverage **96.7%**:

| indicator | w | stress | what it reads |
|---|---|---|---|
| `backlog_quality` | 7 | **89.1** | contracted backlog vs cash actually billed |
| `datacenter_construction` | 7 | 66.7 | construction put in place |
| `circularity` | 12 | 46.5 | filing-cited relationships |
| `private_credit_growth` | 6 | 44.7 | the non-bank lending channel |
| `opposition_pressure` | 8 | 37.0 | filed dockets |
| `leverage` | 6 | 34.1 | 1.15x cohort, incl. operating leases |
| `energisation_delay` | 7 | 31.0 | **60.8-month** queue wait |

`opposition_pressure` and `energisation_delay` are the two the story names as causes:
**local opposition** blocked the on-site generation, and **power queueing** is why a
1,400-acre campus slips. Both were already scored. The model was not blind to the
precondition or the proximate cause.

## 3. The gap, confirmed in code rather than inferred

`leverage`'s own output text states the blind spot verbatim:

> "STILL EXCLUDES off-balance-sheet vehicles (SPVs, joint ventures, sale-leasebacks) that
> finance a growing share of data-centre construction, so true leverage remains higher than
> this ratio."

Project Jupiter **is** that structure: Blue Owl equity, a ~20-bank debt consortium, an SPV
Oracle does not consolidate. Verified: the scored EDGAR cohort is five **technology**
filers, and no indicator reads a price, spread or filing for a **lessor, private-credit
fund, or DC REIT**. The counterparty side — the parties who actually bear the loss — are
not modelled at all.

## 4. The counterparty leg is NOT currently fetchable (gate cannot run)

| attempt | result |
|---|---|
| Yahoo `query1` + `query2`, OWL/DLR/EQIX/BIZD/ARCC/SPY/QQQ/NVDA/ORCL/MSFT | **HTTP 429 on every symbol**, still 429 after a 75-second backoff, at 1 request / 2 s |
| Stooq `q/d/l/?s=owl.us` etc. | HTTP 200 but a **JS proof-of-work challenge page**, not CSV — now gated |
| Control probes, same minute | `fred.stlouisfed.org` **200**, `data.sec.gov` **200** |

The contrast is what makes the diagnosis certain: FRED and EDGAR answered instantly while
every Yahoo symbol refused, so this is **Yahoo rate-limiting this host at the IP level**,
not a network failure and not a symbol problem. The tool has 5 cached Yahoo bodies
(`SPY`, `RSP`, `^GSPC`, `^TNX`, `^VIX`) and none of them is financier-side.

**Consequence: the proposed lead/lag gate on the financier channel CANNOT be run today.**
It is blocked on a source, not on a design question. Re-probe before building — this is
the third consecutive day Yahoo has refused bursts from this host.

## 5. THE DEFECT — Oracle is silently dropped from `backlog_quality`

This was found by following the story's counterparty into the model, and it is worth more
than the indicator that prompted the probe.

**Oracle's RPO history (EDGAR `RevenueRemainingPerformanceObligation`, 33 facts, CIK 1341439):**

| period | RPO |
|---|---|
| 2025-05-31 | $137.8B |
| **2025-08-31** | **$455.3B** ← +230% in one quarter |
| 2025-11-30 | $523.3B |
| 2026-02-28 | $552.6B |
| 2026-05-31 | $638.0B |
| 2026-08-31 | **$664.0B** |

**ORCL does not appear in `backlog_quality`'s output.** The rendered text names GOOGL
(51.4x) and MSFT (9.0x) and reports META as absent — Oracle is not mentioned at all.

### Root cause

`CompanyFacts::series_health` is called with a definition-stability limit of **3.0x**.
Oracle's `2025-05-31 → 2025-08-31` step is **3.30x**, so the series is **rejected as a
probable tag-definition change** and the filer is dropped.

That limit is not arbitrary: its doc comment says it exists because of
"AMZN's 3.29x single-year jump is a tag definition". So the threshold was calibrated to
**one company's artefact** at 3.29x — and a genuine 3.30x disclosure at a different company
falls on the wrong side of it.

**Oracle's 3.30x is a real reported fact, not an artefact.** It is the quarter Oracle
disclosed the ballooning contracted backlog. The guard built to catch Amazon's re-tagging
therefore deletes the single most material disclosure in the model — at the exact
counterparty the user's story is about.

### Second defect: the drop is INVISIBLE

`backlog_quality` populates a `rejected` vector when a series fails health checks, but the
rendered `detail` **prints only `no_rpo`** (the "reports no concept at all" case, i.e.
META). Verified by reading the format string: `rejected` is constructed and **never
printed** by this indicator — 0 occurrences of `rejected` in the output branch.

So a reader sees "META not reported" and concludes the cohort is complete apart from META.
Oracle's absence is silent. This is the failure mode the project's own skill names:
*a gap the reader cannot see is worse than a stated one.*

## 6. Direction — and a correction to what I said before

I told the user `backlog_quality` "reads ORCL RPO $137.8B → $455.3B" at stress 89.1.
**That was wrong.** The 89.1 comes from GOOGL's 51.4x divergence; Oracle contributes
nothing because it is dropped. I had carried the RPO figures forward from an earlier
measurement without re-checking that they were still in the cohort. The correct statement
is the reverse: *the model has Oracle's RPO in the source data and discards it.*

## 7. Recommendation

1. **Make the rejection visible** in `backlog_quality` (print `rejected` alongside
   `no_rpo`). Unambiguous, low-risk, and it converts a silent hole into a stated one.
2. **Re-calibrate the growth guard for RPO specifically.** A large jump in contracted
   backlog is the *thing the indicator exists to detect* — the 3.0x limit imports a
   different tag's failure mode. A genuine mega-contract award and a tag redefinition look
   identical to a ratio, so the distinction cannot be made by magnitude; it needs either a
   higher RPO-specific ceiling or a flag-not-drop treatment that reports the step with its
   dates for a human to read.
3. **Then re-open the financier leg**, once a price source answers — the gate still has to
   be run before anything is scored, exactly as it was for tone.

**Do not score the financier channel on the story.** The bond already traded at 77–90c
before any headline, which is the same lag the tone series showed. The prior is that a
counterparty-price leg would fail the lead/lag gate too. That prior is a hypothesis, not a
finding, and it must be tested rather than assumed.
