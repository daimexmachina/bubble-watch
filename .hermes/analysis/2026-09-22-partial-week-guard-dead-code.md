# inference_demand read a 2-day-old week as a finished one — and a guard that could never fire

**Found:** 2026-09-22, while answering "did the run start okay?" · **Fixed:** `d380edb`

This is the most consequential bug found in this project so far, and it was found by
noticing that two runs disagreed, not by a test.

---

## How it surfaced

The scheduled run succeeded. But its composite was **44.76** against **40.33** from my
manual run the same day. The per-indicator diff located it immediately:

| indicator | manual 09-21 01:24 | scheduled 09-21 20:34 |
|---|---|---|
| **inference_demand** | **26.84** | **96.00** |
| breadth | 44.04 | 51.87 |

A 69-point move in an indicator added one day earlier. Then the giveaway: **both runs
read the same week label (`2026-09-21`) with different token values** — 1.772e13 vs
3.853e13. The same week cannot hold two different totals; that is the signature of
reading an *unfinished* week at two different points in its own life.

## Root cause: the guard was dead code

`drop_partial_trailing_week` dropped the trailing week only when it was **>10x smaller**
than the previous one. That threshold was calibrated against the extreme artefact it was
originally built for — 5.8e10 against 1.29e14, a factor of ~2,200 — and so it could not
fire for a week that was merely **young**, which is the ordinary case.

Measured 2026-09-22:

```
trailing week 2026-09-21   3.88e13
previous week 2026-09-14   1.29e14
ratio = 3.32x   →  under the 10x threshold, so KEPT
```

The date arithmetic shows why the threshold was unworkable: firing at ratio > 10 requires
**fewer than 0.7 of 7 days** to have elapsed. The guard was effectively unreachable.

## The cost: 62.6 stress points, pointing the wrong way

Scoring a 2-day-old week as complete turned **+176.3%** 13-week growth into **−17.5%**:

| | growth | stress |
|---|---|---|
| buggy (partial week kept) | −17.5% | **89.4** |
| correct (partial week dropped) | +176.3% | **26.8** |

Because the anchors **invert** (rising usage = less bubble stress), this inflated the
composite rather than depressing it. It read as **demand collapse at the moment demand
had grown** — and on a weight-10 indicator it moved the composite ~4 points, which is
larger than most genuine day-to-day market movement.

## The fix

A calendar test, which is what completeness actually is: a trailing week whose START
label is less than `WEEK_COMPLETE_DAYS` (7) before today is in progress and is dropped.
Magnitude is gone from the decision entirely, because magnitude cannot distinguish
"young" from "genuinely collapsed".

Week labels are the week's START — confirmed by arithmetic rather than assumed: the
2026-09-21 row held ~28.6% of a full week's tokens on 2026-09-22, i.e. 2 of 7 days, and
3.88e13 ÷ 0.286 = 1.36e14 ≈ the prior complete week.

## Falsified, all three

1. guard never fires → `a_partial_trailing_week_is_dropped_not_read_as_a_collapse` FAILED
2. guard drops complete weeks → `a_COMPLETE_trailing_week_is_kept` FAILED
3. reverted to the old magnitude test → `a_MERELY_YOUNG_week_is_dropped_even_though_it_is_not_small` FAILED

Test 3 is the regression test, and its shape matters: it asserts the **precondition**
that the week is under the old threshold (3.32x < 10x), so it fails if anyone
reintroduces a magnitude test that would let this through again.

## Archive corrected

The two rows written by the buggy build were removed as artefacts:

```
REMOVED  2026-09-21 20:34  comp 44.76  inference_demand 96.00
REMOVED  2026-09-22 20:35  comp 44.20  inference_demand 89.51
```

A corrected run was recorded. The archive now holds honest 2.4 rows at **40.33** and
**40.62**, with `inference_demand` steady at **26.84** across both — which is what a
correctly-functioning indicator reading a real +176% growth should do.

## What this says about the guards

The project's rule is that a check which cannot fail manufactures confidence. This guard
**could** fail — it had passing tests, and one of them exercised the 2,200x case — but it
could not fail *for the case that actually occurs*. A test at the extreme of the artefact
is not coverage of the artefact's ordinary form.

The generalisable lesson: when a guard is written from a single observed example,
calibrate it against the **mechanism** (here: how much time has passed), not against the
**magnitude of that one example**. Magnitude thresholds silently encode the specific
numbers that motivated them.
