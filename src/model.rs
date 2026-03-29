use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub scenario_name: String,
    pub simulations: usize,
    pub accumulation_years: usize,
    pub retirement_years: usize,
    pub initial_portfolio: f64,
    pub annual_contribution: f64,
    pub annual_contribution_real_growth: f64,
    pub annual_spending: f64,
    pub annual_spending_real_growth: f64,
    pub ruin_threshold: f64,
    pub inflation_rate: f64,
    pub inflation_volatility: f64,
    pub crash_chance: f64,
    pub crash_equity_return_shift: f64,
    pub crash_bond_return_shift: f64,
    pub tail_loss_chance: f64,
    pub tail_equity_return_shift: f64,
    pub tail_bond_return_shift: f64,
    pub allocation: PortfolioAllocation,
    pub assets: CapitalMarketAssumptions,
    pub correlations: CorrelationAssumptions,
    pub volatility_model: VolatilityModel,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            scenario_name: "Balanced retirement funding plan".to_string(),
            simulations: 25_000,
            accumulation_years: 15,
            retirement_years: 25,
            initial_portfolio: 550_000.0,
            annual_contribution: 26_000.0,
            annual_contribution_real_growth: 0.01,
            annual_spending: 50_000.0,
            annual_spending_real_growth: 0.005,
            ruin_threshold: 100_000.0,
            inflation_rate: 0.024,
            inflation_volatility: 0.012,
            crash_chance: 0.07,
            crash_equity_return_shift: -0.28,
            crash_bond_return_shift: 0.04,
            tail_loss_chance: 0.03,
            tail_equity_return_shift: -0.14,
            tail_bond_return_shift: -0.04,
            allocation: PortfolioAllocation {
                stocks: 0.65,
                bonds: 0.30,
                cash: 0.05,
            },
            assets: CapitalMarketAssumptions {
                stocks: AssetClassAssumption {
                    expected_return: 0.082,
                    volatility: 0.18,
                },
                bonds: AssetClassAssumption {
                    expected_return: 0.043,
                    volatility: 0.07,
                },
                cash: AssetClassAssumption {
                    expected_return: 0.026,
                    volatility: 0.012,
                },
            },
            correlations: CorrelationAssumptions {
                stock_bond: 0.18,
                stock_cash: 0.05,
                bond_cash: 0.25,
            },
            volatility_model: VolatilityModel {
                persistence: 0.45,
                shock_sensitivity: 0.18,
                crash_boost: 0.50,
                tail_boost: 0.18,
                max_multiplier: 2.60,
                extreme_shock_chance: 0.08,
                extreme_shock_scale: 1.55,
            },
        }
    }
}

impl SimulationConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.simulations < 100 {
            return Err("simulations must be at least 100".to_string());
        }
        if self.accumulation_years + self.retirement_years < 1 {
            return Err("total modeled years must be at least 1".to_string());
        }
        if self.initial_portfolio < 0.0 {
            return Err("initial_portfolio cannot be negative".to_string());
        }
        if self.annual_contribution < 0.0 {
            return Err("annual_contribution cannot be negative".to_string());
        }
        if self.annual_spending < 0.0 {
            return Err("annual_spending cannot be negative".to_string());
        }
        if self.ruin_threshold < 0.0 {
            return Err("ruin_threshold cannot be negative".to_string());
        }
        if !(0.0..=1.0).contains(&self.crash_chance) {
            return Err("crash_chance must be between 0 and 1".to_string());
        }
        if !(0.0..=1.0).contains(&self.tail_loss_chance) {
            return Err("tail_loss_chance must be between 0 and 1".to_string());
        }

        validate_return_shift(self.crash_equity_return_shift, "crash_equity_return_shift")?;
        validate_return_shift(self.crash_bond_return_shift, "crash_bond_return_shift")?;
        validate_return_shift(self.tail_equity_return_shift, "tail_equity_return_shift")?;
        validate_return_shift(self.tail_bond_return_shift, "tail_bond_return_shift")?;

        self.allocation.validate()?;
        self.assets.validate()?;
        self.correlations.validate()?;
        self.volatility_model.validate()?;

        Ok(())
    }

    pub fn total_years(&self) -> usize {
        self.accumulation_years + self.retirement_years
    }

    pub fn contribution_for_year(&self, year: usize, cumulative_inflation: f64) -> f64 {
        let real_growth = (1.0 + self.annual_contribution_real_growth).powi(year as i32);
        self.annual_contribution * cumulative_inflation * real_growth
    }

    pub fn spending_for_year(&self, retirement_year: usize, cumulative_inflation: f64) -> f64 {
        let real_growth = (1.0 + self.annual_spending_real_growth).powi(retirement_year as i32);
        self.annual_spending * cumulative_inflation * real_growth
    }
}

fn validate_return_shift(value: f64, label: &str) -> Result<(), String> {
    if !(-0.95..=0.95).contains(&value) {
        return Err(format!("{label} must stay between -95% and 95%"));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PortfolioAllocation {
    pub stocks: f64,
    pub bonds: f64,
    pub cash: f64,
}

impl PortfolioAllocation {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("allocation.stocks", self.stocks),
            ("allocation.bonds", self.bonds),
            ("allocation.cash", self.cash),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(format!("{label} must be between 0 and 1"));
            }
        }

        let total = self.stocks + self.bonds + self.cash;
        if (total - 1.0).abs() > 0.000_1 {
            return Err(format!(
                "allocation weights must sum to 1.0, but sum to {:.4}",
                total
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AssetClassAssumption {
    pub expected_return: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CapitalMarketAssumptions {
    pub stocks: AssetClassAssumption,
    pub bonds: AssetClassAssumption,
    pub cash: AssetClassAssumption,
}

impl CapitalMarketAssumptions {
    pub fn validate(&self) -> Result<(), String> {
        for (label, assumption) in [
            ("assets.stocks", self.stocks),
            ("assets.bonds", self.bonds),
            ("assets.cash", self.cash),
        ] {
            if assumption.volatility < 0.0 {
                return Err(format!("{label}.volatility cannot be negative"));
            }
            if !(-0.50..=1.00).contains(&assumption.expected_return) {
                return Err(format!(
                    "{label}.expected_return should stay in a plausible annual range"
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CorrelationAssumptions {
    pub stock_bond: f64,
    pub stock_cash: f64,
    pub bond_cash: f64,
}

impl CorrelationAssumptions {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("correlations.stock_bond", self.stock_bond),
            ("correlations.stock_cash", self.stock_cash),
            ("correlations.bond_cash", self.bond_cash),
        ] {
            if !(-1.0..=1.0).contains(&value) {
                return Err(format!("{label} must be between -1 and 1"));
            }
        }

        self.cholesky()
            .map(|_| ())
            .map_err(|_| "correlations must define a positive semidefinite matrix".to_string())
    }

    pub fn cholesky(&self) -> Result<[[f64; 3]; 3], String> {
        let matrix = [
            [1.0, self.stock_bond, self.stock_cash],
            [self.stock_bond, 1.0, self.bond_cash],
            [self.stock_cash, self.bond_cash, 1.0],
        ];

        let mut lower = [[0.0; 3]; 3];
        for row in 0..3 {
            for col in 0..=row {
                let mut value = matrix[row][col];
                for inner in 0..col {
                    value -= lower[row][inner] * lower[col][inner];
                }

                if row == col {
                    if value < -1e-10 {
                        return Err("matrix is not positive semidefinite".to_string());
                    }
                    lower[row][col] = value.max(0.0).sqrt();
                } else {
                    if lower[col][col].abs() < 1e-12 {
                        return Err("matrix is singular".to_string());
                    }
                    lower[row][col] = value / lower[col][col];
                }
            }
        }

        Ok(lower)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VolatilityModel {
    pub persistence: f64,
    pub shock_sensitivity: f64,
    pub crash_boost: f64,
    pub tail_boost: f64,
    pub max_multiplier: f64,
    pub extreme_shock_chance: f64,
    pub extreme_shock_scale: f64,
}

impl VolatilityModel {
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=0.99).contains(&self.persistence) {
            return Err("volatility_model.persistence must be between 0 and 0.99".to_string());
        }
        if self.shock_sensitivity < 0.0 {
            return Err("volatility_model.shock_sensitivity cannot be negative".to_string());
        }
        if self.crash_boost < 0.0 {
            return Err("volatility_model.crash_boost cannot be negative".to_string());
        }
        if self.tail_boost < 0.0 {
            return Err("volatility_model.tail_boost cannot be negative".to_string());
        }
        if self.max_multiplier < 1.0 {
            return Err("volatility_model.max_multiplier must be at least 1.0".to_string());
        }
        if !(0.0..=1.0).contains(&self.extreme_shock_chance) {
            return Err(
                "volatility_model.extreme_shock_chance must be between 0 and 1".to_string(),
            );
        }
        if self.extreme_shock_scale < 1.0 {
            return Err("volatility_model.extreme_shock_scale must be at least 1.0".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MarketRegime {
    Expansion,
    Steady,
    Slowdown,
    Stagflation,
    Recovery,
}

impl MarketRegime {
    pub fn adjustments(self) -> RegimeAdjustments {
        match self {
            Self::Expansion => RegimeAdjustments {
                stock_return_shift: 0.025,
                bond_return_shift: -0.004,
                cash_return_shift: 0.0,
                stock_vol_multiplier: 0.80,
                bond_vol_multiplier: 0.90,
                inflation_shift: -0.003,
            },
            Self::Steady => RegimeAdjustments {
                stock_return_shift: 0.0,
                bond_return_shift: 0.0,
                cash_return_shift: 0.0,
                stock_vol_multiplier: 1.0,
                bond_vol_multiplier: 1.0,
                inflation_shift: 0.0,
            },
            Self::Slowdown => RegimeAdjustments {
                stock_return_shift: -0.03,
                bond_return_shift: 0.01,
                cash_return_shift: 0.002,
                stock_vol_multiplier: 1.15,
                bond_vol_multiplier: 1.10,
                inflation_shift: 0.004,
            },
            Self::Stagflation => RegimeAdjustments {
                stock_return_shift: -0.06,
                bond_return_shift: -0.025,
                cash_return_shift: 0.003,
                stock_vol_multiplier: 1.30,
                bond_vol_multiplier: 1.20,
                inflation_shift: 0.015,
            },
            Self::Recovery => RegimeAdjustments {
                stock_return_shift: 0.015,
                bond_return_shift: 0.003,
                cash_return_shift: 0.0,
                stock_vol_multiplier: 1.05,
                bond_vol_multiplier: 0.95,
                inflation_shift: -0.001,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RegimeAdjustments {
    pub stock_return_shift: f64,
    pub bond_return_shift: f64,
    pub cash_return_shift: f64,
    pub stock_vol_multiplier: f64,
    pub bond_vol_multiplier: f64,
    pub inflation_shift: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathOutcome {
    pub ending_nominal: f64,
    pub ending_real: f64,
    pub worst_drawdown: f64,
    pub geometric_real_return: f64,
    pub failed: bool,
    pub depleted: bool,
    pub floor_breached: bool,
    pub crash_years: usize,
    pub shortfall_years: usize,
    pub yearly_real_values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YearBand {
    pub year: usize,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationReport {
    pub simulations: usize,
    pub years: usize,
    pub elapsed_millis: u128,
    pub mean_ending_real: f64,
    pub p10_ending_real: f64,
    pub p50_ending_real: f64,
    pub p90_ending_real: f64,
    pub best_case_real: f64,
    pub worst_case_real: f64,
    pub failure_probability: f64,
    pub depletion_probability: f64,
    pub floor_breach_probability: f64,
    pub average_shortfall_years: f64,
    pub average_worst_drawdown: f64,
    pub average_crash_years: f64,
    pub average_geometric_real_return: f64,
    pub histogram: Vec<(f64, usize)>,
    pub yearly_bands: Vec<YearBand>,
}

#[cfg(test)]
mod tests {
    use super::{CorrelationAssumptions, SimulationConfig};

    #[test]
    fn rejects_out_of_bounds_probabilities() {
        let mut config = SimulationConfig::default();
        config.crash_chance = 1.5;

        let error = config.validate().expect_err("config should be invalid");
        assert!(error.contains("crash_chance"));
    }

    #[test]
    fn rejects_invalid_allocation_sum() {
        let mut config = SimulationConfig::default();
        config.allocation.stocks = 0.80;

        let error = config.validate().expect_err("config should be invalid");
        assert!(error.contains("sum to 1.0"));
    }

    #[test]
    fn rejects_non_psd_correlation_matrix() {
        let invalid = CorrelationAssumptions {
            stock_bond: 0.95,
            stock_cash: 0.95,
            bond_cash: -0.95,
        };

        let error = invalid.validate().expect_err("matrix should be invalid");
        assert!(error.contains("positive semidefinite"));
    }

    #[test]
    fn rejects_invalid_volatility_model() {
        let mut config = SimulationConfig::default();
        config.volatility_model.extreme_shock_scale = 0.5;

        let error = config
            .validate()
            .expect_err("volatility model should be invalid");
        assert!(error.contains("extreme_shock_scale"));
    }

    #[test]
    fn accepts_default_configuration() {
        let config = SimulationConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.total_years(), 40);
    }
}
