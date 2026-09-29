# The Jev reading queue, wired into the daily run (2026-09-29)

Supersedes nothing; this turns the Jev lab's last open item into a scheduled part of the
instrument. Lab notes and the measured behaviour of the model itself live in
`/home/daim/workspace/typesafe-lab/README.md`.

## The problem this solves, and the boundary it stays inside

`bubble-watch circular` finds 67 disclosure edges, but only 5 carry a `verified_passage` —
a human read. An unread edge scores **zero** in the rubric, and the rubric states that bias
toward CALM explicitly. So the binding constraint was never measurement, it was reading: 62
unread edges is a backlog nobody clears.

`scripts/reading_queue.py` produces a **reading order**. It never classifies, and it never
writes anything into the composite. The only thing it can do is tell a human which filing to
open first. That boundary is the whole design, and it is the same one the lab drew.

## What was added

| change | where | why |
| --- | --- | --- |
| `circular --json` | `src/main.rs` | downstream tooling consumes a typed `ScanResult` instead of screen-scraping a table |
| `--end` defaults to **today** | `src/main.rs` | the old hardcoded `2026-09-20` silently froze the scan window (see trap 1) |
| `scripts/reading_queue.py` | new | EDGAR lookup from cache, passage selection, Jev ranking, control check, artifacts |
| `data/doc-cache` ignored | `.gitignore` | fetched filings are ~3 MB each; 50 already = 1.4 GB, never source |

The ranker reuses the tool's own cache by reproducing its filename hash (FNV-1a over the
URL) and its URL encoding byte-for-byte, so in its normal path it makes **no** extra EDGAR
search requests. Measured on the final run: 106 cache hits, **0** live EDGAR fetches, 14s.

## Two silent failures found and fixed while wiring this up

Both were found by running it, not by reading it, and both would have looked like success.

**Trap 1 — a frozen window.** `circular --end` defaulted to a hardcoded `2026-09-20`. Every
future run would have re-read the same window and been structurally incapable of seeing a
disclosure filed after that date, with no error and no change in output shape: the old edges
keep matching, so the queue keeps looking healthy. A pinned window end is now today unless
explicitly overridden.

**Trap 2 — the cache key contains the dates, and the two sides disagreed.** The window dates
are part of the EDGAR query *and* of the cache filename. The tool resolved an empty `--end`
from its clock; the Python side was still passing the empty string, so **every** search
looked like a miss and the run came back with 65 edges and 0 judged — "nothing to rank",
exit 1. It reported the failure honestly rather than inventing a queue, which is the only
reason it was caught. Both sides now resolve the end date identically, in UTC.

The generalisable rule: when a cache key embeds a derived value, every writer and reader of
that key must derive it the same way, or the failure is a silent zero.

## Measured result (2026-09-29, default live mode, `jev-latest`)

```
65/65 edges judged      15/65 in the tier (financing_p >= 0.7)
control separation +0.54 -> PASS        ~64k input tokens, 14s of Jev wall time
```

The reading queue is **15 edges**, of which **2 have never been read by a human**:

| edge | financing_p | latest |
| --- | --- | --- |
| `AMZN x OpenAI` | 0.96 | 8-K 2026-02-27 |
| `IREN x NVIDIA` | 0.76 | 10-K 2025-08-28 |

Control (truth established by a human reading the primary filing): 13 real structures ranked
1–14, and the one hand-confirmed **refutation** (`ORCL x OpenAI`) at 25/65 with 0.24, below
every one of them.

⚠ **`CRWV x OpenAI` is the boundary case.** It is a hand-confirmed real structure and it
scored 0.78–0.80 across runs — i.e. it sits *at* the 0.7 cut and would fall below a 0.8 cut.
This is the concrete reason the tier default is 0.7 and not the 0.8 the lab used: 0.8 is
inside the measured run-to-run noise band and would drop a known-real edge.

⚠ **Do not read the order inside the tier.** Scores moved ±0.02 between identical runs
(`CRWV x OpenAI` 0.80 then 0.78; mean |Δ| ≈ 0.02 in the lab's v1→v2 comparison), which
reshuffles everything from 0.76 to 0.99. The tier is a list; the ranking within it is noise.

⚠ **The fine-grained `kind` label is not published.** It is written to the CSV for diagnosis
only, and deliberately omitted from the HTML. Measured mean confidence on this corpus was
0.71, as low as 0.22. Rank, don't record — and never write any of these probabilities into
the composite.

## Reproducing

```bash
cd /home/daim/workspace/bubble-watch
python3 scripts/reading_queue.py --strict --out site          # live scan, rank, validate
python3 scripts/reading_queue.py --cut 0.8                    # different tier boundary
python3 scripts/reading_queue.py --offline --end 2026-09-29   # cache-only replay
```

Exit codes: `0` ok, `2` no API key, `3` **control failed** (only with `--strict`), `4` bad
usage. The control is not optional: the 13 real and 1 refuted edges are always included
regardless of `--limit`, because a ranking with no known answer in it is an assertion rather
than a measurement.

## Scheduled, and how to confirm it

`~/.config/systemd/user/bubble-watch-reading-queue.{service,timer}` — 14:15 local daily,
after the 13:30 refresh so the two EDGAR-heavy jobs do not overlap. `--strict` makes a failed
control a **failed unit**, not a quiet queue nobody should trust.

Enabling needs a user session bus, which an agent context does not have:

```bash
systemctl --user daemon-reload
systemctl --user enable --now bubble-watch-reading-queue.timer
systemctl --user list-timers bubble-watch-reading-queue.timer
systemctl --user start bubble-watch-reading-queue.service   # one run, to verify
```

## What remains

- **Enable the timer** (above) — the only unfinished step.
- **Re-validate before changing the question text.** The three question strings are
  byte-identical to the validated lab definitions. Editing any of them (or the criteria)
  invalidates the measured separation and requires re-running the control.
- **Re-triage against fresh filings.** This run's window ends today, so tomorrow's run sees
  anything filed since — which is the point of the window fix.
