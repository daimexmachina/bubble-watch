#!/usr/bin/env bash
# Back up the bubble-watch run archive.
#
# WHY THIS EXISTS. The archive (data/history/runs.jsonl) is gitignored by design:
# it is generated state that would grow the repository daily, and it holds no code.
# But it is ALSO the only copy of the run-to-run history — the trend, the direction
# of travel, and the methodology-versioned baselines. Nothing else records it. If the
# NVMe fails, that history is gone and cannot be reconstructed, because the past
# cannot be re-fetched: every source reports today's values.
#
# So it is copied to the vault, which is already backed up to a private git repo
# every six hours. Deliberately a copy rather than a move: the tool keeps reading
# data/history in place.
set -euo pipefail

# Paths are derived rather than hardcoded, so this script carries no user-specific
# absolute path and works from any checkout or home directory.
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHIVE="$REPO/data/history/runs.jsonl"
DEST_DIR="${HERMES_HOME:-$HOME/.hermes}/knowledge/obsidian_vault/hermes-memory"
DEST="$DEST_DIR/bubble-watch-archive.jsonl"

if [ ! -f "$ARCHIVE" ]; then
  echo "bubble-watch archive not present at $ARCHIVE — nothing to back up"
  exit 0
fi

mkdir -p "$DEST_DIR"

# Only write when the content actually changed, so the vault repo does not accumulate
# a commit every six hours for an unchanged file.
if [ -f "$DEST" ] && cmp -s "$ARCHIVE" "$DEST"; then
  echo "archive unchanged ($(wc -l < "$ARCHIVE") runs); nothing to do"
  exit 0
fi

cp "$ARCHIVE" "$DEST"
echo "copied $(wc -l < "$ARCHIVE") runs to $DEST"

# A README beside it, so a future reader knows what this file is and that it is a
# copy rather than the source of truth.
cat > "$DEST_DIR/bubble-watch-archive.md" <<'DOC'
# bubble-watch run archive (backup copy)

`bubble-watch-archive.jsonl` is a **backup copy** of the run archive from
`<project>/data/history/runs.jsonl` in the bubble-watch checkout.

**The source of truth is the copy in the project directory.** That one is gitignored
because it is generated state that would grow the repository daily; this copy exists
only because it is otherwise the sole record of the run history.

One JSON object per line, appended once per run:

| field | meaning |
|---|---|
| `date` | calendar date; at most one point per date is used for display |
| `generated_at` | full timestamp of the run |
| `composite` | bubble-stress score for that run |
| `coverage` | weighted fraction of the model that was measurable |
| `phase` | early / mid / late / critical |
| `methodology_version` | `schema_version` the run was computed under |
| `stresses` | per-indicator stress, absent entries being gaps rather than zeros |

**Runs are only comparable within a methodology version.** Redefining an indicator
changes what the composite means, so the tool refuses to compute a direction of
travel across a version boundary rather than reporting an accounting change as a
market move.

Restore with: `cp bubble-watch-archive.jsonl <project>/data/history/runs.jsonl`
DOC
echo "wrote $DEST_DIR/bubble-watch-archive.md"
