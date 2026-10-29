//! Plain-English summary.
//!
//! Written to be readable by someone with no finance background. Rules that this
//! module must keep to:
//!
//!   * short sentences, common words, no jargon (there is a test that rejects a
//!     list of finance terms outright);
//!   * every number comes from the report itself, so the wording cannot drift
//!     away from the data;
//!   * it NEVER claims to predict timing — it says the opposite, in plain words;
//!   * anything unmeasurable is stated as unmeasurable, not glossed over.
//!
//! It is generated, not hand-written, for the same reason the rest of the tool
//! is: if the numbers move, the paragraph describing them must move with them.

use crate::model::{IndicatorReading, Reading, Report};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LaymanSummary {
    pub what_this_is: String,
    pub the_score: String,
    pub what_is_stretched: String,
    pub what_is_calm: String,
    pub what_we_cannot_measure: String,
    pub about_timing: String,
    /// Whether things are getting better or worse. This is the section that
    /// answers the question the credit indicators were always meant to answer.
    pub direction_of_travel: String,
    /// Where the sub-questions DISAGREE. Present because the composite averages
    /// questions whose answers can differ by tens of points, and the average hides
    /// that. Empty when the spread is small (< 10 points), because a small spread
    /// needs no explanation. See `Report::sub_scores`.
    #[serde(default)]
    pub the_questions_disagree: String,
    /// The model's PERMANENT holes, declared in `DECLARED_BLIND_SPOTS` rather than inferred from
    /// the run. Stated prominently rather than listed as data quality, because the point is what
    /// the report cannot see even at 100% coverage: a channel that merely failed this morning is a
    /// data-quality gap, and a data-quality gap is what the quality section is for.
    pub known_blind_spots: String,
    pub bottom_line: String,
}

/// Friendly wording for each indicator, so the summary never shows a raw id or a
/// technical label.
fn plain_name(id: &str) -> &'static str {
    match id {
        "valuation_stretch" => "how expensive stocks are compared with their own long-run trend",
        "concentration" => "how much the market depends on a few giant companies",
        "breadth" => "how many companies are rising, not just the biggest ones",
        "issuance" => "how many new shares companies are selling",
        "volatility" => "how jumpy the market is",
        "credit_hy" => "what weaker companies pay to borrow money",
        "credit_ig" => "what strong companies pay to borrow money",
        // Written for a non-specialist, and deliberately NOT called "funding risk" or "a funding
        // crisis": the honest plain description is the plumbing itself, since that is what is being
        // measured and what a reader needs to picture.
        "funding_stress" => "how expensive it is for banks and funds to borrow cash overnight, which is the plumbing that keeps debt rolling and jams first when money gets tight",
        "credit_tail" => "how much more the weakest borrowers pay to borrow than the rest of the high-yield market — reported but deliberately not counted, because the available history is only about three years",
        "cp_funding" => "what the money market charges to roll short-term company IOUs against the Treasury bill rate — reported but deliberately not counted, because this series publishes only a few times a month and is too slow to be a warning",
        "capex_vs_cashflow" => "how much AI companies spend building data centres compared with the cash their business brings in",
        "inference_demand" => "whether the AI compute that has been built is actually being used, and whether that use is still growing",
        "energisation_delay" => "how long a new power project has to wait in the queue before it can actually be switched on",
        "opposition_pressure" => "how often communities and local groups are going to court to try to stop data centres being built",
        // Deliberately says "actually going bust ... rather than from stories about one", because that
        // distinction is the whole reason this indicator reads a court docket rather than the news.
        "distress_events" => "whether AI companies are actually going bust, counted from the court filings that record a bankruptcy rather than from stories about one",
        "public_attention" => "how many people are looking up AI-bubble topics on Wikipedia — reported but deliberately not counted, because people tend to look things up after they are news rather than before",
        "hyperscaler_shock" => "the worst week any of the six biggest AI companies has just had — so a single 12% day at one of them is visible instead of being averaged away by the index",
        "regulatory_risk" => "the betting markets' odds that the Democrats take Congress, and what that might mean for AI regulation — the only forward-looking input in this tool",
        "china_ai_derating" => "how far Chinese AI shares have fallen compared with the rest of the Chinese market — the first non-US market measure in this tool",
        "buildout_opposition_news" => "how often the news reports data-centre plans being WITHDRAWN — reported but deliberately not counted, because this is the local-permitting layer the model used to miss entirely",
        "leveraged_builder_stress" => "how far the heavily borrowed AI builders have fallen from their own highs — the BORROWERS, reported but deliberately not counted until a lead/lag test says it should be",
        "funding_gap" => "how much AI companies spend compared with what they sell",
        "leverage" => "how much debt these companies are carrying",
        "foreign_interest" => "how much foreign money owns US shares",
        "circularity" => "money moving in circles between the companies involved",
        "primary_market_supply" => "how many companies are filing to sell shares for the first time",
        "backlog_quality" => "whether the big orders companies have booked are turning into money actually collected",
        "private_credit_growth" => "how fast lending outside the public bond market is growing",
        "datacenter_construction" => "how fast data-centre building is growing, as measured by the government",
        "grid_cancellations" => "how much announced power-plant capacity has been abandoned",
        "narrative_saturation" => "how many companies are talking about AI in their official filings",
        "depreciation_subsidy" => "whether companies are stretching how slowly they write down their equipment to flatter their profits",
        "frontier_premium" => "how far ahead the best paid-for AI model is compared with the best free one",
        _ => "one of the measurements",
    }
}

/// Describe a 0-100 stress number in words a non-specialist can act on.
fn stress_words(stress: f64) -> &'static str {
    match stress {
        s if s < 20.0 => "low",
        s if s < 35.0 => "fairly low",
        s if s < 50.0 => "middling",
        s if s < 65.0 => "high",
        s if s < 80.0 => "very high",
        _ => "at an extreme",
    }
}

fn scored(r: &IndicatorReading) -> Option<f64> {
    if r.weight <= 0.0 {
        return None; // declared blind spot, never scored
    }
    match &r.reading {
        Reading::Scored { stress, .. } => Some(*stress),
        Reading::Unavailable { .. } => None,
    }
}

/// Join phrases into a readable list: "a", "a and b", "a, b and c".
fn join_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

/// The model's PERMANENT blind spots: what this report cannot see even on a day when every source
/// answers, because nothing it can read records them.
///
/// WHY THIS IS A DECLARATION AND NOT A COMPUTATION
/// ----------------------------------------------
/// The section used to be built from `data_quality.unavailable` filtered to weight 0, i.e. "a
/// declared channel that happened to fail on THIS run", and it then told the reader those entries
/// "would still be missing on a day when every source answered". For a channel that is fetched and
/// reported whenever its source answers, that is false. MEASURED, from the recorded runs in `out/`:
/// `ai-bubble-watch-2026-10-16.json` lists exactly one such entry — `buildout_opposition_news`
/// (weight 0, GDELT, unavailable that day) — and the report published it as a permanent hole in the
/// model, while on 2026-10-11..15 the same section said the model had no holes at all, because on
/// those days no weight-0 channel failed. The list was therefore "whatever went quiet today", which
/// is data quality, not a blind spot. A run's failures must never reach this list; the guard is
/// `the_blind_spot_list_can_never_be_filled_by_todays_failures`, which fails if they do.
///
/// EVERY ENTRY IS GROUNDED IN THIS REPO, so the declaration can be audited rather than believed:
///
///   * a failure filed outside the United States — `config/indicators.toml`, `distress_events`:
///     "US ONLY, so a non-US failure is a declared blind spot and never a zero". Measured: Firmus
///     withdrew its ASX listing and no US source sees it.
///   * an anonymous counterparty — `SPEC.md` §15.7, "The limit that cannot be fixed by reading":
///     "The method cannot see a relationship a filer declines to name ... No amount of further
///     reading resolves it, because the identity is not in the filing."
///   * the QUANTITY of AI-specific credit raised — every channel here that touches credit reads the
///     PRICE of it (`credit_hy`, `credit_ig`, `credit_tail`) or the AGGREGATE market for it
///     (`funding_stress`, `cp_funding`), so none of them can see AI-specific issuance. Measured and
///     sourced in `.hermes/analysis/2026-10-17-news-cloud-selloff-and-ai-financing-fears.md` §4
///     (commit ea97ae9, sha256 e8600068…): global AI-related debt issuance fell from $113bn in June
///     2026 to $23bn in September 2026 — roughly 80% in three months — per the Financial Times from
///     Morgan Stanley data, cited by Axios on 2026-10-14 and 2026-10-16, while ~$18bn of bank loans
///     for an Oracle data-centre campus slid into stressed territory (Axios, 2026-10-16). PERMANENT,
///     not "quiet today": the reporting source is a private research dataset, and individual
///     CDS/bond quotes are ALREADY declared unavailable free in `leveraged_builder_stress`'s
///     rationale, so no source this tool can reach shows it. It is a limit of the instrument, not a
///     gap in today's run.
///
/// EMPTY IS A LEGAL STATE, and it renders the honest fallback: a model with no declared hole says it
/// measures a fixed list and claims nothing beyond it. Do not add a channel here because it went
/// quiet — that is precisely the defect this constant replaced. A NEW entry is a claim about the
/// world, so it needs its grounding written down beside it.
const DECLARED_BLIND_SPOTS: &[&str] = &[
    "a company outside the United States going bust — the count of failures reads United States \
court filings, and this report reads no other country's register",
    "money going in circles where the company does not name the other side — only the relationships \
a filing actually names can be read, so no amount of further reading resolves it",
    "how much new debt the AI complex is raising — this report reads the price of credit and the \
aggregate market for it, and the monthly volume of AI-specific borrowing is published only in \
private research data",
];

/// The wording of the "cannot see at all" section, from that declared list.
///
/// PURE, and separated from `summarize` so both branches can be tested and falsified by name —
/// including the empty one, which a run of the shipped config does not reach while the model
/// declares a hole, and which would otherwise rot untested.
fn blind_spot_text(entries: &[&str]) -> String {
    if entries.is_empty() {
        // Nothing declared: state the bound on the claim rather than reassuring the reader.
        return "This report measures a fixed list of things and claims nothing beyond it."
            .to_string();
    }
    let what: Vec<String> = entries.iter().map(|s| (*s).to_string()).collect();
    format!(
        "There {} this report cannot see at all: {}. These would stay missing on a day when every \
source answered, because nothing it can read would show them. They are not gaps in today's run — \
the checks it failed today are listed separately, and a later run may well fill those. Nothing \
here is guessed at, and none of it is counted in the score.",
        if what.len() == 1 {
            "is one thing"
        } else {
            "are things"
        },
        join_list(&what)
    )
}

pub fn summarize(r: &Report) -> LaymanSummary {
    let what_this_is = "This report tries to answer one question: how much does the AI boom look \
like a bubble right now? It scores a set of measurements from 0 to 100. A higher score means more \
of the warning signs are showing."
        .to_string();

    // Which band the score falls in, in plain words.
    let phase_plain = match r.phase.as_str() {
        "early" => "early",
        "mid" => "middle",
        "late" => "late",
        "critical" => "extreme",
        _ => "unclear",
    };

    // Count the indicators that actually contributed, not the weight total —
    // "weight 100" is not "100 measurements", and printing the weight here was
    // simply wrong.
    let scored_count = r
        .indicators
        .iter()
        .filter(|i| i.weight > 0.0 && i.reading.is_available())
        .count();
    // A weight-0 channel is DECLARED AND UNSCORED, which is NOT the same as unmeasurable.
    // Counting every weight-0 entry here made the headline claim the report "cannot measure at
    // all" things it had in fact measured and rendered. Measured 2026-10-17 on commit d5f0f62
    // (offline, data/cache): five weight-0 channels, THREE of them carrying a reading —
    // leveraged_builder_stress, credit_tail (9.55 @2026-10-15) and cp_funding (-0.11 @2026-10-05)
    // — and the sentence still read "plus 5 things it cannot measure at all". That also
    // contradicts the dashboard's own weight-0 card, which says of exactly these channels
    // "These channels are MEASURED AND DISPLAYED but carry zero weight" (report/html.rs).
    // Only a weight-0 channel that produced NO reading belongs in this clause.
    //
    // AND EVEN THAT IS A STATEMENT ABOUT TODAY, NOT ABOUT THE CHANNEL. "Cannot measure at all" was
    // the same category error the blind-spot section made (fixed in the same change): a weight-0
    // channel is fetched and reported whenever its source answers — `buildout_opposition_news` gave
    // no reading on 2026-10-16 and a reading on every other day in `out/` — so a permanence claim
    // about it is false, while "it could not measure it today" is exactly what the selection says.
    let declared_gaps = r
        .indicators
        .iter()
        .filter(|i| i.weight <= 0.0 && !i.reading.is_available())
        .count();

    let the_score = format!(
        "The score today is {:.0} out of 100, which puts it in the {} range. For scale: a score \
above 55 would look more like the late stage of a bubble, and only true extremes have pushed \
past 75. It is built from {} different measurements{}. That is not a huge number, and it is \
why this report tells you what it cannot check as well as what it can.",
        r.composite,
        phase_plain,
        scored_count,
        if declared_gaps == 1 {
            ", plus 1 thing it could not measure today".to_string()
        } else if declared_gaps > 1 {
            format!(", plus {} things it could not measure today", declared_gaps)
        } else {
            String::new()
        }
    );

    // Anything at or above 50 is genuinely stretched; sort by how much it
    // actually moves the final number.
    let mut stretched: Vec<(&IndicatorReading, f64)> = r
        .indicators
        .iter()
        .filter_map(|i| scored(i).map(|s| (i, s)))
        .filter(|(_, s)| *s >= 50.0)
        .collect();
    stretched.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let what_is_stretched = if stretched.is_empty() {
        "Nothing on the list is currently flashing red. No single measurement scored in the \
high range. That does not prove there is no bubble. It means the things we can measure are \
not showing one yet."
            .to_string()
    } else {
        let items: Vec<String> = stretched
            .iter()
            .take(4)
            .map(|(i, s)| format!("{} ({} out of 100)", plain_name(&i.id), s.round() as i64))
            .collect();
        format!(
            "These measurements are the ones showing strain: {}. Read those as the places where \
the boom is genuinely stretched, rather than the whole picture being stretched.",
            join_list(&items)
        )
    };

    // Calm side: clearly low readings, lowest first.
    let mut calm: Vec<(&IndicatorReading, f64)> = r
        .indicators
        .iter()
        .filter_map(|i| scored(i).map(|s| (i, s)))
        .filter(|(_, s)| *s < 35.0)
        .collect();
    calm.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let what_is_calm = if calm.is_empty() {
        "Nothing is currently sitting in the calm range.".to_string()
    } else {
        let items: Vec<String> = calm
            .iter()
            .take(4)
            .map(|(i, s)| format!("{} ({} out of 100)", plain_name(&i.id), s.round() as i64))
            .collect();
        format!(
            "These measurements look calm: {}. The important thing here is what that means for \
the market's MOOD versus the money being SPENT. Prices are not behaving like a classic bubble \
even where spending is stretched.",
            join_list(&items)
        )
    };

    // WHERE THE ANSWER DEPENDS ON THE QUESTION.
    //
    // This is the section that stops one blended number from hiding a real
    // disagreement. The composite averages questions that can legitimately differ by
    // tens of points, and it always has — but until now a reader saw only the average.
    // Measured 2026-09-26: strain 53.3 vs pricing 25.2, blended to 40.2.
    //
    // States the SPLIT and names the highest and lowest, without asserting which one is
    // right: the model does not know, and picking a winner would be the prior dressed
    // as arithmetic.
    let spread_paragraph = {
        let scored_groups: Vec<(&crate::score::SubScore, &str, &str)> = r
            .sub_scores
            .iter()
            .filter(|s| s.score.is_some() && s.weight > 0.0)
            .filter_map(|s| {
                r.sub_question_labels
                    .iter()
                    .find(|(k, _, _, _)| k == &s.key)
                    .map(|(_, _, label, _)| (s, label.as_str(), ""))
            })
            .collect();
        let mut ranked: Vec<(&crate::score::SubScore, &str)> =
            scored_groups.iter().map(|(s, p, _)| (*s, *p)).collect();
        ranked.sort_by(|a, b| {
            let av = a.0.score.unwrap_or(0.0);
            let bv = b.0.score.unwrap_or(0.0);
            bv.partial_cmp(&av)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.key.cmp(&b.0.key))
        });

        if ranked.len() < 2 {
            String::new()
        } else {
            let hi = ranked[0];
            let lo = ranked[ranked.len() - 1];
            let hi_v = hi.0.score.unwrap_or(0.0);
            let lo_v = lo.0.score.unwrap_or(0.0);
            let gap = hi_v - lo_v;
            if gap < 10.0 {
                String::new()
            } else {
                format!(
                    "The questions do not agree with each other, and that is worth more than the \
single score. The strongest answer comes from {} ({:.0} out of 100). The calmest comes from {} \
({:.0} out of 100). That is a gap of about {:.0} points. The overall score of {:.0} is the two of \
them averaged together. Read the gap, not just the average: it says the strain is showing up in \
one part of the system and not in another, and this report cannot tell you which one will turn \
out to have been right.",
                    hi.1, hi_v, lo.1, lo_v, gap, r.composite
                )
            }
        }
    };

    // The weighted gaps: what the score had to work around today. Weight-0 channels are excluded
    // whatever they do, because they never touch the score; one that went quiet today is data
    // quality like any other silence, and it is listed in the quality section rather than here.
    let gaps: Vec<String> = r
        .data_quality
        .unavailable
        .iter()
        .filter(|u| u.weight > 0.0)
        .map(|u| plain_name(&u.id).to_string())
        .collect();

    let what_we_cannot_measure = if gaps.is_empty() {
        "Every measurement this report attempted today, it managed to take. That is not the same \
as measuring everything that matters — the things it cannot measure at all are set out above."
            .to_string()
    } else {
        format!(
            "There are things this report could not check today: {}. They are left blank rather \
than guessed at, and the score is adjusted to account for their absence.",
            join_list(&gaps)
        )
    };

    // Timing. The single most misused output, so it is stated plainly and
    // without hedging buried in jargon.
    let about_timing = match r.analog.range_months {
        Some(rg) => format!(
            "This report cannot tell you when a bubble might burst. Nobody can. The historical \
comparison puts setups like today's roughly {:.0} to {:.0} months before a peak in the two past \
episodes it is based on. That is a description of what similar situations have looked like. It \
is not a forecast, and it is built on only two examples, so treat it as a rough yardstick at \
best. Bubbles can also run for years longer than anyone expects.",
            rg[0], rg[1]
        ),
        None => "This report cannot tell you when a bubble might burst, and today it is not even \
offering a historical comparison, because too much of the data is missing for that comparison \
to mean anything. What it can tell you is how stretched things look right now."
            .to_string(),
    };

    // One-sentence takeaway, assembled from the two sides.
    let top = stretched.first().map(|(i, s)| (*s, i.id.as_str()));
    let bottom_line = match top {
        Some((s, id)) => format!(
            "In one line: the money being spent looks {} ({}), while the market's mood looks \
calmer than that would suggest. A stretch between spending and pricing like this can go on \
for a long time, and it can also end suddenly. Nothing here tells you which.",
            stress_words(s),
            plain_name(id)
        ),
        None => "In one line: none of the things we can measure look like a bubble in its final \
stages right now. That is not the same as being safe."
            .to_string(),
    };

    // Direction of travel. Written in plain words, and it must never imply a
    // trend the tool refused to compute — when there is no delta, this says so
    // and explains why, rather than staying silent.
    // HOW MUCH OF THIS SCORE COULD HAVE MOVED AT ALL.
    //
    // Computed BEFORE the direction match because it qualifies every branch of it: a change of
    // any size means something different when most of the weight is pinned to an older date.
    // CONTEXT ONLY, derived — never an input to the composite.
    let cadence = match &r.freshness {
        Some(f) if f.total_weight > 0.0 && f.frozen_weight > 0.0 => {
            let live_pct = (100.0 * f.live_weight / f.total_weight).round();
            let oldest = f
                .oldest_change
                .clone()
                .unwrap_or_else(|| "an earlier date".into());
            let age = f.oldest_change_days.unwrap_or(0);
            format!(
                " How much of this could have moved: {:.0}% of the score's weight changed \
within the last {} days. The other {:.0}% has not moved since {} ({} days ago), because {} of \
{} inputs are quarterly-cadence data that only change when new filings arrive. So a small change \
means the FAST half of the model was quiet — it is not evidence that the slow half is safe, \
because the slow half has not been re-measured.",
                live_pct,
                f.window_days,
                100.0 - live_pct,
                oldest,
                age,
                f.frozen_indicators,
                f.frozen_indicators + f.live_indicators
            )
        }
        Some(f) if f.total_weight > 0.0 => format!(
            " How much of this could have moved: all of it — every input changed within the last \
{} days, so a flat reading here reflects measurements that are genuinely being refreshed rather \
than pinned to an older date.",
            f.window_days
        ),
        // No history, or no history read: there is nothing to say about freshness, and
        // inventing a figure would be worse than silence.
        _ => String::new(),
    };

    let direction_of_travel = match &r.trend.delta {
        Some(d) => {
            let change = match d.direction {
                crate::model::Direction::Flat => {
                    // "the numbers are stable" was a FALSE CLAIM and is removed. Measured
                    // 2026-10-16: 49% of the weight had not moved since 2026-09-17 because
                    // those inputs are quarterly filings, so the score CANNOT move much
                    // between filing seasons. Calling a pinned instrument "stable" asserts
                    // something about the world that the data does not support.
                    //
                    // DELIBERATELY NOT CROSS-REFERENCED. An earlier version said "read the
                    // 'how much of this could have moved' note below", which DANGLED whenever
                    // freshness was unavailable and pointed the reader at a note that was not
                    // there. The nuance belongs to the cadence clause, which states it only
                    // when it has the data to know.
                    "barely changed, though whether that reflects a quiet world or an instrument \
that cannot move yet is a separate question"
                }
                crate::model::Direction::Rising => {
                    "has got worse, meaning more of the warning signs are showing than before"
                }
                crate::model::Direction::Falling => {
                    "has got better, meaning fewer of the warning signs are showing than before"
                }
            };
            // Name what moved most, but only when something moved meaningfully.
            let mover = d
                .indicators
                .iter()
                .find(|i| i.direction != crate::model::Direction::Flat)
                .map(|i| {
                    format!(
                        " The biggest single change was in {} ({} points {}).",
                        plain_name(&i.id),
                        (i.delta.abs() * 10.0).round() / 10.0,
                        if i.delta > 0.0 { "worse" } else { "better" }
                    )
                })
                .unwrap_or_default();
            let phase = if d.phase_changed {
                format!(
                    " The overall description also moved from '{}' to '{}', which is the kind of \
change worth paying attention to.",
                    plain_phase(&d.phase_then),
                    plain_phase(&d.phase_now)
                )
            } else {
                String::new()
            };
            format!(
                "Comparing today with {} ({} days ago): the overall score {} ({} points).{} That \
comparison is only made against a previous reading taken under the same conditions, so an \
apples-to-apples comparison is guaranteed{}.{}",
                d.baseline_date,
                d.elapsed_days.round() as i64,
                change,
                format_args!("{:+.1}", d.composite_delta),
                mover,
                phase,
                cadence
            )
        }
        None => {
            // Say that it cannot be computed, and why. Silence here would look
            // like "nothing has changed", which is a different claim entirely.
            //
            // THE RAW REASON IS DELIBERATELY NOT EMBEDDED HERE. It used to be interpolated in
            // full, and because every rejected archive entry contributes its own sentence the
            // plain-English block grew into the single largest wall of text on the page (~3,300
            // words of near-identical refusal). The specifics belong in the technical detail,
            // which is collapsed; this block states the situation in plain terms and says where
            // the detail is. It must stay short enough to actually be read.
            format!(
                "This report cannot yet say whether things are getting better or worse — there is \
no earlier reading it is willing to compare against. It will not compare two readings taken \
under different conditions, because the difference would then reflect the conditions rather \
than the market. Rather than show you a number that means nothing, it says nothing and tells you \
why; the exact reason is in the collapsed note below and in the caveats at the foot of this \
page. Run it again on another day and a comparison will appear.{}",
                cadence
            )
        }
    };

    // PERMANENT blind spots, DECLARED and never inferred from today's data.
    //
    // THIS SECTION USED TO BE BUILT FROM `unavailable ∩ weight == 0` — "a declared channel that
    // happened to fail on this run" — and then told the reader those entries were permanent. For a
    // channel that IS fetched and reported on every other day that is false, and it made the
    // section's content a function of this morning's outages: measured 2026-10-16, the only entry
    // was `buildout_opposition_news` (weight 0, GDELT, one lost day), and the one-channel branch
    // also rendered a double space from a dead `format!` argument. Today's failures are data
    // quality and are reported as data quality; the list below cannot be added to by a bad day.
    let known_blind_spots = blind_spot_text(DECLARED_BLIND_SPOTS);

    LaymanSummary {
        what_this_is,
        the_score,
        what_is_stretched,
        what_is_calm,
        what_we_cannot_measure,
        about_timing,
        direction_of_travel,
        the_questions_disagree: spread_paragraph,
        known_blind_spots,
        bottom_line,
    }
}

/// Plain wording for a phase id, so the summary never shows a raw label.
fn plain_phase(id: &str) -> &'static str {
    match id {
        "early" => "early",
        "mid" => "middle",
        "late" => "late",
        "critical" => "extreme",
        _ => "unclear",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AnalogWindow, DataQuality, IndicatorReading, Provenance, Reading, Report, Trend,
        UnavailableItem,
    };

    fn prov() -> Provenance {
        Provenance {
            source: "t".into(),
            endpoint: "t".into(),
            as_of: "2026-09-17".into(),
            retrieved_at: "2026-09-17T00:00:00Z".into(),
        }
    }

    fn ind(id: &str, weight: f64, stress: Option<f64>) -> IndicatorReading {
        IndicatorReading {
            id: id.into(),
            label: id.into(),
            weight,
            rationale: String::new(),
            reading: match stress {
                Some(s) => Reading::Scored {
                    stress: s,
                    value: 1.0,
                    unit: "u".into(),
                    detail: String::new(),
                    provenance: prov(),
                },
                None => Reading::Unavailable {
                    reason: "down".into(),
                },
            },
            contribution: None,
            group: None,
        }
    }

    fn report(inds: Vec<IndicatorReading>, composite: f64, months: Option<[f64; 2]>) -> Report {
        let unavailable: Vec<UnavailableItem> = inds
            .iter()
            .filter(|i| !i.reading.is_available())
            .map(|i| UnavailableItem {
                id: i.id.clone(),
                label: i.label.clone(),
                weight: i.weight,
                reason: "down".into(),
            })
            .collect();
        Report {
            failures: Vec::new(),
            tool: "bubble-watch".into(),
            version: "0".into(),
            generated_at: "2026-09-17T00:00:00Z".into(),
            composite,
            coverage: 1.0,
            sub_scores: vec![],
            sub_question_labels: vec![],
            freshness: None,
            velocity: None,
            confidence: "high".into(),
            phase: "early".into(),
            phase_label: "Early".into(),
            phase_detail: "d".into(),
            analog: AnalogWindow {
                method: "historical_analog".into(),
                is_probability: false,
                analogs: vec!["a".into(), "b".into()],
                derivation: "d".into(),
                caveat: "c".into(),
                range_months: months,
                band: months.map(|_| "early".to_string()),
                band_note: None,
            },
            trend: Trend::empty("no history in this test"),
            // None in this fixture: with no archive there is no attribution to make, which is
            // exactly the case the drift module reports as "no attribution is possible" rather
            // than as a zero.
            drift: None,
            band_margin: None,
            exposure: Vec::new(),
            falsifiers: Vec::new(),
            judgments: Vec::new(),
            explosiveness: None,
            indicators: inds,
            data_quality: DataQuality {
                total_weight: 100.0,
                available_weight: 100.0,
                coverage: 1.0,
                unavailable,
                notes: vec![],
            },
            sources: vec![],
            headline: String::new(),
            caveats: vec![],
            disclaimer: String::new(),
            layman: LaymanSummary {
                what_this_is: String::new(),
                the_score: String::new(),
                what_is_stretched: String::new(),
                what_is_calm: String::new(),
                what_we_cannot_measure: String::new(),
                about_timing: String::new(),
                direction_of_travel: String::new(),
                the_questions_disagree: String::new(),
                known_blind_spots: String::new(),
                bottom_line: String::new(),
            },
        }
    }

    #[test]
    fn mentions_the_actual_score() {
        let r = report(
            vec![ind("volatility", 8.0, Some(20.0))],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        assert!(
            s.the_score.contains("32"),
            "score must appear: {}",
            s.the_score
        );
        assert!(s.the_score.contains("early"));
    }

    #[test]
    fn counts_measurements_not_weight() {
        // Regression: this once reported the weight total (100) as the number of
        // measurements, which is simply false.
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("credit_hy", 12.0, Some(16.0)),
                ind("foreign_interest", 0.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.the_score.contains("2 different measurements"),
            "must count the scored indicators: {}",
            s.the_score
        );
        assert!(
            !s.the_score.contains("100 separate measurements"),
            "must not report the weight total as a count"
        );
        assert!(
            s.the_score.contains("1 thing it could not measure today"),
            "a declared weight-0 channel with no reading today must be acknowledged as a gap for \
TODAY: {}",
            s.the_score
        );
        assert!(
            !s.the_score.contains("cannot measure at all"),
            "nothing in the score sentence is described as unmeasurable in principle: {}",
            s.the_score
        );
    }

    #[test]
    fn a_measured_weight_zero_channel_is_not_called_unmeasurable() {
        // THE FALSE CLAIM THIS REMOVES. Weight 0 means DECLARED AND UNSCORED, not unmeasurable:
        // the channel is measured, rendered on the weight-0 card and listed in the report, so
        // telling the reader the report "cannot measure at all" something it just displayed is
        // the summary contradicting itself. Measured 2026-10-17 on commit d5f0f62 (offline, same
        // cache): 5 weight-0 channels, 3 of them WITH a reading (leveraged_builder_stress,
        // credit_tail 9.55 @2026-10-15, cp_funding -0.11 @2026-10-05), and the sentence read
        // "plus 5 things it cannot measure at all" — wrong for three of the five.
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                // Weight 0 WITH a reading: measured and rendered, never scored.
                ind("credit_tail", 0.0, Some(50.0)),
                // Weight 0 WITHOUT a reading: a genuine gap for this run.
                ind("foreign_interest", 0.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.the_score.contains("1 thing it could not measure today"),
            "only the weight-0 channel that produced no reading is counted, and as a TODAY gap: {}",
            s.the_score
        );
        assert!(
            !s.the_score.contains("2 things it could not measure today"),
            "a weight-0 channel that produced a reading is MEASURED, not unmeasured: {}",
            s.the_score
        );
    }

    #[test]
    fn the_blind_spot_list_can_never_be_filled_by_todays_failures() {
        // THE DEFECT THIS CARD FIXES. The section was `unavailable ∩ weight == 0` — a channel that
        // happened to lose its source that morning — published as a PERMANENT hole in the model
        // ("they would still be missing on a day when every source answered"). Measured
        // 2026-10-16 (`out/ai-bubble-watch-2026-10-16.json`): the only entry was
        // `buildout_opposition_news`, weight 0, one lost GDELT day, while on 2026-10-11..15 the
        // very same section said the model had no holes at all. The list is now a DECLARATION, so
        // this property holds: every weight-0 channel in the configuration failing at once still
        // adds nothing to it.
        let silent = [
            "buildout_opposition_news",
            "public_attention",
            "leveraged_builder_stress",
            "credit_tail",
            "cp_funding",
        ];
        let mut inds = vec![ind("volatility", 8.0, Some(20.0))];
        for id in silent {
            inds.push(ind(id, 0.0, None));
        }
        let s = summarize(&report(inds, 32.4, None));
        for id in silent {
            assert!(
                !s.known_blind_spots.contains(plain_name(id)),
                "{} went quiet today, which is data quality, not a permanent blind spot: {}",
                id,
                s.known_blind_spots
            );
        }
        assert!(
            !s.known_blind_spots.contains("permanent"),
            "the section must not claim permanence for anything the run produced: {}",
            s.known_blind_spots
        );
        for entry in DECLARED_BLIND_SPOTS {
            assert!(
                s.known_blind_spots.contains(*entry),
                "a declared blind spot must still be rendered: {} missing from {}",
                entry,
                s.known_blind_spots
            );
        }
    }

    #[test]
    fn the_declared_blind_spots_are_rendered_in_both_grammatical_numbers() {
        // The section must survive its own list changing length: the old one-channel branch was
        // where the double space lived, and a list that renders "are things ... : a company"
        // for a single entry is the same class of defect. PURE, so both branches are reachable.
        //
        // THE PLURAL BRANCH IS CHECKED AT N=3, NOT ONLY N=2, because that is what the shipped
        // constant renders since the QUANTITY-of-AI-credit entry was declared (2026-10-17): the
        // three-entry join goes through `join_list`'s many-branch, whose ", and " separator is a
        // different arm from the two-entry join's " and ". Testing only the pair would leave the
        // shipped rendering path uncovered.
        let one = blind_spot_text(&["a thing that cannot be seen"]);
        assert!(
            one.contains(
                "There is one thing this report cannot see at all: a thing that cannot be seen."
            ),
            "one entry must read in the singular: {}",
            one
        );
        let two = blind_spot_text(&["x", "y"]);
        assert!(
            two.contains("There are things this report cannot see at all: x and y."),
            "two entries must be joined into a list: {}",
            two
        );
        let three = blind_spot_text(&["x", "y", "z"]);
        assert!(
            three.contains("There are things this report cannot see at all: x, y, and z."),
            "three entries must be joined with the many-branch separator: {}",
            three
        );
        // And the list the report actually ships is rendered by this branch, entry by entry.
        assert_eq!(
            DECLARED_BLIND_SPOTS.len(),
            3,
            "the shipped declaration changed length; the N=3 case above is then no longer the one \
the report renders, and this test's third case must be re-pointed before it stops covering it"
        );
    }

    #[test]
    fn the_blind_spot_text_bounds_the_claim_when_nothing_is_declared() {
        // A model that declares no hole must say what it does NOT claim, rather than stay silent
        // (an empty section reads as "nothing to see here") or reassure the reader.
        assert_eq!(
            blind_spot_text(&[]),
            "This report measures a fixed list of things and claims nothing beyond it."
        );
    }

    #[test]
    fn no_part_of_the_summary_contains_a_double_space() {
        // THE RENDER WART. The removed sentence was a `format!` whose SECOND argument was an empty
        // string in BOTH branches, so a one-entry list rendered
        // "There is one more thing  it also cannot measure at all" — visible in the recorded
        // 2026-10-16 report. An empty argument is invisible in review; this catches it. The blocks
        // are joined with a newline rather than a space so the join cannot manufacture the very
        // defect under test.
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("credit_tail", 0.0, Some(50.0)),
                ind("buildout_opposition_news", 0.0, None),
                ind("capex_vs_cashflow", 14.0, None),
            ],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        let all = [
            &s.what_this_is,
            &s.the_score,
            &s.what_is_stretched,
            &s.what_is_calm,
            &s.what_we_cannot_measure,
            &s.about_timing,
            &s.direction_of_travel,
            &s.the_questions_disagree,
            &s.known_blind_spots,
            &s.bottom_line,
        ]
        .iter()
        .map(|b| b.as_str())
        .collect::<Vec<_>>()
        .join("\n");
        assert!(
            !all.contains("  "),
            "a double space reached the plain-English summary: {:?}",
            all
        );
    }

    #[test]
    fn avoids_finance_jargon() {
        // The whole point of this module. If a term from this list appears, the
        // summary has failed its one job.
        let r = report(
            vec![
                ind("credit_hy", 12.0, Some(16.0)),
                ind("capex_vs_cashflow", 14.0, Some(42.0)),
                ind("funding_gap", 8.0, Some(68.0)),
            ],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        let all = format!(
            "{} {} {} {} {} {} {} {} {}",
            s.what_this_is,
            s.the_score,
            s.what_is_stretched,
            s.what_is_calm,
            s.what_we_cannot_measure,
            s.about_timing,
            s.direction_of_travel,
            s.known_blind_spots,
            s.bottom_line
        )
        .to_lowercase();
        for term in [
            "composite",
            "log-linear",
            "option-adjusted",
            "renormaliz",
            "oas",
            "percentile",
            "basis point",
            "z-score",
            "hyperscaler",
            "capex",
            "spread",
            "correlation",
            "volatility",
            "drawdown",
        ] {
            assert!(
                !all.contains(term),
                "jargon leaked into the summary: {}",
                term
            );
        }
    }

    #[test]
    fn never_claims_to_predict_when() {
        let r = report(
            vec![ind("volatility", 8.0, Some(20.0))],
            32.4,
            Some([30.0, 72.0]),
        );
        let s = summarize(&r);
        assert!(
            s.about_timing.contains("cannot tell you when"),
            "must state plainly that it cannot time the market"
        );
        assert!(
            s.about_timing.contains("not a forecast"),
            "must state plainly that the comparison is not a forecast"
        );
    }

    #[test]
    fn reports_gaps_rather_than_hiding_them() {
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("credit_hy", 12.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.what_we_cannot_measure.contains("could not check"),
            "a missing measurement must be admitted: {}",
            s.what_we_cannot_measure
        );
    }

    #[test]
    fn says_so_when_nothing_is_stretched() {
        let r = report(vec![ind("volatility", 8.0, Some(10.0))], 10.0, None);
        let s = summarize(&r);
        assert!(s.what_is_stretched.contains("Nothing on the list"));
        assert!(s.bottom_line.contains("not the same as being safe"));
    }

    #[test]
    fn names_the_most_stretched_measurement() {
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("funding_gap", 8.0, Some(68.0)),
            ],
            40.0,
            None,
        );
        let s = summarize(&r);
        assert!(
            s.what_is_stretched
                .contains("spend compared with what they sell"),
            "the biggest strain should be named: {}",
            s.what_is_stretched
        );
    }

    #[test]
    fn every_configured_indicator_has_a_plain_name() {
        // Regression guard. A new indicator that falls through to "one of the
        // measurements" produces a summary sentence that names nothing, which is
        // useless to the non-specialist this module exists for. Caught exactly
        // that when backlog_quality shipped.
        let path = std::path::Path::new("config/indicators.toml");
        if !path.exists() {
            return;
        }
        let text = std::fs::read_to_string(path).unwrap();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("id = \"") {
                if let Some(id) = rest.strip_suffix('\"') {
                    assert_ne!(
                        plain_name(id),
                        "one of the measurements",
                        "indicator '{}' has no plain-English name; add it to plain_name()",
                        id
                    );
                }
            }
        }
    }

    #[test]
    fn ignores_weight_zero_declared_gaps() {
        // A weight-0 channel must never be scored, and must not appear in the weighted-gaps
        // sentence. It is also NOT a permanent blind spot, whatever it does on a given run:
        // see DECLARED_BLIND_SPOTS, which is the only source for that list now.
        let r = report(
            vec![
                ind("volatility", 8.0, Some(20.0)),
                ind("foreign_interest", 0.0, None),
            ],
            32.4,
            None,
        );
        let s = summarize(&r);
        assert!(
            !s.what_we_cannot_measure.contains("foreign"),
            "a declared weight-0 channel's silence is not a weighted gap: {}",
            s.what_we_cannot_measure
        );
    }

    #[test]
    fn is_deterministic() {
        let mk = || {
            report(
                vec![
                    ind("a", 10.0, Some(50.0)),
                    ind("b", 10.0, Some(50.0)),
                    ind("c", 10.0, Some(20.0)),
                ],
                40.0,
                Some([30.0, 72.0]),
            )
        };
        assert_eq!(summarize(&mk()), summarize(&mk()));
    }

    // -----------------------------------------------------------------------
    // Cadence: a flat score must never be described as stability
    // -----------------------------------------------------------------------

    fn fresh(
        live_w: f64,
        frozen_w: f64,
        oldest: &str,
        age: i64,
        live_n: usize,
        frozen_n: usize,
    ) -> crate::history::Freshness {
        crate::history::Freshness {
            live_indicators: live_n,
            frozen_indicators: frozen_n,
            live_weight: live_w,
            frozen_weight: frozen_w,
            total_weight: live_w + frozen_w,
            window_days: 7,
            newest_change: Some("2026-10-15".into()),
            oldest_change: Some(oldest.into()),
            oldest_change_days: Some(age),
            frozen_ids: vec!["backlog_quality".into()],
        }
    }

    /// A FLAT delta, which is the branch that used to lie.
    fn flat_trend() -> crate::model::Trend {
        crate::model::Trend {
            points: vec![],
            delta: Some(crate::model::TrendDelta {
                baseline_date: "2026-10-15".into(),
                baseline_composite: 40.0,
                baseline_coverage: 1.0,
                composite_delta: 0.1,
                elapsed_days: 1.0,
                direction: crate::model::Direction::Flat,
                phase_now: "mid".into(),
                phase_then: "mid".into(),
                phase_changed: false,
                indicators: vec![],
            }),
            reason: None,
            warnings: vec![],
            recorded: false,
        }
    }

    #[test]
    fn a_flat_comparison_never_claims_the_numbers_are_stable() {
        // THE FALSE CLAIM THIS REMOVES. The flat branch read "the numbers are stable, not getting
        // better or worse". Measured 2026-10-16 that was false: 49% of the weight had not moved
        // since 2026-09-17, so the instrument was PINNED, and "stable" asserted something about
        // the world that the data did not support.
        let mut r = report(vec![ind("a", 10.0, Some(50.0))], 40.0, Some([30.0, 72.0]));
        r.trend = flat_trend();
        r.freshness = Some(fresh(93.0, 90.0, "2026-09-17", 29, 11, 12));

        let s = summarize(&r);
        assert!(
            !s.direction_of_travel.contains("the numbers are stable"),
            "a pinned instrument must never be described as stable: {}",
            s.direction_of_travel
        );
        assert!(
            s.direction_of_travel.contains("could have moved"),
            "the cadence note must accompany a flat reading: {}",
            s.direction_of_travel
        );
        assert!(
            s.direction_of_travel.contains("49%"),
            "the pinned SHARE must be stated: {}",
            s.direction_of_travel
        );
        assert!(
            s.direction_of_travel.contains("2026-09-17"),
            "the date the slow half last moved must be named: {}",
            s.direction_of_travel
        );
        assert!(
            s.direction_of_travel.contains("12 of 23"),
            "the COUNT of pinned inputs must be named, not only their weight: {}",
            s.direction_of_travel
        );
    }

    #[test]
    fn the_no_comparison_branch_still_states_the_cadence() {
        // When no trend can be computed the reader has even less to go on, so the cadence
        // note matters MORE here, not less.
        let mut r = report(vec![ind("a", 10.0, Some(50.0))], 40.0, Some([30.0, 72.0]));
        r.freshness = Some(fresh(93.0, 90.0, "2026-09-17", 29, 11, 12));

        let s = summarize(&r);
        assert!(
            s.direction_of_travel.contains("could have moved"),
            "the cadence note must appear when no comparison was made: {}",
            s.direction_of_travel
        );
        assert!(
            s.direction_of_travel.contains("49%"),
            "{}",
            s.direction_of_travel
        );
    }

    #[test]
    fn the_cadence_note_is_absent_rather_than_invented_without_freshness() {
        // No archive was read, so there is nothing to say about freshness. Silence is correct;
        // a guessed figure would be worse than no figure.
        let mut r = report(vec![ind("a", 10.0, Some(50.0))], 40.0, Some([30.0, 72.0]));
        r.trend = flat_trend();
        r.freshness = None;

        let s = summarize(&r);
        assert!(
            !s.direction_of_travel.contains("could have moved"),
            "no freshness must mean no cadence claim: {}",
            s.direction_of_travel
        );
        // AND NO DANGLING POINTER TO IT. An earlier version cross-referenced "the note below",
        // which sent the reader to a note that did not exist whenever freshness was unavailable.
        assert!(
            !s.direction_of_travel.contains("note below"),
            "the text must not point at a cadence note that is not there: {}",
            s.direction_of_travel
        );
    }

    #[test]
    fn a_fully_live_score_says_so_instead_of_implying_everything_is_pinned() {
        // The complement: with nothing frozen, the clause must not claim a pinned share. Otherwise
        // the fix would satisfy its own test by always crying staleness, which is its own failure.
        let mut r = report(vec![ind("a", 10.0, Some(50.0))], 40.0, Some([30.0, 72.0]));
        r.trend = flat_trend();
        r.freshness = Some(fresh(183.0, 0.0, "2026-10-15", 1, 24, 0));

        let s = summarize(&r);
        assert!(
            s.direction_of_travel.contains("all of it"),
            "a fully live score must say so: {}",
            s.direction_of_travel
        );
        assert!(
            !s.direction_of_travel.contains("has not moved since"),
            "nothing is pinned, so no pinned date may be claimed: {}",
            s.direction_of_travel
        );
    }
}
