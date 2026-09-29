#!/usr/bin/env python3
"""reading_queue.py -- rank bubble-watch's unread circular-financing edges by whether
they deserve a human read, using Jev (TypeSafe System One) as the judgment layer.

WHY THIS EXISTS. `bubble-watch circular` finds the disclosure edges, but only a handful
carry a `verified_passage` — a human read. An unread edge scores ZERO in the rubric, and
the rubric states that bias toward CALM explicitly. So the binding constraint on this
instrument is not measurement, it is READING: 67 edges is a backlog nobody clears.

This produces a READING ORDER, not a result.

    It never classifies anything. It never writes a finding into the composite.
    The only thing it can do is tell a human which filing to open first.

That boundary is the whole design. The lab measured what Jev is and is not good at on
this exact corpus (/home/daim/workspace/typesafe-lab/README.md):

  * separation is the real result: real structures +0.58..+0.68 above refutations,
    0/8 false-keeps in every run. Use it as a RANKING signal.
  * the fine-grained `kind` label is NOT usable (mean confidence 0.71, as low as 0.22).
    It is printed for context and must never be recorded as a finding. See "RANK, DON'T
    RECORD" below.
  * run-to-run variance is +-1 edge on n=23, so never set a threshold exactly on an
    observed value. Read the TIER, not the rank order: nine edges sit between 0.90 and
    0.99, where +-0.02 of noise reshuffles them.

BUILT-IN CONTROL. Every run is validated against 14 edges whose truth a human established
by reading the primary filings: 13 real structures and 1 hand-confirmed REFUTATION
(ORCL x OpenAI, where the name appears only in a model-integration list). A ranking that
floats the refutation into the reading queue is worse than no ranking — it reproduces the
exact false positive this project says keyword counting cannot avoid. If the control
fails, the script says so loudly and exits non-zero (see --strict).

COST. ~990 input / ~160 output tokens per edge, ~2.3 s of Jev time. A full 65-edge sweep
is ~75k in-tokens, i.e. trivially affordable; the binding constraint stays the human read.

Usage
  python3 scripts/reading_queue.py                    # rank all candidate edges
  python3 scripts/reading_queue.py --limit 12         # top 12 by hit count (+ controls)
  python3 scripts/reading_queue.py --cut 0.7          # different tier boundary
  python3 scripts/reading_queue.py --edgar            # allow live EDGAR fetches on a miss
  python3 scripts/reading_queue.py --out site/        # where the queue artifacts land
  python3 scripts/reading_queue.py --strict           # exit 3 if the control fails

Exit codes: 0 ok, 2 no API key, 3 control failed (only with --strict), 4 bad usage.
"""
from __future__ import annotations

import argparse
import csv
import gzip
import html
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)

# ------------------------------------------------------------------ Jev client ----
# Self-contained on purpose: this runs from systemd on a timer, and a cross-repo import
# into the scratch lab would be a fragile dependency for a daily job. The reference
# client and the full measured notes live in /home/daim/workspace/typesafe-lab/.
ENDPOINT = "https://api.typesafe.ai/v1/systemone"
MODEL = "jev-latest"
ENV_FILE = os.path.expanduser("~/.hermes/.env")
TIMEOUT = 60


def load_key() -> str | None:
    """$TYPESAFE_API_KEY, else the TYPESAFE_API_KEY= line in ~/.hermes/.env."""
    key = os.environ.get("TYPESAFE_API_KEY", "").strip()
    if key:
        return key
    try:
        with open(ENV_FILE, encoding="utf-8") as fh:
            for line in fh:
                if line.startswith("TYPESAFE_API_KEY="):
                    return line.split("=", 1)[1].strip()
    except OSError:
        return None
    return None


def jev_call(state: dict, questions: dict, key: str) -> dict:
    """POST one state + question set, retrying TRANSIENT failures.

    This runs unattended on a nightly timer. A single 520 or a dropped connection would
    otherwise silently drop an edge -- or worse, drop a control -- from the queue. Only
    429/5xx and transport errors are retried; a 4xx is our own malformed request and
    retrying it would just burn the run, so it propagates immediately.
    """
    body = {"state": state, "model": MODEL, "questions": questions}
    last: Exception | None = None
    for attempt in range(3):
        req = urllib.request.Request(
            ENDPOINT,
            data=json.dumps(body).encode("utf-8"),
            headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            if exc.code < 500 and exc.code != 429:
                raise
            last = exc
        except Exception as exc:  # noqa: BLE001 - transport
            last = exc
        if attempt < 2:
            time.sleep(2 ** attempt)
    raise last  # type: ignore[misc]


# ------------------------------------------------------------------- questions ----
# Kept byte-identical to the validated lab definitions. The criteria strings come from
# the project's own rubric enum doc comments, not from a paraphrase, so the judgment
# text and the thing being measured stay aligned. Changing any string here invalidates
# the measured control separation and requires re-validation.
QUESTIONS = {
    "looks_like_financing": {
        "type": "noul",
        "instructions": (
            "Does this passage describe a FINANCIAL entanglement between the filer and "
            "the named counterparty - an investment, equity stake, warrant, loan, "
            "guarantee, revenue concentration, related-party transaction, or the filer "
            "funding/financing the counterparty - as opposed to a product mention, "
            "competitor comparison, marketing or model-integration list, or unrelated "
            "reference?"
        ),
    },
    "mention_kind": {
        "type": "choice",
        "instructions": "What is the nature of the named counterparty in this passage?",
        "criteria": {
            "financing_structure": "The filer invests in, funds, guarantees, lends to, or holds equity in the counterparty",
            "customer_concentration": "The counterparty is disclosed as a major customer or revenue source",
            "supply_agreement": "A supply, purchase or services agreement with stated commitments",
            "related_party_transaction": "A related-party transaction with a named magnitude",
            "absorbed_debt_or_merger": "The filer carries the counterparty's debt, or has merged with it",
            "product_or_model_mention": "A product, model or platform is named, not a business relationship",
            "competitor_or_marketing": "Competitive comparison, marketing, or a list of vendors",
            "self_reference": "The filer naming itself",
            "other_unrelated": "None of the above / incidental mention",
        },
    },
    "worth_human_read": {
        "type": "noul",
        "instructions": (
            "Would a financial analyst tracking circular-financing risk in AI capital "
            "expenditure want to read this disclosure in full?"
        ),
    },
}

# The rubric's own exclusions: a company naming ITSELF is not an edge.
SELF_MATCHES = {("CRWV", "CoreWeave"), ("NVDA", "NVIDIA")}

# ------------------------------------------------------- known-answer controls ----
# Every pair below was established by a HUMAN READING THE PRIMARY FILING -- not by the
# model, and not by the rubric's keyword scan. They are the only ground truth this queue
# has, so they are both the accuracy control and the "already read" set.
#
# Two provenance tiers, kept explicit because they mean different things:
#
#   TOOL-READ (5). The rubric carries a `verified_passage` for these; the scan reports
#   them, so they are read from the JSON at runtime and a newly verified edge leaves the
#   read set automatically.
#     MSFT x OpenAI  ASC 850 revenue disclosure
#     AMD x OpenAI   warrant / 8-K
#     SPCX x Tesla   related-party purchase
#     SPCX x xAI     merger with absorbed debt
#     ORCL x OpenAI  REFUTATION - the name appears only in a model-integration list
#
#   LAB-ADJUDICATED (9). Read by hand in the Jev lab on 2026-09-29 but NOT recorded in
#   the tool. If they were not named here they would be re-queued as "never read" forever.
KNOWN_READ_TOOL = {
    ("MSFT", "OpenAI"),
    ("AMD", "OpenAI"),
    ("SPCX", "Tesla"),
    ("SPCX", "xAI"),
}
KNOWN_REFUTED = {
    ("ORCL", "OpenAI"),
}
KNOWN_TRUE_LAB = {
    ("INTC", "NVIDIA"),     # 215M shares / $5.0B to NVIDIA, Sep 2025
    ("SMCI", "Crusoe"),     # credit-agreement schedule: holds 0.24% of Crusoe
    ("CRM", "Anthropic"),   # XBRL axis member + privately-held equity securities note
    ("APLD", "CoreWeave"),  # 15-yr lease, ~$5.2B contracted revenue
    ("AMZN", "Anthropic"),  # nonvoting preferred stock
    ("CRWV", "OpenAI"),     # Mar 2025 MSA inside a credit-agreement definition
    ("NVDA", "OpenAI"),     # "finalizing an investment and partnership agreement"
    ("CRWV", "NVIDIA"),     # Jan 2026: NVIDIA invested $2B in Class A common
    ("APLD", "NVIDIA"),     # NVIDIA was a PIPE purchaser in the Sep 2024 placement
}
KNOWN_TRUE = KNOWN_READ_TOOL | KNOWN_TRUE_LAB
KNOWN_ANSWERED = KNOWN_TRUE | KNOWN_REFUTED


# --------------------------------------------------------------- EDGAR plumbing ----
UA = "bubble-watch/0.1 (research; contact: research@example.invalid)"
PAUSE = 0.25
WINDOW = 700
MAX_DOC = 4_000_000


def fnv_cache_path(cache_dir: str, url: str) -> str:
    """Reproduce bubble-watch's cache filename exactly (src/http.rs::cache_path).

    FNV-1a over the URL, 64-bit. This is the seam that makes the bridge cheap: a URL
    built the same way as the scan's is a cache HIT, so no EDGAR request is made. If
    this ever drifts from the Rust implementation the symptom is a silent cache MISS,
    i.e. slower and noisier, never wrong.
    """
    h = 0xCBF29CE484222325
    for b in url.encode("utf-8"):
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return os.path.join(cache_dir, f"{h:016x}.body")


def bw_quote(s: str) -> str:
    """Reproduce bubble-watch's urlencode (src/sources/circularity.rs::urlencode)."""
    out = []
    for b in s.encode("utf-8"):
        c = chr(b)
        if (c.isalnum() and b < 128) or c in "-_.~":
            out.append(c)
        else:
            out.append(f"%{b:02X}")
    return "".join(out)


def search_url(cik: str, name: str, form: str, start: str, end: str) -> str:
    return (
        f"https://efts.sec.gov/LATEST/search-index?q={bw_quote(chr(34) + name + chr(34))}"
        f"&forms={form}&dateRange=custom&startdt={start}&enddt={end}&ciks={cik}"
    )


def fetch_live(url: str) -> str | None:
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept-Encoding": "gzip"})
    try:
        with urllib.request.urlopen(req, timeout=40) as r:
            raw = r.read(MAX_DOC)
            if r.headers.get("Content-Encoding") == "gzip":
                raw = gzip.decompress(raw)
            return raw.decode("utf-8", "replace")
    except Exception as e:  # noqa: BLE001 - reported, never silently zero
        print(f"    fetch failed: {type(e).__name__} {e}", file=sys.stderr)
        return None


def locate_filing(cik: str, name: str, start: str, end: str, cache_dir: str,
                  allow_edgar: bool) -> tuple[dict | None, tuple[int, int]]:
    """Find the filing behind this edge, preferring the scan's own cached responses.

    Returns (filing, (cache_hits, live_fetches)). A cache HIT costs nothing and adds no
    rate-limit exposure; only --edgar may touch the network, and only on a miss.
    """
    hits = live = 0
    for form in ("10-K", "8-K", "10-Q"):
        url = search_url(cik, name, form, start, end)
        path = fnv_cache_path(cache_dir, url)
        body = None
        if os.path.exists(path):
            raw = open(path, "rb").read()
            try:
                body = gzip.decompress(raw).decode("utf-8", "replace")
            except gzip.BadGzipFile:
                # Stored uncompressed. Catch BadGzipFile SPECIFICALLY: a blanket except here
                # would silently substitute undecoded binary for a corrupt-but-real gzip
                # stream, which surfaces later as an unparseable search response -- i.e. the
                # edge quietly drops out of the queue instead of reporting a problem.
                body = raw.decode("utf-8", "replace")
            except Exception as e:  # noqa: BLE001 - reported, never a silent zero
                print(f"    cache read failed {path}: {type(e).__name__}: {e}", file=sys.stderr)
                body = None
            hits += 1
        elif allow_edgar:
            body = fetch_live(url)
            time.sleep(PAUSE)
            live += 1
        if not body:
            continue
        try:
            d = json.loads(body)
        except json.JSONDecodeError:
            continue
        docs = d.get("hits", {}).get("hits") or []
        if not docs:
            continue
        h = docs[0]
        src = h.get("_source", {})
        fid = h.get("_id") or h.get("id") or ""
        if ":" not in fid:
            continue
        adsh, doc = fid.split(":", 1)
        return (
            {
                "form": form,
                "date": src.get("file_date"),
                "url": f"https://www.sec.gov/Archives/edgar/data/{int(cik)}/"
                       f"{adsh.replace('-', '')}/{doc}",
            },
            (hits, live),
        )
    return None, (hits, live)


def fetch_document(url: str, search_cache_dir: str, doc_cache_dir: str,
                   allow_edgar: bool) -> tuple[str | None, int, int]:
    """Primary document text.

    The tool's own cache only ever holds SEARCH responses -- `circularity` calls `get()`
    on search-index URLs and never fetches the Archives document. So documents are cached
    separately here (plain UTF-8, one file per URL) and are read back on later runs. That
    is not just speed: it means a nightly queue re-reads filings it already has rather
    than asking sec.gov for ~65 multi-megabyte documents every night.
    """
    mine = fnv_cache_path(doc_cache_dir, url).replace(".body", ".txt")
    if os.path.exists(mine):
        return open(mine, encoding="utf-8", errors="replace").read(), 0, 0
    theirs = fnv_cache_path(search_cache_dir, url)
    if os.path.exists(theirs):
        try:
            return gzip.decompress(open(theirs, "rb").read()).decode("utf-8", "replace"), 1, 0
        except Exception:  # noqa: BLE001
            return open(theirs, "rb").read().decode("utf-8", "replace"), 1, 0
    if not allow_edgar:
        return None, 0, 0
    body = fetch_live(url)
    time.sleep(PAUSE)
    if body:
        os.makedirs(doc_cache_dir, exist_ok=True)
        try:
            with open(mine, "w", encoding="utf-8") as fh:
                fh.write(body)
        except OSError as e:  # a cache write failure must not lose the fetched body
            print(f"    doc-cache write failed: {e}", file=sys.stderr)
    return body, 0, 1


def strip_html(s: str) -> str:
    s = re.sub(r"<(script|style)[^>]*>.*?</\1>", " ", s, flags=re.S | re.I)
    s = re.sub(r"<[^>]+>", " ", s)
    s = html.unescape(s)
    s = s.replace("\u00a0", " ")
    return re.sub(r"\s+", " ", s).strip()


def passage_around(text: str, name: str, top: int = 3) -> str:
    """The primary-source excerpt a human would read.

    This is the FIXED selector from the lab. The first version scored windows by a bag
    of finance-ish keywords and on CRWV x NVIDIA handed the human a revenue-by-geography
    table because an unrelated 'NVIDIA' mention sat near it -- i.e. it reproduced, inside
    our own selector, the keyword-counting failure the project criticises elsewhere.
    Proximity to a FINANCIAL VERB (and to the disclosure's own heading) now outranks
    generic finance words, and the top non-overlapping windows are returned JOINED, so
    the human gets the evidence rather than one lucky crop.
    """
    hits = [m.start() for m in re.finditer(re.escape(name), text, re.I)]
    if not hits:
        return ""
    strong = (
        "acquired", "purchase price", "private placement", "entered into an agreement",
        "invested", "investment and partnership", "warrant", "convertible note",
        "preferred stock", "shares of our common stock", "equity method",
        "related party", "revenue from", "lease", "commitment",
    )
    weak = (
        "invest", "equity", "loan", "guarantee", "revenue", "stake", "agreement", "note",
        "securities", "fair value", "balance sheet",
    )
    scored = []
    for h in hits[:60]:
        seg = text[max(0, h - WINDOW): h + WINDOW]
        low = seg.lower()
        s = sum(3 for k in strong if k in low) + sum(1 for k in weak if k in low)
        scored.append((s, -h, h))
    scored.sort(reverse=True)
    chosen, seen = [], []
    for _s, _neg, h in scored:
        lo, hi = max(0, h - WINDOW), h + WINDOW
        if any(lo < b and hi > a for a, b in seen):
            continue
        seen.append((lo, hi))
        chosen.append(text[lo:hi].strip())
        if len(chosen) >= top:
            break
    return "\n---\n".join(chosen) if len(chosen) > 1 else (chosen[0] if chosen else "")


# ----------------------------------------------------------------------- edges ----
def load_edges(edges_file: str | None, cache_dir: str, end: str, offline: bool) -> tuple[list[dict], dict]:
    """Edges plus scan metadata (silent counterparties and errors are NOT dropped).

    The scan runs LIVE by default, because the queue's job is to reflect filings as they
    are published. Window end is left to the tool's clock unless `end` pins it. Note that
    cache keys embed the window dates, so an `--offline` replay only hits the cache when
    `--end` matches whatever window was cached; a live scan reports the misses it makes
    rather than presenting an empty result as a clean one.
    """
    if edges_file:
        raw = json.load(open(edges_file, encoding="utf-8"))
        edges = raw["edges"] if isinstance(raw, dict) else raw
        meta = raw if isinstance(raw, dict) else {}
        return edges, meta
    cmd = [os.path.join(REPO, "target", "release", "bubble-watch"),
           "circular", "--json", "--cache-dir", cache_dir]
    if offline:
        cmd.append("--offline")
    if end:
        cmd += ["--end", end]
    out = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO, timeout=1800)
    if out.returncode != 0:
        sys.exit(f"bubble-watch circular --json failed ({out.returncode}): {out.stderr[:400]}")
    res = json.loads(out.stdout)
    return res["edges"], res


def load_ciks() -> dict:
    src = open(os.path.join(REPO, "src", "sources", "circularity.rs"), encoding="utf-8").read()
    blk = src[src.index("const FILERS"):]
    return dict(re.findall(r'\(\s*"([A-Z]+)",\s*"(\d+)"', blk, re.S))


# ----------------------------------------------------------------------- render ----
def render_html(rows: list[dict], meta: dict, cut: float, generated: str,
                control_ok: bool = True, never_read: set | None = None) -> str:
    """A reading list, deliberately not a report: no verdicts, no composite input."""
    never_read = never_read or set()

    def esc(s):
        return html.escape(str(s if s is not None else ""))

    body = []
    for i, r in enumerate(rows, 1):
        # The unread edges are the actual deliverable: everything else in the tier has
        # already been adjudicated by a human, so the badge is the difference between
        # "here is the reading order" and "here is what is still outstanding".
        fresh = (" <span class=fresh>never read</span>"
                 if (r["filer"], r["counterparty"]) in never_read else "")
        body.append(
            f"<tr><td class=r>{i}</td>"
            f"<td><b>{esc(r['filer'])}</b> &times; {esc(r['counterparty'])}{fresh}</td>"
            f"<td class=n>{r['financing_p']:.2f}</td><td class=n>{r['worth_read_p']:.2f}</td>"
            f"<td>{esc(r.get('form') or '')} {esc(r.get('filing_date') or '')}</td>"
            f"<td><a href=\"{esc(r.get('url') or '#')}\">filing</a></td></tr>"
        )
    silent = "".join(f"<li>{esc(s)}</li>" for s in (meta.get("silent") or []))
    errors = "".join(f"<li>{esc(e)}</li>" for e in (meta.get("errors") or []))
    # A queue built on a failed control is not a weaker queue, it is an unvalidated one.
    # It is rendered with the failure on top rather than withheld, because withholding it
    # would hide the fact that the run happened and failed.
    fail_banner = "" if control_ok else (
        '<div class=fail><b>The control did NOT hold on this run.</b> An edge a human '
        'confirmed as a REFUTATION ranked above one they confirmed as a real structure. '
        '<b>Do not use this list.</b> Re-run before reading anything here.</div>'
    )
    return f"""<!doctype html><html lang=en><meta charset=utf-8>
<title>bubble-watch reading queue</title>
<style>
 body{{font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;
   max-width:60rem;margin:2rem auto;padding:0 1rem;color:#1a1a1a}}
 h1{{font-size:1.4rem;margin-bottom:.2rem}} .sub{{color:#666;margin-top:0}}
 .warn{{background:#fff8e1;border-left:4px solid #f0a500;padding:.7rem 1rem;margin:1rem 0}}
 .fail{{background:#fdecea;border-left:4px solid #c62828;padding:.9rem 1rem;margin:1rem 0;
   color:#7f1d1d;font-size:1.05rem}}
 table{{border-collapse:collapse;width:100%;margin-top:1rem}}
 th,td{{text-align:left;padding:.4rem .6rem;border-bottom:1px solid #e5e5e5}}
 td.r{{color:#999;width:2.5rem}} td.n{{text-align:right;font-variant-numeric:tabular-nums}}
 th{{background:#fafafa;font-size:.85rem;text-transform:uppercase;letter-spacing:.03em;color:#555}}
 a{{color:#0b6bcb}}
 .fresh{{background:#e8f0fe;color:#1a56db;border-radius:3px;padding:.05rem .35rem;
   font-size:.72rem;text-transform:uppercase;letter-spacing:.04em;vertical-align:middle}}
</style>
<h1>Reading queue &mdash; circular-financing disclosure edges</h1>
<p class=sub>Generated {esc(generated)}. Ranked by Jev <code>financing_p</code>
(tier cut {cut}). <b>This is a reading order, not a finding.</b></p>
{fail_banner}
<div class=warn><b>How to read this.</b> An unread edge scores zero in the rubric, which
biases the composite toward CALM. Nothing here is evidence of anything until a human opens
the filing. Jev's fine-grained <i>kind</i> label is deliberately not shown: measured mean
confidence was 0.71, as low as 0.22, so it is not publishable. Treat the tier as a list and
the order within it as noise (&plusmn;0.02 run-to-run reshuffles the 0.90&ndash;0.99 band).
Never write these probabilities into the composite.</div>
<table><thead><tr><th>#</th><th>Edge</th><th>financing_p</th><th>worth_read</th>
<th>Latest</th><th></th></tr></thead><tbody>{''.join(body)}</tbody></table>
{'<h2>Counterparties named nowhere (coverage limit, not safety)</h2><ul>' + silent + '</ul>' if silent else ''}
{'<h2>Failed queries (reported, not treated as zeros)</h2><ul>' + errors + '</ul>' if errors else ''}
</html>"""


# ------------------------------------------------------------------------- main ----
def main() -> None:
    ap = argparse.ArgumentParser(
        description="Rank bubble-watch's unread circular-financing edges as a reading "
                    "queue, using Jev. Produces a reading order, never a finding.")
    ap.add_argument("--edges", default=None, help="ScanResult JSON; default: run circular --offline --json")
    ap.add_argument("--limit", type=int, default=0, help="top N edges by hit count (0 = all)")
    ap.add_argument("--cut", type=float, default=0.7,
                    help="financing_p tier boundary (default 0.7; see note in --help body)")
    ap.add_argument("--start", default="2024-01-01")
    # Deliberately NOT defaulted to a date here. An empty value is passed through so the
    # tool resolves the window end from its own clock. A pinned default on this side would
    # freeze the window invisibly: new disclosures would simply never be scanned, with no
    # error anywhere and no change in output shape.
    ap.add_argument("--end", default="",
                    help="window end (default: let bubble-watch use today)")
    ap.add_argument("--cache-dir", default=os.path.join(REPO, "data", "cache"))
    ap.add_argument("--doc-cache-dir", default=os.path.join(REPO, "data", "doc-cache"),
                    help="cache for fetched Archives documents (the tool's own cache holds "
                         "search responses only)")
    ap.add_argument("--out", default=os.path.join(REPO, "site"), help="dir for queue artifacts")
    ap.add_argument("--csv", default=None)
    ap.add_argument("--json-out", default=None)
    ap.add_argument("--edgar", action="store_true",
                    help="allow live EDGAR document fetches even in --offline scan mode")
    ap.add_argument("--offline", action="store_true",
                    help="scan from cache only (reproducible replay; needs a matching --end)")
    ap.add_argument("--strict", action="store_true", help="exit 3 if the control check fails")
    args = ap.parse_args()

    key = load_key()
    if not key:
        sys.exit(2)

    # The tool resolves an empty --end from its own UTC clock. Resolve it here the SAME
    # way, because the window dates are part of the cache key and of the EDGAR query: if
    # the two sides disagreed by even one day, every search would look like a cache miss
    # and the queue would come back EMPTY with no error anywhere -- a live scan that
    # silently judges nothing. UTC, to match chrono::Utc::now() exactly.
    end_date = args.end or time.strftime("%Y-%m-%d", time.gmtime())
    # A live scan may also fetch documents live: a newly published filing cannot be in any
    # cache yet. In --offline replay mode nothing touches the network, by definition.
    live = args.edgar or not args.offline

    cache_dir = args.cache_dir
    edges, meta = load_edges(args.edges, cache_dir, end_date, args.offline)
    ciks = load_ciks()

    # ALREADY-READ, DERIVED FROM THE TOOL. The scan reports a `verified_passage` on the
    # edges a human actually read, so that set is read from the data rather than asserted.
    # A newly verified edge therefore leaves the "never read" count on its own, and this
    # list cannot drift away from the rubric the way a hardcoded copy would.
    tool_read = {(e["filer"], e["counterparty"]) for e in edges if e.get("verified_passage")}
    already_read = tool_read | KNOWN_READ_TOOL | KNOWN_TRUE_LAB
    print(f"already read: {len(already_read)} edges "
          f"({len(tool_read)} carry a verified_passage in the scan, "
          f"{len(KNOWN_READ_TOOL | KNOWN_TRUE_LAB)} named here)")

    cand = [e for e in edges if (e["filer"], e["counterparty"]) not in SELF_MATCHES]
    cand.sort(key=lambda e: -e["hits"])
    sample = list(cand) if args.limit <= 0 else cand[: args.limit]

    # Controls are always in the run: a ranking with no known answer in it proves nothing.
    have = {(e["filer"], e["counterparty"]) for e in sample}
    for e in cand:
        if (e["filer"], e["counterparty"]) in KNOWN_ANSWERED \
                and (e["filer"], e["counterparty"]) not in have:
            sample.append(e)

    print(f"{len(edges)} edges; {len(SELF_MATCHES)} self-matches excluded -> {len(cand)} candidates; "
          f"judging {len(sample)} ({len(KNOWN_TRUE)} real + {len(KNOWN_REFUTED)} refuted "
          f"controls included regardless of --limit)")
    print(f"model={MODEL}  window_end={end_date}  cache={cache_dir}  live_edgar={'allowed' if live else 'off'}\n")

    rows, chits, clive = [], 0, 0
    for i, e in enumerate(sample, 1):
        filer, cp = e["filer"], e["counterparty"]
        t0 = time.time()
        cik = ciks.get(filer)
        if not cik:
            print(f"[{i}/{len(sample)}] {filer} x {cp}: no CIK, skipped")
            continue
        filing, (h, lv) = locate_filing(cik, cp, args.start, end_date, cache_dir, live)
        chits += h
        clive += lv
        if not filing:
            print(f"[{i}/{len(sample)}] {filer} x {cp}: no filing located"
                  f"{'' if live else ' (cached only; --edgar to fetch)'}")
            continue
        doc, dh, dl = fetch_document(filing["url"], cache_dir, args.doc_cache_dir, live)
        chits += dh
        clive += dl
        if not doc:
            print(f"[{i}/{len(sample)}] {filer} x {cp}: document not in cache")
            continue
        passage = passage_around(strip_html(doc), cp)
        if len(passage) < 80:
            print(f"[{i}/{len(sample)}] {filer} x {cp}: passage too short to judge")
            continue
        try:
            ans = jev_call({"filer": filer, "counterparty": cp, "passage": passage},
                           QUESTIONS, key)["answers"]
        except Exception as exc:  # noqa: BLE001 - a judge failure must not invent a score
            print(f"[{i}/{len(sample)}] {filer} x {cp}: JUDGE FAILED "
                  f"{type(exc).__name__}: {exc}")
            continue
        rows.append({
            "filer": filer, "counterparty": cp, "hits": e["hits"],
            "form": filing["form"], "filing_date": filing["date"], "url": filing["url"],
            "passage_chars": len(passage),
            "financing_p": ans["looks_like_financing"]["noul"],
            "worth_read_p": ans["worth_human_read"]["noul"],
            # Retained in the CSV for diagnosis ONLY. Never published, never a finding.
            "kind_diagnostic_do_not_record": ans["mention_kind"]["choice"],
            "kind_conf_diagnostic": ans["mention_kind"]["confidence"],
            "seconds": round(time.time() - t0, 1),
        })
        print(f"[{i}/{len(sample)}] {filer} x {cp}: financing={rows[-1]['financing_p']:.2f} "
              f"worth_read={rows[-1]['worth_read_p']:.2f} [{filing['form']} {filing['date']}]")

    if not rows:
        sys.exit("no edges were judged; nothing to rank")

    # Rank by the financing signal, tie-break on whether a human read is worth it.
    rows.sort(key=lambda r: (-(r["financing_p"] or 0), -(r["worth_read_p"] or 0)))

    tier = [r for r in rows if (r["financing_p"] or 0) >= args.cut]
    never_read = [r for r in tier if (r["filer"], r["counterparty"]) not in
                  (already_read | KNOWN_REFUTED)]

    print(f"\n=== READING QUEUE (tier cut financing_p >= {args.cut}) ===")
    print(f"{'#':<4}{'EDGE':<26}{'FIN':>6}{'READ':>6}  LATEST")
    for n, r in enumerate(tier, 1):
        print(f"{n:<4}{r['filer'] + ' x ' + r['counterparty']:<26}"
              f"{r['financing_p']:>6.2f}{r['worth_read_p']:>6.2f}  "
              f"{r.get('form')} {r.get('filing_date')}")
    print(f"\n{len(tier)}/{len(rows)} judged edges in the tier; "
          f"{len(never_read)} of them were never read by a human.")
    print("Use the TIER as a reading list, not the rank order within it.")

    # ---------------------------------------------------------------- control ----
    failed = False
    print("\n=== CONTROL (truth established by a human reading the filing) ===")
    for r in rows:
        pair = (r["filer"], r["counterparty"])
        if pair in KNOWN_TRUE:
            print(f"  real      {r['filer']} x {r['counterparty']:<10} "
                  f"financing={r['financing_p']:.2f}  rank {rows.index(r) + 1}/{len(rows)}")
        elif pair in KNOWN_REFUTED:
            print(f"  REFUTED   {r['filer']} x {r['counterparty']:<10} "
                  f"financing={r['financing_p']:.2f}  rank {rows.index(r) + 1}/{len(rows)}")
    reals = [r for r in rows if (r["filer"], r["counterparty"]) in KNOWN_TRUE]
    refs = [r for r in rows if (r["filer"], r["counterparty"]) in KNOWN_REFUTED]
    if reals and refs:
        worst_real = min(r["financing_p"] for r in reals)
        for ref in refs:
            sep = worst_real - ref["financing_p"]
            ok = sep > 0
            failed = failed or not ok
            print(f"\n  separation {worst_real:.2f} - {ref['financing_p']:.2f} = {sep:+.2f} "
                  f"-> {'PASS' if ok else 'FAIL: the refutation outranks a real structure'}")
    elif not reals or not refs:
        # A control that is absent is not a passing control.
        failed = True
        print(f"\n  FAIL: incomplete control (reals={len(reals)}, refuted={len(refs)}). "
              f"The ranking is unvalidated.")
    if failed:
        print("\n  !! The control did not hold. Do NOT act on this queue. !!")
    else:
        print("\n  control held.")

    # ------------------------------------------------------------------ cost ----
    n = len(rows)
    print(f"\n=== COST ===\n  {n} edges judged; cache hits={chits}, live EDGAR fetches={clive}")
    print(f"  ~{n * 991:,} input tokens (measured 991/edge); "
          f"{sum(r['seconds'] for r in rows):.0f}s wall, {sum(r['seconds'] for r in rows) / n:.1f}s/edge")

    # ---------------------------------------------------------------- outputs ----
    generated = time.strftime("%Y-%m-%d %H:%M %Z")
    if args.out:
        os.makedirs(args.out, exist_ok=True)
        qp = os.path.join(args.out, "reading-queue.json")
        with open(qp, "w", encoding="utf-8") as fh:
            json.dump({"generated": generated, "model": MODEL, "cut": args.cut,
                       "count": len(tier), "never_read": len(never_read),
                       "control_ok": not failed, "queue": tier,
                       "all": rows, "scan_silent": meta.get("silent"),
                       "scan_errors": meta.get("errors")}, fh, indent=2)
        hp = os.path.join(args.out, "reading-queue.html")
        with open(hp, "w", encoding="utf-8") as fh:
            fh.write(render_html(tier, meta, args.cut, generated, control_ok=not failed,
                                 never_read={(r["filer"], r["counterparty"]) for r in never_read}))
        print(f"\nwrote {qp}\nwrote {hp}")
    if args.csv:
        with open(args.csv, "w", newline="", encoding="utf-8") as fh:
            w = csv.DictWriter(fh, fieldnames=list(rows[0]))
            w.writeheader()
            w.writerows(rows)
        print(f"wrote {args.csv}")
    if args.json_out:
        with open(args.json_out, "w", encoding="utf-8") as fh:
            json.dump({"queue": tier, "all": rows}, fh, indent=2)
        print(f"wrote {args.json_out}")

    if args.strict and failed:
        sys.exit(3)


if __name__ == "__main__":
    main()
