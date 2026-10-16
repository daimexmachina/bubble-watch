# bubble-watch — the proper fix: declared inputs, fair scheduling, and where signal can actually come from

**Written:** 2026-10-16 ~15:30 PDT
**Goal:** Stop fixing instances of one defect class by hand, remove the structural cause of the
`distress_events` starvation, and point the remaining effort at the only channel that can actually
carry the signal Daim keeps asking for.

**Architecture:** three independent problems, in leverage order. (1) The `Indicator` trait declares
no inputs, so nothing can verify an indicator's inputs are all present — every silent degradation
this project has suffered is an instance of that hole. (2) The fetcher has no per-host request
budget and no notion of fairness, so with a ~5-per-21-hours quota and 10 needed requests the same
five queries win forever. (3) The sentiment leg is provably unable to lead price, so "weight the
media more" cannot be delivered by weighting — the signal has to come from hard data.

**Tech stack:** Rust, existing `Fetcher` (TTL + circuit breaker + bounded backoff, all added
2026-10-16), existing `IndicatorCfg`/anchors pattern, `data/history/runs.jsonl` archive, systemd
user timer for the daily run.

---

## What 2026-10-16 actually established

Facts, all measured, that the plan depends on:

| fact | evidence |
|---|---|
| CourtListener allows ~**5 requests per ~21 hours** | `HTTP 429 {"detail":"Request was throttled. Expected available in 76078 seconds."}` |
| The model makes **10** court requests per run (5 queries × 2 date windows) | `tracked_queries()` 3 + `insolvency_queries()` 2 |
| The first 5 win; `distress_events` sits 7th–10th and **never had a turn** | scripted replay; live `SOURCE FAILURES` |
| The 45 s backoff **could not work** against a 21 h window | refusals are immediate; `RATE-LIMITED` fired 0 times |
| `opposition_pressure` silently omitted a query for **16 consecutive runs** | 28 archived reports compared |
| The **media/sentiment leg cannot be scored** — tone FOLLOWS price | v1.4 Task 7 pre-registered gate: peak at lag **+7**, p = 0.36 vs a 2000-shuffle null |

That last row matters most for the plan, so it is restated plainly: **"put more weight on media
reporting" was tested on 2026-09-26 and refuted.** Not refused — *tested*, with a pre-registered
gate, against an empirically estimated null. Sentiment lags. Weighting it would be weighting a
trailing indicator, which is why the score does not move on bad news.

---

## P1 — Kill the defect class: indicators must DECLARE their inputs

**The problem.** `src/indicators/mod.rs:21-24`:

```rust
pub trait Indicator {
    fn id(&self) -> &'static str;
    fn evaluate(&self, ctx: &Ctx) -> Reading;
}
```

An indicator reaches into `obs` and sums whatever it finds. Nothing knows what it *needed*. Every
defect found today is this one:

- `opposition_pressure` summed 2 of 3 queries for 16 runs — **no check that 3 were required**
- `distress_events` GAPped for days with no reason — **no check, and no declaration**
- `report` discarded the observations — **nothing surfaced the failure**
- `--offline` runs recorded as measurements — **nothing knew it wasn't a measurement**

Fixing these individually is what I did today. It is the wrong shape of fix: the next indicator
will do the same thing.

**The fix.** Add a declared-input contract to the trait, and enforce it in one place.

### Task 1 — Declare the contract

**Files:** Modify `src/indicators/mod.rs:21-24`

```rust
/// One source input an indicator depends on, so completeness can be CHECKED rather than assumed.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// A named query within a source's observation vector, e.g. ("courtlistener", "injunction").
    Query(&'static str, &'static str),
    /// A whole channel that must be non-empty, e.g. ("insolvency", "").
    Channel(&'static str),
}

pub trait Indicator {
    fn id(&self) -> &'static str;
    fn evaluate(&self, ctx: &Ctx) -> Reading;
    /// What this indicator needs in order to be a COMPLETE measurement.
    ///
    /// Default is empty, which means "declares nothing" — and a check that fires on an empty
    /// declaration is worthless, so the default must be treated as UNKNOWN, not as satisfied.
    /// Indicators opt in; the audit in Task 5 lists the ones that have not.
    fn requires(&self) -> Vec<Input> { Vec::new() }
}
```

**Step 1: failing test** — `src/indicators/mod.rs`
```rust
#[test]
fn an_indicator_that_declares_nothing_is_treated_as_UNKNOWN_not_as_complete() {
    struct Undeclared;
    impl Indicator for Undeclared {
        fn id(&self) -> &'static str { "undeclared" }
        fn evaluate(&self, _ctx: &Ctx) -> Reading { Reading::Unavailable { reason: "n/a".into() } }
    }
    assert_eq!(Undeclared.requires(), Vec::new());
    assert!(!contract_satisfied(&Undeclared, &Observations::default()),
        "an empty declaration must not read as 'all inputs present'");
}
```
**Step 2:** `cargo test --release --lib contract_satisfied` → expect FAIL (`contract_satisfied` undefined).
**Step 3:** implement `contract_satisfied(ind: &dyn Indicator, obs: &Observations) -> bool` returning
`false` when `requires()` is empty.
**Step 4:** re-run → PASS.
**Step 5:** commit `feat(indicators): declare required inputs; empty means UNKNOWN, not satisfied`.

### Task 2 — Enforce it at the one place every indicator passes through

**Files:** Modify `src/indicators/mod.rs` (`evaluate_one`, ~line 27)

After `ind.evaluate(ctx)` returns `Reading::Scored`, if `!contract_satisfied(...)`, append the
`INCOMPLETE MEASURE` disclosure (reuse the exact wording already shipped for the court
indicators, so there is one phrasing in the codebase, not two).

**Step 1: failing test** — build a `Scored` reading from a 2-of-3 fixture and assert the rendered
detail carries `INCOMPLETE MEASURE`.
**Step 2:** run → expect FAIL.
**Step 3:** implement the enforcement.
**Step 4:** run → PASS. Also confirm the existing `opposition_pressure` and `distress_events`
substring assertions still pass, since they now run through the shared path.
**Step 5:** commit `feat(indicators): enforce declared inputs centrally — one phrasing, one place`.

### Task 3 — Migrate the two court indicators onto the contract

**Files:** Modify `src/indicators/fundamentals.rs`

Delete the hand-rolled `absent`/`subset_note` blocks added today and return `requires()` instead.
**Verification:** the two existing tests (`opposition_pressure_declares_a_query_that_went_missing…`,
`distress_events_declares_a_missing_query_too`) must pass **unchanged** — they are the regression
net proving the central mechanism reproduces the hand-rolled behaviour exactly.

### Task 4 — Falsify

Falsification is the acceptance criterion, not a formality:

- delete an input from a fixture → the indicator **must** declare `INCOMPLETE MEASURE`, not score
- make `requires()` empty → the audit in Task 5 **must** list the indicator
- make `contract_satisfied` return `true` unconditionally → Task 1's test **must** fail

### Task 5 — Audit the remaining 21 indicators

**Files:** Create `src/indicators/audit.rs`; wire into `bubble-watch doctor`

Report every indicator that declares nothing, and every declared input the current `Observations`
do not satisfy. This is what turns "we fixed the two we knew about" into "we can see the rest".

**Why this is P1:** it is the difference between having found four bugs and having closed the hole
that produces them.

---

## P2 — Kill the starvation: a request budget and a fair rotation

**The problem.** With ~5 slots and 10 needed requests, first-come-first-served means the same five
always win. The 7-day TTL shipped today is a **bet** that the spread works out; it is unverified
because the quota is spent.

**The fix.** Give `Fetcher` a per-host budget and order rate-limited fetches by *least recently
refreshed*, so the queries that lost last time go first this time. With 5 slots and 10 queries,
every query refreshes within ~2 runs — deterministic, not hopeful.

### Task 6 — Record when each query last succeeded

**Files:** Modify `src/http.rs`

Expose `pub fn cache_age(&self, url: &str) -> Option<Duration>` (the same mtime read the TTL path
already does). No new state.

### Task 7 — Order the court fetches least-recently-refreshed first

**Files:** Modify `src/sources/courtlistener.rs`, `src/sources/mod.rs`

Before the two fetch loops, sort `tracked_queries() ++ insolvency_queries()` by `cache_age`
descending (never-refreshed sorts first).

**Step 1: failing test** — with a fake cache where `injunction` is fresh and `lawsuit` is not,
assert the fetch order puts `lawsuit` first.
**Step 2:** run → FAIL. **Step 3:** implement. **Step 4:** PASS.
**Step 5:** commit `feat(court): fetch least-recently-refreshed queries first, so none starve forever`.

### Task 8 — Declare the budget, and say when it bit

**Files:** Modify `src/http.rs`, `src/sources/mod.rs`, `src/model.rs`

Cap court requests per run at the measured quota (5). Past the cap, serve cache or declare the
shortfall in `Report.failures` (field shipped today) with the words *"not attempted — request
budget spent; will be retried first next run"*. The rotation makes that sentence **true**.

### Task 9 — Falsify by starvation harness

A scripted run with the quota artificially set to 2 must show: two queries served fresh,
eight declared, and the *next* run serving a different two. That is the property the current
code cannot demonstrate.

### Decision this depends on: a CourtListener token

The code has **no auth support at all** (`grep token|api_key src/sources/courtlistener.rs` → empty).
A free CourtListener account raises the limit enough to make the whole budget question moot. It is
~30 minutes of code plus a registration **you** would have to do. If you want it, it becomes P2's
Task 0 and P2 mostly evaporates.

---

## P3 — Signal, and being honest about where it can exist

**The uncomfortable finding, restated because it governs everything below:** the media/sentiment
leg cannot be scored. v1.4 Task 7 pre-registered that gate and it came back **NULL** — tone peaks
at lag **+7**, i.e. it *follows* price, p = 0.36 against an empirically estimated null. So the
"the score should move on bad news" intuition was already tested and the data says bad news trails
the market rather than presaging it. **Weighting it harder would make the score worse, not better**
— it would add a trailing indicator with real weight.

That leaves two honest routes to a more responsive score, and they are hard-data routes:

### Route A — make the existing hard signal count (cheap, no new sources)
`opposition_pressure` (court filings — the *hard* version of "datacentre resistance") scored at
weight 8 while silently reading 2 of 3 queries for 16 runs. Now complete. Before adding sources,
measure what the existing panel does with its inputs actually intact: re-run the three archived
dates with the fully-populated indicator and see how much of the "too conservative" gap was the
silent subset rather than the weights. **This is the first thing to do, because it may answer the
complaint without any new data.**

### Route B — the demand side and international competition (scoped, partially built)
`.hermes/plans/v1.3-demand-side-plan.md` shipped `inference_demand` and `energisation_delay`.
Still unbuilt and requiring free-registration keys you would have to obtain: Japan e-Stat, METI,
KOSIS, data.go.kr, Comtrade. These are the sources that show *where revenue growth gets restricted*,
which is the question you have been asking.

**Recommendation:** Route A first (no new cost, possibly answers the question), then Route B's
sources in priority order once the keys exist.

---

## Decisions I need from you

1. **A CourtListener account/token?** Removes P2 entirely. Your action to register; ~30 min of my code.
2. **International API keys?** Gates Route B. Which of Japan e-Stat / METI / KOSIS / data.go.kr /
   Comtrade do you want first? Each is a free registration.
3. **Ordering.** My recommendation: **P1 → Route A → P2 → Route B.** P1 first because it is the
   hole that produced everything else and it is independent of any key; Route A second because it
   is cheap and may resolve the complaint that started this.

---

## Risks and open questions

- **P1 changes a shared path all 23 indicators flow through.** Mitigated by making the default
  `requires()` empty and treating empty as UNKNOWN — so nothing changes behaviour until an
  indicator opts in, and Task 5's audit makes the un-migrated set visible rather than invisible.
- **The 7-day TTL is still unverified.** It cannot be tested until the quota rolls over
  (~2026-10-17). If it fails, P2 supersedes it rather than layering on it.
- **`distress_events` needs its own post-rollover run** to confirm the channel heals at all.
- **Unresolved:** whether Route A's counterfactual is even computable — the archive stores
  composite/coverage but not per-query court counts, so it may need re-fetching the pre-10-01
  window, which the quota may not allow. If so, Route A degrades to "wait and compare forward".

## Validation

- Every task above ships with a test that is **falsified before it is kept** — the mutation is the
  thing that must run, not just the test.
- No diagnostic run without `--no-record` (now enforced in `pipeline_with_history`, not by memory).
- The archive must stay at one row per day; `tests/archive_integrity.rs` guards it.
