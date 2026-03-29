# Monte-Carlo-Risk-Simulator-Playground

A Rust playground for Monte Carlo portfolio risk simulation, built to be both educational and a little theatrical.

## What this project is

This project simulates thousands of future portfolio paths under uncertainty:

- stock / bond / cash portfolio mixes
- correlated asset returns
- volatility clustering and extreme-shock years
- inflation erosion
- crash years and tail-loss events
- retirement withdrawals and spending shortfalls
- drawdowns, failure probability, and percentile wealth bands

The goal is not to predict the future exactly. The goal is to make uncertainty visible and give a strong, tangible reason to appreciate Rust while doing it.

## Why Rust is a strong fit

Rust makes this kind of project compelling because it combines:

- fast numeric loops without a runtime
- fearless parallelism with `rayon`
- strong types for financial assumptions and output models
- straightforward serialization with `serde`
- a clean path from terminal prototype to web service or GUI later

This is the kind of codebase where Rust feels very different from many scripting-language implementations: you get performance, structure, and concurrency without giving up readability.

## Current shape

The simulator is a CLI app that:

- runs many portfolio paths either sequentially or in parallel
- can benchmark repeated sequential vs parallel runs
- models multiple market regimes including slowdown and stagflation
- uses correlated stock / bond / cash assumptions
- lets volatility rise after shocks and decay gradually over time
- supports accumulation and retirement phases
- reports P10 / median / P90 real wealth outcomes
- estimates failure, depletion, drawdown, and shortfall metrics
- prints an ASCII wealth cone and ending-wealth histogram
- supports JSON config files for scenario tweaking

## Project layout

- `src/main.rs` wires up the CLI
- `src/model.rs` defines scenario and report types
- `src/simulation.rs` runs the Monte Carlo engine
- `src/report.rs` renders the terminal output
- `scenarios/aggressive_accumulation.json` is a sample scenario

## How to run

Once Rust is installed:

```bash
cargo run
```

Run with the sample config:

```bash
cargo run -- --config scenarios/aggressive_accumulation.json
```

Force sequential execution:

```bash
cargo run -- --config scenarios/aggressive_accumulation.json --sequential
```

Export a starter config:

```bash
cargo run -- --write-example-config scenarios/my_scenario.json
```

Print JSON output instead of the terminal report:

```bash
cargo run -- --config scenarios/aggressive_accumulation.json --json
```

Override the number of simulation paths:

```bash
cargo run -- --config scenarios/aggressive_accumulation.json --simulations 100000
```

Run a large-scale benchmark with metrics:

```bash
cargo run --release -- --config scenarios/aggressive_accumulation.json --simulations 100000 --benchmark --benchmark-runs 5
```

That benchmark mode reports:

- average sequential runtime
- average parallel runtime
- best run for each mode
- paths per second
- overall speedup
- whether sequential and parallel results still match

## Project takeaways

- Monte Carlo simulation is about exploring a distribution of futures, not predicting one exact future.
- Sequence risk matters as much as average return, especially once withdrawals begin.
- Diversification helps, but correlation and stress behavior matter a lot during bad regimes.
- Volatility is more realistic when it clusters and persists instead of resetting every year.
- Clear modeling pays off: separating allocation, market assumptions, inflation, withdrawals, and failure rules makes the simulator easier to trust and extend.
- Rust is a strong fit for simulation-heavy work because it combines performance, explicit data modeling, and safe parallel execution.
- Validation and tests are part of financial modeling, not just code hygiene.

## Good next directions

- compare Rust parallel performance against Python or JavaScript
- add taxes, account types, or withdrawal ordering rules
- add glide paths instead of a fixed allocation
- render charts in the browser with a small frontend
- benchmark the engine with `criterion`

## Note

The project has been compiled and tested in the `monte-carlo-rust` conda environment.
