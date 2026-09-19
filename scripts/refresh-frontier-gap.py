#!/usr/bin/env python3
"""Refresh the LMArena frontier-gap fixture.

WHY THIS EXISTS AS A COMMITTED SCRIPT. `frontier_premium` reads
`tests/fixtures/arena_frontier_gap.json`, which is extracted from LMArena's parquet
dataset rather than fetched at run time (the full file is 56 MB and needs a parquet
reader the crate deliberately does not depend on). That makes the fixture a committed
artefact — and a committed artefact does not update itself. The indicator carries a
120-day staleness guard which turns it into a REPORTED GAP rather than silently
reporting a frozen value, so past that limit the indicator disappears from the
composite until someone re-runs this.

The extraction originally lived in a scratch file, which meant the procedure would
have been lost. A fixture the project depends on but cannot reproduce is a liability,
so the method is versioned here alongside the data it produces.

REQUIRES: pyarrow. Install it in a throwaway venv rather than the project:

    python3 -m venv /tmp/arena-venv
    /tmp/arena-venv/bin/pip install pyarrow
    /tmp/arena-venv/bin/python scripts/refresh-frontier-gap.py

THEN: commit the updated fixture. The integration test
`a_fixture_backed_indicator_refuses_to_report_a_stale_value` asserts the shipped fixture
is not already stale, so a refresh that is forgotten will fail loudly rather than
quietly dropping an indicator.

WHAT IT DOES. Downloads LMArena's leaderboard dataset, keeps `category == 'overall'`,
and for each leaderboard publication date computes the best PROPRIETARY rating minus the
best NON-PROPRIETARY one. That difference is the frontier premium — the moat.

The licence field is the leaderboard's own, which is the best available; a "Proprietary"
label is not literally "weights unreleased", and the indicator's output says so.
"""

import json
import os
import sys
import urllib.request

BASE = "https://huggingface.co/datasets/lmarena-ai/leaderboard-dataset/resolve/main/text/full-00000-of-00001.parquet"
UA = "bubble-watch/0.1 (research tool; contact via repository)"

# Resolve the repo root from this file's location, so the script works from anywhere.
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO, "tests", "fixtures", "arena_frontier_gap.json")


def main() -> int:
    try:
        import pyarrow.parquet as pq
    except ImportError:
        sys.stderr.write(
            "pyarrow is required. Install it in a throwaway venv:\n"
            "  python3 -m venv /tmp/arena-venv\n"
            "  /tmp/arena-venv/bin/pip install pyarrow\n"
            "  /tmp/arena-venv/bin/python scripts/refresh-frontier-gap.py\n"
        )
        return 2

    # Download to a temp path; the file is 56MB so this is not done at run time.
    tmp = "/tmp/arena_full.parquet"
    if not os.path.exists(tmp):
        print(f"downloading {BASE}")
        req = urllib.request.Request(BASE, headers={"User-Agent": UA})
        with urllib.request.urlopen(req, timeout=300) as r, open(tmp, "wb") as f:
            f.write(r.read())
    else:
        print(f"using cached {tmp}")

    rows = pq.read_table(tmp).to_pylist()
    overall = [r for r in rows if r.get("category") == "overall"]
    if not overall:
        sys.stderr.write("no 'overall' rows found — the dataset schema may have changed\n")
        return 3

    by_date = {}
    for r in overall:
        by_date.setdefault(r["leaderboard_publish_date"], []).append(r)

    series = []
    for date in sorted(by_date):
        day = by_date[date]
        prop = [r for r in day if r["license"] == "Proprietary"]
        open_ = [r for r in day if r["license"] != "Proprietary"]
        if not prop or not open_:
            # A day with only one side cannot produce a difference; skipped rather than
            # treated as a zero gap, which would look like parity.
            continue
        bp = max(prop, key=lambda r: r["rating"])
        bo = max(open_, key=lambda r: r["rating"])
        series.append({
            "date": date,
            "best_proprietary": round(bp["rating"], 1),
            "best_open": round(bo["rating"], 1),
            "gap": round(bp["rating"] - bo["rating"], 1),
            "best_proprietary_model": bp["model_name"],
            "best_open_model": bo["model_name"],
            "n_proprietary": len(prop),
            "n_open": len(open_),
        })

    if not series:
        sys.stderr.write("extraction produced no series\n")
        return 4

    gaps = [s["gap"] for s in series]
    payload = {
        "source": "LMArena leaderboard dataset "
                  "(huggingface.co/datasets/lmarena-ai/leaderboard-dataset), text/full parquet, "
                  "category='overall'",
        "note": "Frontier-vs-open capability gap: best proprietary model rating minus best "
                "non-proprietary, per leaderboard publication date. Extracted once because the "
                "source is parquet; the RESULT is committed so the test suite needs no parquet "
                "reader. Regenerate with scripts/refresh-frontier-gap.py.",
        "interpretation": "The frontier premium compressed from ~135 Elo (2024-02) to ~32 "
                          "(2026-09), and briefly went NEGATIVE in early 2025.",
        "series": series,
    }

    # Preserve the original retrieval date semantics: this run IS the retrieval.
    import datetime
    payload["retrieved"] = datetime.date.today().isoformat()

    # Write keys in a stable order so the diff is readable.
    ordered = {
        "source": payload["source"],
        "retrieved": payload["retrieved"],
        "note": payload["note"],
        "interpretation": payload["interpretation"],
        "series": payload["series"],
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        json.dump(ordered, f, indent=1)
        f.write("\n")

    print(f"wrote {OUT}")
    print(f"  {len(series)} snapshots, {series[0]['date']} .. {series[-1]['date']}")
    print(f"  latest gap {series[-1]['gap']:+.1f} "
          f"({series[-1]['best_proprietary_model']} vs {series[-1]['best_open_model']})")
    print(f"  gap range {min(gaps):+.1f} .. {max(gaps):+.1f}")
    print("\nNow commit the updated fixture. The staleness test will fail if it is left "
          "uncommitted past its limit.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
