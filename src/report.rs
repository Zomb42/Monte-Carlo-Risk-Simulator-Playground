use std::time::Duration;

use crate::model::SimulationConfig;
use crate::model::SimulationReport;

pub fn render_terminal_report(
    config: &SimulationConfig,
    report: &SimulationReport,
    elapsed: Duration,
    used_parallel: bool,
) -> String {
    let mut lines = Vec::new();

    lines.push(format!("=== {} ===", config.scenario_name));
    lines.push(format!(
        "Simulations: {} | Horizon: {} years ({} accumulation + {} retirement) | Engine: {} | Wall clock: {:.2?}",
        report.simulations,
        report.years,
        config.accumulation_years,
        config.retirement_years,
        if used_parallel { "Rayon parallel" } else { "Sequential" },
        elapsed
    ));
    lines.push(String::new());

    lines.push("Plan assumptions".to_string());
    lines.push(format!(
        "  Start: {} | Contribution: {} growing at {} real | Retirement spending: {} growing at {} real",
        money(config.initial_portfolio),
        money(config.annual_contribution),
        pct(config.annual_contribution_real_growth),
        money(config.annual_spending),
        pct(config.annual_spending_real_growth)
    ));
    lines.push(format!(
        "  Real wealth floor: {} | Inflation baseline: {} | Inflation volatility: {}",
        money(config.ruin_threshold),
        pct(config.inflation_rate),
        pct(config.inflation_volatility)
    ));
    lines.push(String::new());

    lines.push("Portfolio construction".to_string());
    lines.push(format!(
        "  Allocation: Stocks {} | Bonds {} | Cash {}",
        pct(config.allocation.stocks),
        pct(config.allocation.bonds),
        pct(config.allocation.cash)
    ));
    lines.push(format!(
        "  Capital markets: Stocks {} return / {} vol | Bonds {} return / {} vol | Cash {} return / {} vol",
        pct(config.assets.stocks.expected_return),
        pct(config.assets.stocks.volatility),
        pct(config.assets.bonds.expected_return),
        pct(config.assets.bonds.volatility),
        pct(config.assets.cash.expected_return),
        pct(config.assets.cash.volatility)
    ));
    lines.push(format!(
        "  Correlations: stock-bond {:.2} | stock-cash {:.2} | bond-cash {:.2}",
        config.correlations.stock_bond,
        config.correlations.stock_cash,
        config.correlations.bond_cash
    ));
    lines.push(format!(
        "  Stress events: crash chance {} (stocks {}, bonds {}) | tail-loss chance {} (stocks {}, bonds {})",
        pct(config.crash_chance),
        pct(config.crash_equity_return_shift),
        pct(config.crash_bond_return_shift),
        pct(config.tail_loss_chance),
        pct(config.tail_equity_return_shift),
        pct(config.tail_bond_return_shift)
    ));
    lines.push(format!(
        "  Volatility model: persistence {:.2} | shock sensitivity {:.2} | extreme-shock years {} at {:.2}x scale",
        config.volatility_model.persistence,
        config.volatility_model.shock_sensitivity,
        pct(config.volatility_model.extreme_shock_chance),
        config.volatility_model.extreme_shock_scale
    ));
    lines.push(String::new());

    lines.push("Outcome snapshot".to_string());
    lines.push(format!(
        "  P10 real ending wealth: {}",
        money(report.p10_ending_real)
    ));
    lines.push(format!(
        "  Median real ending wealth: {}",
        money(report.p50_ending_real)
    ));
    lines.push(format!(
        "  P90 real ending wealth: {}",
        money(report.p90_ending_real)
    ));
    lines.push(format!(
        "  Mean real ending wealth: {} | Worst case: {} | Best case: {}",
        money(report.mean_ending_real),
        money(report.worst_case_real),
        money(report.best_case_real)
    ));
    lines.push(format!(
        "  Failure probability: {} | Depletion probability: {} | Floor-breach probability: {}",
        pct(report.failure_probability),
        pct(report.depletion_probability),
        pct(report.floor_breach_probability)
    ));
    lines.push(format!(
        "  Avg shortfall years: {:.2} | Avg worst drawdown: {} | Avg real market return: {}",
        report.average_shortfall_years,
        pct(report.average_worst_drawdown),
        pct(report.average_geometric_real_return)
    ));
    lines.push(format!(
        "  Avg crash years per path: {:.2}",
        report.average_crash_years
    ));
    lines.push(String::new());

    lines.push("Wealth cone".to_string());
    lines.extend(render_year_bands(report));
    lines.push(String::new());

    lines.push("Distribution of final real wealth".to_string());
    lines.extend(render_histogram(report));
    lines.push(String::new());

    lines.push("Interpretation".to_string());
    lines.push(
        "  Failure means the plan either ran short of retirement spending or fell below the real wealth floor after retirement began.".to_string(),
    );
    lines.push(format!(
        "  This run finished its core aggregation in {} ms, which makes it easy to compare sequential and parallel Rust implementations on the same model.",
        report.elapsed_millis
    ));

    lines.join("\n")
}

fn render_year_bands(report: &SimulationReport) -> Vec<String> {
    let mut lines = Vec::new();
    let step = if report.years <= 10 {
        1
    } else if report.years <= 25 {
        5
    } else {
        6
    };

    let selected: Vec<_> = report
        .yearly_bands
        .iter()
        .filter(|band| band.year == 0 || band.year == report.years || band.year % step == 0)
        .collect();

    let max_value = selected
        .iter()
        .map(|band| band.p90)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    for band in selected {
        let width = 44usize;
        let p10_width = ((band.p10 / max_value) * width as f64).round() as usize;
        let p50_width = ((band.p50 / max_value) * width as f64).round() as usize;
        let p90_width = ((band.p90 / max_value) * width as f64).round() as usize;

        lines.push(format!(
            "  Year {:>2}: {:<44} p10 {} | p50 {} | p90 {}",
            band.year,
            cone_bar(p10_width, p50_width, p90_width, width),
            money(band.p10),
            money(band.p50),
            money(band.p90),
        ));
    }

    lines
}

fn render_histogram(report: &SimulationReport) -> Vec<String> {
    let mut lines = Vec::new();
    let max_count = report
        .histogram
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1)
        .max(1);

    for (start, count) in &report.histogram {
        let width = ((*count as f64 / max_count as f64) * 36.0).round() as usize;
        lines.push(format!(
            "  {:>12} | {:<36} {}",
            money(*start),
            "#".repeat(width),
            count
        ));
    }

    lines
}

fn cone_bar(p10_width: usize, p50_width: usize, p90_width: usize, width: usize) -> String {
    let mut chars = vec![' '; width];

    for ch in chars.iter_mut().take(p90_width.min(width)) {
        *ch = '.';
    }
    for ch in chars.iter_mut().take(p50_width.min(width)) {
        *ch = '=';
    }
    for ch in chars.iter_mut().take(p10_width.min(width)) {
        *ch = '#';
    }

    chars.into_iter().collect()
}

fn pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn money(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let abs = value.abs();

    if abs >= 1_000_000_000.0 {
        format!("{sign}${:.2}B", abs / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{sign}${:.2}M", abs / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{sign}${:.0}K", abs / 1_000.0)
    } else {
        format!("{sign}${abs:.0}")
    }
}
