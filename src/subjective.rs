//! Subjective judgment, declared rather than smuggled.
//!
//! WHY THIS MODULE EXISTS. This tool has always contained subjective judgment — every
//! anchor in `config/indicators.toml` encodes a human opinion about what "high" means,
//! and the eight headline indicators are taken from a named analyst house. Those
//! opinions were INVISIBLE: they lived as calibration constants, so a reader could not
//! tell an arithmetic result from an encoded belief.
//!
//! This module makes judgment explicit, sourced, and falsifiable. It never enters the
//! composite — averaging a belief into a measurement would destroy the distinction the
//! whole project rests on — and it is reported beside the score like the GSADF test and
//! the falsifiers.
//!
//! THE PRIOR IT ENCODES. When trillions of dollars are at stake, concealment has an
//! enormous payoff, and the people involved are sophisticated. Assuming that
//! participants MANAGE DISCLOSURE to their advantage is not cynicism, it is the
//! forensic starting position: ask who benefits and what they would do. That prior is
//! legitimate and belongs in the tool.
//!
//! THE DISCIPLINE IT ENFORCES. `must_not` is required on every entry and is the whole
//! point: a prior tells you WHERE TO LOOK, never what you will find. An unfalsifiable
//! entry — "they are hiding something, and good disclosure proves it as much as bad" —
//! is rejected by construction, because it would make every outcome confirm the thesis.
//! A test asserts every entry carries a real falsifier.
//!
//! EVIDENCE CLASSES, kept distinct because they are not equally strong:
//!
//!   * `Judgment` — an interpretation with no numeric measurement behind it. The
//!     weakest class, and labelled as such;
//!   * `Measured` — a real number the tool fetches and reports;
//!   * `MeasuredNotAutomated` — a real number obtained by hand and verified, but not yet
//!     fetched automatically. Honest about the gap rather than implying daily coverage.

/// How strong the evidence behind an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// An interpretation. No number behind it. Weakest, and said so.
    Judgment,
    /// A number the tool fetches and reports on every run.
    Measured,
    /// A number verified by hand but not yet fetched automatically. The gap is stated
    /// rather than implied away.
    MeasuredNotAutomated,
}

impl Basis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Basis::Judgment => "judgment",
            Basis::Measured => "measured",
            Basis::MeasuredNotAutomated => "measured, not automated",
        }
    }
}

/// How much weight a reader should give an entry. Deliberately coarse — a finer scale
/// would imply a precision the evidence cannot support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }
}

/// Which way the entry bears on the bubble thesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bears {
    /// Supports the thesis that this is a bubble.
    Supports,
    /// Argues against it.
    Against,
    /// Could cut either way, and the entry says why.
    Ambiguous,
}

impl Bears {
    pub fn as_str(&self) -> &'static str {
        match self {
            Bears::Supports => "supports",
            Bears::Against => "against",
            Bears::Ambiguous => "ambiguous",
        }
    }
}

/// One declared piece of judgment or measured-but-unscored evidence.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Judgment {
    pub id: String,
    /// The claim, stated plainly.
    pub claim: String,
    pub basis: Basis,
    /// What is actually behind the claim.
    pub evidence: String,
    pub bears: Bears,
    pub confidence: Confidence,
    /// HOW THIS COULD BE SHOWN WRONG. Required, and the reason the module is not
    /// astrology: an entry with no falsifier would make every outcome confirm it.
    pub must_not: String,
}

/// The panel. Fixed and hand-authored, which is the honest thing for judgment — but
/// every entry names its evidence class so a reader can discount the weak ones.
pub fn build() -> Vec<Judgment> {
    vec![
        Judgment {
            id: "disclosure-management".into(),
            claim: "Participants manage disclosure to their advantage, and the material \
                    assumptions are where that shows, not the disclosure count."
                .into(),
            basis: Basis::Measured,
            evidence: "Tested rather than assumed. Disclosure DENSITY is rising, not \
                       falling (MSFT 248 -> 266 distinct XBRL concepts a year; ORCL 283 -> 310), \
                       so 'they are hiding more' fails on the count. What IS measurable sits in \
                       the assumptions: ORCL extended its implied useful life 11.1 -> 16.1 years \
                       while gross PP&E grew 3.5x; MSFT tags no purchase-obligation concept at \
                       all; AMZN's gross PP&E changes definition mid-series (a 1,827-day gap \
                       bridged by a finance-lease-inclusive tag). Three of nine filers have an \
                       unusable gross-PP&E series, which is itself the finding."
                .into(),
            bears: Bears::Supports,
            confidence: Confidence::High,
            must_not: "Shown wrong if the material-assumption measures come back clean across \
                       the cohort — that is, if no filer extends a depreciation schedule while \
                       its asset base grows, and the previously-absent disclosures appear. The \
                       depreciation detector currently flags ORCL ONLY, and returns a clean \
                       result for MSFT, NVDA, TSLA, AAPL and AVGO, so it is not a \
                       everything-is-rotten test."
                .into(),
        },
        Judgment {
            id: "accounting-digits-clean".into(),
            claim: "The reported NUMBERS do not show the fingerprint of estimation or \
                    fabrication."
                .into(),
            basis: Basis::Measured,
            evidence: "Two forensic tests, both of which can clear a company. Scale-normalised \
                       round-number clustering is the meaningful one: after dividing out each \
                       filer's reporting scale, values ending in exact thousands run ~0.1% \
                       against ~0.1% expected for genuinely measured data, so all six cohort \
                       filers come back CLEAN. Benford's law was attempted and REJECTED as a \
                       test here — at n=16,000-30,000 observations it is so over-powered that it \
                       rejected all six companies on deviations of a single percentage point, \
                       which is a property of the sample size, not of the filings."
                .into(),
            bears: Bears::Against,
            confidence: Confidence::Medium,
            must_not: "Shown wrong if the digit distribution shifts, or if a rounding signal \
                       appears at a different reporting scale. This test covers the INCOME \
                       STATEMENT and BALANCE SHEET line items only; it says nothing about the \
                       footnote disclosures, which is where the assumption-level manipulation \
                       actually lives."
                .into(),
        },
        Judgment {
            id: "ai-news-tone-deteriorating".into(),
            claim: "Public tone on AI is deteriorating at a measurable rate.".into(),
            basis: Basis::MeasuredNotAutomated,
            evidence: "GDELT's tone series for articles mentioning \"artificial intelligence\" \
                       over 25 months: monthly mean tone fell from +0.819 to +0.057, a linear \
                       slope of -0.032 tone points per month (-0.38 per year) with r = -0.886 \
                       against time. This is CONTEXT, not a warning: sentiment is known to \
                       FOLLOW price rather than lead it, which is exactly why it is reported \
                       here and not scored."
                .into(),
            bears: Bears::Supports,
            confidence: Confidence::Low,
            must_not: "Shown irrelevant as a predictor if tone turns out to move AFTER price \
                       rather than before it, which is the expected direction and would make \
                       this a coincident indicator with no forward value. Should only be \
                       trusted if a lead relationship is ever demonstrated against forward \
                       returns on a sample large enough to test."
                .into(),
        },
        Judgment {
            id: "frontier-premium".into(),
            claim: "The frontier-model premium over open-weight models is the moat that \
                    justifies the capex, and it is measurable as a price ratio."
                .into(),
            basis: Basis::MeasuredNotAutomated,
            evidence: "From OpenRouter's public model catalogue (417 priced models): the median \
                       input price of open-weight models is $0.200 per million tokens against \
                       $1.250 for closed models, so closed models hold a 6.3x premium. The full \
                       spread runs from $0.017/M to $150/M, a factor of 8,824x. This is a market \
                       estimate of how much better the frontier is assumed to be. A compressing \
                       ratio would mean the moat is closing, which is a leading indicator for \
                       the revenue that has to justify the spending."
                .into(),
            bears: Bears::Ambiguous,
            confidence: Confidence::Low,
            must_not: "Shown wrong if the ratio is stable or widening, which would mean open \
                       models are NOT converging on the frontier and the premium is real. Also \
                       weak because price reflects served cost and positioning as well as \
                       capability — a cheap model may simply be a small one, not an equal one. \
                       The correct companion is a capability score, which this tool does not \
                       yet fetch."
                .into(),
        },
        Judgment {
            id: "circular-financing".into(),
            claim: "Circular financing is real, is disclosed as a first-order concern by \
                    central banks, and remains this model's largest blind spot."
                .into(),
            basis: Basis::Judgment,
            evidence: "Named by BIS Working Paper 1367 and by the IMF's April 2026 GFSR, which \
                       carries a figure captioned 'Artificial Intelligence Circularity' and \
                       states that firms have 'increasingly relied on circular financing'. \
                       Partially visible in this tool through backlog_quality, which scores 89.1: \
                       ORCL's backlog exploded 137.8B -> 455.3B in a single quarter while \
                       deferred revenue — cash actually billed — went 10.7B -> 13.4B."
                .into(),
            bears: Bears::Supports,
            confidence: Confidence::Medium,
            must_not: "Shown wrong if the backlog converts. The test is available and is already \
                       built: if RPO growth were matched by rising deferred revenue, the \
                       commitments would be being billed and paid, and the divergence signal \
                       would disappear. It has not disappeared."
                .into(),
        },
        Judgment {
            id: "capability-vs-spend".into(),
            claim: "Capital spending is accelerating while the capability curve that is meant \
                    to justify it shows signs of plateau."
                .into(),
            basis: Basis::Judgment,
            evidence: "The spending side is measured and extreme: data-centre construction grew \
                       58.5% year over year, and cohort capex/revenue rose from 0.190 to 0.239 in \
                       a year. The capability side is NOT measured by this tool, and no free \
                       machine-readable capability series is integrated, so the comparison rests \
                       on published benchmark reporting rather than on a number this tool \
                       fetches. That gap is the reason this entry is a judgment and not a \
                       measurement."
                .into(),
            bears: Bears::Supports,
            confidence: Confidence::Low,
            must_not: "Shown wrong if a capability series is integrated and the curve is still \
                       rising steeply — spending on a curve that is still climbing is investment, \
                       not excess. This entry is the weakest in the panel precisely because half \
                       of the comparison is unmeasured, which is stated rather than hidden."
                .into(),
        },
    ]
}

/// How many entries currently bear against the thesis. Reported so a reader can see the
/// panel is not uniformly one-directional — a panel that only ever confirmed would be
/// the failure mode this module exists to avoid.
pub fn against_count(entries: &[Judgment]) -> usize {
    entries.iter().filter(|e| e.bears == Bears::Against).count()
}

/// Entries carrying only interpretation, with no measured number behind them.
pub fn judgment_only_count(entries: &[Judgment]) -> usize {
    entries
        .iter()
        .filter(|e| e.basis == Basis::Judgment)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_carries_a_real_falsifier() {
        // THE test that keeps this module honest. An entry with no way to be shown wrong
        // would make every outcome confirm it, which is the definition of the astrology
        // this project refuses.
        for e in build() {
            assert!(
                e.must_not.len() > 60,
                "entry '{}' has no substantive falsifier ({} chars)",
                e.id,
                e.must_not.len()
            );
            assert!(
                e.must_not.contains("Shown") || e.must_not.contains("wrong"),
                "entry '{}' does not say how it could be wrong: {}",
                e.id,
                e.must_not
            );
        }
    }

    #[test]
    fn every_entry_states_its_evidence_class_and_confidence() {
        for e in build() {
            assert!(!e.claim.is_empty(), "{} has no claim", e.id);
            assert!(!e.evidence.is_empty(), "{} has no evidence", e.id);
            // a Measured entry must cite at least one number
            if e.basis != Basis::Judgment {
                assert!(
                    e.evidence.chars().any(|c| c.is_ascii_digit()),
                    "entry '{}' claims to be measured but cites no number",
                    e.id
                );
            }
        }
    }

    #[test]
    fn the_panel_is_not_uniformly_one_directional() {
        // A panel that always confirms would be exactly the failure mode to avoid.
        let entries = build();
        assert!(
            against_count(&entries) >= 1,
            "at least one entry must bear against the thesis, or this is not a panel but a \
             conclusion looking for support"
        );
        assert!(
            entries.iter().any(|e| e.bears == Bears::Supports),
            "and at least one must support it, or it is not a panel either"
        );
    }

    #[test]
    fn weak_evidence_is_labelled_weak() {
        // Judgment-only entries must not claim high confidence.
        for e in build() {
            if e.basis == Basis::Judgment {
                assert_ne!(
                    e.confidence,
                    Confidence::High,
                    "entry '{}' is pure judgment and must not claim high confidence",
                    e.id
                );
            }
        }
        assert!(judgment_only_count(&build()) >= 1);
    }

    #[test]
    fn ids_are_unique() {
        let e = build();
        let mut ids: Vec<&str> = e.iter().map(|x| x.id.as_str()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate judgment id");
    }

    #[test]
    fn the_panel_is_deterministic() {
        assert_eq!(build(), build());
    }
}
