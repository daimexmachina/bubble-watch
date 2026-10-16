# distress_events: first live measurement, and a precision limit it exposes

**Run 2026-10-16, immediately after building.** The same two queries and the same date windows the
indicator uses, queried directly against CourtListener.

## The measurement

| query | prior year | trailing year | change |
|---|---|---|---|
| `"chapter 11" "artificial intelligence"` | 147 | 61 | **−58.5%** |
| `"chapter 11" "data center"` | 76 | 31 | **−59.2%** |
| **combined (what the indicator scores)** | **223** | **92** | **−58.7%** |

So the channel reads **−58.7%**, below the −50% anchor, i.e. **LOW stress — calm.** AI-related
insolvency docket activity FELL by well over half year-on-year.

**This cuts against the "we are close to a crash" reading**, and it is the second independent measure
today to do so, after Oracle's own debt/CFO falling from 7.95× (FY2022) to 4.05× (FY2026). Two different
sources, two different methods, same direction. That is reported because it is what the data says, not
because it is the wanted answer.

## A PRECISION LIMIT THIS EXPOSED, which must be fixed or declared

The queries match docket **text**, not company identity or case type. A docket that merely *mentions*
"chapter 11" and "artificial intelligence" — an adversary proceeding, a lawsuit against an AI company,
a contract dispute naming one — counts exactly as much as an AI company's own bankruptcy petition. The
622-docket figure quoted when choosing the source has the same character.

So the indicator currently measures **"dockets whose text contains both phrases"**, which is broader
than its label ("AI-exposed company insolvencies") claims. The label overstates the precision.

Two consequences, stated plainly:
1. The `−58.7%` is a real measurement of that broader quantity, and the DIRECTION is informative — the
   activity around AI insolvency did fall. But it is not a count of company failures.
2. The fall may partly reflect docket-indexing effects rather than a genuine decline in failures.
   RECAP is crowd-sourced and its coverage is not stationary, so two adjacent 365-day windows are not
   guaranteed to be equally well covered. This has NOT been checked and is the obvious next test: compare
   a fixed query's count across several historical windows to see whether coverage drifts.

## What would fix the precision

Narrow the query from text-match to case identity — e.g. filter on `court` being a bankruptcy court and
match the debtor's NAME, or use CourtListener's docket `nature_of_suit` / party fields rather than a
full-text `q`. That would make the count mean "AI-exposed debtors in bankruptcy" rather than "dockets
mentioning the words". Not done here.

## Status

Built and committed (`9189942`), first live reading taken, 438 tests green. Scored on its justification
rather than on demonstrated timing — the lead/lag gate has NOT been run on it, and given what the media
gate showed today (commentary trails; a petition is filed the day of) the honest framing is that this
channel is CONTEMPORANEOUS, which is what a distress measure should be, not that it leads.
