#!/usr/bin/env bash
# Daily refresh: fetch live sources, record the run, regenerate the site.
#
# Run from systemd (bubble-watch-refresh.timer) rather than as an agent cron
# turn: the job is deterministic, so it costs zero model tokens and cannot
# hallucinate. The tool itself refuses to impute a missing source, so a FRED
# outage degrades to a reported gap rather than a wrong number.
#
# Exit codes are the tool's own (2 config, 3 all sources failed, 4 coverage
# below floor). They are propagated so systemd records a failed unit instead of
# a silent success.
set -euo pipefail

REPO=/home/daim/workspace/bubble-watch
cd "$REPO"

# Live run: this is the point of the daily refresh. --offline is deliberately
# NOT used here.
BIN="$REPO/target/release/bubble-watch"

# Rebuild only if the binary is missing (keeps the timer fast and predictable).
if [ ! -x "$BIN" ]; then
  cargo build --release >/dev/null 2>&1
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
