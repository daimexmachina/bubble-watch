# bubble-watch — operational cost of the new sources

**Date:** 2026-09-21 · Measured after adding the demand-side and LBNL sources.

Recorded because adding a source has a **recurring** cost on a Raspberry Pi, not just a one-off
development cost — and because I did not check this when I added them.

---

## The unattended path still works — measured

Ran `scripts/daily-refresh.sh` live, exactly as systemd invokes it (no `--offline`):

```
exit=0    elapsed=371s    systemd cap: 900s   (41% of budget)
archive: 60 -> 61 rows
coverage: 100%   indicators: 22
inference_demand:      26.84
energisation_delay:    31.01
```

Both new indicators scored from live data on the unattended path, which is the check that matters:
a source that only works interactively would degrade to a silent gap at 13:32 every day.

**Headroom is comfortable but not large.** 371s of 900s. A cold cache, a slow hour, or FRED
misbehaving could push it — and `TimeoutStartSec=900` means a timeout is a *failed unit*, not a
partial run.

## Daily download grew 67%

| | MB/day |
|---|---|
| existing sources (EIA-860M 13.9, Fed Z.1 8.0, EDGAR ~2, Yahoo ~1) | 24.9 |
| **added by this session** (LBNL 15.5, OpenRouter 1.1, Fed Register ~0.01) | **+16.6** |
| **new total** | **41.5** |

The LBNL queue workbook alone is **15.5 MB of the 16.6 MB added** — the single largest source in the
project, larger than the EIA and Z.1 archives it now sits beside.

## The cache does NOT prevent the re-download

Worth stating plainly because it is counter-intuitive: `Fetcher`'s cache is **keyed on URL with no
TTL**, so a live run refetches every source in full. The cache exists so `--offline` can reproduce a
run and so a *repeated* URL within one run is not fetched twice — not to avoid daily re-downloads.

So the 15.5 MB is a **recurring** 15.5 MB/day, not a one-time cost.

Current cache: 848 bodies, 49 MB, and it does not grow per-run (no duplicate bodies — the large files
are distinct URLs). Disk is not a constraint: **298 GB free, 34% used.**

## What this means, and what I did about it

Nothing needs changing right now — 371s of 900s and 41.5 MB/day on a machine with 298 GB free is
fine. But it is worth knowing **before** the next source is added, and there are two cheap levers if
it ever becomes tight:

1. **A staleness gate on the LBNL fetch.** The workbook is a dated release
   (`.../2026-05/...thru2025.xlsx`) whose content changes at most annually. Refetching 15.5 MB daily
   to re-derive the same 21 rows is wasteful; a cache TTL of, say, 7 days on that URL alone would cut
   the daily total from 41.5 MB to ~26 MB with no loss of information, because the underlying file
   cannot have changed.
2. **A cache TTL mechanism generally.** `cache_path` already keys reliably on URL; adding an
   age check would let any slow-moving source opt in.

**Not done now** — the plan did not call for it and the measured headroom does not justify it. Filed
as a known cost with a stated remedy, which is the honest version.

## The lesson

I added a 15.5 MB daily dependency without measuring its recurring cost, on a device the project's
own notes describe as thermal- and resource-sensitive. The measurement took two minutes and should
have preceded the decision. Worth doing deliberately next time a source is considered: state the
payload size *per run* alongside the endpoint's value.
