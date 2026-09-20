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
    /// THE SUPPLIER HOLDS ITS CUSTOMER'S TRADABLE EQUITY. The vendor sells into a company whose
    /// shares it owns, so its own P&L moves with the customer's valuation and the customer's
    /// ability to keep buying is supported by the vendor's own balance sheet. Distinct from a
    /// private stake because the position is MARKED TO MARKET: the vendor's earnings now include
    /// the market's estimate of its customer's prospects.
    ///
    /// Ranked low for its size — NVIDIA disclosed the position as an investment without a
    /// material gain — but recorded because the RELATIONSHIP is the point, not the amount: this is
    /// the purest form of selling to a company you own.
    SupplierHoldingCustomerEquity,
    /// THE RESELLER CARRYING A LONG-TERM LIABILITY IT HAS PASSED ON. A vendor signs a multi-year
    /// lease, sublicenses the entire obligation to an AI customer, and remains liable if the
    /// sublicensee fails — while the obligation is disclosed as an OFF-BALANCE-SHEET arrangement.
    /// The vendor books the relationship but not the asset, and the counterparty is a private AI
    /// company whose ability to pay is unrated.
    ///
    /// Ranked ABOVE the landlord structure: the landlord at least owns the asset it is secured on,
    /// whereas here the vendor has no asset at all and a contingent liability the size of the whole
    /// contract term.
    VendorSublicensingItsOwnLeaseLiability,
    /// THE PHYSICAL BUILDOUT FINANCED ON A TENANT'S CREDIT. A datacentre landlord borrows at a
    /// speculative rate to build, leases to an AI company whose own credit is unrated or
    /// sub-investment-grade, and the LANDLORD'S bondholders carry the risk — not the hyperscaler.
    /// The credit chain then runs further: the tenant's ability to pay depends on its own
    /// customers, which are the labs, whose funding depends on the hyperscalers.
    ///
    /// Ranked below the hyperscaler structures because the magnitudes are smaller and the exposure
    /// sits with bondholders rather than with a named technology company — but recorded separately
    /// because it is the only structure here where the risk lands on parties NOT in the AI trade.
    LandlordDebtSecuredByTenantCredit,
    /// THE LARGEST AGGREGATE IN THE DATASET, and the least committed. The vendor ORGANISES
    /// third-party capital dedicated to buying its own product — standing up financing
    /// platforms with large asset managers so institutional money funds the demand for the
    /// vendor's output. The vendor's own balance sheet is not directly at risk; what is at
    /// risk is that the demand it is forecasting exists only because it arranged the capital.
    ///
    /// Ranked BELOW the supply amplifier despite being larger, because it is an ANNOUNCEMENT:
    /// NVIDIA states it is "subject to definitive agreements". A number with that qualifier
    /// should not outrank a signed commitment. The qualifier is in the falsifier, not buried.
    VendorOrganisedThirdPartyCapital,
    /// THE COMPLETE LOOP: the vendor invests in the counterparty, signs a multi-year contract
    /// to sell it compute, AND extends it a credit facility — with the facility gated on its
    /// OWN delivery of that compute. All three flows run between the same two parties, so the
    /// revenue, the equity and the credit are one arrangement.
    ///
    /// Ranked BELOW the upstream supply amplifier on size ($100B vs $279B) but recorded as its
    /// own class because it is structurally distinct: the supply commitment is the vendor
    /// betting on demand, whereas this is the vendor FINANCING the counterparty that pays it,
    /// with the drawdown conditioned on the vendor performing. Amazon -> Anthropic.
    VendorFinancingItsOwnCustomer,
    /// THE AMPLIFIER, and the largest single number in the model. The vendor commits upstream
    /// to buy supply on the belief that the demand it is itself financing is real. If the
    /// financed customer fails, the vendor holds BOTH a guarantee over the customer's leases
    /// AND a supply commitment it may no longer need.
    ///
    /// Ranked above the guarantee because it is larger, and because it is NOT contingent on a
    /// default: a guarantee becomes payable if something goes wrong, whereas a supply
    /// commitment has already been made. NVIDIA's supply and capacity commitments rose from
    /// $119B to $279B in one quarter.
    SupplyCommitmentScaledToFinancedDemand,
    /// THREE THINGS AT ONCE: the vendor takes an equity stake
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
            Structure::SupplyCommitmentScaledToFinancedDemand => 50.0,
            Structure::VendorOrganisedThirdPartyCapital => 42.0,
            Structure::LandlordDebtSecuredByTenantCredit => 35.0,
            Structure::VendorSublicensingItsOwnLeaseLiability => 38.0,
            Structure::SupplierHoldingCustomerEquity => 28.0,
            Structure::VendorFinancingItsOwnCustomer => 48.0,
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
            Structure::SupplyCommitmentScaledToFinancedDemand => {
                "upstream supply commitments scaled to demand the vendor itself finances"
            }
            Structure::VendorOrganisedThirdPartyCapital => {
                "vendor organising third-party capital to fund demand for its own product"
            }
            Structure::LandlordDebtSecuredByTenantCredit => {
                "datacentre buildout financed on a speculative-grade tenant's credit"
            }
            Structure::VendorSublicensingItsOwnLeaseLiability => {
                "vendor sublicensing a long-term lease while remaining liable for it"
            }
            Structure::SupplierHoldingCustomerEquity => {
                "supplier holding its customer's tradable equity"
            }
            Structure::VendorFinancingItsOwnCustomer => {
                "vendor investing in, contracting with, and lending to the same counterparty"
            }
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
        filer: "CRWV",
        counterparty: "OpenAI",
        structure: Structure::VendorFinancingItsOwnCustomer,
        scale: Scale::Severe,
        citation: "CoreWeave FY2025 10-K (filed 2026-03-02): \"In May 2025, we entered into a \
                   master services agreement with OpenAI OpCo, LLC (\"OpenAI\") and in September \
                   2025, we entered into an order form under this master services agreement \
                   pursuant to which OpenAI has committed to pay us up to approximately $6.5 \
                   billion through May 31, 2031\" AND, separately: \"in March 2025, we entered \
                   into a master services agreement with OpenAI, a private company, pursuant to \
                   which OpenAI has committed to pay us up to approximately $11.9 billion through \
                   October 2030.\" The concentration note adds: \"We recognized an aggregate of \
                   approximately 67% of our revenue from our top customer, Microsoft, for the year \
                   ended December 31, 2025.\"",
        magnitude: "$6.5 BILLION committed through 2031 PLUS $11.9 BILLION committed through 2030 \
                    = $18.4 BILLION of OpenAI commitments to CoreWeave, against a company whose \
                    revenue is 67% from Microsoft and 77% from its top two customers.",
        falsifier: "Shown wrong if the OpenAI order forms are not drawn — they are commitments to \
                    pay UP TO an amount, not take-or-pay guarantees, and CoreWeave's own risk \
                    section warns of 'customers in their early stages and/or private companies \
                    that may have increased risk of insolvency' and that OpenAI is 'a private \
                    company' whose ability to pay is unrated. Also falsified if Microsoft's 67% \
                    share falls as OpenAI ramps, which would mean the concentration is \
                    transitional rather than structural.",
    },
    VerifiedEdge {
        filer: "CRWV",
        counterparty: "MSFT",
        structure: Structure::VendorFinancingItsOwnCustomer,
        scale: Scale::Severe,
        citation: "CoreWeave FY2025 10-K (filed 2026-03-02), concentration of credit risk note, \
                   verbatim: \"We recognized an aggregate of approximately 67% of our revenue from \
                   our top customer, Microsoft, for the year ended December 31, 2025. We \
                   recognized an aggregate of approximately 77% of our revenue from our top two \
                   customers for the year ended December 31, 2024, and approximately 73% of our \
                   revenue for the year ended December 31, 2023, from our top three customers.\" \
                   Read alongside the MSFT 10-K, which discloses a $13.0B funding commitment to \
                   OpenAI and a $24.1B revenue relationship with it, and the CRWV 10-K, which \
                   records OpenAI committing $18.4B to CoreWeave.",
        magnitude:
            "67% of CoreWeave's FY2025 revenue is MICROSOFT — a single customer. CoreWeave's \
                    total revenue is roughly $5B scale, so this is a concentrated, \
                    single-counterparty business.",
        falsifier: "Shown wrong if the 67% is transitional while OpenAI's $18.4B ramps and the \
                    customer base broadens. CoreWeave states it expects OpenAI to become 'a \
                    significant customer in future periods', which if realised would falsify the \
                    concentration reading without falsifying the circularity.",
    },
    VerifiedEdge {
        filer: "AMZN",
        counterparty: "Anthropic",
        structure: Structure::VendorFinancingItsOwnCustomer,
        scale: Scale::Severe,
        citation: "Amazon Q2 2026 10-Q (filed 2026-07-31), Note on equity investments, verbatim: \
                   \"Anthropic — From Q3 2023 to Q4 2025, we invested $8.0 billion in convertible \
                   notes from Anthropic, which are classified as available-for-sale and reported \
                   at fair value ... and as Level 3 assets\" ... \"In Q2 2026, AWS and Anthropic \
                   announced an expansion of the strategic collaboration and existing multi-year \
                   commitment by more than $100.0 billion over 10.0 years, which includes \
                   contractual obligations related to the performance of AWS chips\" ... \
                   \"Additionally, we entered into a financing arrangement to make available to \
                   Anthropic an aggregate facility not to exceed $20.0 billion that will expire \
                   \"30 months after an Anthropic liquidity event, including an initial public \
                   offering ... At inception, there is no amount available to be drawn against \
                   and as we reach certain delivery milestones of compute' [text continues].\"",
        magnitude: "$8.0B invested in Anthropic convertible notes (Q3 2023-Q4 2025), converted in \
                    part to nonvoting preferred stock; MORE THAN $100 BILLION of AWS cloud \
                    commitment over 10 years; PLUS a financing facility of up to $20.0 BILLION, \
                    drawable only as compute delivery milestones are reached, expiring 30 months \
                    after an Anthropic liquidity event such as an IPO.",
        falsifier: "Shown wrong if the cloud commitment and the equity stake are independent \
                    transactions between unrelated parties — but they are not: AWS announced the \
                    cloud expansion and holds the equity, and the $20B facility only becomes \
                    drawable as Amazon HITS COMPUTE DELIVERY MILESTONES, which ties the credit to \
                    the vendor's own performance rather than to the counterparty's credit. Also \
                    falsified if the convertible notes convert and the stake is monetised without \
                    the cloud commitment being drawn. The notes were Level 3 assets — the filer's \
                    own valuation, not a market price.",
    },
    VerifiedEdge {
        filer: "AMZN",
        counterparty: "OpenAI",
        structure: Structure::VendorFinancingItsOwnCustomer,
        scale: Scale::Severe,
        citation: "Amazon Q2 2026 10-Q, Note on equity investments, verbatim: \"In Q1 2026, AWS \
                   and OpenAI Group PBC (\"OpenAI\") announced an expansion of the existing $38.0 \
                   billion multi-year commitment and commercial arrangement with OpenAI by $100.0 \
                   billion over 8.0 years, which includes contractual obligations related to the \
                   performance of AWS chips\" ... \"We also invested $15.0 billion in Series C \
                   Preferred Stock of OpenAI and entered into an equity commitment letter \
                   agreement ... pursuant to which we agreed to purchase additional shares of \
                   Series C Preferred Stock ... with an aggregate purchase price of $35.0 \
                   billion. In Q2 2026, we invested $13.7 billion of the Commitment Amount in \
                   Series C Preferred Stock. We account for our $28.7 billion investment in \
                   Series C Preferred Stock recorded on our consolidated balance sheet as of \
                   June 30, 2026.\"",
        magnitude: "an existing $38.0 BILLION AWS commitment expanded BY $100.0 BILLION over 8 \
                    years; $15.0B invested in OpenAI Series C plus a $35.0B equity commitment \
                    letter, of which $13.7B was drawn in Q2 2026, for a $28.7 BILLION carrying \
                    value on the balance sheet at 2026-06-30.",
        falsifier: "Shown wrong if the equity investment and the cloud commitment are shown to be \
                    arm's-length and independently negotiated. The strongest counter is that the \
                    cloud arrangement is a commercial contract that would exist regardless of the \
                    equity — but Amazon itself describes the two in the SAME note and the \
                    commitment includes 'contractual obligations related to the performance of \
                    AWS chips', which makes AWS chip performance a contractual term of revenue it \
                    is simultaneously investing to obtain.",
    },
    VerifiedEdge {
        filer: "NVDA",
        counterparty: "CoreWeave",
        structure: Structure::SupplierHoldingCustomerEquity,
        scale: Scale::Small,
        citation: "NVIDIA FY2026 Q1 10-Q (filed 2025-05-28), note on marketable securities, \
                   verbatim: \"(1) The balance as of the first quarter of fiscal year 2026 includes \
                   an investment in CoreWeave, Inc., or CoreWeave, which was reclassified from \
                   non-marketable equity securities to marketable securities following public \
                   market trading.\" The XBRL records it under the axis member \
                   nvda:CoreWeaveInc.Member within us-gaap:PubliclyHeldEquitySecuritiesMember at \
                   FairValueInputsLevel1, i.e. marked to a quoted price.",
        magnitude: "the position is not separately quantified in the excerpt read, and NVIDIA \
                    states that \"net unrealized gains on investments in publicly-held equity \
                    securities held at period end were not significant\". The RELATIONSHIP is the \
                    substance: NVIDIA sells GPUs to CoreWeave while holding CoreWeave's listed \
                    shares, and CoreWeave is a company whose revenue is 67% Microsoft and which has \
                    committed $18.4B to OpenAI.",
        falsifier: "Shown wrong if the holding is immaterial AND passive — a small financial \
                    investment a supplier happens to hold is not a structure. NVIDIA describes the \
                    gain as not significant, which is a genuine counterweight and is why this entry \
                    is scored LOW (28) rather than with the guarantee and supply commitments. It \
                    would be falsified outright if the position is held in a segregated fund rather \
                    than on NVIDIA's own balance sheet, or if it is a legacy pre-IPO position being \
                    wound down rather than a strategic holding.",
    },
    VerifiedEdge {
        filer: "SMCI",
        counterparty: "Lambda",
        structure: Structure::VendorSublicensingItsOwnLeaseLiability,
        scale: Scale::Moderate,
        citation: "Super Micro Computer Form 8-K filed 2024-06-21, Item 1.01 and Item 2.03, \
                   verbatim: \"the Company entered into a Master Colocation Services Agreement ... \
                   to lease certain data center space\" ... \"the Company has agreed to lease 21 MW \
                   of a multi-tenanted facility from the Supplier for a term of 10 years. The \
                   Company's aggregate financial obligation for the term of the Service Order is \
                   estimated to be $600.0 million\" ... \"Concurrent with the execution of the MCSA \
                   and the Service Order, the Company entered into that certain Sublicense ... with \
                   Lambda, Inc. (the \"Sublicensee\") to sublicense all of the Company's rights and \
                   obligations with respect to the Data Center Space.\" Critically: \"The payments \
                   owed by the Company under the Service Order ... may be accelerated by the \
                   Supplier in the event of a default ... Upon such acceleration, the Company has a \
                   right to seek reimbursement for such accelerated payments from Sublicensee\" — \
                   i.e. SMCI remains liable and must chase Lambda. Filed under Item 2.03, \
                   \"Creation of a Direct Financial Obligation or an Obligation under an \
                   Off-Balance Sheet Arrangement of a Registrant\".",
        magnitude: "a $600.0 MILLION aggregate obligation over 10 years for 21 MW, sublicensed in \
                    full to Lambda, a PRIVATE company. SMCI has a right of reimbursement, not a \
                    release from the obligation.",
        falsifier: "Shown wrong if the sublicense transfers the liability outright rather than \
                    leaving SMCI with a reimbursement right — the filing says SMCI 'has a right to \
                    seek reimbursement', which is a claim against Lambda, not a novation. Also \
                    falsified if Lambda is well-capitalised and the reimbursement right is \
                    effectively money-good, or if the arrangement is a disclosed agency or \
                    pass-through rather than SMCI's own obligation. The word 'sublicense' rather \
                    than 'assign' is the load-bearing detail.",
    },
    VerifiedEdge {
        filer: "APLD",
        counterparty: "CoreWeave",
        structure: Structure::LandlordDebtSecuredByTenantCredit,
        scale: Scale::Severe,
        citation: "Applied Digital Form 8-K/A filed 2026-04-01 (amending the 8-K of 2025-06-02), \
                   verbatim: \"On March 30, 2026, the Company entered into a series of agreements \
                   intended to enhance the credit of the tenants under the data center leases for \
                   two of its three Polaris Forge 1 data centers in Ellendale, North Dakota: the \
                   Company's 100 MW data center (\"ELN-02\") and the Company's 150 MW data center \
                   (\"ELN-03\"), both currently leased to CoreWeave, Inc.\" ... \"CoreWeave Parent \
                   informed us that it was refinancing certain of its debt obligations with \
                   respect to ELN-02 and ELN-03, and that the refinanced indebtedness received an \
                   investment grade credit rating of A3. These ratings compare favorably to \
                   CoreWeave Parent's credit rating of BB.\" The filings include two \
                   Unconditional Springing Guaranties of Payment and Performance from CoreWeave \
                   Parent and a $50,000,000 letter of credit, described as \"credit enhancement\" \
                   because the landlord found the tenant's standalone credit insufficient.",
        magnitude: "250 MW of datacentre capacity (100 MW + 150 MW) leased to CoreWeave, against \
                    which the LANDLORD borrowed on the strength of its own 9.250% notes due 2030. \
                    CoreWeave Parent's own credit rating is BB (speculative grade); the $50M letter \
                    of credit and two springing guaranties exist to bridge that gap.",
        falsifier: "Shown wrong if the refinanced debt's A3 rating genuinely reflects the credit of \
                    the leases rather than the structure, since an investment-grade rating on the \
                    financing means the market is NOT pricing this as speculative. CoreWeave's own \
                    credit rating is BB — a stated two-notch gap between the tenant and the debt \
                    raised against its leases, which is the fact this entry rests on. Also falsified \
                    if CoreWeave's rating improves, closing the gap.",
    },
    VerifiedEdge {
        filer: "NVDA",
        counterparty: "(capital providers)",
        structure: Structure::VendorOrganisedThirdPartyCapital,
        scale: Scale::Severe,
        citation: "NVIDIA Q2 FY2027 earnings release, Form 8-K Exhibit 99.1 filed 2026-08-26, \
                   verbatim: \"Announced strategic partnerships to establish independent compute \
                   financing platforms with Apollo, BlackRock, Blackstone, Brookfield, Goldman \
                   Sachs and KKR to mobilize over $500 billion of third-party capital for the \
                   buildout of AI infrastructure over time, subject to definitive agreements.\" \
                   The same release records the demand this capital would serve: \"the NVIDIA Vera \
                   Rubin platform is ramping into full production with racks running at partners \
                   including CoreWeave, Google Cloud, Microsoft Azure, Oracle Cloud Infrastructure \
                   and Nebius.\"",
        magnitude: "OVER $500 BILLION of third-party capital, from SIX NAMED institutions \
                    (Apollo, BlackRock, Blackstone, Brookfield, Goldman Sachs, KKR). This is the \
                    largest single figure anywhere in this research — larger than NVIDIA's own \
                    $279B supply commitments and its $105B guarantee combined.",
        falsifier: "THE WEAKEST COMMITMENT IN THE SET, and it is marked as such: NVIDIA states it \
                    is \"subject to definitive agreements\", so this is an ANNOUNCEMENT, not a \
                    signed obligation, and it may not close. Shown wrong if the platforms are \
                    never established, if the capital is raised but deployed to non-NVIDIA \
                    infrastructure, or if the $500B is a cumulative multi-year aspiration rather \
                    than committed capacity. It is also NOT NVIDIA's own money: the balance-sheet \
                    risk sits with the institutions, which is precisely why it is ranked below a \
                    signed supply commitment despite being larger.",
    },
    VerifiedEdge {
        filer: "NVDA",
        counterparty: "(upstream supply chain)",
        structure: Structure::SupplyCommitmentScaledToFinancedDemand,
        scale: Scale::Severe,
        citation: "NVIDIA FY2027 Q2 10-Q (filed 2026-08-26), Note 8 commitments, verbatim: \
                   \"We have significantly increased our supply and capacity commitments from \
                   $119 billion last quarter to $279 billion as of July 26, 2026 to meet future \
                   demand.\" And on the financing side: \"In August 2026, we entered into \
                   memoranda of understanding with large capital providers regarding independent \
                   financing platforms through which the providers would raise and deploy \
                   third-party capital for the buildout of AI infrastructure.\"",
        magnitude: "$279 BILLION of supply and capacity commitments as of 2026-07-26, up from \
                    $119 billion the prior quarter — an increase of $160 BILLION (+134%) in a \
                    single quarter. This is the largest single figure in the model and it EXCEEDS \
                    the $105B guarantee. The capital-provider MOUs are NOT quantified, and NVIDIA \
                    states they 'may not lead to definitive agreements.'",
        falsifier:
            "Shown wrong if the supply commitments are cancellable or reschedulable without \
                    penalty — NVIDIA states elsewhere that 'these agreements may be cancelable, \
                    rescheduled, or adjustable for our business needs prior to placing firm \
                    orders', which would make this inventory flexibility rather than a fixed \
                    obligation — or if the demand materialises so the supply is consumed as \
                    planned. The counter-fact is specific and material, and it is why this edge is \
                    recorded with its own falsifier rather than folded into the guarantee.",
    },
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
    // THE ANONYMITY FINDING — not a circularity, and recorded because it is the LIMIT of what
    // this method can see. Oracle's FY2026 10-K reports RPO of $638 billion (up from $138
    // billion) and attributes it to "certain significant cloud contracts" WITHOUT NAMING ANY
    // COUNTERPARTY. The names that do appear — OpenAI, Anthropic, Microsoft, Meta — are in
    // COMPETITOR lists and model-integration lists, not customer disclosures.
    //
    // This is the largest contract in the industry and it is deliberately anonymous. Oracle is
    // compliant: its 10-K states "No single customer accounted for 10% or more of our total
    // revenues". But the method cannot see the relationship, and no amount of reading will fix
    // that, because the identity is not in the filing.
    VerifiedEdge {
        filer: "ORCL",
        counterparty: "(backlog counterparty — not named)",
        structure: Structure::Refuted,
        scale: Scale::Undisclosed,
        citation:
            "Oracle FY2026 10-K (filed 2026-06-22): \"Remaining performance obligations were \
                   $638 billion and $138 billion as of May 31, 2026 and 2025, respectively. The \
                   increase in remaining performance obligations as of May 31, 2026 in comparison \
                   to May 31, 2025 was primarily attributable to certain significant cloud \
                   contracts that were entered into during the period.\" NO COUNTERPARTY IS \
                   NAMED. Separately: \"No single customer accounted for 10% or more of our total \
                   revenues in fiscal 2026, 2025 or 2024.\"",
        magnitude:
            "$638 BILLION of RPO, up from $138 billion — a 362% increase in one year — with \
                    no counterparty attributed. This is the largest single financial figure \
                    encountered anywhere in this research and the model CANNOT attribute it.",
        falsifier:
            "Not falsifiable in the usual sense, and that is the point: it is a LIMIT rather \
                    than a claim. It would be RESOLVED if Oracle named the counterparty in a \
                    later filing, or if a counterparty disclosed its own side — which is how the \
                    CoreWeave links were established, from the other end of the relationship. \
                    Read as: the model is blind here, by the filer's design and with full \
                    compliance.",
    },
    // SECOND REFUTATION, and a DIFFERENT KIND from the first. Oracle's was a model-integration
    // list; this is a COMPETITOR list. Both produce a hit and neither is a relationship, which is
    // the whole reason the rubric requires a human to read and classify rather than scoring counts.
    //
    // The pattern is worth recording because it will recur: in a market where everyone builds AI,
    // every issuer names its rivals in the risk factors. A detector that scored mentions would
    // score competitors as counterparties.
    VerifiedEdge {
        filer: "NVDA",
        counterparty: "Tesla",
        structure: Structure::Refuted,
        scale: Scale::Undisclosed,
        citation: "NVIDIA FY2026 10-K: Tesla appears ONLY in the competition risk factor, listed \
                   among \"companies with internal teams designing SoC products for their own \
                   products and services, such as Tesla, Inc.\" alongside AMD, Broadcom, Intel, \
                   Qualcomm, Renesas and Samsung. That is a list of RIVALS, not counterparties. \
                   Tesla is named three times across NVIDIA's 10-Ks and never in a commercial, \
                   investment or related-party context.",
        magnitude: "none attributable to a circular structure — the mentions are a competitor list.",
        falsifier: "Shown wrong if a future filing names Tesla as a customer, supplier or investee \
                    in a commercial context rather than in the risk factors. At present the mention \
                    carries no commercial content at all.",
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

/// Mean severity per read edge, as a percentage of the maximum a single edge can express.
///
/// THE SCORED QUANTITY. Effort-independent (adding an average edge leaves it flat) and
/// market-facing (a severe edge raises it, a refutation lowers it).
pub fn mean_severity_pct() -> f64 {
    if VERIFIED.is_empty() {
        return 0.0;
    }
    (rubric_total() / VERIFIED.len() as f64) / rubric_ceiling() * 100.0
}

/// The rubric total. Pure function of the verified set.
pub fn rubric_total() -> f64 {
    VERIFIED
        .iter()
        .map(|e| e.structure.points() + e.scale.points())
        .sum()
}

/// How many edges the scan knows about but which have not been read and classified.
///
/// THE PROBLEM THIS SOLVES. The scored quantity is the fraction of the expressible scale, and
/// the ceiling was `max_points * VERIFIED.len()` — the number of edges READ. Every time an edge
/// is read it left the denominator and entered the numerator, so the fraction rose as a
/// function of READING EFFORT rather than of what the market is doing. Left alone, the composite
/// would drift upward every time someone did more work, which is a measurement artifact and not
/// a fact about the AI economy.
///
/// The measured scan finds 47 real ecosystem edges: 67 hits, minus 2 SELF-MATCHES ("NVDA x
/// NVIDIA" and "CRWV x CoreWeave" are companies naming THEMSELVES) and 18 generic-supplier
/// mentions where the name is a product or component reference rather than a counterparty.
///
/// This was 48 in an earlier draft, from a filter that caught only the NVIDIA self-match and
/// missed CoreWeave's. The error was found by re-deriving the count with a ticker-to-name map
/// instead of an ad-hoc exclusion list. Recorded because the comment IS the audit trail for a
/// stated number, and a wrong audit trail is worse than no number. Using that as the denominator means reading an edge moves points into
/// the numerator while the denominator stays put — so the reading reflects what was FOUND rather
/// than how hard anyone looked.
///
/// A REFUTATION STILL LOWERS THE SCORE. It contributes 0 points, so it dilutes the mean toward
/// zero — which is the property that makes this a measurement rather than an alarm. Tested.
///
/// Reported as COVERAGE, separately from the score, because "how much has been verified" is a
/// confidence statement about the instrument and not a measure of market stress.
pub const SCANNED_EDGES: usize = 47;
pub fn rubric_ceiling() -> f64 {
    // Uses the TRUE maximum structure rather than a named variant, so adding a stronger
    // structure in future raises the ceiling automatically instead of silently understating
    // how much of the scale the present evidence occupies.
    let max = [
        Structure::SupplyCommitmentScaledToFinancedDemand,
        Structure::SupplierHoldingCustomerEquity,
        Structure::VendorSublicensingItsOwnLeaseLiability,
        Structure::LandlordDebtSecuredByTenantCredit,
        Structure::VendorOrganisedThirdPartyCapital,
        Structure::VendorFinancingItsOwnCustomer,
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
    // THE CEILING IS THE MAXIMUM A SINGLE EDGE CAN EXPRESS.
    //
    // Not a multiple of the read count, and not a multiple of the scanned count. Both of those
    // were tried and both are wrong:
    //
    //   * `max * VERIFIED.len()` (read count) makes the reading rise from READING EFFORT —
    //     every edge read left the denominator and entered the numerator;
    //   * `max * SCANNED_EDGES` measures COVERAGE, not stress. It answers "how much of the
    //     possible exposure have you verified", which is a CONFIDENCE question, and it buried a
    //     genuine 65% reading down to 12% purely because most edges are unread.
    //
    // The quantity that is both effort-independent AND about the market is the MEAN severity per
    // read edge, expressed against the maximum one edge can express. Reading an average edge
    // leaves it flat; reading a severe edge raises it; reading a refutation lowers it. Coverage
    // is reported separately, as the confidence caveat it actually is.
    max + Scale::Severe.points()
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
    fn a_small_position_is_recorded_as_a_relationship_not_a_magnitude() {
        // THE ENTRY SCORES LOW ON PURPOSE. NVIDIA's CoreWeave gain was "not significant", and a
        // rubric that ranked this alongside a $105B guarantee would be a size contest. What makes
        // it worth recording is the RELATIONSHIP — selling GPUs to a company you own — and the
        // entry must say so rather than implying a large exposure.
        let e = VERIFIED
            .iter()
            .find(|e| e.structure == Structure::SupplierHoldingCustomerEquity)
            .expect("the NVDA/CoreWeave edge must be present");
        assert!(
            e.magnitude.contains("not significant") || e.magnitude.contains("RELATIONSHIP"),
            "the entry must state that the magnitude is small and the relationship is the point"
        );
        assert!(
            e.falsifier.contains("immaterial") || e.falsifier.contains("passive"),
            "and must name the strongest counter: a small passive holding is not a structure"
        );
        // Ranked below the guarantee and the supply commitment, since its disclosed gain was not
        // material.
        assert!(
            Structure::SupplierHoldingCustomerEquity.points()
                < Structure::SupplierGuaranteeAndInvestment.points(),
            "a non-material position must rank below a $105B guarantee"
        );
    }

    #[test]
    fn the_sublicense_records_that_the_liability_was_not_transferred() {
        // THE WORD IS THE FINDING. "Sublicense" leaves SMCI liable; "assign" would have released
        // it. The entry rests on that distinction plus Item 2.03's off-balance-sheet framing, so
        // both must be in the record or the entry is just "a vendor leased a building".
        let e = VERIFIED
            .iter()
            .find(|e| e.structure == Structure::VendorSublicensingItsOwnLeaseLiability)
            .expect("the SMCI edge must be present");
        assert!(
            e.citation
                .contains("sublicense all of the Company's rights"),
            "the sublicense language must be quoted"
        );
        assert!(
            e.citation.contains("right to seek reimbursement")
                || e.citation.contains("right of reimbursement"),
            "and the reimbursement right, which is what shows liability was retained: {}",
            e.citation
        );
        assert!(
            e.citation.contains("Off-Balance Sheet") || e.citation.contains("off-balance"),
            "and the Item 2.03 off-balance-sheet disclosure"
        );
        assert!(
            e.magnitude.contains("$600.0 MILLION"),
            "the size must be stated"
        );
        assert!(
            e.magnitude.contains("PRIVATE"),
            "and that Lambda is private"
        );
        // It ranks above the landlord structure: the landlord owns the asset, SMCI owns nothing.
        assert!(
            Structure::VendorSublicensingItsOwnLeaseLiability.points()
                > Structure::LandlordDebtSecuredByTenantCredit.points()
        );
    }

    #[test]
    fn the_landlord_structure_records_the_rating_gap_that_justifies_it() {
        // The entry's whole claim rests on a STATED two-notch gap: the tenant is rated BB, and the
        // debt raised against its leases received A3. If that gap is not in the record, the entry
        // is just "a landlord has a tenant", which is not a circular structure at all.
        let e = VERIFIED
            .iter()
            .find(|e| e.structure == Structure::LandlordDebtSecuredByTenantCredit)
            .expect("the APLD edge must be present");
        assert!(
            e.citation.contains("BB"),
            "the tenant's speculative rating must be stated"
        );
        assert!(
            e.citation.contains("A3"),
            "and the rating on the refinanced debt"
        );
        assert!(
            e.citation.to_lowercase().contains("credit enhancement")
                || e.citation.contains("letter of credit"),
            "and the credit support that exists because the tenant alone was insufficient"
        );
        assert!(
            e.magnitude.contains("250 MW"),
            "the capacity must be quantified"
        );
        // It must rank below the hyperscaler structures: smaller magnitude, and the exposure
        // lands on bondholders rather than on a named technology company.
        assert!(
            Structure::LandlordDebtSecuredByTenantCredit.points()
                < Structure::VendorFinancingItsOwnCustomer.points()
        );
    }

    #[test]
    fn the_largest_figure_does_not_outrank_a_signed_commitment() {
        // $500B of third-party capital is the biggest number in the research, and it is ranked
        // BELOW NVIDIA's $279B supply commitment — because it is an ANNOUNCEMENT ("subject to
        // definitive agreements") rather than a signed obligation, and because the balance-sheet
        // risk sits with the institutions rather than with NVIDIA. Size must not decide rank.
        assert!(
            Structure::VendorOrganisedThirdPartyCapital.points()
                < Structure::SupplyCommitmentScaledToFinancedDemand.points(),
            "the announced $500B must rank below a signed $279B commitment"
        );
        let e = VERIFIED
            .iter()
            .find(|e| e.structure == Structure::VendorOrganisedThirdPartyCapital)
            .expect("the capital-provider edge must be present");
        assert!(
            e.magnitude.contains("$500 BILLION"),
            "the size must be stated"
        );
        assert!(
            e.falsifier.contains("subject to definitive agreements"),
            "the qualifier MUST be recorded, or the largest number would read as committed: {}",
            e.falsifier
        );
        assert!(
            e.falsifier.contains("ANNOUNCEMENT"),
            "and it must be labelled an announcement"
        );
    }

    #[test]
    fn refutations_cover_more_than_one_kind_of_false_positive() {
        // A detector's failure modes are not all the same, and a refutation set that only proves
        // one of them is only half a check. The set must show at least TWO distinct kinds of
        // mention-that-is-not-a-relationship: a MODEL list (Oracle naming OpenAI as an integration
        // option) and a COMPETITOR list (NVIDIA naming Tesla among rivals). In a market where every
        // issuer discusses AI, both will recur.
        let refuted: Vec<_> = VERIFIED
            .iter()
            .filter(|e| e.structure == Structure::Refuted)
            .collect();
        assert!(
            refuted.len() >= 2,
            "at least two refutations are needed to cover two failure modes"
        );
        assert!(
            refuted
                .iter()
                .any(|e| e.citation.to_lowercase().contains("model")),
            "a model-list refutation must be present"
        );
        assert!(
            refuted
                .iter()
                .any(|e| e.citation.to_lowercase().contains("compet")
                    || e.citation.to_lowercase().contains("rival")),
            "and a competitor-list refutation"
        );
        // Every refutation contributes nothing.
        for e in &refuted {
            assert_eq!(
                e.structure.points(),
                0.0,
                "{} must contribute nothing",
                e.counterparty
            );
        }
    }

    #[test]
    fn a_blind_spot_lowers_the_reading_rather_than_raising_it() {
        // The ORCL anonymity finding is the largest number in the research ($638B) and it
        // contributes NOTHING, because a figure with no counterparty cannot be scored as a
        // circular structure. It is recorded as a Refuted-class entry so that it DILUTES the
        // mean. If the largest number in the dataset could raise the score simply by being
        // large, this would be a size contest rather than a measurement.
        let blind = VERIFIED
            .iter()
            .find(|e| e.counterparty.contains("not named"))
            .expect("the anonymity finding must be recorded");
        assert_eq!(blind.structure, Structure::Refuted);
        assert_eq!(blind.structure.points(), 0.0, "it must contribute nothing");
        assert!(
            blind.magnitude.contains("$638 BILLION"),
            "the size must be stated"
        );
        // And it must say the method is blind, not that the arrangement is absent.
        assert!(
            blind.falsifier.contains("LIMIT") || blind.falsifier.contains("blind"),
            "must read as a coverage limit: {}",
            blind.falsifier
        );
    }

    #[test]
    fn the_microsoft_openai_coreweave_chain_is_fully_representable() {
        // The clearest COMPLETE LOOP found: Microsoft invests in OpenAI; OpenAI commits $18.4B
        // to CoreWeave; CoreWeave buys NVIDIA GPUs; and 67% of CoreWeave's revenue IS Microsoft.
        // Every link is in a filing, and all four must be present or the loop cannot be read.
        let has = |f: &str, c: &str| VERIFIED.iter().any(|e| e.filer == f && e.counterparty == c);
        assert!(has("MSFT", "OpenAI"), "Microsoft -> OpenAI");
        assert!(has("CRWV", "OpenAI"), "OpenAI -> CoreWeave");
        assert!(has("CRWV", "MSFT"), "CoreWeave's revenue is Microsoft");
        assert!(has("NVDA", "OpenAI"), "NVIDIA in the same loop");
        // And the CoreWeave figures must be exact, because a rounded commitment would make the
        // loop look smaller than it is.
        let cw = VERIFIED
            .iter()
            .find(|e| e.filer == "CRWV" && e.counterparty == "OpenAI")
            .unwrap();
        assert!(
            cw.magnitude.contains("18.4"),
            "the total must be stated: {}",
            cw.magnitude
        );
        let ms = VERIFIED
            .iter()
            .find(|e| e.filer == "CRWV" && e.counterparty == "MSFT")
            .unwrap();
        assert!(
            ms.magnitude.contains("67%"),
            "the concentration must be stated"
        );
    }

    #[test]
    fn the_complete_loop_is_recognised_where_all_three_flows_share_one_counterparty() {
        // Amazon invests in the lab, contracts to sell it compute, AND lends to it — with the
        // facility drawable only as Amazon hits compute delivery milestones. Three flows, two
        // parties. That is structurally distinct from a supply commitment, and it is why it is
        // its own class rather than folded into one.
        let amzn: Vec<_> = VERIFIED.iter().filter(|e| e.filer == "AMZN").collect();
        assert_eq!(amzn.len(), 2, "both AMZN counterparties must be recorded");
        for e in &amzn {
            assert_eq!(e.structure, Structure::VendorFinancingItsOwnCustomer);
            assert!(
                e.magnitude.contains("BILLION"),
                "both must carry a magnitude: {}",
                e.magnitude
            );
        }
        // The Anthropic edge specifically must record the milestone gate, because that is what
        // ties the CREDIT to the vendor's own performance rather than the counterparty's credit.
        let a = amzn.iter().find(|e| e.counterparty == "Anthropic").unwrap();
        assert!(
            a.citation.contains("delivery milestones") || a.citation.contains("milestone"),
            "the compute-delivery gate must be recorded: {}",
            a.citation
        );
        assert!(
            a.citation.contains("Level 3"),
            "and the Level 3 classification, since these are the filer's own valuations"
        );
    }

    #[test]
    fn the_upstream_amplifier_ranks_above_the_guarantee() {
        // The supply commitment is larger than the guarantee AND is not contingent on a
        // default — a guarantee becomes payable if something goes wrong, whereas a supply
        // commitment has already been made. The ordering says so.
        assert!(
            Structure::SupplyCommitmentScaledToFinancedDemand.points()
                > Structure::SupplierGuaranteeAndInvestment.points()
        );
        let e = VERIFIED
            .iter()
            .find(|e| e.structure == Structure::SupplyCommitmentScaledToFinancedDemand)
            .expect("the supply-commitment edge must be present");
        assert!(
            e.magnitude.contains("$279 BILLION"),
            "the size must be stated: {}",
            e.magnitude
        );
        // And the counter-fact must be recorded, because NVIDIA itself says the agreements may
        // be cancellable — which is the difference between a fixed obligation and flexibility.
        assert!(
            e.falsifier.contains("cancelable") || e.falsifier.contains("cancelable"),
            "the cancellability counter-fact must be recorded: {}",
            e.falsifier
        );
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
        // And the largest exposure found is marked severe. TARGETED BY STRUCTURE, not by filer:
        // NVIDIA now has THREE entries, so "the first NVDA edge" silently became the CoreWeave
        // equity position (correctly Small) and this assertion failed for the wrong reason. Same
        // bug class as the earlier counter-fact test, which is why the lookup is by structure
        // wherever the structure is what the test is about.
        let nvda = VERIFIED
            .iter()
            .find(|e| e.filer == "NVDA" && e.structure == Structure::SupplierGuaranteeAndInvestment)
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
        // Target the GUARANTEE edge specifically: NVIDIA now has two entries, and finding
        // "the first NVDA edge" would silently test the wrong one after any reordering.
        let n = VERIFIED
            .iter()
            .find(|e| e.filer == "NVDA" && e.structure == Structure::SupplierGuaranteeAndInvestment)
            .expect("the guarantee edge must exist");
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
            Structure::SupplyCommitmentScaledToFinancedDemand.points()
                > Structure::SupplierGuaranteeAndInvestment.points(),
            "the upstream amplifier outranks the guarantee"
        );
        assert!(
            Structure::SupplyCommitmentScaledToFinancedDemand.points()
                > Structure::VendorFinancingItsOwnCustomer.points(),
            "the largest single commitment outranks the complete loop on size"
        );
        assert!(
            Structure::VendorFinancingItsOwnCustomer.points()
                > Structure::SupplierGuaranteeAndInvestment.points(),
            "a vendor financing its own customer outranks a guarantee"
        );
        assert!(
            Structure::SupplierGuaranteeAndInvestment.points()
                > Structure::EquityForPurchases.points()
        );
        assert!(Structure::EquityForPurchases.points() > Structure::AbsorbedUnitDebt.points());
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
        assert!(t > 0.0 && c > 0.0, "total {} ceiling {}", t, c);
        // With the ceiling being the maximum a SINGLE edge can express, the mean severity is
        // bounded 0-100 by construction and the present evidence sits in the upper-middle: real
        // severe structures are present, and there is headroom above.
        let m = mean_severity_pct();
        assert!(
            (0.4..=1.0).contains(&(m / 100.0)),
            "mean severity should occupy a middle-to-high part of the scale, got {:.1}%",
            m
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
        let n = VERIFIED.len() as f64;
        let max = rubric_ceiling();
        let before = mean_severity_pct();

        // An AVERAGE edge leaves the mean exactly flat, because it IS the mean.
        let avg = t / n;
        let after_avg = ((t + avg) / (n + 1.0)) / max * 100.0;
        assert!(
            (after_avg - before).abs() < 1e-9,
            "an average edge must leave the mean exactly flat: {:.4} -> {:.4}",
            before,
            after_avg
        );

        // A REFUTATION (0 points) LOWERS the mean, because it dilutes toward zero.
        let after_refutation = (t / (n + 1.0)) / max * 100.0;
        assert!(
            after_refutation < before,
            "a refutation must LOWER the mean: {:.4} -> {:.4}",
            before,
            after_refutation
        );

        // A maximum-strength edge RAISES it.
        let after_max = ((t + max) / (n + 1.0)) / max * 100.0;
        assert!(
            after_max > before,
            "a severe edge must RAISE the mean: {:.4} -> {:.4}",
            before,
            after_max
        );

        // And the property is EFFORT-INDEPENDENT: appending any number of average edges cannot
        // move it, which is what stops the composite drifting up as more edges are read.
        let many = ((t + avg * 30.0) / (n + 30.0)) / max * 100.0;
        assert!(
            (many - before).abs() < 1e-9,
            "reading thirty average edges must not move the reading: {:.4} -> {:.4}",
            before,
            many
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
        assert!(
            VERIFIED
                .iter()
                .any(|e| e.filer == "NVDA"
                    && e.structure == Structure::SupplierGuaranteeAndInvestment)
        );
    }
}
