//! Measures the cost of a solution callback.
//!
//! The model has `FREE` binary variables and `NUM_VARS - FREE` followers that
//! each equal one binary variable. One worker enumerates all `2^FREE`
//! solutions, so every run reports the same number of solutions, each with
//! `NUM_VARS` values. (An optimization model stops after a few improvements,
//! so it does not give a reproducible count.)
//!
//! The solver spends about 10 ms on each solution, which hides the callback in
//! the wall time, so each case times the work inside its callback instead:
//!
//! 1. reading the borrowed solution's length,
//! 2. reading every variable of the borrowed solution,
//! 3. copying the whole response with `to_owned_message`, which costs about
//!    what every solution cost before callbacks borrowed the response.
//!
//! Run it with:
//!
//! ```text
//! cargo run --release --example bench_events -p cpsat
//! ```

use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use cpsat::prelude::*;
use cpsat::proto::{CpModelProto, SatParameters};

const NUM_VARS: usize = 20_000;
const FREE: usize = 8;
const RUNS: usize = 5;

/// The work one case does inside its callback.
type Work<'a> = &'a (dyn Fn(CpSolverResponseRef<'_>) -> i64 + Sync);

fn build() -> (CpModel, Vec<IntVar>) {
    let mut model = CpModel::new();
    let mut vars: Vec<IntVar> = (0..FREE).map(|_| model.new_var(0..=1)).collect();
    for i in FREE..NUM_VARS {
        let follower = model.new_var(0..=1);
        model.add(follower.eq(vars[i % FREE]));
        vars.push(follower);
    }
    (model, vars)
}

fn parameters() -> SatParameters {
    SatParameters::default()
        .with_num_workers(1)
        .with_random_seed(1)
        .with_cp_model_presolve(false)
        .with_enumerate_all_solutions(true)
        .with_max_time_in_seconds(120.0)
}

/// Solves once and returns the solution count and the mean time inside `work`.
fn time_callback(
    proto: &CpModelProto,
    parameters: &SatParameters,
    mut work: impl FnMut(CpSolverResponseRef<'_>) -> i64 + Send,
) -> (u32, Duration) {
    let mut solutions = 0;
    let mut inside = Duration::ZERO;
    let mut checksum = 0;
    CpSolver::new(parameters)
        .add_solution_callback(|response| {
            let start = Instant::now();
            checksum += work(response);
            inside += start.elapsed();
            solutions += 1;
            ControlFlow::Continue(())
        })
        .solve(proto)
        .unwrap();
    std::hint::black_box(checksum);
    (solutions, inside / solutions)
}

fn main() {
    let (model, vars) = build();
    let proto = model.proto();
    let parameters = parameters();
    println!("{NUM_VARS} vars, 2^{FREE} solutions, median of {RUNS} runs, 1 worker");

    let cases: [(&str, Work<'_>); 3] = [
        ("borrow solution", &|response| {
            response
                .solution()
                .map_or(0, |solution| solution.values().len().try_into().unwrap())
        }),
        ("read all values", &|response| {
            response.solution().map_or(0, |solution| {
                vars.iter().map(|&var| solution.value(var)).sum()
            })
        }),
        ("copy response", &|response| {
            response.to_owned_message().unwrap().solution[0]
        }),
    ];
    for (name, work) in cases {
        let mut runs: Vec<(u32, Duration)> = (0..RUNS)
            .map(|_| time_callback(proto, &parameters, work))
            .collect();
        runs.sort_by_key(|&(_, time)| time);
        let (solutions, time) = runs[RUNS / 2];
        let micros = time.as_secs_f64() * 1e6;
        println!("{name:<16} {micros:>9.2} us per solution  ({solutions} solutions)");
    }
}
