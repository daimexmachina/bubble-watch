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
"$BIN" report --out "$REPO/out" --name "ai-bubble-watch-$(date +%F)" \
  --history-dir "$REPO/data/history" --no-record >/dev/null || true

exit $rc
