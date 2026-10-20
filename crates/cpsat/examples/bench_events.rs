//! Measures the overhead of the solver's solution callback path.
//!
//! The model has `FREE` binary variables and `NUM_VARS - FREE` followers that
//! each equal one binary variable. One worker enumerates all `2^FREE`
//! solutions, so every run does the same work and reports the same number of
//! solution events, each carrying `NUM_VARS` values. (An optimization model
//! stops after a few improvements or is limited by search speed, so it does
//! not give a reproducible event count.)
//!
//! Each case solves the model `RUNS` times and prints the median wall time and
//! solutions per second. The cases differ only in the callback:
//!
//! 1. no callback,
//! 2. a callback that counts solution events,
//! 3. a callback that reads every variable of each solution.
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

/// A run's wall time and number of solution events.
struct Run {
    time: Duration,
    events: u32,
}

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

fn timed(solve: impl FnOnce() -> u32) -> Run {
    let start = Instant::now();
    let events = solve();
    Run {
        time: start.elapsed(),
        events,
    }
}

fn no_callback(proto: &CpModelProto, parameters: &SatParameters) -> Run {
    timed(|| {
        proto.solve(parameters).unwrap();
        0
    })
}

fn count_events(proto: &CpModelProto, parameters: &SatParameters) -> Run {
    timed(|| {
        let mut events = 0;
        CpSolver::new(parameters)
            .on_event(EventSet::SOLUTION, |event| {
                if let Event::Solution(_) = event {
                    events += 1;
                }
                ControlFlow::Continue(())
            })
            .solve(proto)
            .unwrap();
        events
    })
}

fn read_solution(proto: &CpModelProto, parameters: &SatParameters, vars: &[IntVar]) -> Run {
    timed(|| {
        let mut events = 0;
        let mut checksum = 0_i64;
        CpSolver::new(parameters)
            .on_event(EventSet::SOLUTION, |event| {
                if let Event::Solution(view) = event {
                    events += 1;
                    if let Some(solution) = view.solution() {
                        checksum += vars.iter().map(|&var| solution.value(var)).sum::<i64>();
                    }
                }
                ControlFlow::Continue(())
            })
            .solve(proto)
            .unwrap();
        std::hint::black_box(checksum);
        events
    })
}

/// The run with the median wall time.
fn median(mut runs: Vec<Run>) -> Run {
    runs.sort_by_key(|run| run.time);
    runs.swap_remove(runs.len() / 2)
}

fn main() {
    let (model, vars) = build();
    let proto = model.proto();
    let parameters = parameters();
    println!("{NUM_VARS} vars, 2^{FREE} solutions, {RUNS} runs, 1 worker");

    let base = median((0..RUNS).map(|_| no_callback(proto, &parameters)).collect());
    let counted = median(
        (0..RUNS)
            .map(|_| count_events(proto, &parameters))
            .collect(),
    );
    let read = median(
        (0..RUNS)
            .map(|_| read_solution(proto, &parameters, &vars))
            .collect(),
    );

    let events = counted.events;
    let base_seconds = base.time.as_secs_f64();
    println!("no callback      median {base_seconds:>8.3} s");
    for (name, run) in [("count events", &counted), ("read all values", &read)] {
        let seconds = run.time.as_secs_f64();
        println!(
            "{name:<16} median {seconds:>8.3} s  {:>9.1} solutions/s  {:>7.2} us per event over no callback  ({} events)",
            f64::from(run.events) / seconds,
            (seconds - base_seconds) * 1e6 / f64::from(events),
            run.events,
        );
    }
}
