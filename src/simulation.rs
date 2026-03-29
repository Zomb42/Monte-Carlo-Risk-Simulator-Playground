use std::time::Instant;

use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::{Distribution, StandardNormal};
use rayon::prelude::*;

use crate::model::{MarketRegime, PathOutcome, SimulationConfig, SimulationReport, YearBand};

pub fn run_parallel(config: &SimulationConfig) -> SimulationReport {
    let start = Instant::now();
    let paths: Vec<PathOutcome> = (0..config.simulations)
        .into_par_iter()
        .map(|index| simulate_path(config, index as u64 + 1))
        .collect();

    build_report(config, paths, start.elapsed().as_millis())
}

pub fn run_sequential(config: &SimulationConfig) -> SimulationReport {
    let start = Instant::now();
    let mut paths = Vec::with_capacity(config.simulations);
    for index in 0..config.simulations {
        paths.push(simulate_path(config, index as u64 + 1));
    }

    build_report(config, paths, start.elapsed().as_millis())
}

fn simulate_path(config: &SimulationConfig, seed: u64) -> PathOutcome {
    let mut rng = StdRng::seed_from_u64(seed.wrapping_mul(9_973));
    let total_years = config.total_years();
    let cholesky = config
        .correlations
        .cholesky()
        .expect("validated correlation matrix");

    let mut nominal = config.initial_portfolio;
    let mut cumulative_inflation = 1.0;
    let mut yearly_real_values = Vec::with_capacity(total_years + 1);
    yearly_real_values.push(config.initial_portfolio);

    let mut peak_real = config.initial_portfolio.max(1.0);
    let mut worst_drawdown: f64 = 0.0;
    let mut crash_years = 0usize;
    let mut shortfall_years = 0usize;
    let mut depleted = false;
    let mut floor_breached = false;
    let mut real_growth_factor = 1.0;
    let mut volatility_state = VolatilityState::default();

    for year in 0..total_years {
        let regime = sample_regime(&mut rng);
        let inflation = sample_inflation(config, regime, &mut rng);

        if year >= config.accumulation_years {
            let retirement_year = year - config.accumulation_years;
            let spending = config.spending_for_year(retirement_year, cumulative_inflation);
            let funded_spending = nominal.min(spending);
            nominal -= funded_spending;

            if funded_spending + 1e-6 < spending {
                shortfall_years += 1;
                depleted = true;
            }
        }

        let market = sample_market_returns(config, regime, volatility_state, &cholesky, &mut rng);
        if market.crash_event {
            crash_years += 1;
        }
        volatility_state = market.next_volatility_state;

        nominal *= 1.0 + market.portfolio_return;
        nominal = nominal.max(0.0);

        let next_cumulative_inflation = cumulative_inflation * (1.0 + inflation);
        if year < config.accumulation_years {
            nominal += config.contribution_for_year(year, next_cumulative_inflation);
        }

        cumulative_inflation = next_cumulative_inflation;
        real_growth_factor *= ((1.0 + market.portfolio_return) / (1.0 + inflation)).max(0.01);

        let real = nominal / cumulative_inflation.max(0.000_1);
        if year >= config.accumulation_years && real < config.ruin_threshold {
            floor_breached = true;
        }

        peak_real = peak_real.max(real);
        if peak_real > 0.0 {
            let drawdown = (1.0 - (real / peak_real)).clamp(0.0, 1.0);
            worst_drawdown = worst_drawdown.max(drawdown);
        }

        yearly_real_values.push(real);
    }

    let ending_real = *yearly_real_values.last().unwrap_or(&0.0);
    let geometric_real_return = if total_years > 0 {
        real_growth_factor.powf(1.0 / total_years as f64) - 1.0
    } else {
        0.0
    };

    PathOutcome {
        ending_nominal: nominal,
        ending_real,
        worst_drawdown,
        geometric_real_return,
        failed: depleted || floor_breached,
        depleted,
        floor_breached,
        crash_years,
        shortfall_years,
        yearly_real_values,
    }
}

fn sample_regime(rng: &mut StdRng) -> MarketRegime {
    let roll = rng.gen::<f64>();
    if roll < 0.32 {
        MarketRegime::Expansion
    } else if roll < 0.62 {
        MarketRegime::Steady
    } else if roll < 0.82 {
        MarketRegime::Slowdown
    } else if roll < 0.92 {
        MarketRegime::Stagflation
    } else {
        MarketRegime::Recovery
    }
}

fn sample_inflation(config: &SimulationConfig, regime: MarketRegime, rng: &mut StdRng) -> f64 {
    let adjustments = regime.adjustments();
    let z: f64 = StandardNormal.sample(rng);
    (config.inflation_rate + adjustments.inflation_shift + config.inflation_volatility * z)
        .clamp(-0.01, 0.18)
}

fn sample_market_returns(
    config: &SimulationConfig,
    regime: MarketRegime,
    volatility_state: VolatilityState,
    cholesky: &[[f64; 3]; 3],
    rng: &mut StdRng,
) -> AnnualMarketSnapshot {
    let volatility_model = config.volatility_model;
    let adjustments = regime.adjustments();
    let shock_scale = if rng.gen_bool(volatility_model.extreme_shock_chance) {
        volatility_model.extreme_shock_scale
    } else {
        1.0
    };
    let normals = correlated_normals(cholesky, rng, shock_scale);

    let mut stock_return = config.assets.stocks.expected_return
        + adjustments.stock_return_shift
        + config.assets.stocks.volatility
            * adjustments.stock_vol_multiplier
            * volatility_state.stock
            * normals[0];
    let mut bond_return = config.assets.bonds.expected_return
        + adjustments.bond_return_shift
        + config.assets.bonds.volatility
            * adjustments.bond_vol_multiplier
            * volatility_state.bond
            * normals[1];
    let cash_return = (config.assets.cash.expected_return
        + adjustments.cash_return_shift
        + config.assets.cash.volatility * normals[2])
        .clamp(-0.03, 0.15);

    let crash_event = rng.gen_bool(config.crash_chance);
    if crash_event {
        stock_return += config.crash_equity_return_shift;
        bond_return += config.crash_bond_return_shift;
    }

    let tail_event = rng.gen_bool(config.tail_loss_chance);
    if tail_event {
        stock_return += config.tail_equity_return_shift;
        bond_return += config.tail_bond_return_shift;
    }

    stock_return = stock_return.clamp(-0.95, 1.50);
    bond_return = bond_return.clamp(-0.60, 0.60);

    let portfolio_return = config.allocation.stocks * stock_return
        + config.allocation.bonds * bond_return
        + config.allocation.cash * cash_return;

    AnnualMarketSnapshot {
        portfolio_return: portfolio_return.clamp(-0.90, 1.20),
        crash_event,
        next_volatility_state: VolatilityState {
            stock: update_volatility_level(
                volatility_state.stock,
                normals[0].abs(),
                regime,
                crash_event,
                tail_event,
                volatility_model,
                AssetKind::Stock,
            ),
            bond: update_volatility_level(
                volatility_state.bond,
                normals[1].abs(),
                regime,
                crash_event,
                tail_event,
                volatility_model,
                AssetKind::Bond,
            ),
        },
    }
}

fn correlated_normals(cholesky: &[[f64; 3]; 3], rng: &mut StdRng, shock_scale: f64) -> [f64; 3] {
    let z: [f64; 3] = [
        StandardNormal.sample(rng),
        StandardNormal.sample(rng),
        StandardNormal.sample(rng),
    ];
    let scaled = [z[0] * shock_scale, z[1] * shock_scale, z[2] * shock_scale];

    [
        cholesky[0][0] * scaled[0],
        cholesky[1][0] * scaled[0] + cholesky[1][1] * scaled[1],
        cholesky[2][0] * scaled[0] + cholesky[2][1] * scaled[1] + cholesky[2][2] * scaled[2],
    ]
}

fn update_volatility_level(
    current: f64,
    shock_magnitude: f64,
    regime: MarketRegime,
    crash_event: bool,
    tail_event: bool,
    model: crate::model::VolatilityModel,
    asset: AssetKind,
) -> f64 {
    let excess_shock = (shock_magnitude - 0.75).max(0.0);
    let regime_boost = match (asset, regime) {
        (AssetKind::Stock, MarketRegime::Slowdown) => 0.08,
        (AssetKind::Stock, MarketRegime::Stagflation) => 0.18,
        (AssetKind::Stock, MarketRegime::Recovery) => 0.04,
        (AssetKind::Bond, MarketRegime::Slowdown) => 0.04,
        (AssetKind::Bond, MarketRegime::Stagflation) => 0.10,
        (AssetKind::Bond, MarketRegime::Recovery) => 0.02,
        _ => 0.0,
    };
    let crash_factor = match asset {
        AssetKind::Stock => 1.0,
        AssetKind::Bond => 0.45,
    };
    let tail_factor = match asset {
        AssetKind::Stock => 0.75,
        AssetKind::Bond => 0.40,
    };

    let next = 1.0
        + (current - 1.0) * model.persistence
        + model.shock_sensitivity * excess_shock
        + if crash_event {
            model.crash_boost * crash_factor
        } else {
            0.0
        }
        + if tail_event {
            model.tail_boost * tail_factor
        } else {
            0.0
        }
        + regime_boost;

    next.clamp(0.70, model.max_multiplier)
}

fn build_report(
    config: &SimulationConfig,
    paths: Vec<PathOutcome>,
    elapsed_millis: u128,
) -> SimulationReport {
    let mut endings: Vec<f64> = paths.iter().map(|path| path.ending_real).collect();
    endings.sort_by(|a, b| a.total_cmp(b));

    let failure_count = paths.iter().filter(|path| path.failed).count();
    let depletion_count = paths.iter().filter(|path| path.depleted).count();
    let floor_breach_count = paths.iter().filter(|path| path.floor_breached).count();
    let mean_ending_real = endings.iter().sum::<f64>() / endings.len() as f64;
    let average_worst_drawdown =
        paths.iter().map(|path| path.worst_drawdown).sum::<f64>() / paths.len() as f64;
    let average_crash_years = paths
        .iter()
        .map(|path| path.crash_years as f64)
        .sum::<f64>()
        / paths.len() as f64;
    let average_shortfall_years = paths
        .iter()
        .map(|path| path.shortfall_years as f64)
        .sum::<f64>()
        / paths.len() as f64;
    let average_geometric_real_return = paths
        .iter()
        .map(|path| path.geometric_real_return)
        .sum::<f64>()
        / paths.len() as f64;

    let yearly_bands = (0..=config.total_years())
        .map(|year| {
            let mut values: Vec<f64> = paths
                .iter()
                .map(|path| path.yearly_real_values[year])
                .collect();
            values.sort_by(|a, b| a.total_cmp(b));

            YearBand {
                year,
                p10: percentile_sorted(&values, 0.10),
                p50: percentile_sorted(&values, 0.50),
                p90: percentile_sorted(&values, 0.90),
            }
        })
        .collect();

    SimulationReport {
        simulations: config.simulations,
        years: config.total_years(),
        elapsed_millis,
        mean_ending_real,
        p10_ending_real: percentile_sorted(&endings, 0.10),
        p50_ending_real: percentile_sorted(&endings, 0.50),
        p90_ending_real: percentile_sorted(&endings, 0.90),
        best_case_real: *endings.last().unwrap_or(&0.0),
        worst_case_real: *endings.first().unwrap_or(&0.0),
        failure_probability: failure_count as f64 / config.simulations as f64,
        depletion_probability: depletion_count as f64 / config.simulations as f64,
        floor_breach_probability: floor_breach_count as f64 / config.simulations as f64,
        average_shortfall_years,
        average_worst_drawdown,
        average_crash_years,
        average_geometric_real_return,
        histogram: build_histogram(&endings, 14),
        yearly_bands,
    }
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let rank = ((values.len() - 1) as f64 * percentile.clamp(0.0, 1.0)).round() as usize;
    values[rank]
}

fn build_histogram(sorted_values: &[f64], buckets: usize) -> Vec<(f64, usize)> {
    if sorted_values.is_empty() || buckets == 0 {
        return Vec::new();
    }

    let min = *sorted_values.first().unwrap_or(&0.0);
    let max = *sorted_values.last().unwrap_or(&0.0);
    let width = ((max - min) / buckets as f64).max(1.0);
    let mut histogram = vec![(0.0, 0usize); buckets];

    for (index, bucket) in histogram.iter_mut().enumerate() {
        bucket.0 = min + width * index as f64;
    }

    for value in sorted_values {
        let mut index = ((value - min) / width).floor() as usize;
        if index >= buckets {
            index = buckets - 1;
        }
        histogram[index].1 += 1;
    }

    histogram
}

struct AnnualMarketSnapshot {
    portfolio_return: f64,
    crash_event: bool,
    next_volatility_state: VolatilityState,
}

#[derive(Clone, Copy)]
struct VolatilityState {
    stock: f64,
    bond: f64,
}

impl Default for VolatilityState {
    fn default() -> Self {
        Self {
            stock: 1.0,
            bond: 1.0,
        }
    }
}

#[derive(Clone, Copy)]
enum AssetKind {
    Stock,
    Bond,
}

#[cfg(test)]
mod tests {
    use super::{run_parallel, run_sequential};
    use crate::model::SimulationConfig;

    fn small_config() -> SimulationConfig {
        SimulationConfig {
            simulations: 500,
            accumulation_years: 4,
            retirement_years: 4,
            ..SimulationConfig::default()
        }
    }

    #[test]
    fn sequential_run_builds_consistent_report() {
        let config = small_config();
        let report = run_sequential(&config);

        assert_eq!(report.simulations, config.simulations);
        assert_eq!(report.years, config.total_years());
        assert_eq!(report.yearly_bands.len(), config.total_years() + 1);
        assert_eq!(report.histogram.len(), 14);
        assert!(report.p10_ending_real <= report.p50_ending_real);
        assert!(report.p50_ending_real <= report.p90_ending_real);
        assert!((0.0..=1.0).contains(&report.failure_probability));
        assert!((0.0..=1.0).contains(&report.depletion_probability));
    }

    #[test]
    fn parallel_and_sequential_reports_match() {
        let config = small_config();
        let sequential = run_sequential(&config);
        let parallel = run_parallel(&config);

        assert_eq!(sequential.p10_ending_real, parallel.p10_ending_real);
        assert_eq!(sequential.p50_ending_real, parallel.p50_ending_real);
        assert_eq!(sequential.p90_ending_real, parallel.p90_ending_real);
        assert_eq!(sequential.worst_case_real, parallel.worst_case_real);
        assert_eq!(sequential.best_case_real, parallel.best_case_real);
        assert_eq!(sequential.failure_probability, parallel.failure_probability);
    }
}
