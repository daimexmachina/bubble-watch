//! Credit indicators — the non-equity half of the picture.
//!
//! These depend on FRED, which is an OPTIONAL source: during probing it stopped
//! answering this host entirely. When it is down these two indicators are
//! reported as gaps and the composite rests on less. That is the correct
//! behaviour — but it means the composite becomes an *equity-price-heavy*
//! reading, and the report says so explicitly in its caveats.

use super::{Ctx, Indicator};
use crate::model::{Reading, Series};

fn credit_reading(ctx: &Ctx, id: &str, _label: &str, what: &str) -> Reading {
    let ic = match ctx.cfg.indicator(id) {
        Some(c) => c,
        None => {
            return Reading::Unavailable {
                reason: "not configured".into(),
            }
        }
    };
    let Some(series_id) = ic.fred_series.as_deref() else {
        return Reading::Unavailable {
            reason: format!("indicator '{}' has no fred_series configured", id),
        };
    };
    let Some(s) = ctx.obs.fred.get(series_id) else {
        let why = ctx
            .obs
            .failures
            .iter()
            .find(|f| f.source == "fred" && f.endpoint == series_id)
            .map(|f| f.reason.clone())
            .unwrap_or_else(|| "not retrieved".into());
        return Reading::Unavailable {
            reason: format!(
                "FRED series {} unavailable ({}) — OPTIONAL source, treated as a coverage gap",
                series_id, why
            ),
        };
    };
    let Some(level) = s.latest_value() else {
        return Reading::Unavailable {
            reason: format!("FRED series {} returned no observations", series_id),
        };
    };
    let stress = crate::score::interpolate(level, &ic.anchors);

    // Series length and span, so the reader can see whether the scale is anchored
    // on observed history or on extrapolation. This is the concrete reason the
    // investment-grade indicator moved to a 40-year series.
    let n = s.points.len();
    let span = match (s.points.first(), s.points.last()) {
        (Some(a), Some(b)) => format!("{} to {}", a.date, b.date),
        _ => "unknown span".to_string(),
    };
    let history_note = if n < 200 {
        format!(
            " HISTORY WARNING: this series has only {} observations ({}), which spans too \
             little time to contain a full stress episode, so the upper part of the scale is \
             extrapolated rather than observed.",
            n, span
        )
    } else {
        format!(" History: {} observations spanning {}.", n, span)
    };

    // If another part of the report reads the same series, say so. Counting one
    // series twice as though it were two pieces of evidence is the exact
    // double-counting error already fixed once in this model.
    let shared = if id == "credit_ig" && SHARES_SERIES_WITH_FALSIFIER {
        format!(
            " NOT INDEPENDENT: the falsification test for credit conditions reads the same {} \
             series, so this indicator and that test are the same measurement read two ways \
             (a level here, a historical percentile there). Do not treat them as two pieces \
             of evidence.",
            series_id
        )
    } else {
        String::new()
    };

    Reading::Scored {
        stress,
        value: level,
        unit: ic.unit.clone(),
        detail: format!(
            "{} {} as of {}. Direction: WIDE spread = high stress, tight = low stress \
             (not inverted). A tight spread means credit is currently cheap and the market \
             is charging little for this risk; it is NOT scored as a bubble signal, because \
             HY spreads have a ~3.0% median over the available record and a level-based \
             complacency score would read nearly the whole post-2009 period as a bubble. \
             The informative signal is the DIRECTION OF TRAVEL from a tight base — widening \
             is the early warning. Watch the change, not the level.{}{}",
            what, level, s.provenance.as_of, history_note, shared
        ),
        provenance: s.provenance.clone(),
    }
}

pub struct CreditHy;
impl Indicator for CreditHy {
    fn id(&self) -> &'static str {
        "credit_hy"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        credit_reading(
            ctx,
            self.id(),
            "US high-yield credit spreads",
            "ICE BofA US High Yield option-adjusted spread",
        )
    }
}

/// Investment-grade credit conditions, measured on a series with 40 years of
/// history rather than two and a half.
///
/// WHY THE SERIES CHANGED. This indicator previously read the ICE BofA US Corporate
/// option-adjusted spread, which is the *better* measure of investment-grade
/// conditions — but every ICE BofA series on FRED begins 2023-09-18. Verified from
/// this host 2026-09-17: BAMLH0A0HYM2, BAMLC0A0CM, BAMLH0A1HYBB and the rest all
/// start on that date, and no free pre-2023 high-yield or IG option-adjusted spread
/// exists.
///
/// The consequence was not cosmetic. With only ~2.5 years of history the anchors
/// could not be anchored on any observed stress episode: the series had never seen
/// 2000, 2008 or 2020, so the top of its scale was an extrapolation rather than a
/// reference. Baa-minus-10Y carries 10,177 observations from 1986 and includes all
/// three episodes, so the same scale can now be justified by history.
///
/// WHAT IT MEASURES NOW: the yield on Baa-rated corporate debt minus the 10-year
/// Treasury — a credit-risk premium for the lower half of investment grade.
///
/// DIRECTION is unchanged: wide = high stress, tight = low stress. The config
/// already documents why the complacency reading (tight spreads as the bubble
/// signal) is deliberately not used: HY OAS has a ~3% median over its record and is
/// rarely wide, so a level-based complacency score would read most of the last
/// decade as a bubble — near-constant, and carrying no timing information.
///
/// ONE HONEST CONSEQUENCE, disclosed rather than left for a reader to notice: the
/// falsification test for credit conditions also uses BAA10Y, so this indicator and
/// that test are NOT independent measurements. They are the same series read two
/// ways — a level-scored indicator and a historical-percentile falsifier. The output
/// says so, because counting one series twice as though it were two pieces of
/// evidence is exactly the double-counting error already fixed once in this model.
pub struct CreditIg;
impl Indicator for CreditIg {
    fn id(&self) -> &'static str {
        "credit_ig"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        credit_reading(
            ctx,
            self.id(),
            "US investment-grade credit spreads",
            "Moody's Baa corporate yield less the 10-year Treasury",
        )
    }
}

/// Weight-0 series must carry NO anchors, and that is enforced rather than documented.
///
/// A scale attached to an unscored series invites exactly one later move — raise the weight and
/// take the score — without ever running the pre-registered gate the project requires. The
/// measured-but-unscored channels each refuse that configuration loudly instead of rendering a
/// number nobody agreed to trust. See `funding_stress`'s siblings in `config/indicators.toml`.
fn assert_unscored(ic: &crate::config::IndicatorCfg) {
    assert!(
        ic.anchors.is_empty(),
        "{} is weight {} and must carry NO anchors, but the config supplies {}. Do not attach a \
         scale to a weight-0 series.",
        ic.id,
        ic.weight,
        ic.anchors.len()
    );
}

/// Resolve both legs of a configured FRED spread, naming whichever is missing.
///
/// A spread has two halves and the failure mode is that ONE of them breaks: a reader told only
/// that the indicator is "unavailable" cannot tell which. The reason therefore carries the series
/// id and the transport's own message for it.
fn spread_legs<'a>(
    ctx: &'a Ctx,
    ic: &'a crate::config::IndicatorCfg,
) -> Result<(&'a Series, &'a Series, &'a str, &'a str), Reading> {
    let (Some(a_id), Some(b_id)) = (ic.fred_series.as_deref(), ic.fred_series_b.as_deref()) else {
        return Err(Reading::Unavailable {
            reason: format!(
                "indicator '{}' is a spread and needs both fred_series and fred_series_b",
                ic.id
            ),
        });
    };
    let missing = |sid: &str| {
        let why = ctx
            .obs
            .failures
            .iter()
            .find(|f| f.source == "fred" && f.endpoint == sid)
            .map(|f| f.reason.clone())
            .unwrap_or_else(|| "not retrieved".into());
        Reading::Unavailable {
            reason: format!(
                "FRED series {} unavailable ({}) — OPTIONAL source, treated as a coverage gap",
                sid, why
            ),
        }
    };
    let Some(a) = ctx.obs.fred.get(a_id) else {
        return Err(missing(a_id));
    };
    let Some(b) = ctx.obs.fred.get(b_id) else {
        return Err(missing(b_id));
    };
    Ok((a, b, a_id, b_id))
}

/// Pair two FRED legs on COMMON dates and return `fred_series` MINUS `fred_series_b`, oldest
/// first, newest last.
///
/// WHY PAIRING IS NOT OPTIONAL. The two legs have different holiday calendars and different
/// publication lags, so reading each leg's own latest value would silently subtract two different
/// days — and on a funding-stress series a one-day mismatch is a large fraction of the signal.
/// The single implementation here is deliberate: `funding_stress`, `credit_tail` and `cp_funding`
/// must not each carry their own copy of this rule, because two copies are how one spread acquires
/// two subtly different definitions.
fn paired_spread<'a>(a: &'a Series, b: &'a Series) -> Vec<(&'a str, f64)> {
    let b_by_date: std::collections::BTreeMap<&str, f64> = b
        .points
        .iter()
        .map(|p| (p.date.as_str(), p.value))
        .collect();
    a.points
        .iter()
        .filter_map(|p| {
            b_by_date
                .get(p.date.as_str())
                .map(|bv| (p.date.as_str(), p.value - *bv))
        })
        .collect()
}

/// Observed distribution of a paired series, so a weight-0 channel can report its own measured
/// range on every run instead of quoting a figure that was true once.
struct Dist {
    n: usize,
    p1: f64,
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    max_date: String,
}

fn dist(paired: &[(&str, f64)]) -> Dist {
    let mut vals: Vec<f64> = paired.iter().map(|(_, v)| *v).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| {
        let i = (vals.len() as f64 - 1.0) * q;
        let lo = i.floor() as usize;
        let hi = (lo + 1).min(vals.len() - 1);
        vals[lo] + (vals[hi] - vals[lo]) * (i - lo as f64)
    };
    let (max_date, max) = paired
        .iter()
        .fold(("".to_string(), f64::MIN), |acc, (d, v)| {
            if *v > acc.1 {
                (d.to_string(), *v)
            } else {
                acc
            }
        });
    Dist {
        n: vals.len(),
        p1: at(0.01),
        p50: at(0.50),
        p95: at(0.95),
        p99: at(0.99),
        max,
        max_date,
    }
}

/// How often the paired series actually publishes, over the trailing 365 days.
///
/// This is measured at run time rather than asserted, because the whole "is this a fast channel"
/// question is a cadence question and a hard-coded claim about cadence would be the exact
/// unmeasured assertion this project forbids. Returns `(count, median_gap_days, max_gap_days)`;
/// `(0, 0, 0)` if the dates will not parse.
fn trailing_year_cadence(paired: &[(&str, f64)]) -> (usize, i64, i64) {
    let parse = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    let Some(last) = paired.last().and_then(|(d, _)| parse(d)) else {
        return (0, 0, 0);
    };
    let cutoff = last - chrono::Duration::days(365);
    let recent: Vec<chrono::NaiveDate> = paired
        .iter()
        .filter_map(|(d, _)| parse(d))
        .filter(|d| *d >= cutoff)
        .collect();
    let mut gaps: Vec<i64> = recent
        .windows(2)
        .map(|w| (w[1] - w[0]).num_days())
        .collect();
    gaps.sort_unstable();
    let median = gaps.get(gaps.len() / 2).copied().unwrap_or(0);
    let max = gaps.last().copied().unwrap_or(0);
    (recent.len(), median, max)
}

/// Money-market funding stress: the secured overnight rate less the administered rate.
///
/// WHY THIS EXISTS. The model had no money-market indicator at all. Credit spreads measure the
/// PRICE of risk; they do not measure whether the machinery that rolls the debt is still working.
/// In a funding event the money market is where it shows up first and most violently, and nothing
/// here could see it. Measured before building: over 2,132 paired observations this spread's
/// observed maximum is +2.950 on 2019-09-17, the September 2019 repo-market seizure — so unlike the
/// ICE credit indices the scale is anchored on a REAL funding crisis rather than an extrapolation.
///
/// WHAT IT MEASURES. `fred_series` minus `fred_series_b`, i.e. SOFR less EFFR: the rate paid to
/// borrow cash overnight against Treasury collateral, less the rate paid on reserves. The spread is
/// normally slightly NEGATIVE, because reserves are abundant and secured borrowing is cheap
/// relative to the administered rate. Positive readings mean secured funding is being bid away from
/// the administered rate, which is what a reserve squeeze looks like from the outside.
///
/// DIRECTION: wide/positive = high stress. Not inverted — a positive spread is pressure.
///
/// WINDOW LIMITATION, DECLARED RATHER THAN IMPLIED AWAY: SOFR begins 2018-04-03, so this scale
/// contains no 2000, 2008 or 2020 point. It is anchored on the observed distribution since 2018
/// plus the September 2019 seizure. That is a real event but a single one, and a reader should not
/// read the top of this scale as a full-cycle calibration.
///
/// NOT A LEADING INDICATOR, and no claim is made that it is. An earlier probe in this project tested
/// whether attention leads repricing and found all pairs peaking at lag 0, and the tone series is
/// reported but deliberately unscored for that reason. This indicator claims only to measure a
/// channel that was previously invisible.
pub struct FundingStress;

impl Indicator for FundingStress {
    fn id(&self) -> &'static str {
        "funding_stress"
    }

    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let id = self.id();
        let Some(ic) = ctx.cfg.indicator(id) else {
            return Reading::Unavailable {
                reason: "not configured".into(),
            };
        };
        // Pair on COMMON dates (see `paired_spread`): the two legs have different holiday
        // calendars and publication lags, and reading each leg's own latest value would subtract
        // two different days — on a funding-stress series a large fraction of the signal.
        let (a, b, a_id, b_id) = match spread_legs(ctx, ic) {
            Ok(v) => v,
            Err(r) => return r,
        };
        let paired = paired_spread(a, b);

        let Some((date, spread)) = paired.last().copied() else {
            return Reading::Unavailable {
                reason: format!(
                    "FRED series {} and {} share no observation date, so the spread is undefined \
                     rather than zero",
                    a_id, b_id
                ),
            };
        };
        let oldest = paired.first().map(|(d, _)| *d).unwrap_or(date);
        let n = paired.len();

        let stress = crate::score::interpolate(spread, &ic.anchors);

        // The window is short by construction (SOFR begins 2018), so state it every time rather
        // than only when it is inconvenient.
        let window_note = format!(
            " Window: {} paired observations, {} to {}. This spread is defined only where both \
             legs publish, and it contains no 2000, 2008 or 2020 point — the top of the scale rests \
             on the single September 2019 episode, so read it as pressure relative to the last few \
             years rather than against a full cycle.",
            n, oldest, date
        );

        // Neither leg is read by any other indicator, and that is worth stating rather than
        // leaving implicit: the credit indicators read BAMLH0A0HYM2 and BAA10Y, not these. If that
        // ever changes, this indicator must say so — an earlier draft carried a `shared` block that
        // was provably dead code because the list it consulted was empty, so it was removed rather
        // than kept as decoration.
        Reading::Scored {
            stress,
            value: spread,
            unit: ic.unit.clone(),
            detail: format!(
                "{} minus {} = {:.2} as of {}. The secured overnight rate less the administered \
                 rate: positive means secured funding is being bid away from the administered rate, \
                 which is pressure; negative is the normal state when reserves are abundant.{}{}",
                a_id,
                b_id,
                spread,
                date,
                window_note,
                history_caveat(n)
            ),
            // Provenance names BOTH legs and dates the spread at the paired date, not at either
            // leg's own latest observation — labelling it with one leg's `as_of` would misdate the
            // measurement by however far the two calendars disagree.
            provenance: crate::model::Provenance {
                source: "fred".into(),
                endpoint: format!("{} - {}", a_id, b_id),
                as_of: date.to_string(),
                retrieved_at: a
                    .provenance
                    .retrieved_at
                    .clone()
                    .max(b.provenance.retrieved_at.clone()),
            },
        }
    }
}

/// A one-line caveat when the paired window is too short to have seen a stressed market.
fn history_caveat(n: usize) -> String {
    if n < 500 {
        format!(
            " HISTORY WARNING: only {} paired observations, too few to contain a full funding \
             cycle, so the upper part of this scale is extrapolated from a single episode rather \
             than observed across several.",
            n
        )
    } else {
        String::new()
    }
}

/// "one day before" / "N days before", so a generated sentence never prints a bare number where
/// a phrase belongs. Used by `credit_tail`, whose maximum does NOT always coincide with the
/// current reading.
fn days_before_phrase(days: i64) -> String {
    match days {
        0 => "on the same date as".to_string(),
        1 => "one day before".to_string(),
        n if n > 1 => format!("{} days before", n),
        // A future date cannot occur (the maximum is taken from the window that ends at the
        // current reading), but printing the truth beats printing a placeholder if one ever does.
        n => format!("{} day(s) after", n.abs()),
    }
}

/// Credit TAIL dispersion — the weakest borrowers against the broad high-yield market.
///
/// WHY THIS EXISTS. A crash is a TAIL event: a high-yield index can widen modestly while the
/// weakest credits inside it blow out. The difference between the two is the piece an index level
/// cannot show. This measures CCC-and-lower option-adjusted spread less the broad high-yield
/// option-adjusted spread — `BAMLH0A3HYC` minus `BAMLH0A0HYM2`, both ICE BofA.
///
/// WEIGHT 0, DELIBERATELY UNSCORED. The channel is measured, rendered and auditable, and it is
/// arithmetically incapable of moving the composite or the coverage fraction. Anchors set what a
/// score MEANS and adding weight orphans cross-version comparison, so both are the owner's
/// decision and neither is taken here: the measured ladder is proposed in the config, the weight
/// stays 0.0, and `assert_unscored` refuses a config that attaches a scale anyway.
///
/// THE LIMITATION THAT MATTERS MOST IS THE WINDOW. The ICE BofA indices are licence-windowed
/// server-side by FRED to roughly three years (measured 2026-10-17: both legs return 786
/// observations from 2023-10-17), so this series has never seen 2008 or 2020 and cannot be
/// anchored to either. That is stated in the output on every run rather than implied away.
pub struct CreditTail;

impl Indicator for CreditTail {
    fn id(&self) -> &'static str {
        "credit_tail"
    }

    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let id = self.id();
        let Some(ic) = ctx.cfg.indicator(id) else {
            return Reading::Unavailable {
                reason: "not configured".into(),
            };
        };
        // Weight 0 or a scale, never both. See `assert_unscored`.
        assert_unscored(ic);
        let (a, b, a_id, b_id) = match spread_legs(ctx, ic) {
            Ok(v) => v,
            Err(r) => return r,
        };
        let paired = paired_spread(a, b);

        let Some((date, spread)) = paired.last().copied() else {
            return Reading::Unavailable {
                reason: format!(
                    "FRED series {} and {} share no observation date, so the dispersion is \
                     undefined rather than zero",
                    a_id, b_id
                ),
            };
        };
        let oldest = paired.first().map(|(d, _)| *d).unwrap_or(date);
        let d = dist(&paired);

        // THE MAXIMUM IS NOT ALWAYS THE CURRENT READING. Saying it was is a MEASURED falsehood:
        // on 2026-10-17 the repo's own cache put the window maximum at 9.57 on 2026-10-14 while
        // the latest PAIRED reading was 9.55 on 2026-10-15, and the sentence printed both dates
        // while calling the maximum "the CURRENT reading". The intent — the top of the range is a
        // very recent observation, not a historical extreme — is kept; the premise now states
        // where the maximum actually sits.
        let max_clause = if d.max_date == date {
            "NOTE that the maximum IS the current reading, so the top of the observed range is \
             not yet a historical extreme."
                .to_string()
        } else {
            format!(
                "NOTE that the maximum is dated {}, {} the current reading on {}, so the top of \
                 the observed range is not yet a historical extreme.",
                d.max_date,
                days_before_phrase(crate::sources::edgar::days_between(&d.max_date, date)),
                date
            )
        };

        Reading::Scored {
            // WEIGHT 0: nothing is scored. The value is measured and rendered; the stress field is
            // 0.0 because no anchor has been agreed, and `assert_unscored` refuses a config that
            // supplies one.
            stress: 0.0,
            value: spread,
            unit: ic.unit.clone(),
            detail: format!(
                "{} minus {} = {:.2} as of {} — the weakest borrowers versus the broad high-yield \
                 market. A crash is a TAIL event: the index can widen modestly while the weakest \
                 credits blow out, and this gap is what an index level cannot show. DIRECTION: \
                 wider = more dispersion = more stress. WINDOW LIMITATION, DECLARED: the ICE BofA \
                 indices are licence-windowed SERVER-SIDE BY FRED — a licensing restriction, not \
                 this tool's transport — to roughly three years, so this window is {} to {}, {} \
                 observations, and it has never seen 2008 or 2020. The range below therefore rests \
                 on recent episodes only and cannot be anchored to either crisis. MEASURED over \
                 that window: p1 {:.2}, median {:.2}, p95 {:.2}, p99 {:.2}, maximum {:.2} on {}. \
                 {} WEIGHT 0 BY DESIGN: this cannot move the composite \
                 or the coverage fraction, and no anchor ladder is applied — the measured ladder is \
                 PROPOSED in config/indicators.toml and setting it is the owner's decision. NOT A \
                 LEADING INDICATOR and no predictive claim is made: it makes a tail visible, it \
                 does not forecast a turn.",
                a_id, b_id, spread, date, oldest, date, d.n, d.p1, d.p50, d.p95, d.p99, d.max, d.max_date,
                max_clause
            ),
            provenance: crate::model::Provenance {
                source: "fred".into(),
                endpoint: format!("{} - {}", a_id, b_id),
                as_of: date.to_string(),
                retrieved_at: a
                    .provenance
                    .retrieved_at
                    .clone()
                    .max(b.provenance.retrieved_at.clone()),
            },
        }
    }
}

/// Commercial-paper funding: the paper rate less a Treasury bill.
///
/// WHY THIS EXISTS. `funding_stress` measures the secured OVERNIGHT market; this measures the
/// term money market where non-bank issuers actually roll paper, and it carries the deepest
/// history of any funding candidate — which is the whole reason to build it.
///
/// WEIGHT 0, DELIBERATELY UNSCORED, for the same reason as `credit_tail`: anchors and weight are
/// the owner's decision, the measured ladder is proposed in the config, and `assert_unscored`
/// refuses a config that attaches a scale to a weight-0 series.
///
/// THE LEGS, AND WHY THEY WERE CHOSEN FROM THE DATA. `DCPF1M` (30-day AA FINANCIAL commercial
/// paper) against `DTB3` (3-month Treasury bill, secondary market, discount basis). The T-bill
/// leg was chosen from measurement: `DTB3` carries history from 1954, so it never truncates the
/// paper leg — the paired series spans the paper leg's own full window — and it is the
/// conventional benchmark for the commercial-paper spread. The alternative measured, and the
/// reason it was not taken, is recorded in the config.
///
/// THE CADENCE IS THE LIMITATION, AND IT IS MEASURED AT RUN TIME rather than asserted. The paper
/// leg is sparse: it is NOT a daily series, its newest common date is set by that leg, and the
/// channel is therefore the best-anchored and the worst as a FAST one. It must never be described
/// as a fast channel, and the output says so on every run with the trailing-year numbers to back
/// it.
pub struct CpFunding;

impl Indicator for CpFunding {
    fn id(&self) -> &'static str {
        "cp_funding"
    }

    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let id = self.id();
        let Some(ic) = ctx.cfg.indicator(id) else {
            return Reading::Unavailable {
                reason: "not configured".into(),
            };
        };
        // Weight 0 or a scale, never both. See `assert_unscored`.
        assert_unscored(ic);
        let (a, b, a_id, b_id) = match spread_legs(ctx, ic) {
            Ok(v) => v,
            Err(r) => return r,
        };
        let paired = paired_spread(a, b);

        let Some((date, spread)) = paired.last().copied() else {
            return Reading::Unavailable {
                reason: format!(
                    "FRED series {} and {} share no observation date, so the funding spread is \
                     undefined rather than zero",
                    a_id, b_id
                ),
            };
        };
        let oldest = paired.first().map(|(d, _)| *d).unwrap_or(date);
        let d = dist(&paired);
        let (year_n, median_gap, max_gap) = trailing_year_cadence(&paired);
        let a_last = a.points.last().map(|p| p.date.as_str()).unwrap_or("");
        let b_last = b.points.last().map(|p| p.date.as_str()).unwrap_or("");

        Reading::Scored {
            // WEIGHT 0: measured and rendered, never scored. See `credit_tail` for the same rule.
            stress: 0.0,
            value: spread,
            unit: ic.unit.clone(),
            detail: format!(
                "{} minus {} = {:+.2} as of {} — commercial paper less a Treasury bill, i.e. what \
                 the money market charges to roll paper against the short risk-free rate. DIRECTION: \
                 wider = higher stress; the spread sits just above zero in normal times and blows \
                 out when paper cannot be rolled. CADENCE LIMITATION, DECLARED AND MEASURED AT RUN \
                 TIME: this is the BEST-ANCHORED funding series in the model and the WORST as a \
                 FAST one. {} is not a daily series — over the trailing year it published {} \
                 values, median gap {} days, longest gap {} days — and the newest common date ({}) \
                 is set by that leg, whose last observation is {} while the Treasury leg runs to \
                 {}. It is NOT a fast channel and must NOT be read as a daily funding gauge; the \
                 measured cadence above is the reason. WHAT IT IS GOOD FOR IS THE WINDOW: {} to {}, \
                 {} paired observations spanning the 2008 financial crisis and the March 2020 COVID \
                 spike, which no other funding series here can reach. MEASURED over that window: p1 \
                 {:+.2}, median {:+.2}, p95 {:+.2}, p99 {:+.2}, maximum {:+.2} on {}. WEIGHT 0 BY \
                 DESIGN: this cannot move the composite or the coverage fraction, and no anchor \
                 ladder is applied — the measured ladder is PROPOSED in config/indicators.toml and \
                 setting it is the owner's decision. NOT A LEADING INDICATOR and no predictive \
                 claim is made.",
                a_id,
                b_id,
                spread,
                date,
                a_id,
                year_n,
                median_gap,
                max_gap,
                date,
                a_last,
                b_last,
                oldest,
                date,
                d.n,
                d.p1,
                d.p50,
                d.p95,
                d.p99,
                d.max,
                d.max_date
            ),
            provenance: crate::model::Provenance {
                source: "fred".into(),
                endpoint: format!("{} - {}", a_id, b_id),
                as_of: date.to_string(),
                retrieved_at: a
                    .provenance
                    .retrieved_at
                    .clone()
                    .max(b.provenance.retrieved_at.clone()),
            },
        }
    }
}

/// True when the same series backs another part of the report, so the output can
/// say that the two are not independent. Kept as a named function rather than a
/// comment so the statement lives in the code that renders it.
pub const SHARES_SERIES_WITH_FALSIFIER: bool = true;

/// Private-credit lending growth: the financing channel the public spreads miss.
///
/// WHY THIS INDICATOR EXISTS. `credit_hy` and `credit_ig` read PUBLIC bond index
/// spreads. BIS Bulletin 120 (Jan 2026) documents that AI-related financing is
/// shifting from operating cash flow toward debt with **private credit playing a
/// rapidly increasing role** — and private credit appears in no index spread,
/// because it is not traded. This measures that channel's size and growth.
///
/// Measured from this host 2026-09-17, from the Fed's own F4.4 table:
///   all sectors' private-credit loans   $1.1tn (2024:Q1) -> $1.5tn (2026:Q2),
///   roughly +36% in ten quarters, while the public high-yield index spread has
///   barely moved.
///
/// THE LIMITATION IS SEVERE AND IS STATED IN THE OUTPUT, not only here. Z.1
/// reports private credit as a TOTAL for the entire economy. It is NOT AI-specific
/// and nothing in the free data attributes a private loan to a data-centre or GPU
/// borrower. So this measures the size and growth of the CHANNEL, not the amount
/// reaching AI. A high reading is consistent with the BIS thesis and does not
/// confirm it. It is included because a growing share of this cycle's debt is
/// being originated where the tool was not looking, and silently ignoring that
/// would be worse than measuring it with a caveat.
///
/// SCORED ON GROWTH, NOT LEVEL. The level of economy-wide private credit is not a
/// bubble signal at all; a large and stable stock is unremarkable. What matters is
/// rapid expansion, which is what "private credit is playing a rapidly increasing
/// role" actually asserts. Scoring the level would read every year as alarming.
pub struct PrivateCreditGrowth;

impl Indicator for PrivateCreditGrowth {
    fn id(&self) -> &'static str {
        "private_credit_growth"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };

        let Some(series) = ctx.obs.yahoo.get("PRIVATE_CREDIT_ALL") else {
            return Reading::Unavailable {
                reason: "Fed Z.1 private-credit series unavailable: the release archive could \
                         not be retrieved or the table was not parseable"
                    .into(),
            };
        };

        let Some(growth) = crate::sources::z1::yoy_growth(&series.points) else {
            return Reading::Unavailable {
                reason: format!(
                    "private-credit series has only {} observations, too few for a \
                     year-over-year comparison",
                    series.points.len()
                ),
            };
        };

        let last = series.points.last();
        let prior = series.points.get(series.points.len().saturating_sub(5));
        let level_note = match (last, prior) {
            (Some(l), Some(p)) => format!(
                "Level: ${:.2}tn at {}, against ${:.2}tn a year earlier.",
                l.value / 1e12,
                l.date,
                p.value / 1e12
            ),
            _ => String::new(),
        };

        let stress = crate::score::interpolate(growth, &ic.anchors);

        Reading::Scored {
            stress,
            value: growth,
            unit: ic.unit.clone(),
            detail: format!(
                "Private-credit lending grew {:.1}% year over year. {}. This is the channel \
                 BIS Bulletin 120 identifies as taking a rapidly increasing role in financing \
                 the buildout, and it appears in NEITHER of the public credit-spread indicators \
                 above because private credit is not traded and so has no index spread. \
                 CRITICAL LIMITATION, please do not over-read this number: Z.1 reports private \
                 credit as a TOTAL FOR THE WHOLE ECONOMY. It is not AI-specific, and nothing in \
                 the free data attributes an individual private loan to a data-centre or GPU \
                 borrower. This measures the size and growth of the financing CHANNEL, not the \
                 amount of it reaching AI. Growth in the channel is consistent with the BIS \
                 thesis and does not establish it. Scored on growth rather than level: the \
                 outstanding stock of private credit is not itself a bubble signal, whereas \
                 rapid expansion is what the BIS claim actually asserts.",
                growth, level_note
            ),
            provenance: series.provenance.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::model::{Observations, Point, Provenance, Series};

    /// A minimal config carrying just the two credit indicators, so these tests
    /// do not depend on the shipped file or on the current working directory.
    fn cfg() -> Config {
        Config {
            meta: Meta {
                schema_version: "1".into(),
            },
            phase: PhaseCfg {
                early_max: 35.0,
                mid_max: 55.0,
                late_max: 75.0,
            },
            coverage_floor: CoverageFloor {
                low_below: 0.6,
                high_above: 0.85,
            },
            trend: TrendCfg::defaults(),
            gsadf: GsadfCfg::default(),
            analog: AnalogCfg {
                method: "historical_analog".into(),
                analogs: vec![],
                derivation: String::new(),
                caveat: String::new(),
            },
            analog_band: vec![],
            sub_question: vec![SubQuestionCfg {
                key: "pricing".into(),
                label: "P".into(),
                role: "r".into(),
                plain: "p".into(),
                order: 1,
            }],
            indicator: vec![
                IndicatorCfg {
                    id: "credit_hy".into(),
                    group: Some("pricing".into()),
                    label: "HY".into(),
                    weight: 12.0,
                    unit: "pct".into(),
                    source: "fred".into(),
                    rationale: String::new(),
                    // Same shape as the shipped anchors: tight spread = LOW stress.
                    anchors: vec![
                        [2.0, 5.0],
                        [2.8, 18.0],
                        [3.5, 30.0],
                        [5.0, 50.0],
                        [7.0, 75.0],
                        [10.0, 95.0],
                    ],
                    fred_series: Some("BAMLH0A0HYM2".into()),
                    fred_series_b: None,
                },
                IndicatorCfg {
                    id: "credit_ig".into(),
                    label: "IG".into(),
                    weight: 6.0,
                    unit: "pct".into(),
                    source: "fred".into(),
                    rationale: String::new(),
                    anchors: vec![[0.5, 8.0], [0.8, 20.0], [1.5, 55.0]],
                    group: Some("pricing".into()),
                    fred_series: Some("BAMLC0A0CM".into()),
                    fred_series_b: None,
                },
            ],
        }
    }

    /// Synthetic series built for the test. These are deliberately constructed
    /// INPUTS for exercising arithmetic — they are not presented as observations
    /// of the real market anywhere in the tool's output.
    fn synth(vals: &[f64]) -> Series {
        Series {
            provenance: Provenance {
                source: "synthetic-test-input".into(),
                endpoint: "test://synthetic".into(),
                as_of: "2026-09-16".into(),
                retrieved_at: "2026-09-16T00:00:00Z".into(),
            },
            points: vals
                .iter()
                .enumerate()
                .map(|(i, v)| Point {
                    date: format!("2026-09-{:02}", (i % 28) + 1),
                    value: *v,
                })
                .collect(),
        }
    }

    fn ctx_with<'a>(cfg: &'a Config, obs: &'a Observations) -> Ctx<'a> {
        Ctx { obs, cfg }
    }

    #[test]
    fn tight_spread_scores_low_stress() {
        // Not inverted: a tight spread is cheap credit, reported as low stress.
        // The informative signal is direction of travel, not level.
        let c = cfg();
        let mut obs = Observations::default();
        obs.fred
            .insert("BAMLH0A0HYM2".into(), synth(&[2.4, 2.6, 2.76]));
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Scored { stress, value, .. } => {
                assert!((value - 2.76).abs() < 1e-9);
                assert!(
                    stress < 25.0,
                    "tight spread should read calm, got {}",
                    stress
                );
            }
            other => panic!("expected a scored reading, got {:?}", other),
        }
    }

    #[test]
    fn wide_spread_scores_high_stress() {
        let c = cfg();
        let mut obs = Observations::default();
        obs.fred
            .insert("BAMLH0A0HYM2".into(), synth(&[2.76, 5.5, 8.0]));
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Scored { stress, value, .. } => {
                assert!((value - 8.0).abs() < 1e-9);
                assert!(
                    stress > 70.0,
                    "wide spread should read stressed, got {}",
                    stress
                );
            }
            other => panic!("expected a scored reading, got {:?}", other),
        }
    }

    #[test]
    fn stress_increases_monotonically_as_spreads_widen() {
        let c = cfg();
        let mut prev = -1.0;
        for level in [2.0, 3.0, 4.0, 5.0, 7.0, 10.0] {
            let mut obs = Observations::default();
            obs.fred.insert("BAMLH0A0HYM2".into(), synth(&[level]));
            if let Reading::Scored { stress, .. } = CreditHy.evaluate(&ctx_with(&c, &obs)) {
                assert!(
                    stress > prev,
                    "stress must rise with spread: {} vs {}",
                    stress,
                    prev
                );
                prev = stress;
            } else {
                panic!("expected scored at {}", level);
            }
        }
    }

    #[test]
    fn missing_fred_series_is_a_reported_gap_not_a_zero() {
        let c = cfg();
        let obs = Observations::default();
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        assert!(!r.is_available(), "must not invent a credit reading");
        match r {
            Reading::Unavailable { reason } => {
                assert!(
                    reason.contains("BAMLH0A0HYM2"),
                    "gap must name the series: {}",
                    reason
                );
                assert!(
                    reason.contains("OPTIONAL"),
                    "gap must state that FRED is optional: {}",
                    reason
                );
            }
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn ig_indicator_uses_its_own_series_and_anchors() {
        let c = cfg();
        let mut obs = Observations::default();
        obs.fred.insert("BAMLC0A0CM".into(), synth(&[1.5]));
        // HY is absent, IG present: IG must still score.
        let r = CreditIg.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Scored { stress, .. } => {
                assert!(
                    (stress - 55.0).abs() < 1e-6,
                    "anchor hit expected, got {}",
                    stress
                );
            }
            other => panic!("expected scored, got {:?}", other),
        }
    }

    #[test]
    fn credit_gap_reason_surfaces_the_upstream_failure() {
        let c = cfg();
        let mut obs = Observations::default();
        obs.failures.push(crate::model::SourceFailure {
            source: "fred".into(),
            endpoint: "BAMLH0A0HYM2".into(),
            reason: "connection reset by peer".into(),
        });
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Unavailable { reason } => assert!(
                reason.contains("connection reset by peer"),
                "the real cause must be propagated, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // funding_stress — the two-leg spread
    // -----------------------------------------------------------------------

    /// A series with EXPLICIT dates, so the date-pairing can be controlled exactly.
    /// `synth` above cycles through the month and cannot express a calendar mismatch.
    fn dated(pairs: &[(&str, f64)]) -> Series {
        Series {
            provenance: Provenance {
                source: "synthetic-test-input".into(),
                endpoint: "test://synthetic".into(),
                as_of: pairs
                    .last()
                    .map(|(d, _)| (*d).to_string())
                    .unwrap_or_default(),
                retrieved_at: "2026-10-16T00:00:00Z".into(),
            },
            points: pairs
                .iter()
                .map(|(d, v)| Point {
                    date: (*d).to_string(),
                    value: *v,
                })
                .collect(),
        }
    }

    /// A config carrying the funding_stress spread on top of the shared fixtures.
    fn cfg_funding() -> Config {
        let mut c = cfg();
        c.indicator.push(IndicatorCfg {
            id: "funding_stress".into(),
            group: Some("pricing".into()),
            label: "FS".into(),
            weight: 7.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![
                [-0.10, 5.0],
                [0.00, 15.0],
                [0.07, 30.0],
                [0.18, 50.0],
                [0.45, 70.0],
                [1.00, 88.0],
                [2.95, 100.0],
            ],
            fred_series: Some("SOFR".into()),
            fred_series_b: Some("EFFR".into()),
        });
        c
    }

    #[test]
    fn funding_stress_pairs_on_the_newest_COMMON_date_not_each_legs_latest() {
        // THE BUG THIS INDICATOR'S CUSTOM PAIRING EXISTS TO PREVENT.
        //
        // The two legs have different holiday calendars and publication lags, so their latest
        // observations are often different days. Reading each leg's own latest value and subtracting
        // would silently compare two different dates — and here that turns a calm +0.10 into +1.10,
        // which the anchors score as near-severe. The error is largest exactly when the market moves,
        // which is when it matters most.
        let c = cfg_funding();
        let mut obs = Observations::default();
        obs.fred.insert(
            "SOFR".into(),
            dated(&[
                ("2026-09-01", 4.00),
                ("2026-09-02", 4.00),
                ("2026-09-03", 5.00), // this leg runs a day ahead
            ]),
        );
        obs.fred.insert(
            "EFFR".into(),
            dated(&[("2026-09-01", 3.90), ("2026-09-02", 3.90)]), // this leg stops a day earlier
        );

        match FundingStress.evaluate(&ctx_with(&c, &obs)) {
            Reading::Scored {
                value,
                provenance,
                detail,
                ..
            } => {
                assert!(
                    (value - 0.10).abs() < 1e-9,
                    "must subtract on the newest COMMON date (4.00 - 3.90 = 0.10), got {}",
                    value
                );
                assert_eq!(
                    provenance.as_of, "2026-09-02",
                    "the spread must be dated at the paired date, not either leg's own latest"
                );
                assert!(
                    !detail.contains("1.10"),
                    "the mismatched subtraction must not appear anywhere: {}",
                    detail
                );
            }
            other => panic!("expected Scored, got {:?}", other),
        }
    }

    #[test]
    fn funding_stress_widening_is_MORE_stress_not_less() {
        // DIRECTION, guarded because this series is negative most of the time (56.9% of its
        // observations). An inverted anchor ladder would look superficially plausible and would
        // report an easing funding market as a squeeze.
        let c = cfg_funding();
        let scored = |v: f64| {
            let mut obs = Observations::default();
            obs.fred.insert("SOFR".into(), dated(&[("2026-09-01", v)]));
            obs.fred
                .insert("EFFR".into(), dated(&[("2026-09-01", 0.0)]));
            match FundingStress.evaluate(&ctx_with(&c, &obs)) {
                Reading::Scored { stress, .. } => stress,
                other => panic!("expected Scored, got {:?}", other),
            }
        };
        let calm = scored(-0.10);
        let tight = scored(0.10);
        let squeezed = scored(1.00);
        assert!(
            calm < tight && tight < squeezed,
            "stress must rise with the spread: {} / {} / {}",
            calm,
            tight,
            squeezed
        );
        assert!(calm < 20.0, "a deeply negative spread is calm: {}", calm);
    }

    #[test]
    fn funding_stress_refuses_without_both_legs_and_names_which() {
        // A spread with one leg silently missing must not be scored as though the other leg were
        // the spread. It must say which half is absent.
        let c = cfg_funding();
        let mut obs = Observations::default();
        obs.fred
            .insert("SOFR".into(), dated(&[("2026-09-01", 4.0)]));
        match FundingStress.evaluate(&ctx_with(&c, &obs)) {
            Reading::Unavailable { reason } => assert!(
                reason.contains("EFFR"),
                "the missing leg must be named, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn funding_stress_calls_a_disjoint_calendar_undefined_rather_than_zero() {
        // No shared date means the spread is UNDEFINED. Returning 0.0 would be a fabricated
        // measurement of a quiet funding market, which is the most dangerous possible default.
        let c = cfg_funding();
        let mut obs = Observations::default();
        obs.fred
            .insert("SOFR".into(), dated(&[("2026-09-01", 4.0)]));
        obs.fred
            .insert("EFFR".into(), dated(&[("2026-10-01", 3.9)]));
        match FundingStress.evaluate(&ctx_with(&c, &obs)) {
            Reading::Unavailable { reason } => assert!(
                reason.contains("share no observation date"),
                "must say the calendars are disjoint, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn funding_stress_declares_its_short_window_on_every_run() {
        // The scale's top rests on a single episode (September 2019) because SOFR begins 2018. That
        // limitation must travel WITH the number every time, not sit in a config comment a reader
        // never sees.
        let c = cfg_funding();
        let mut obs = Observations::default();
        let sofr: Vec<(String, f64)> = (1..=20)
            .map(|i| (format!("2026-09-{:02}", i), 4.0))
            .collect();
        let sofr: Vec<(&str, f64)> = sofr.iter().map(|(d, v)| (d.as_str(), *v)).collect();
        obs.fred.insert("SOFR".into(), dated(&sofr));
        obs.fred.insert(
            "EFFR".into(),
            dated(&[("2026-09-01", 3.9), ("2026-09-02", 3.9)]),
        );
        match FundingStress.evaluate(&ctx_with(&c, &obs)) {
            Reading::Scored { detail, .. } => {
                assert!(
                    detail.contains("no 2000, 2008 or 2020 point"),
                    "the window limitation must be stated in the output: {}",
                    detail
                );
                assert!(
                    detail.contains("paired observations"),
                    "the paired count must be stated so the reader can judge the span: {}",
                    detail
                );
            }
            other => panic!("expected Scored, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // credit_tail and cp_funding — the two weight-0 measured channels
    // -----------------------------------------------------------------------

    /// Both new channels ship at weight 0 with NO anchors, and that is asserted rather than
    /// trusted: a later edit that quietly gives either one weight would change the composite and
    /// orphan cross-version comparison, which is an owner decision.
    #[test]
    fn the_two_new_channels_ship_at_weight_zero_with_no_anchors() {
        let path = std::path::Path::new("config/indicators.toml");
        if !path.exists() {
            return; // unit tests may run from a different cwd
        }
        let cfg = Config::load(path).expect("shipped config must parse");
        for id in ["credit_tail", "cp_funding"] {
            let ic = cfg
                .indicator(id)
                .unwrap_or_else(|| panic!("{} must be configured", id));
            assert_eq!(ic.weight, 0.0, "{} must ship at weight 0.0", id);
            assert!(
                ic.anchors.is_empty(),
                "{} must ship with EMPTY anchors — the measured ladder belongs in the rationale",
                id
            );
            // The legs must be declared as data, not hardcoded in the indicator.
            assert!(
                ic.fred_series.is_some() && ic.fred_series_b.is_some(),
                "{} is a spread and must name BOTH legs in the config",
                id
            );
        }
    }

    /// A weight-0 series that carries a scale invites the next reader to raise the weight and score
    /// it without the owner ever deciding. Both channels must refuse that configuration loudly.
    /// Two separate tests so a failure names which channel regressed.
    #[test]
    #[should_panic(expected = "must carry NO anchors")]
    fn scoring_credit_tail_without_an_owner_decision_is_refused() {
        let mut cfg = Config::load(std::path::Path::new("config/indicators.toml")).expect("config");
        for ic in cfg.indicator.iter_mut() {
            if ic.id == "credit_tail" {
                ic.anchors = vec![[4.0, 0.0], [10.0, 100.0]];
            }
        }
        let mut obs = Observations::default();
        obs.fred.insert(
            "BAMLH0A3HYC".into(),
            dated(&[("2026-09-01", 8.0), ("2026-09-02", 8.1)]),
        );
        obs.fred.insert(
            "BAMLH0A0HYM2".into(),
            dated(&[("2026-09-01", 3.0), ("2026-09-02", 3.1)]),
        );
        let _ = CreditTail.evaluate(&ctx_with(&cfg, &obs));
    }

    #[test]
    #[should_panic(expected = "must carry NO anchors")]
    fn scoring_cp_funding_without_an_owner_decision_is_refused() {
        let mut cfg = Config::load(std::path::Path::new("config/indicators.toml")).expect("config");
        for ic in cfg.indicator.iter_mut() {
            if ic.id == "cp_funding" {
                ic.anchors = vec![[-0.2, 0.0], [3.4, 100.0]];
            }
        }
        let mut obs = Observations::default();
        obs.fred
            .insert("DCPF1M".into(), dated(&[("2026-09-01", 4.1)]));
        obs.fred
            .insert("DTB3".into(), dated(&[("2026-09-01", 4.0)]));
        let _ = CpFunding.evaluate(&ctx_with(&cfg, &obs));
    }

    /// `credit_tail` measures the dispersion on the newest COMMON date and, being weight 0, reports
    /// it with a stress of 0.0 — the number is rendered, not scored.
    #[test]
    fn credit_tail_reports_the_dispersion_and_stays_unscored() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "credit_tail".into(),
            group: Some("pricing".into()),
            label: "CT".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("BAMLH0A3HYC".into()),
            fred_series_b: Some("BAMLH0A0HYM2".into()),
        });
        let mut obs = Observations::default();
        obs.fred.insert(
            "BAMLH0A3HYC".into(),
            dated(&[("2026-09-01", 8.00), ("2026-09-02", 8.90)]),
        );
        // The HY leg stops a day earlier: the spread must be taken on the common date.
        obs.fred
            .insert("BAMLH0A0HYM2".into(), dated(&[("2026-09-01", 3.00)]));

        match CreditTail.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Scored {
                stress,
                value,
                detail,
                provenance,
                ..
            } => {
                assert_eq!(stress, 0.0, "a weight-0 channel must never carry a score");
                assert!(
                    (value - 5.00).abs() < 1e-9,
                    "must subtract on the newest COMMON date (8.00 - 3.00), got {}",
                    value
                );
                assert_eq!(
                    provenance.as_of, "2026-09-01",
                    "the dispersion must be dated at the paired date"
                );
                assert!(
                    detail.contains("licence-windowed"),
                    "the ICE licence cap must travel with the number: {}",
                    detail
                );
                assert!(
                    detail.contains("2008 or 2020"),
                    "the window limitation must name the crises it cannot see: {}",
                    detail
                );
                // The fixture's only paired date IS the maximum, so this run exercises the
                // same-date branch. The other branch is guarded by
                // `credit_tail_does_not_call_an_earlier_maximum_the_current_reading`.
                assert!(
                    detail.contains("maximum IS the current reading"),
                    "when the maximum is the current reading the output may say so: {}",
                    detail
                );
            }
            other => panic!("expected Scored, got {:?}", other),
        }
    }

    /// THE MEASURED FALSEHOOD THIS REMOVES. On 2026-10-17 the repo's own cache put the window
    /// maximum at 9.57 on 2026-10-14 while the latest PAIRED reading was 9.55 on 2026-10-15, and
    /// the detail string printed both dates while calling the maximum "the CURRENT reading" — a
    /// sentence that contradicts its own numbers. The intent (the top of the range is a very
    /// recent observation, not a historical extreme) is kept; where the maximum SITS is now
    /// stated from the data rather than asserted.
    #[test]
    fn credit_tail_does_not_call_an_earlier_maximum_the_current_reading() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "credit_tail".into(),
            group: Some("pricing".into()),
            label: "CT".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("BAMLH0A3HYC".into()),
            fred_series_b: Some("BAMLH0A0HYM2".into()),
        });
        let mut obs = Observations::default();
        obs.fred.insert(
            "BAMLH0A3HYC".into(),
            dated(&[("2026-10-14", 9.57), ("2026-10-15", 9.55)]),
        );
        obs.fred.insert(
            "BAMLH0A0HYM2".into(),
            dated(&[("2026-10-14", 3.00), ("2026-10-15", 3.00)]),
        );

        match CreditTail.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Scored { value, detail, .. } => {
                assert!(
                    (value - 6.55).abs() < 1e-9,
                    "the newest COMMON date must be read, got {}",
                    value
                );
                // Case-insensitive, because the defect this guards is a CLAIM ("the maximum is
                // the current reading") and a copy-edit to caps ("IS") must not be able to
                // smuggle it back past the guard.
                let lower = detail.to_lowercase();
                assert!(
                    !lower.contains("maximum is the current reading")
                        && !lower.contains("maximum is the most recent reading"),
                    "the maximum is 6.57 on 2026-10-14 against a current reading of 6.55 on \
                     2026-10-15, so the output must not call the maximum the current reading: {}",
                    detail
                );
                assert!(
                    detail.contains("maximum 6.57 on 2026-10-14"),
                    "the maximum and its own date must be stated: {}",
                    detail
                );
                assert!(
                    detail.contains("one day before"),
                    "the position of the maximum must be stated, not left to the reader: {}",
                    detail
                );
                assert!(
                    detail.contains("the current reading on 2026-10-15"),
                    "the current reading's date must be named: {}",
                    detail
                );
            }
            other => panic!("expected Scored, got {:?}", other),
        }
    }

    #[test]
    fn credit_tail_names_the_missing_leg() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "credit_tail".into(),
            group: Some("pricing".into()),
            label: "CT".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("BAMLH0A3HYC".into()),
            fred_series_b: Some("BAMLH0A0HYM2".into()),
        });
        let mut obs = Observations::default();
        obs.fred
            .insert("BAMLH0A3HYC".into(), dated(&[("2026-09-01", 8.0)]));
        match CreditTail.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Unavailable { reason } => assert!(
                reason.contains("BAMLH0A0HYM2"),
                "the missing leg must be named, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn credit_tail_calls_a_disjoint_calendar_undefined_rather_than_zero() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "credit_tail".into(),
            group: Some("pricing".into()),
            label: "CT".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("BAMLH0A3HYC".into()),
            fred_series_b: Some("BAMLH0A0HYM2".into()),
        });
        let mut obs = Observations::default();
        obs.fred
            .insert("BAMLH0A3HYC".into(), dated(&[("2026-09-01", 8.0)]));
        obs.fred
            .insert("BAMLH0A0HYM2".into(), dated(&[("2026-10-01", 3.0)]));
        match CreditTail.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Unavailable { reason } => assert!(
                reason.contains("share no observation date"),
                "a fabricated 0.0 would read as a calm tail, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    fn weekly_series(id: &str, start: &str, n: usize, v: f64) -> (String, Series) {
        let s = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap();
        let pairs: Vec<(String, f64)> = (0..n)
            .map(|i| {
                (
                    (s + chrono::Duration::days(7 * i as i64))
                        .format("%Y-%m-%d")
                        .to_string(),
                    v,
                )
            })
            .collect();
        let refs: Vec<(&str, f64)> = pairs.iter().map(|(d, v)| (d.as_str(), *v)).collect();
        (id.to_string(), dated(&refs))
    }

    fn daily_series(id: &str, start: &str, n: usize, v: f64) -> (String, Series) {
        let s = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap();
        let pairs: Vec<(String, f64)> = (0..n)
            .map(|i| {
                (
                    (s + chrono::Duration::days(i as i64))
                        .format("%Y-%m-%d")
                        .to_string(),
                    v,
                )
            })
            .collect();
        let refs: Vec<(&str, f64)> = pairs.iter().map(|(d, v)| (d.as_str(), *v)).collect();
        (id.to_string(), dated(&refs))
    }

    /// `cp_funding` must declare its cadence MEASURED FROM THE DATA, not assert a remembered figure,
    /// and the output must not present the channel as fast. The synthetic paper leg publishes once a
    /// week while the Treasury leg publishes daily — the real shape of DCPF1M against DTB3.
    #[test]
    fn cp_funding_measures_its_own_cadence_and_does_not_claim_to_be_fast() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "cp_funding".into(),
            group: Some("pricing".into()),
            label: "CP".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("DCPF1M".into()),
            fred_series_b: Some("DTB3".into()),
        });
        let mut obs = Observations::default();
        let (k, paper) = weekly_series("DCPF1M", "2025-07-01", 60, 5.00);
        let (k2, bill) = daily_series("DTB3", "2025-07-01", 420, 4.60);
        obs.fred.insert(k, paper);
        obs.fred.insert(k2, bill);

        match CpFunding.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Scored {
                stress,
                value,
                detail,
                ..
            } => {
                assert_eq!(stress, 0.0, "a weight-0 channel must never carry a score");
                assert!(
                    (value - 0.40).abs() < 1e-9,
                    "the spread is paper minus bill on the common dates, got {}",
                    value
                );
                assert!(
                    detail.contains("not a daily series"),
                    "the cadence limitation must be stated in the output: {}",
                    detail
                );
                assert!(
                    detail.contains("median gap 7 days"),
                    "the cadence must be MEASURED from the series, not asserted: {}",
                    detail
                );
                assert!(
                    detail.contains("NOT a fast channel"),
                    "the output must refuse the fast-channel reading explicitly: {}",
                    detail
                );
            }
            other => panic!("expected Scored, got {:?}", other),
        }
    }

    /// The newest paired date is set by the SPARSE leg. A run where the paper leg lags must date the
    /// reading by the paper leg, not by the fresher Treasury leg.
    #[test]
    fn cp_funding_dates_the_reading_by_the_sparse_leg() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "cp_funding".into(),
            group: Some("pricing".into()),
            label: "CP".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("DCPF1M".into()),
            fred_series_b: Some("DTB3".into()),
        });
        let mut obs = Observations::default();
        obs.fred.insert(
            "DCPF1M".into(),
            dated(&[("2026-09-01", 5.0), ("2026-10-05", 5.1)]),
        );
        obs.fred.insert(
            "DTB3".into(),
            dated(&[
                ("2026-09-01", 4.6),
                ("2026-10-05", 4.6),
                ("2026-10-15", 4.6),
            ]),
        );
        match CpFunding.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Scored {
                provenance, detail, ..
            } => {
                assert_eq!(
                    provenance.as_of, "2026-10-05",
                    "the reading is dated by the sparse paper leg, not the fresher Treasury leg"
                );
                assert!(
                    detail.contains("2026-10-15"),
                    "the output must show how far the Treasury leg runs ahead: {}",
                    detail
                );
            }
            other => panic!("expected Scored, got {:?}", other),
        }
    }

    #[test]
    fn cp_funding_names_the_missing_leg() {
        let mut cfg = cfg();
        cfg.indicator.push(IndicatorCfg {
            id: "cp_funding".into(),
            group: Some("pricing".into()),
            label: "CP".into(),
            weight: 0.0,
            unit: "pct".into(),
            source: "fred".into(),
            rationale: String::new(),
            anchors: vec![],
            fred_series: Some("DCPF1M".into()),
            fred_series_b: Some("DTB3".into()),
        });
        let mut obs = Observations::default();
        obs.fred
            .insert("DCPF1M".into(), dated(&[("2026-09-01", 5.0)]));
        match CpFunding.evaluate(&ctx_with(&cfg, &obs)) {
            Reading::Unavailable { reason } => assert!(
                reason.contains("DTB3"),
                "the missing leg must be named, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }
}
