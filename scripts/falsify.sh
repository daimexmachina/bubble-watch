#!/usr/bin/env bash
# Falsify a guard: break it, watch the named test fail, restore, and PROVE the
# restore happened.
#
# WHY THIS EXISTS
# ---------------
# Three times during the v1.4 work a falsification leaked into the working tree and
# the suite did not notice, because the leak made the suite report "1 failed" in a
# run nobody read closely. The mechanism was always the same:
#
#     cp file /tmp/f.bak          # back up
#     ... mutate, test ...        # the falsification
#     cp /tmp/f.bak file; rm -f /tmp/f.bak   # restore, then DELETE the backup
#     cp /tmp/f.bak file 2>/dev/null         # <-- silent no-op: the backup is gone
#
# The second `cp` fails, `2>/dev/null` hides it, and the falsified code stays in the
# tree looking exactly like restored code. A leaked falsification is worse than a
# failing test, because it silently DISABLES the guard it was written to prove.
#
# HOW THIS SCRIPT PREVENTS IT
#   1. The backup is restored by an `trap ... EXIT`, so it runs even if the script
#      is interrupted, and the backup file is never deleted before the restore.
#   2. After restoring, the file's sha256 is compared against the hash taken BEFORE
#      the mutation. A mismatch is a hard error, not a warning.
#   3. `verify` mode re-checks a whole repo against a manifest of hashes, so a leak
#      can be detected after the fact rather than only prevented.
#
# AND THE ARTIFACT, WHICH IS THE OTHER HALF OF THE SAME LEAK
#   Restoring the SOURCE is not enough: `cargo test --release` below also builds the
#   `bubble-watch` BINARY target, so before this step existed every falsification
#   left `target/release/bubble-watch` built from the MUTATED source with a clean
#   tree — the mutation present in the thing that RUNS, absent from the thing that
#   is reviewed. `scripts/daily-refresh.sh` rebuilt only when the binary was
#   MISSING, so the next 13:32 run would have executed that binary and RECORDED its
#   numbers into `data/history/runs.jsonl`. Measured 2026-10-17: a binary left by a
#   falsification printed "plus 5 things it cannot measure at all", offline from the
#   same cache, where the committed source prints 2.
#
#   So after the restore this script REBUILDS the binary and VERIFIES it with
#   `bubble-watch artifact-check`, which exits non-zero unless the binary's baked
#   source fingerprint is the tree beside it. If a plain rebuild does not repair the
#   artifact, it forces one (`cargo clean -p bubble-watch`) rather than trusting
#   cargo to have had work to do; if it still cannot be verified, the run FAILS.
#
# USAGE
#   scripts/falsify.sh <file> <old_substring> <new_substring> <test_filter>
#
# `old_substring`/`new_substring` are passed to python via environment variables and
# applied with str.replace(old, new, 1). The mutation must make the named test FAIL;
# a mutation that does not is reported as "guar[d] could not be falsified", which is
# itself a finding: the test does not exercise the code path it claims to.
#
#   scripts/falsify.sh --manifest <path>   # write hashes of all tracked src files
#   scripts/falsify.sh --verify   <path>   # compare tracked src files to the manifest
#
# WHAT THIS DOES NOT DO
# It cannot tell you whether the GUARD IS CORRECT, only that the TEST CAN FAIL. A
# guard calibrated to the wrong mechanism fails nothing and passes everything.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

# The binary `cargo test --release` builds from the mutated source. Checked and
# rebuilt after every falsification; see `rebuild_artifact`.
BIN="$REPO/target/release/bubble-watch"

die() { printf '\033[31mFAIL\033[0m %s\n' "$*" >&2; exit 1; }
ok()  { printf '\033[32m ok \033[0m %s\n' "$*"; }
info(){ printf '      %s\n' "$*"; }

# `grep -q` closes the pipe at the first match, so the upstream printf dies of
# SIGPIPE (141) and — under `set -o pipefail`, which this script sets — the PIPELINE
# status becomes 141, not 0. The `if` then takes the else branch. Measured on a
# 150 KB blob: a matching secret reported as a pass, 20 times out of 20. Every
# content test here therefore uses a here-string (`<<<`), which has no upstream
# writer to kill, or `grep -c`. This is the same defect recorded in the pre-push
# guard's own history; it is not re-derived here, it is inherited deliberately.
has() { grep -qE "$1" <<< "$2"; }

# GLOBAL, not `local`. An EXIT trap fires after the function's local scope is
# destroyed, so `trap 'cp "$backup" ...' EXIT` with a local backup fails with
# "backup: unbound variable" under `set -u` — the backstop silently does nothing.
# This was a real defect: the harness leaked a falsification on its first use.
BACKUP=""
FILE_UNDER_TEST=""
RESTORED=0

# --------------------------------------------------------------------------
# verify mode: compare the working tree against a manifest of recorded hashes
# --------------------------------------------------------------------------
manifest_write() {
    local out="${1:-.falsify-manifest}"
    : > "$out"
    git ls-files 'src/*.rs' | while read -r f; do
        printf '%s  %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$f" >> "$out"
    done
    info "wrote $(wc -l < "$out") hashes to $out"
}

manifest_verify() {
    local m="${1:-.falsify-manifest}"
    [ -f "$m" ] || die "no manifest at $m"
    local bad=0
    while read -r want f; do
        [ -f "$f" ] || { printf '  MISSING  %s\n' "$f"; bad=$((bad+1)); continue; }
        local got; got="$(sha256sum "$f" | cut -d' ' -f1)"
        if [ "$got" != "$want" ]; then printf '  CHANGED  %s\n' "$f"; bad=$((bad+1)); fi
    done < "$m"
    if [ "$bad" -eq 0 ]; then ok "all tracked src files match the manifest"; else
        die "$bad file(s) differ from the manifest — a falsification may have leaked"
    fi
}

# --------------------------------------------------------------------------
# the artifact: rebuild it after a restore, and PROVE the rebuild happened
# --------------------------------------------------------------------------
#
# THE HALF OF THE LEAK THAT RESTORING THE SOURCE DOES NOT COVER. `cargo test --release`
# builds the BINARY target from the mutated source, so the run above leaves
# `target/release/bubble-watch` a MUTANT with a clean, hash-verified tree. That binary is
# invisible from the outside: same path, same name, and `--verify` passes because the
# SOURCE is fine.
#
# WHY IT IS VERIFIED RATHER THAN ASSUMED. Evidence #2 on the card that filed this defect
# reported `cargo build --release` printing "Finished in 0.07s" and NOT rebuilding the
# mutant. That specific no-op did not reproduce here (a plain `cargo build --release` after
# a falsification took 25.8s and reproduced the pre-experiment binary byte for byte), but
# the fix must not depend on cargo's mood: the rebuild is followed by `artifact-check`,
# and if that still fails the crate's artifacts are removed and it is built again. A
# falsification that cannot leave a trustworthy binary FAILS here instead of finishing
# with an encouraging line.
ARTIFACT_STALE=0

rebuild_artifact() {
    if [ ! -x "$BIN" ]; then
        info "no $BIN to rebuild (nothing was built from the mutation)"
        ARTIFACT_STALE=0
        return 0
    fi
    ARTIFACT_STALE=1
    cargo build --release >/dev/null 2>&1 \
        || die "the binary could not be rebuilt after the restore — it may still be the mutant."
    if ! "$BIN" artifact-check >/dev/null 2>&1; then
        info "the rebuild did not repair the artifact; removing this crate's artifacts and retrying"
        cargo clean -p bubble-watch >/dev/null 2>&1 || true
        cargo build --release >/dev/null 2>&1 \
            || die "the binary could not be rebuilt after 'cargo clean -p bubble-watch'."
    fi
    local verdict
    if ! verdict="$("$BIN" artifact-check 2>&1)"; then
        printf '%s\n' "$verdict" | sed 's/^/      /'
        die "the binary at $BIN STILL cannot account for the restored source, so it may have been
      built from the mutation. Do NOT run anything that records a run, and do not commit."
    fi
    ARTIFACT_STALE=0
    ok "artifact rebuilt and verified against the restored source"
    printf '%s\n' "$verdict" \
        | grep -E '^  (baked fingerprint|tree fingerprint|verdict)' | sed 's/^/      /' || true
}

# --------------------------------------------------------------------------
# mutation mode
# --------------------------------------------------------------------------
restore_now() {
    [ -n "$BACKUP" ] && [ -n "$FILE_UNDER_TEST" ] || return 0
    [ "$RESTORED" -eq 0 ] || return 0
    RESTORED=1
    cp "$BACKUP" "$FILE_UNDER_TEST"
    local a before
    a="$(sha256sum "$FILE_UNDER_TEST" | cut -d' ' -f1)"
    before="$(head -1 "$BACKUP.sha" 2>/dev/null || echo '')"
    if [ -n "$before" ] && [ "$a" != "$before" ]; then
        printf '\n\033[31mFAIL\033[0m emergency restore of %s did not match %s\n' \
               "$FILE_UNDER_TEST" "${before:0:16}" >&2
        exit 1
    fi
    printf '\n      (emergency restore ran for %s; sha %s)\n' "$FILE_UNDER_TEST" "${a:0:16}"
    # The source is back, but the BINARY was built from the mutated source and the run
    # was interrupted before `mutate_test` could rebuild it. Say so and repair it: the
    # interrupt is exactly when a mutant artifact is most likely to be left behind and
    # least likely to be noticed.
    if [ "$ARTIFACT_STALE" -eq 1 ]; then
        printf '\n\033[31mFAIL\033[0m the interrupted run may have left %s built from the MUTATED source\n' "$BIN" >&2
        printf '      rebuilding it now; if this is interrupted too, run: cargo build --release\n' >&2
        rebuild_artifact
    fi
}
trap restore_now EXIT

mutate_test() {
    local file="$1" old="$2" new="$3" filter="$4"
    [ -f "$file" ] || die "no such file: $file"

    FILE_UNDER_TEST="$file"
    local before; before="$(sha256sum "$file" | cut -d' ' -f1)"
    BACKUP="$(mktemp "${TMPDIR:-/tmp}/falsify.XXXXXX")"
    cp "$file" "$BACKUP"
    printf '%s\n' "$before" > "$BACKUP.sha"
    RESTORED=0

    echo "== falsifying: $file"
    info "test filter: $filter"
    info "sha256 before: ${before:0:16}"

    OLD="$old" NEW="$new" FILE="$file" python3 - <<'PY'
import os
p = os.environ["FILE"]
s = open(p).read()
old, new = os.environ["OLD"], os.environ["NEW"]
if old not in s:
    raise SystemExit(f"mutation anchor NOT FOUND in {p} — the code has moved, so this "
                     f"falsification tested nothing:\n  {old[:120]!r}")
n = s.count(old)
open(p, "w").write(s.replace(old, new, 1))
print(f"      anchor found ({n} occurrence(s)); first replaced")
PY

    local mutated; mutated="$(sha256sum "$file" | cut -d' ' -f1)"
    [ "$mutated" != "$before" ] || die "mutation did not change the file"
    info "sha256 mutated: ${mutated:0:16}"

    # Split the filter into words. Passing "--lib some::test" as ONE argv makes cargo
    # treat it as a single name that matches nothing, run ZERO tests, and print
    # "test result: ok. 0 passed" — a check that cannot run, reading as a pass. That
    # is the exact defect this harness exists to catch, and the harness had it.
    # shellcheck disable=SC2206
    local fargs=($filter)

    # From here on a mutation can reach the BINARY target, so an interruption from here
    # to the end of `rebuild_artifact` below leaves a mutant artifact behind.
    ARTIFACT_STALE=1

    local out rc=0
    out="$(cargo test --release "${fargs[@]}" 2>&1)" || rc=$?

    # A filter that selected nothing proves nothing. Refuse to read "0 passed" as ok.
    # Count tests that EXECUTED, i.e. passed + failed + ignored — not just passes. A
    # falsification run legitimately reports "0 passed; 1 failed", and counting only
    # passes would call that "zero tests ran" and refuse a test that did run.
    # NOTE: cargo writes "0 passed;" WITH A SEMICOLON, so the fields are "passed;"
    # not "passed" — an equality test on $i silently matches nothing and the counter
    # reports 0 for every run. Strip the punctuation first.
    local ran; ran="$(awk '/^test result:/ {line=$0; gsub(/[;:,]/," ",line); n=split(line,f," ");
        for(i=1;i<=n;i++) if(f[i]=="passed"||f[i]=="failed"||f[i]=="ignored") s+=f[i-1]} END{print s+0}' <<< "$out")"
    if [ "${ran:-0}" -eq 0 ]; then
        grep -E '^test result' <<< "$out" | head -3 | sed 's/^/      /' || true
        die "the filter '$filter' selected ZERO tests — a check that never ran cannot
      be a passing check. Fix the filter, then re-run."
    fi
    info "tests actually executed: $ran"

    if has '^test result: FAILED' "$out"; then
        ok "guard IS falsifiable — the named test failed when broken"
        printf '%s' "$out" | grep -E 'panicked at' | head -2 | sed 's/^/      /' || true
    else
        printf '\033[33mNOTE\033[0m the test ran (%s test(s)) and still PASSED with the guard broken.\n' "$ran"
        info "That means this test does not exercise the mutated path — the guard's"
        info "real check is elsewhere, or the test is calibrated to the wrong mechanism."
    fi

    # Restore and PROVE it. This is the step whose absence caused the leaks.
    cp "$BACKUP" "$file"
    local after; after="$(sha256sum "$file" | cut -d' ' -f1)"
    if [ "$after" != "$before" ]; then
        die "RESTORE FAILED: ${file} is ${after:0:16}, expected ${before:0:16}. The file is still falsified. Do NOT commit."
    fi
    ok "restore verified by hash (${after:0:16})"
    RESTORED=1
    rm -f "$BACKUP" "$BACKUP.sha"
    BACKUP=""
    FILE_UNDER_TEST=""

    # ... AND PROVE THE ARTIFACT TOO. The source being clean is not the property that
    # matters for the archive: the BINARY is what runs and what gets recorded, and the
    # `cargo test` above built it from the mutated source. This rebuilds it and refuses
    # to finish unless it can account for the restored source.
    rebuild_artifact
    return 0
}

case "${1:-}" in
    --manifest) manifest_write "${2:-.falsify-manifest}" ;;
    --verify)   manifest_verify "${2:-.falsify-manifest}" ;;
    ""|-h|--help) sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' ;;
    *) [ $# -eq 4 ] || die "usage: falsify.sh <file> <old> <new> <test_filter>"
       mutate_test "$@" ;;
esac
