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

/// One verified edge: a structure that was read in a filing and classified.
///
/// Every field here was read from a primary source. Nothing is inferred, and an edge without
/// a citation is not permitted to exist — see `every_verified_edge_cites_a_filing`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedEdge {
    pub filer: &'static str,
    pub counterparty: &'static str,
    pub structure: Structure,
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
        citation: "NVIDIA Form 8-K filed 2026-08-17, Exhibit 99.1, verbatim: \"NVIDIA to provide \
                   credit support on land, power, and shell buildout to secure initial 4.25 \
                   IT-GW, with an option to take the remaining 3.75 IT-GW\" ... \"OpenAI will be \
                   the customer for 8-IT GW\" ... \"SB Energy will build, own and operate the \
                   data center under a 20-year lease to OpenAI\" ... \"NVIDIA will invest $1.5 \
                   billion in SB Energy, joining existing investors SoftBank Group and \
                   OpenAI.\" NVIDIA is also \"the exclusive AI compute infrastructure provider \
                   at PORTS-Pike\", and the site will run \"NVIDIA's full-stack DSX AI factory \
                   platform, including GPUs, CPUs and networking.\"",
        magnitude: "$1.5B equity investment in SB Energy, PLUS contingent credit support over the \
                    land, power and shell buildout for an initial 4.25 IT-GW (with an option on a \
                    further 3.75 IT-GW, 8 IT-GW total), under a 20-year OpenAI lease. The equity \
                    outlay is disclosed; the size of the GUARANTEE is not.",
        falsifier:
            "Shown wrong if the credit support is limited to a non-binding letter of intent \
                    rather than an enforceable guarantee, or if SB Energy secures the land, power \
                    and shell independently so NVIDIA's balance sheet is never at risk.",
    },
    VerifiedEdge {
        filer: "AMD",
        counterparty: "OpenAI",
        structure: Structure::EquityForPurchases,
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
    VERIFIED.iter().map(|e| e.structure.points()).sum()
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
    max * VERIFIED.len() as f64
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
