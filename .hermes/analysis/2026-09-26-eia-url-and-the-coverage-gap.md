# The 96.7% was an EIA URL bug — fixed, and it was hiding a second defect

**Resolved 2026-09-26.** Daim noticed recent runs read 97% where every earlier run was 100%.
The cause was a single wrong path segment, and finding it exposed a reporting defect worth more
than the fix itself.

## 1. The arithmetic was exact, and there was one cause

```
grid_cancellations  weight 6 of 183  =  3.28%   ->  coverage 100 - 3.28 = 96.72%   ✓
```

Every non-100% run in the 72-row archive was missing exactly this indicator. First failure
**2026-09-25T20:33:44**. The archive has only ever held four coverage values: 100%, 96.7%, 96.6%,
87.4% (the last is an `--offline` run, also missing SEC/CourtListener/Wikipedia by design).

## 2. Root cause: one wrong path segment

`eia.rs` requested `/eia860m/xls/july_generator2026.xlsx`. July was no longer the current month, so
EIA answered with its **landing page**:

```
http=200   content-type=text/html   bytes=55725      <- not a workbook
```

The parser correctly refused to read HTML as xlsx. **The guard was working** — a 200 was not treated
as success. The *request* was wrong. The same file at `/eia860m/archive/xls/` returned
`http=200, xlsx, 13932124 bytes`, containing exactly the sheets the parser looks for by name
(`Planned`, `Canceled or Postponed`). Verified by downloading it and reading the sheet names, so no
parsing change was needed.

## 3. A fixed filename cannot work, and neither can a single directory

EIA **rotates** the month, so a hardcoded name is guaranteed to rot. Worse, and discovered only by
probing both directories:

| vintage | `/eia860m/xls/` (current) | `/eia860m/archive/xls/` |
|---|---|---|
| september 2026 | HTML 55KB | HTML 55KB |
| **august 2026** | **xlsx 14.0MB** | HTML 55KB |
| **july 2026** | HTML 55KB | **xlsx 13.9MB** |

**August existed only in the current directory; July only in the archive.** A month lives in exactly
one of them, because EIA publishes to `/xls/` and later moves it to `/archive/xls/`.

This produced a bug in my own first fix: the resolver tried the archive path only, so it silently
settled for **July — one month stale — while reporting coverage of 100%**. That is the same class of
failure as the original (a stale read presented as fresh), and it is why the fix now walks both
directories per candidate month.

Also measured: the index page **links** `/xls/october_generator2026.xlsx` and that URL returns the
55KB HTML page. **A link on the index is not evidence a file exists** — only the response bytes are.

## 4. The defect worth more than the URL

The real error **was** captured (`sources/mod.rs` pushes a `SourceFailure`; health read
`unavailable, failed: 1`). What reached the reader was:

> "the workbook could not be retrieved or the sheets were not parseable"

One sentence covering both a network fault and a parse fault, stating neither. A reader could not
tell **"EIA is down" (wait)** from **"our URL is stale" (fix it)** — and the true answer was *fix it*,
sitting there for two days while the report implied a source outage. The indicator now surfaces the
actual failure, and the resolver's error names every candidate tried, in which directory, and why
each was rejected.

## 5. What was changed

- `resolve_newest_workbook(reference_ym, months_back, fetch)` — the walk, with **fetching injected**
  so the walk itself is testable (see §7).
- `candidate_months()` — starts **one month behind** the reference date and descends, because EIA
  links the current month's filename before it resolves.
- `is_workbook(bytes)` — checks ZIP magic `PK\x03\x04`. Status codes and content headers are **not**
  authoritative here, since the failure mode is a 200 with an HTML body.
- `current_path_url()` / `archive_path_url()` — both tried per month.
- `as_of` is now **derived** from the vintage actually read. It was hardcoded `"July 2026"`, so the
  field a reader uses to judge staleness had itself gone stale — and had already moved on to August.
- The dead `pub const URL` (the hardcoded month) is removed, not left dormant.

**Result:** coverage 100%, 183 of 183 weight, `as_of` = "August 2026", `grid_cancellations` reads
**39.1%** (August) where July read 38.7%. Composite 40.1767 -> 40.7824.

## 6. No methodology bump, and my earlier claim was wrong

I said restoring the source "requires a methodology bump 2.5 -> 2.6". **That was wrong**, and the
archive settles it. Coverage already varies *within* a single version:

```
methodology 2.4: coverage values seen = [96.6, 100.0]
methodology 2.5: coverage values seen = [87.4, 96.7, 100.0]
```

The config defines a methodology change as adding/removing an indicator or changing weights
("Total weight 175 -> 183, so runs archived under 2.4 are non-comparable"). Here **no weight, no
anchor and no indicator set changed** — weights are untouched at 183. Renormalising over available
weight is the designed behaviour for a missing source, so the composite moved because a source
became *measurable*, not because the model was redefined.

Checked independently: the strip-out test is consistent to 4e-8, which is 25,000x below the printed
resolution (3 decimals). The residual is not a market move and not a redefinition.

## 7. Falsification — including a vacuous test I wrote and had to discard

Every guard was deliberately broken and watched to fail:

| guard | broken by | result |
|---|---|---|
| `is_workbook` | made it always `true` | **FAILED** as required |
| `candidate_months` | made it start at the current month | **FAILED** as required |
| failure message | made it generic again | **FAILED** as required |
| **the walk** | reverted to archive-only | **passed — VACUOUS** |

The fourth is the important one. My first two-path test asserted on the **URL builders**, which are
unchanged by removing the walk logic, so it passed while the bug was present. It could not fail on
the bug it was written for. Fixed by extracting `resolve_newest_workbook` with the fetcher injected,
then asserting which vintage comes back when a month exists in only one directory. Re-broken, it now
reports `("july", 2026)` against an expected `("august", 2026)` — it fails, correctly, and passes on
the fixed code.

This is the same trap recorded after the sub-scores work: **a passing check that cannot fail
manufactures confidence.** The URL-builder test would have shipped as "coverage of the new logic".

## 8. Open — flagged, not changed

The report reads *"Direction of travel: flat +0.6 over 1 day(s) against the 2026-09-26 baseline
(96.72131147540985% coverage)"*. That +0.6 is mostly the **restored indicator**, not a market move:
the baseline lacked a source, so the two numbers were computed over different weight sets.

By the archive's own precedent, comparing across a coverage change is permitted within a version. But
presenting it as "direction of travel" has the same shape as the failure the methodology guard
exists to prevent — an accounting change reported as a market move. Whether the trend code should
also refuse a baseline across a coverage change is an **owner decision** (it changes what the tool
refuses to say), so it is recorded here rather than changed.

## 9. Not concluded

- Not concluded the score is now "correct" — 3.28% of weight was missing and is now present, which
  is a better measurement, not a validated one.
- Not concluded the archive naming scheme is stable. The resolver fails loudly if four consecutive
  months all read as non-workbooks, and says the naming pattern needs revisiting.
