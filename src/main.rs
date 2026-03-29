mod model;
mod report;
mod simulation;

use std::{fs, path::PathBuf, process::ExitCode, time::Instant};

use clap::Parser;
use model::SimulationConfig;
use report::{render_benchmark_report, render_terminal_report};
use simulation::{run_benchmark, run_parallel, run_sequential};

#[derive(Debug, Parser)]
#[command(
    name = "Monte Carlo Risk Simulator Playground",
    about = "A Rust-powered simulator for stress testing long-term portfolio outcomes."
)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    write_example_config: Option<PathBuf>,

    #[arg(long, help = "Print the aggregated report as JSON.")]
    json: bool,

    #[arg(long, help = "Run without Rayon to compare sequential execution.")]
    sequential: bool,

    #[arg(long, help = "Override the number of Monte Carlo paths to simulate.")]
    simulations: Option<usize>,

    #[arg(
        long,
        help = "Run repeated sequential and parallel benchmark passes.",
        conflicts_with = "sequential"
    )]
    benchmark: bool,

    #[arg(
        long,
        default_value_t = 5,
        help = "Number of benchmark repetitions to run.",
        requires = "benchmark"
    )]
    benchmark_runs: usize,
}

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<(), String> {
    let cli = Cli::parse();

    if let Some(path) = cli.write_example_config {
        let config = SimulationConfig::default();
        let serialized = serde_json::to_string_pretty(&config)
            .map_err(|err| format!("failed to serialize example config: {err}"))?;
        fs::write(&path, serialized)
            .map_err(|err| format!("failed to write config to {}: {err}", path.display()))?;
        println!("Wrote example config to {}", path.display());
        return Ok(());
    }

    let config = match cli.config {
        Some(path) => {
            let raw = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read config {}: {err}", path.display()))?;
            serde_json::from_str::<SimulationConfig>(&raw)
                .map_err(|err| format!("failed to parse config {}: {err}", path.display()))?
        }
        None => SimulationConfig::default(),
    };

    let mut config = config;
    if let Some(simulations) = cli.simulations {
        config.simulations = simulations;
    }

    config.validate()?;

    if cli.benchmark {
        if cli.benchmark_runs == 0 {
            return Err("benchmark_runs must be at least 1".to_string());
        }

        let benchmark = run_benchmark(&config, cli.benchmark_runs);
        if cli.json {
            let payload = serde_json::to_string_pretty(&benchmark)
                .map_err(|err| format!("failed to serialize benchmark report: {err}"))?;
            println!("{payload}");
        } else {
            println!("{}", render_benchmark_report(&config, &benchmark));
        }
        return Ok(());
    }

    let start = Instant::now();
    let report = if cli.sequential {
        run_sequential(&config)
    } else {
        run_parallel(&config)
    };
    let elapsed = start.elapsed();

    if cli.json {
        let payload = serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize report: {err}"))?;
        println!("{payload}");
    } else {
        let rendered = render_terminal_report(&config, &report, elapsed, !cli.sequential);
        println!("{rendered}");
    }

    Ok(())
}
