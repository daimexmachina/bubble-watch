#!/usr/bin/env bash
# Daily refresh: fetch live sources, record the run, regenerate the site.
#
# Run from systemd (bubble-watch-refresh.timer) rather than as an agent cron
# turn: the job is deterministic, so it costs zero model tokens and cannot
# hallucinate. The tool itself refuses to impute a missing source, so a FRED
# outage degrades to a reported gap rather than a wrong number.
#
# Exit codes are the tool's own (2 config, 3 all sources failed, 4 coverage below
# floor, 5 asked to record and did not) plus 5 from this script when the binary
# cannot be made to account for the source tree. They are propagated so systemd
# records a failed unit instead of a silent success.
set -euo pipefail

REPO="${BUBBLE_WATCH_REPO:-/home/daim/workspace/bubble-watch}"
# ^ the tree this script acts on. Overridable ONLY so the build guard BELOW can be
# exercised end-to-end against a COPY of the repo (with its target/ directory) — running
# this script against the live tree appends a row to data/history/runs.jsonl, which is
# why the guard had no end-to-end test while it was hard-coded. The default is the real
# tree, and a `BUBBLE_WATCH_REPO` value that is not a bubble-watch checkout fails the
# `cd` below or the `artifact-check` after it, not silently.
cd "$REPO"

# Live run: this is the point of the daily refresh. --offline is deliberately
# NOT used here.
BIN="$REPO/target/release/bubble-watch"

# REBUILD WHEN THE BINARY CANNOT ACCOUNT FOR THE SOURCE TREE, not only when it is
# missing — the old `[ ! -x "$BIN" ]` test is why this hazard existed.
#
# `scripts/falsify.sh` mutates a source file, runs `cargo test --release` (which builds
# the BINARY target from the mutated source too), then restores and hash-verifies the
# SOURCE. Restoring the source leaves the mutation PRESENT IN THE THING THAT RUNS and
# ABSENT FROM THE THING THAT IS REVIEWED: the tree is clean, `--verify` passes, and the
# binary at this path is a mutant. Measured 2026-10-17: a binary left behind by a
# falsification printed "plus 5 things it cannot measure at all", offline from the same
# cache, where the committed source prints 2. The old check here rebuilt only when the
# binary was MISSING, so the next 13:32 run would have executed that binary and
# `site` would have RECORDED it.
#
# `artifact-check` compares the fingerprint of the source tree baked into the binary at
# build time with the tree on disk now, and exits non-zero when they differ — which is
# also what a source edit that was never built looks like. A missing binary is built, a
# binary that cannot account for the tree is rebuilt, and NOTHING is run until the check
# passes: a row written to `data/history/runs.jsonl` by a binary built from other source
# is a falsified measurement in the archive, and afterwards it is indistinguishable from
# a real one.
if [ ! -x "$BIN" ]; then
  echo "no binary at $BIN — building"
  cargo build --release
elif ! "$BIN" artifact-check >/dev/null 2>&1; then
  echo "binary does not account for the source tree — rebuilding"
  cargo build --release
fi

# ... AND REBUILD AGAIN, HARDER, IF THAT DID NOT REPAIR IT.
#
# `cargo build` decides whether to recompile by MTIME, and the restore classes this guard
# exists for PRESERVE mtimes: `cp -p`, `rsync -a`, `tar -x`, `git archive`+extract, or a
# restored backup bring the source back with its ORIGINAL timestamp, which is OLDER than
# the mutant artifact already in target/. Cargo then reports "Finished in 0.08s" and the
# mutant survives the plain rebuild. MEASURED on this tree 2026-10-17: after a `cp -p`
# restore a plain `cargo build --release` finished in 0.07s, left
# target/release/bubble-watch byte-identical (still built from the mutated source,
# artifact-check MISMATCH, exit 5), and the same content with only a `touch` recompiled
# for 25.9s and read VERIFIED. So "the tree is clean, `--verify` passes and `cargo build`
# finished" is consistent with a mutant binary — and without the fallback below this
# script would have run the no-op build and then REFUSED, exit 5, at every 13:32 run from
# then on, while the archive developed the hole `doctor` exists to report.
#
# `--release` IS NOT OPTIONAL HERE. MEASURED: `cargo clean -p bubble-watch` (no --release)
# cleans the DEV profile and leaves the release artifact exactly where it was — it removed
# 4026 files / 2.0 GiB and the mutant binary did not move, so the rebuild that followed
# was still a no-op. With `--release` the crate's release artifacts go (30 files /
# 36.9 MiB in the measured leak) INCLUDING its build-script output, so build.rs re-runs
# and re-bakes the source fingerprint; the forced rebuild then reads VERIFIED.
if ! "$BIN" artifact-check >/dev/null 2>&1; then
  echo "the rebuild did not repair the artifact — removing this crate's release artifacts and rebuilding" >&2
  if ! cargo clean -p bubble-watch --release || ! cargo build --release; then
    echo "REFUSING TO RUN: this crate could not be rebuilt, so $BIN cannot be trusted." >&2
    exit 5
  fi
fi

if ! "$BIN" artifact-check; then
  echo "REFUSING TO RUN: $BIN is not this tree's build, so a run from it would record a" >&2
  echo "measurement this source never produced. Nothing was recorded." >&2
  exit 5
fi

# Regenerate the site, appending this run to the archive and rebuilding the
# dashboard. Output goes to stdout, captured by journald.
"$BIN" site --out "$REPO/site" --history-dir "$REPO/data/history"
rc=$?

# Also keep a dated copy of the JSON report for offline inspection.
#
# The DATE COMES FROM THE RUN, NOT FROM THE SHELL. Using `date +%F` takes the LOCAL
# date, while the archive records runs in UTC — so after 17:00 Pacific the two
# disagree and a report gets filed under yesterday's name next to an archive entry
# dated tomorrow. Deriving both from the same generated_at keeps them consistent.
RUN_DATE="$("$BIN" score --json 2>/dev/null \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["generated_at"][:10])' 2>/dev/null \
  || true)"
# Fall back to the local date only if the run could not be read, and say so.
if [ -z "$RUN_DATE" ]; then
  RUN_DATE="$(date +%F)"
  echo "WARNING: could not read generated_at from the run; falling back to the local date $RUN_DATE" >&2
fi

"$BIN" report --out "$REPO/out" --name "ai-bubble-watch-$RUN_DATE" \
  --history-dir "$REPO/data/history" --no-record >/dev/null || true

# ---------------------------------------------------------------- gap check ----
#
# A daily series with a hole in it fails SILENTLY: nothing errors, and `trend` spans
# the gap as though the missing days never existed. On 2026-09-24 this host was off
# across the 13:30 timer slot and `Persistent=true` did not fire a catch-up, so that
# day vanished with no trace anywhere. This runs the check every day so a missed run
# announces itself instead of being found by hand weeks later.
#
# It is a WARNING, not a failure: the run above may have succeeded perfectly, and a
# non-zero exit here would mark a healthy unit as failed and hide the real signal.
# Checked over 7 days, which is long enough to catch a one-day gap promptly.
if ! "$BIN" doctor --history-dir "$REPO/data/history" --days 7; then
  echo "WARNING: the archive has a gap. The run above may still have succeeded; see the list." >&2
fi

exit $rc
