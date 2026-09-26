//! Indicator registry.
//!
//! The *config* decides which indicators exist, their order and their weights.
//! This module only supplies implementations. If the config declares a scored
//! indicator with no implementation, that is a hard error rather than a silent
//! omission — a weight that vanishes without saying so is exactly the kind of
//! quiet dishonesty this project is built to avoid.

pub mod credit;
pub mod fundamentals;
pub mod market;

use crate::config::{Config, IndicatorCfg};
use crate::model::{IndicatorReading, Observations, Reading};

pub struct Ctx<'a> {
    pub obs: &'a Observations,
    pub cfg: &'a Config,
}

pub trait Indicator {
    fn id(&self) -> &'static str;
    fn evaluate(&self, ctx: &Ctx) -> Reading;
}

/// Build the reading record for one configured indicator.
pub fn evaluate_one(ctx: &Ctx, ic: &IndicatorCfg) -> IndicatorReading {
    let reading = match find(&ic.id) {
        Some(ind) => ind.evaluate(ctx),
        None => {
            if ic.weight == 0.0 {
                // Declared blind spot: carried in the report, no effect on score.
                Reading::Unavailable {
                    reason: format!(
                        "DECLARED GAP — weight 0, so this never touches the composite. It is \
                         listed so the blind spot is visible rather than absent. {}",
                        ic.rationale
                    ),
                }
            } else {
                // A weighted indicator with no implementation would silently
                // shrink coverage and misstate confidence. Refuse instead.
                Reading::Unavailable {
                    reason: format!(
                        "IMPLEMENTATION MISSING for weighted indicator '{}' — this is a bug, \
                         not a data gap",
                        ic.id
                    ),
                }
            }
        }
    };

    IndicatorReading {
        id: ic.id.clone(),
        label: ic.label.clone(),
        weight: ic.weight,
        rationale: ic.rationale.clone(),
        reading,
        contribution: None,
    }
}

/// Evaluate every configured indicator, in config order.
pub fn evaluate_all(ctx: &Ctx) -> Vec<IndicatorReading> {
    ctx.cfg
        .indicator
        .iter()
        .map(|ic| evaluate_one(ctx, ic))
        .collect()
}

pub fn find(id: &str) -> Option<Box<dyn Indicator>> {
    match id {
        "valuation_stretch" => Some(Box::new(market::ValuationStretch)),
        "concentration" => Some(Box::new(market::Concentration)),
        "breadth" => Some(Box::new(market::Breadth)),
        "volatility" => Some(Box::new(market::Volatility)),
        "credit_hy" => Some(Box::new(credit::CreditHy)),
        "credit_ig" => Some(Box::new(credit::CreditIg)),
        "private_credit_growth" => Some(Box::new(credit::PrivateCreditGrowth)),
        "foreign_interest" => Some(Box::new(fundamentals::ForeignOwnership)),
        "datacenter_construction" => Some(Box::new(fundamentals::DataCenterConstruction)),
        "grid_cancellations" => Some(Box::new(fundamentals::GridCancellations)),
        "narrative_saturation" => Some(Box::new(fundamentals::NarrativeSaturation)),
        "depreciation_subsidy" => Some(Box::new(fundamentals::DepreciationSubsidy)),
        "frontier_premium" => Some(Box::new(fundamentals::FrontierPremium)),
        "circularity" => Some(Box::new(fundamentals::CircularFinancing)),
        "capex_vs_cashflow" => Some(Box::new(fundamentals::CapexVsCashflow)),
        "funding_gap" => Some(Box::new(fundamentals::FundingGap)),
        "leverage" => Some(Box::new(fundamentals::Leverage)),
        "issuance" => Some(Box::new(fundamentals::Issuance)),
        "primary_market_supply" => Some(Box::new(fundamentals::PrimaryMarketSupply)),
        "backlog_quality" => Some(Box::new(fundamentals::BacklogQuality)),
        "inference_demand" => Some(Box::new(fundamentals::InferenceDemand)),
        "energisation_delay" => Some(Box::new(fundamentals::EnergisationDelay)),
        "opposition_pressure" => Some(Box::new(fundamentals::OppositionPressure)),
        "public_attention" => Some(Box::new(fundamentals::PublicAttention)),
        _ => None,
    }
}

/// Every id this build can actually compute — used by `sources` to warn when
/// the config asks for something the binary does not implement.
pub const IMPLEMENTED: &[&str] = &[
    "valuation_stretch",
    "concentration",
    "breadth",
    "volatility",
    "credit_hy",
    "credit_ig",
    "private_credit_growth",
    "foreign_interest",
    "datacenter_construction",
    "grid_cancellations",
    "narrative_saturation",
    "depreciation_subsidy",
    "frontier_premium",
    "capex_vs_cashflow",
    "funding_gap",
    "leverage",
    "issuance",
    "primary_market_supply",
    "backlog_quality",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_implemented_id_resolves() {
        for id in IMPLEMENTED {
            assert!(
                find(id).is_some(),
                "listed as implemented but not found: {}",
                id
            );
        }
    }

    #[test]
    fn unknown_id_has_no_implementation() {
        assert!(find("definitely_not_an_indicator").is_none());
    }

    #[test]
    fn every_shipped_config_indicator_is_implemented_or_weightless() {
        // Guards against a config/code drift that would silently drop weight.
        let path = std::path::Path::new("config/indicators.toml");
        if !path.exists() {
            return; // unit tests may run from a different cwd
        }
        let cfg = Config::load(path).expect("shipped config must parse");
        for ic in &cfg.indicator {
            if ic.weight > 0.0 {
                assert!(
                    find(&ic.id).is_some(),
                    "weighted indicator '{}' has no implementation",
                    ic.id
                );
            }
        }
    }
}
