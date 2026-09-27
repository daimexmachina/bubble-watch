# The 96.7% is a URL bug, not a data outage — and the reader is told neither

**Investigated 2026-09-26** after Daim noticed the last runs sit at 97% while every earlier run was
100%. Answer: `grid_cancellations` (weight 6 of 183 = **3.28%**) has been failing, and it has been
failing for a fixable reason that the report never states.

## 1. The arithmetic is exact, and there is only one cause

```
grid_cancellations  weight 6 / 183  =  3.28%   ->  coverage 100 - 3.28 = 96.72%   ✓
```

Every non-100% run in the 72-row archive is missing exactly this indicator. The archive contains
only four distinct coverage values, ever: **100%, 96.7%, 96.6%, 87.4%** — and the 87.4% row is my own
`--offline` run (which also lacks the SEC, CourtListener and Wikipedia sources by design, 23 weight).

First failure: **2026-09-25T20:33:44** (96.57%). It has failed on every live run since.

## 2. The root cause: one wrong path segment

`src/sources/eia.rs` requests the current-month directory:

```
https://www.eia.gov/electricity/data/eia860m/xls/july_generator2026.xlsx
```

But July 2026 is no longer the current month, so EIA **serves its landing page instead**, with:

```
http=200   content-type=text/html   bytes=55725      <- an HTML page, not a workbook
```

The parser correctly refuses to read a non-zip as xlsx, so the indicator degrades to a declared gap.
**This is the guard working** — a 200 is not treated as success. The defect is that the *request* is
wrong, and that the *reason* never reaches the reader.

The same file at the archive path returns the real thing:

```
https://www.eia.gov/electricity/data/eia860m/archive/xls/july_generator2026.xlsx
http=200   content-type=application/vnd.openxmlformats-...sheet   bytes=13932124   <- 13.9 MB
```

And the workbook contains **exactly the sheets the parser looks for by name**:

```
Operating · Planned · Retired · Canceled or Postponed · Operating_PR · Planned_PR · Retired_PR
                                ^^^^^^^   ^^^^^^^^^^^^^^^^^^^^^
```

So the download path is the only fault. No code change is needed to parse it — verified by
downloading the correct URL and reading its sheet names.

## 3. The month rotates, which is why a fixed filename cannot work

EIA's *current* directory serves whichever month is newest, so hardcoding a month in the current
directory is a guaranteed future break, and the archive directory needs a month that actually
exists. Probed live:

| path | result |
|---|---|
| `/xls/july_generator2026.xlsx` (what the code uses) | 200 `text/html` 55KB — landing page |
| `/archive/xls/july_generator2026.xlsx` | 200 **xlsx 13.9MB** |
| `/xls/august_generator2026.xlsx` (current month) | 200 **xlsx 14.0MB** |
| `/xls/october_generator2026.xlsx` (listed but not published) | 200 `text/html` 55KB — landing page |

Note the last row: the index page *links* `october_generator2026.xlsx` in the current directory and
it still returns the HTML page. **A link on the index page is not proof the file exists** — the only
reliable signal is the content type / size of the response itself.

## 4. The second defect: the reader is given a vague reason while the real one is known

The failure IS captured. `sources/mod.rs:211` pushes a `SourceFailure` with the real error, and the
report's source health records `status: unavailable, failed: 1`.

But what reaches the reader is:

> "EIA-860M planned/cancelled inventories unavailable: the workbook could not be retrieved or the
> sheets were not parseable"

That sentence covers two unrelated failures — a network problem and a parse problem — and states
neither. The actual cause (`content-type text/html, 55725 bytes`) exists in the process and is
dropped. So a reader cannot distinguish "EIA is down" (wait) from "our URL is stale" (fix it), which
is exactly the distinction that matters here: the real answer is *fix it*, and it has been sitting
there since 2026-09-25 while the report implied a source outage.

## 5. Not concluded

- Not concluded the model is miscalibrated: 3.28% of weight is missing, and because the composite
  renormalizes over *available* weight, the score is not dragged toward a midpoint — it is computed
  from 177/183 with coverage stating the shortfall. Restoring the source will **change the reading**,
  which is why it is a methodology-visible change rather than an invisible repair.
- Not concluded that scraping the index page is right. It is one option, and the probe above shows
  index links can point at files that do not resolve, so a scraper would need the same content-type
  guard before trusting what it finds.
