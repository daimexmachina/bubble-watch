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

REPO=/home/daim/workspace/bubble-watch
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
