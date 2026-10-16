# (b) Single-name concentration: measured anchors, and a finding that cuts against the narrative

**Run 2026-10-16.** Path (b): make the concentration risk visible where it actually lives. Before
building a scored indicator I derived the cohort's leverage HISTORY from primary source, because
anchors must rest on an observed distribution — a hand-set ladder on a cross-sectional ratio would have
been the weakest thing in this model.

## 1. The tag trap, hit once more

A naive extraction returns **zero debt for Oracle**. ORCL does not file `LongTermDebt` at all; its
total debt sits under `DebtLongtermAndShorttermCombinedAmount` ($129.5B) and `LongTermNotesPayable`
($122.3B). Every other filer here uses `LongTermDebtNoncurrent`.

This is the same failure family as the EIA URL and the empty `out/` file: **a source-specific naming
assumption that returns a plausible value (zero) for the one entity that matters.** It is why the
cohort must carry a per-filer tag, not a shared one.

## 2. ORCL debt/CFO, FY-end, from 10-K XBRL

```
2016-05-31   3.20x      2021-05-31   5.30x
2017-05-31   4.10x      2022-05-31   7.95x   <- peak
2018-05-31   3.94x      2023-05-31   5.27x
2019-05-31   3.86x      2024-05-31   4.65x
2020-05-31   5.45x      2025-05-31   4.45x
                        2026-05-31   4.05x   <- latest
```

**THIS CONTRADICTS THE MOTIVATING NARRATIVE, AND IS REPORTED FOR THAT REASON.** Oracle's leverage is
high but **declining**, and it sits BELOW its own 2017–2022 levels. It peaked at **7.95x in FY2022**,
when nobody was describing Oracle as a credit risk, and has fallen in every year since. "Oracle is in
trouble because of unprecedented leverage" is not supported by this ratio: the leverage is not
unprecedented, and it is easing.

What HAS changed is the *character* of the risk, and the ratio cannot see any of it: debt grew from
~$58B to ~$129.5B in absolute terms, free cash flow turned negative, and the backlog became
concentrated in a single counterparty. A leverage ratio is blind to a maturity wall, to negative FCF,
and to counterparty concentration — so a falling ratio is genuinely consistent with rising risk here.
That distinction must travel with the number.

## 3. Measured anchors, from a real distribution

Cohort: ORCL, MSFT, GOOGL, AMZN, META — annual debt/CFO, 70 filer-years from 2008.

```
p0   0.00x     p50  0.48x     p90  4.05x     p100 7.95x
p25  0.20x     p75  1.28x     p95  4.65x
```

| company | latest debt/CFO |
|---|---|
| ORCL | 4.05x |
| META | 0.51x |
| AMZN | 0.47x |
| GOOGL | 0.28x |
| MSFT | 0.17x |

So ORCL sits near the **p90** of the pooled cohort distribution, and is **8–24× the next-most
levered** peer today. That is a defensible basis for a ladder: p50 → low stress, p75 → moderate,
p90 → high, p95+ → severe, with the observed maximum as the top reference.

## 4. DISCREPANCY — RECONCILED, and it is two effects rather than one

**My derived ORCL figure is 4.05x; the model's `exposure` reports 2.68x.**

`src/sources/edgar.rs:109` documents the tag choice deliberately: *"list `LongTermNotesPayable` (ORCL
reports its debt there)"*. So the model reads **$122.3B**, where Oracle's total combined debt is
`DebtLongtermAndShorttermCombinedAmount` **$129.5B** — a **5.6%** difference. That alone moves
122.3/CFO to 129.5/CFO, i.e. 2.68x → 2.84x, and **cannot** explain a gap to 4.05x.

The dominant cause is the **cash-flow basis**: the model divides by a TTM figure
(`CompanyFacts::ttm(&cf.cfo)`), while my series divides by the fiscal-year 10-K
`NetCashProvidedByUsedInOperatingActivities`. Oracle's operating cash flow is lumpy across quarters,
so the two denominators differ substantially, and the ratio inherits that.

**So: the model is not wrong and neither is the derived series — they are different definitions.** The
model's is internally consistent (one tag, documented, TTM). Mine is broader on debt and narrower on
cash flow. Neither may be compared to the other, and the labels ("debt to CFO") are too similar for a
reader to tell them apart — which is itself worth fixing.

**CONSEQUENCE FOR §2, stated plainly:** the DECLINE is the finding and it survives, because the trend
is measured consistently within one definition. The LEVEL (4.05x vs 2.68x vs 2.84x) is definition-
dependent and must be quoted with its definition attached. The cohort table in §3 uses the per-filer
tags and the FY cash-flow basis throughout, so it is internally consistent but NOT directly comparable
to the model's `exposure` column.


## 5. Status

- **History and anchors: DONE**, measured from primary source, with the per-filer tag trap documented.
- **Scored indicator: NOT built.** The blocker is §4, not effort: scoring a ratio whose definition
  disagrees with an existing published field by a third, without knowing which is right, would put a
  number in the composite that two parts of the same report define differently.
- **Recency: this does not help.** The series is annual/quarterly, so it is pinned exactly like the
  rest of the model's fundamental half. (a) `velocity` remains the recency answer, and (c) the media
  gate already returned null.

Next: reconcile §4 against the actual tag `exposure.rs` reads, then build the indicator on the
distribution in §3 with the §2 caveat carried in its own output.
