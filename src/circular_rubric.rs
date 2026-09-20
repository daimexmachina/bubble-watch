//! The verified circular-financing rubric.
//!
//! WHY THIS IS A RUBRIC AND NOT A RATIO, STATED PLAINLY.
//!
//! Every other indicator in this model is a NUMBER retrieved from a source: a spread, a
//! percentage, a ratio. Circular financing is not available as a number, because it is a
//! RELATIONSHIP between two companies and no free source publishes "share of counterparty
//! revenue funded by the investor's own capital". The honest options were therefore:
//!
//!   (a) leave it at weight 0 as a declared gap, or
//!   (b) state the scoring rule explicitly, apply it only to edges whose filing text was
//!       actually READ, and let a reader disagree with the weights.
//!
//! (a) was the original choice and it was too conservative: it treats a documented, citable,
//! falsifiable structure as though it were unknowable. (b) is what this module does.
//!
//! WHAT MAKES THIS FALSIFIABLE RATHER THAN ASTROLOGY.
//!
//! Each element is a specific structure named in a specific filing, with a citation and a
//! magnitude. Any of them can be shown wrong:
//!   * the filing could be amended or the deal unwound, removing the edge;
//!   * a structure could turn out on reading to be something else — and one DID. Oracle's
//!     "OpenAI" hits are a model-integration list, not a contract, so they score ZERO and are
//!     recorded as an explicit refutation. An unfalsifiable indicator could not have that
//!     outcome; this one produced it on its first real test.
//!   * the cohort could be shown to omit a larger structure, which would raise the reading.
//!
//! So the reading is a LOWER BOUND with a stated direction of bias, which is the same
//! discipline applied everywhere else in this tool.
//!
//! DIRECTION OF BIAS, WHICH IS THE IMPORTANT DISCLOSURE. Only edges whose text was READ
//! contribute. There are ~48 real edges in the scan and five have been read. An unread edge
//! scores ZERO, not partially, so this indicator UNDERSTATES circular financing and biases
//! the composite toward CALM. Treat a low reading as "nothing has been verified yet", never
//! as "nothing is there".

use serde::{Deserialize, Serialize};

/// What KIND of structure was found. The severity ordering is the judgment in this module
/// and is stated here so it can be argued with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Structure {
    /// THREE THINGS AT ONCE, which is why it ranks highest: the vendor takes an equity stake
    /// in the developer, extends CONTINGENT CREDIT SUPPORT (a guarantee) over the
    /// infrastructure the customer will occupy, and thereby finances the demand for its own
    /// product. NVIDIA -> SB Energy/OpenAI at PORTS-Pike.
    ///
    /// Ranked above `EquityForPurchases` because it puts the vendor's balance sheet at
    /// contingent risk ON TOP of the equity outlay, and because the guaranteed asset exists
    /// only to consume the vendor's output. A dilution is bounded by the share count; a
    /// guarantee is bounded by whatever the buildout costs.
    SupplierGuaranteeAndInvestment,
    /// Equity handed to a customer, vesting against its PURCHASES. The consideration for the
    /// revenue is the vendor's own stock, so the revenue and the equity are the same
    /// transaction seen twice. AMD -> OpenAI.
    EquityForPurchases,
    /// A parent absorbing a loss-making AI unit's debt onto its own balance sheet, with
    /// related-party interest. The buildout cost and the lab's losses become one credit.
    /// SpaceX -> xAI.
    AbsorbedUnitDebt,
    /// Related-party revenue disclosed under ASC 850 at a material magnitude, alongside an
    /// equity-method stake. Circular in effect, and disclosed in full. MSFT -> OpenAI.
    RelatedPartyRevenueWithStake,
    /// Purchases from an entity under common control. A real related-party flow but a supply
    /// relationship rather than financing. SpaceX -> Tesla.
    RelatedPartySupply,
    /// READ AND REFUTED. The mention exists but is NOT a financing relationship — e.g. a
    /// model-integration list. Scores zero and is recorded so the instrument's ability to
    /// come back LOWER is visible rather than invisible.
    Refuted,
}

impl Structure {
    /// Rubric points. DELIBERATELY COARSE: the gaps between these are meant to be arguable,
    /// and a finer scale would imply a precision this evidence cannot support.
    pub fn points(self) -> f64 {
        match self {
            Structure::SupplierGuaranteeAndInvestment => 45.0,
            Structure::EquityForPurchases => 40.0,
            Structure::AbsorbedUnitDebt => 30.0,
            Structure::RelatedPartyRevenueWithStake => 25.0,
            Structure::RelatedPartySupply => 10.0,
            Structure::Refuted => 0.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Structure::SupplierGuaranteeAndInvestment => {
                "supplier guaranteeing and investing in its customer's infrastructure"
            }
            Structure::EquityForPurchases => "equity issued as consideration for purchases",
            Structure::AbsorbedUnitDebt => "parent absorbing its AI unit's debt",
            Structure::RelatedPartyRevenueWithStake => "related-party revenue with an equity stake",
            Structure::RelatedPartySupply => "related-party supply under common control",
            Structure::Refuted => "read and REFUTED — not a financing relationship",
        }
    }
}

/// How large the disclosed exposure is relative to the filer, where a magnitude was
/// disclosed. BOUNDED and COARSE on purpose: it can add at most 15 points, so a huge
/// guarantee can make an edge read stronger without being able to dominate the rubric.
///
/// WHY THIS EXISTS. A flat per-structure score treats a $105B guarantee and an unquantified
/// one as equally severe, which is a real weakness in a rubric that otherwise insists on
/// magnitudes. The ratio used is the disclosed exposure over the filer's market
/// capitalisation, because that is what makes an exposure dangerous rather than merely large.
///
/// THE CEILING IS DELIBERATELY LOW. Only some edges disclose a magnitude at all, so a large
/// severity range would make the reading depend on which filings happened to quantify the
/// number rather than on what is actually happening. 15 points is enough to differentiate and
/// not enough to swamp the structure classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scale {
    /// No magnitude disclosed, so nothing can be added. Not zero-severity — UNKNOWN.
    Undisclosed,
    /// Under 5% of the filer's market cap.
    Small,
    /// 5% to 20%.
    Moderate,
    /// Over 20% of the filer's market cap: an exposure that could plausibly impair the filer.
    Severe,
}

impl Scale {
    pub fn points(self) -> f64 {
        match self {
            Scale::Undisclosed => 0.0,
            // BOUNDED BELOW THE WEAKEST STRUCTURE (RelatedPartySupply = 10), deliberately.
            // A size term that could outrank a structure class would turn the rubric into a
            // size contest, where the largest guarantee always dominates regardless of what
            // kind of arrangement it is.
            Scale::Small => 2.0,
            Scale::Moderate => 5.0,
            Scale::Severe => 8.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Scale::Undisclosed => "exposure not quantified in the filing",
            Scale::Small => "under 5% of the filer's market cap",
            Scale::Moderate => "5-20% of the filer's market cap",
            Scale::Severe => "over 20% of the filer's market cap",
        }
    }
}

/// One verified edge: a structure that was read in a filing and classified.
///
/// Every field here was read from a primary source. Nothing is inferred, and an edge without
/// a citation is not permitted to exist — see `every_verified_edge_cites_a_filing`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedEdge {
    pub filer: &'static str,
    pub counterparty: &'static str,
    pub structure: Structure,
    /// How large the disclosed exposure is relative to the filer. Bounded; see `Scale`.
    pub scale: Scale,
    /// The specific filing and what it says, in the tool's own words but grounded in quotes.
    pub citation: &'static str,
    /// The disclosed magnitude, or a statement that none was disclosed.
    pub magnitude: &'static str,
    /// What would show this edge is wrong. Required: an edge that cannot be refuted is the
    /// definition of the unfalsifiable claim this project refuses.
    pub falsifier: &'static str,
}

/// THE VERIFIED SET. Adding an entry is a deliberate act: it requires having READ a filing,
/// extracted a magnitude, and written what would falsify it.
pub const VERIFIED: &[VerifiedEdge] = &[
    VerifiedEdge {
        filer: "NVDA",
        counterparty: "OpenAI",
        structure: Structure::SupplierGuaranteeAndInvestment,
        scale: Scale::Severe,
        citation: "NVIDIA FY2027 Q2 10-Q (filed 2026-08-26), Note 8 and the derivatives note, \
                   verbatim: \"In August 2026, we entered into guarantees with SB Energy Corp. to \
                   provide credit support on the land, power, and shell buildout at SB Energy's \
                   PORTS Technology Campus in Pike County, Ohio, covering leases for \
                   approximately 4.25 gigawatts of IT load. The campus will exclusively host our \
                   compute under 20-year leases to OpenAI, subject to limited exceptions, with \
                   our obligation capped at $105 billion in the aggregate\" ... \"Each guarantee \
                   generally becomes effective upon commencement of the applicable lease, with \
                   corresponding guarantee amounts increasing as each of nine data centers is \
                   placed in service, which is expected to begin in fiscal year 2029.\" NVIDIA \
                   classifies them as CREDIT DERIVATIVES. The 8-K of 2026-08-17 adds a $1.5B \
                   equity investment in SB Energy, joining SoftBank and OpenAI as investors; the \
                   commitments table shows land/power/shell guarantees at a $3,529M notional \
                   alongside $4,800M of public company warrants and a $1,000M equity forward.",
        magnitude: "$105 BILLION maximum aggregate guarantee over 20-year OpenAI leases at \
                    PORTS-Pike (4.25 IT-GW of ~8 IT-GW), on top of a $1.5B equity investment in \
                    SB Energy. Present notional booked: $3,529M of land/power/shell guarantees, \
                    $4,800M warrants, $1,000M equity forward. $712M sits in escrow to mitigate \
                    exposure. THIS IS THE LARGEST SINGLE CIRCULAR EXPOSURE FOUND IN THE MODEL \
                    and, because the guarantees only become effective when leases commence from \
                    FY2029, almost none of it is on the balance sheet today.",
        falsifier: "Shown wrong if the guarantees are never called because OpenAI's leases are \
                    assigned to a better-rated tenant, if the obligation is novated to the \
                    capital providers NVIDIA is negotiating independent financing platforms with, \
                    or if OpenAI achieves an investment-grade rating (NVIDIA states the \
                    guarantees TERMINATE on that event) — any of which would remove the exposure. \
                    COUNTER-FACTS NVIDIA DISCLOSES AND A ONE-SIDED READING WOULD OMIT: the \
                    guarantees are limited to defined portions of lease and power payments, NOT \
                    the full cost of the site or all tenant obligations; exposure declines as \
                    OpenAI pays and falls to zero over each 20-year term; OpenAI has agreed to \
                    reimburse and indemnify, though NVIDIA warns it 'may not recover amounts \
                    promptly or in full'; the fair value recognised was NOT significant; and \
                    NVIDIA expects its large cloud customers to continue securing land, power and \
                    shell INDEPENDENTLY, i.e. this structure is disclosed as the exception rather \
                    than the norm.",
    },
    VerifiedEdge {
        filer: "AMD",
        counterparty: "OpenAI",
        structure: Structure::EquityForPurchases,
        scale: Scale::Moderate,
        citation: "AMD Form 8-K, 2025-10-06, Item 1.01 Material Definitive Agreement: AMD issued \
                   OpenAI OpCo, LLC a warrant for up to 160,000,000 shares at a $0.01 exercise \
                   price, vesting in tranches tied to purchases of AMD Instinct GPUs — first \
                   tranche on delivery of 1 GW, full vesting at 6 GW, with stock-price targets \
                   escalating to $600/share for the final tranche. Item 3.02 records it as an \
                   unregistered sale of equity securities.",
        magnitude: "160,000,000 shares = 9.8% of AMD's 1,632,475,042 shares outstanding \
                    (EDGAR dei:EntityCommonStockSharesOutstanding, 2026-07-29). AMD's CFO: the \
                    partnership is 'expected to deliver tens of billions of dollars in revenue'.",
        falsifier: "Shown wrong if the warrant is cancelled or expires unexercised, if the 6 GW \
                    purchase commitment is not met so the tranches do not vest, or if AMD \
                    discloses the shares as compensation rather than consideration for purchases.",
    },
    VerifiedEdge {
        filer: "SPCX",
        counterparty: "xAI",
        structure: Structure::AbsorbedUnitDebt,
        scale: Scale::Moderate,
        citation: "SpaceX (SPCX) FY2026 10-Q. xAI merged into SpaceX on 2026-02-02 (agreement \
                   filed as S-1 Exhibit 2.1); the merger appears in equity as 'Conversion of \
                   redeemable convertible preferred stock pursuant to xAI Merger'. xAI's debt \
                   sits on SpaceX's own balance sheet via the XBRL axis members \
                   spcx:XAI12.5SecuredSeniorNotesMember, spcx:XAIFixedRateTermLoanMember and \
                   spcx:XAIFloatingRateTermLoanMember.",
        magnitude: "xAI acquired at a $250B valuation; related-party interest expense of $327M in \
                    Q2 2026 and $513M in H1 2026; $37,474M of preferred converted in the merger.",
        falsifier: "Shown wrong if the xAI debt is ring-fenced in a non-recourse subsidiary \
                    rather than consolidated at SpaceX, or if the related-party interest is \
                    eliminated on consolidation rather than borne by the parent.",
    },
    VerifiedEdge {
        filer: "MSFT",
        counterparty: "OpenAI",
        structure: Structure::RelatedPartyRevenueWithStake,
        scale: Scale::Small,
        citation:
            "MSFT FY2026 10-K: 'Microsoft is a major investor in OpenAI and will continue to \
                   receive revenue-sharing payments.' Under ASC 850 Related Party Disclosures: \
                   'For fiscal year 2026, we recorded revenue from commercial arrangements with \
                   OpenAI, inclusive of revenue-sharing payments, of $24.1 billion, and accounts \
                   receivable from OpenAI as of June 30, 2026 was $6.0 billion.' The stake is \
                   equity-method, with a $6.5B net gain described as primarily 'the dilution gain \
                   from the OpenAI Recapitalization'.",
        magnitude: "revenue from OpenAI $24.1B (FY2026), receivable $6.0B, total funding \
                    commitments $13.0B, stake gain $6.5B; equity-method investments doubled from \
                    $6.0B to $12.0B over the year.",
        falsifier:
            "Shown wrong if the revenue from OpenAI is shown to be unrelated to Microsoft's \
                    investment — e.g. if a third party independently funds the same purchases — or \
                    if the OpenAI stake ceases to be equity-method accounted.",
    },
    VerifiedEdge {
        filer: "SPCX",
        counterparty: "Tesla",
        structure: Structure::RelatedPartySupply,
        scale: Scale::Small,
        citation:
            "SPCX FY2026 10-Q, Note 17 Related Party Transactions: 'During the three and six \
                   months ended June 30, 2026, the Company purchased $295 million and $329 \
                   million, respectively, of Megapack products from Tesla, Inc.' Also disclosed: \
                   $506 million of Megapacks and $131 million of Cybertrucks purchased as of \
                   December 31, 2025, and an equipment lease with Valor Equity Partners.",
        magnitude: "$329M of Megapack purchases in H1 2026; $637M cumulative with the 2025 \
                    figures.",
        falsifier: "Shown wrong if the purchases are shown to be at arm's-length market prices \
                    with no element of related-party preference — i.e. a pure supply contract.",
    },
    // THE REFUTATION. Kept in the set deliberately: it is the evidence that this rubric can
    // come back LOWER, which is what separates it from a one-way alarm.
    VerifiedEdge {
        filer: "ORCL",
        counterparty: "OpenAI",
        structure: Structure::Refuted,
        scale: Scale::Undisclosed,
        citation: "Oracle's two OpenAI hits are 8-K earnings releases, not its 10-K or 10-Qs, and \
                   in both the name appears only as a MODEL PROVIDER: \"...including Google's \
                   Gemini, OpenAI's ChatGPT, xAI's Grok, etc.—directly on top of the Oracle \
                   Database\". The same release reports RPO up 359% to $455 billion from 'four \
                   multi-billion-dollar contracts with three different customers' WITHOUT NAMING \
                   ANY OF THEM, and the FY2026 10-K adds 'No single customer accounted for 10% or \
                   more of our total revenues in fiscal 2026, 2025 or 2024.'",
        magnitude: "none attributable to a circular structure — the mention is a model list, not \
                    a contract.",
        falsifier: "Shown wrong if Oracle names OpenAI as a contract counterparty in a future \
                    filing, which would convert this from a refutation into a real edge.",
    },
];

/// The rubric total. Pure function of the verified set.
pub fn rubric_total() -> f64 {
    VERIFIED
        .iter()
        .map(|e| e.structure.points() + e.scale.points())
        .sum()
}

/// The maximum the current rubric can express, used to frame the reading honestly rather than
/// implying the scale is open-ended. This is the sum of the strongest structure a single edge
/// can carry, times the number of verified edges — i.e. "what this set would read if every
/// edge were the strongest kind". It exists so a reader can see how much of the scale the
/// present evidence actually occupies.
pub fn rubric_ceiling() -> f64 {
    // Uses the TRUE maximum structure rather than a named variant, so adding a stronger
    // structure in future raises the ceiling automatically instead of silently understating
    // how much of the scale the present evidence occupies.
    let max = [
        Structure::SupplierGuaranteeAndInvestment,
        Structure::EquityForPurchases,
        Structure::AbsorbedUnitDebt,
        Structure::RelatedPartyRevenueWithStake,
        Structure::RelatedPartySupply,
        Structure::Refuted,
    ]
    .iter()
    .map(|s| s.points())
    .fold(0.0_f64, f64::max);
    // The maximum a single edge can express is the strongest structure at the largest scale.
    (max + Scale::Severe.points()) * VERIFIED.len() as f64
}

/// A one-line summary of what was counted, for the indicator detail.
pub fn summary() -> String {
    let mut by_struct: Vec<(Structure, usize)> = Vec::new();
    for e in VERIFIED {
        match by_struct.iter_mut().find(|(s, _)| *s == e.structure) {
            Some((_, n)) => *n += 1,
            None => by_struct.push((e.structure, 1)),
        }
    }
    by_struct
        .iter()
        .map(|(s, n)| format!("{} x{}", s.label(), n))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_verified_edge_cites_a_filing_and_states_a_falsifier() {
        // An edge without a citation is an assertion; an edge without a falsifier is the
        // unfalsifiable claim this project refuses. Both are required, structurally.
        for e in VERIFIED {
            assert!(
                e.citation.len() > 120,
                "{} x {} needs a real citation",
                e.filer,
                e.counterparty
            );
            assert!(
                e.citation.contains("10-K")
                    || e.citation.contains("10-Q")
                    || e.citation.contains("8-K")
                    || e.citation.contains("S-1"),
                "{} x {} must name the filing: {}",
                e.filer,
                e.counterparty,
                e.citation
            );
            assert!(
                e.falsifier.len() > 60,
                "{} x {} needs a real falsifier",
                e.filer,
                e.counterparty
            );
            assert!(
                e.magnitude.len() > 20,
                "{} x {} needs a magnitude or an explicit statement of none",
                e.filer,
                e.counterparty
            );
        }
    }

    #[test]
    fn a_quantified_exposure_outweighs_an_unquantified_one_of_the_same_shape() {
        // A flat per-structure score would treat a $105B guarantee and an unquantified one as
        // equally severe. The scale term differentiates them, and it is BOUNDED so that a
        // single large number cannot swamp the structure classification.
        assert!(Scale::Severe.points() > Scale::Moderate.points());
        assert!(Scale::Moderate.points() > Scale::Small.points());
        assert!(Scale::Small.points() > Scale::Undisclosed.points());
        assert_eq!(Scale::Undisclosed.points(), 0.0);
        // The bound: a scale term can never be worth as much as the weakest structure.
        assert!(
            Scale::Severe.points() < Structure::RelatedPartySupply.points(),
            "scale must stay subordinate to structure, or the rubric becomes a size contest"
        );
        // And the largest exposure found is marked severe.
        let nvda = VERIFIED
            .iter()
            .find(|e| e.filer == "NVDA")
            .expect("the NVDA guarantee must be in the set");
        assert_eq!(
            nvda.scale,
            Scale::Severe,
            "a $105B guarantee over OpenAI leases is severe by the stated ratio"
        );
    }

    #[test]
    fn the_nvda_guarantee_carries_its_counter_facts_not_only_its_size() {
        // A one-sided entry would report the largest number in the model and omit the six
        // limits NVIDIA states in the same note. The falsifier field is where those live, and
        // this asserts they are actually there.
        let n = VERIFIED.iter().find(|e| e.filer == "NVDA").unwrap();
        assert!(
            n.magnitude.contains("$105 BILLION"),
            "the size must be stated"
        );
        let lower = n.falsifier.to_lowercase();
        for cf in [
            "not the full cost",
            "reimburse and indemnify",
            "not significant",
            "independently",
        ] {
            assert!(
                lower.contains(cf),
                "the NVDA entry must carry the counter-fact {:?}: {}",
                cf,
                n.falsifier
            );
        }
        assert!(
            n.citation.contains("credit derivatives") || n.citation.contains("CREDIT DERIVATIVES"),
            "NVIDIA's own classification of the guarantee must be recorded"
        );
    }

    #[test]
    fn the_rubric_can_come_back_lower_and_the_refutation_proves_it() {
        // THE PROPERTY THAT MAKES THIS NOT AN ALARM. A reading of "every edge is a
        // circularity" would be unfalsifiable in the direction that matters. This set
        // contains an edge that was READ and REFUTED, which is why the rubric is a
        // measurement rather than a tally of mentions.
        let refuted: Vec<_> = VERIFIED
            .iter()
            .filter(|e| e.structure == Structure::Refuted)
            .collect();
        assert!(
            !refuted.is_empty(),
            "the verified set must retain a refutation, or the rubric has no downside"
        );
        assert_eq!(
            Structure::Refuted.points(),
            0.0,
            "a refutation must contribute nothing"
        );
        // And the refuted ORCL edge is the one that proves it, because Oracle's RPO growth is
        // exactly the kind of thing a naive tally would score as the strongest signal.
        assert!(refuted
            .iter()
            .any(|e| e.filer == "ORCL" && e.counterparty == "OpenAI"));
    }

    #[test]
    fn equity_as_purchase_consideration_outranks_a_related_party_supply_deal() {
        // The ordering is the judgment in this module, so it is asserted rather than left
        // implicit: handing a customer your own stock to make it buy from you is a stronger
        // circularity than buying components from an affiliate.
        assert!(
            Structure::EquityForPurchases.points() > Structure::AbsorbedUnitDebt.points(),
            "equity-as-consideration is the strongest form"
        );
        assert!(
            Structure::AbsorbedUnitDebt.points() > Structure::RelatedPartyRevenueWithStake.points()
        );
        assert!(
            Structure::RelatedPartyRevenueWithStake.points()
                > Structure::RelatedPartySupply.points()
        );
        assert!(Structure::RelatedPartySupply.points() > Structure::Refuted.points());
    }

    #[test]
    fn the_total_is_the_sum_of_the_edges_and_occupies_part_of_the_ceiling() {
        let t = rubric_total();
        let c = rubric_ceiling();
        assert!(t > 0.0 && c >= t, "total {} ceiling {}", t, c);
        // The present evidence occupies roughly half the expressible scale, which is the
        // honest framing: real structures are present AND the scale retains headroom for
        // structures not yet read.
        assert!(
            (0.4..0.85).contains(&(t / c)),
            "the present reading should occupy a middle part of the scale, got {:.2}",
            t / c
        );
    }

    #[test]
    fn reading_more_edges_does_not_by_itself_raise_the_scored_fraction() {
        // THE PROPERTY THAT KEEPS THIS A MEASUREMENT RATHER THAN AN EFFORT GAUGE.
        //
        // The stress score interpolates on the FRACTION of the expressible scale, not the
        // absolute total, precisely so that discovering more edges does not mechanically
        // raise the reading. This simulates the two cases that matter:
        //
        //   * appending an AVERAGE edge (points equal to the current average) must leave the
        //     fraction essentially unchanged — finding more of the same tells you nothing new;
        //   * appending a REFUTATION must LOWER it, because a refutation adds to the ceiling
        //     and nothing to the total. A rubric that could not fall would be an alarm.
        let t = rubric_total();
        let c = rubric_ceiling();

        // Average edge = current total over number of edges, appended as a sixth edge.
        let avg = t / VERIFIED.len() as f64;
        let new_c = c + Structure::SupplierGuaranteeAndInvestment.points();
        let frac_before = t / c;
        let frac_after_avg = (t + avg) / new_c;
        assert!(
            (frac_after_avg - frac_before).abs() < 0.03,
            "an average edge must leave the fraction ~flat: {:.3} -> {:.3}",
            frac_before,
            frac_after_avg
        );

        // A refutation contributes 0 points but adds a full ceiling slot.
        let frac_after_refutation = t / (c + Structure::SupplierGuaranteeAndInvestment.points());
        assert!(
            frac_after_refutation < frac_before,
            "a refutation must LOWER the scored fraction: {:.3} -> {:.3}",
            frac_before,
            frac_after_refutation
        );
    }

    #[test]
    fn the_clearest_measured_structures_are_present() {
        // AMD->OpenAI (equity for purchases) and SPCX->xAI (absorbed debt) are the two
        // strongest structures found in the whole session. If either silently disappears the
        // indicator would understate without saying so.
        assert!(VERIFIED
            .iter()
            .any(|e| e.filer == "AMD" && e.structure == Structure::EquityForPurchases));
        assert!(VERIFIED
            .iter()
            .any(|e| e.filer == "SPCX" && e.structure == Structure::AbsorbedUnitDebt));
    }
}
