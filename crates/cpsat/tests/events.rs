//! Events and cancellation, mirroring Python's solution, best-bound, and log
//! callbacks and `stop_search`.

mod common;

use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;
use std::time::{Duration, Instant};

use buffa::MessageView;
use common::{hard_model, long_params};
use cpsat::proto::{CpModelProto, CpSolverStatus, SatParameters};
use cpsat::{Event, EventSet, Solver, StopHandle};

/// Generous, because CI runners are slow and run tests in parallel. A stop
/// that fails runs for the full 60 seconds of `long_params`.
const STOPPED_WITHIN: Duration = Duration::from_secs(30);

fn tiny_model() -> CpModelProto {
    let mut model = CpModelProto::default();
    let x = model.add_int_var(0, 3);
    model.maximize(&[(x, 1)]);
    model
}

#[test]
fn event_set_combines_kinds() {
    let set = EventSet::SOLUTION | EventSet::LOG;
    assert!(set.contains(EventSet::SOLUTION));
    assert!(!set.contains(EventSet::BEST_BOUND));
    let mut all = set;
    all |= EventSet::BEST_BOUND;
    assert_eq!(all, EventSet::ALL);
}

#[test]
fn solution_view_reads_values_and_converts_to_owned() {
    let mut owned = None;
    let mut seen = None;
    Solver::default()
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                seen = view.value(0);
                owned = Some(view.to_owned_message().unwrap());
            }
            ControlFlow::Continue(())
        })
        .solve(&tiny_model())
        .unwrap();
    assert_eq!(seen, Some(3));
    assert_eq!(owned.unwrap().value(0), Some(3));
}

#[test]
fn reports_best_bounds() {
    let mut bounds = Vec::new();
    let params = SatParameters::default()
        .with_max_time_in_seconds(2.0)
        .with_num_workers(8);
    Solver::new(params)
        .on_event(EventSet::BEST_BOUND, |event| {
            if let Event::BestBound(bound) = event {
                bounds.push(bound);
            }
            ControlFlow::Continue(())
        })
        .solve(&hard_model())
        .unwrap();
    assert_ne!(bounds, Vec::<f64>::new());
    // The model maximizes, so each new bound is no larger than the last.
    assert!(bounds.is_sorted_by(|a, b| a >= b), "{bounds:?}");
}

#[test]
fn forwards_logs_and_parameters() {
    let params = SatParameters::default()
        .with_log_search_progress(true)
        .with_log_to_stdout(false)
        .with_num_workers(3);
    let mut log = String::new();
    Solver::new(params)
        .on_event(EventSet::LOG, |event| {
            if let Event::Log(line) = event {
                log.push_str(line);
                log.push('\n');
            }
            ControlFlow::Continue(())
        })
        .solve(&tiny_model())
        .unwrap();
    // OR-Tools 9.15 echoes non-default parameters, which proves they reached
    // C++. The wording may change with the pinned version.
    assert!(log.contains("num_workers: 3"), "{log}");
}

#[test]
fn callback_break_stops_search() {
    let start = Instant::now();
    let response = Solver::new(long_params())
        .on_event(EventSet::SOLUTION, |_| ControlFlow::Break(()))
        .solve(&hard_model())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
    assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
}

#[test]
fn stop_handle_works_inside_callback() {
    let stop = StopHandle::new();
    let response = Solver::new(long_params())
        .on_event(EventSet::SOLUTION, |_| {
            stop.stop();
            ControlFlow::Continue(())
        })
        .stop(&stop)
        .solve(&hard_model())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
}

#[test]
fn stops_from_another_thread() {
    let stop = StopHandle::new();
    let start = Instant::now();
    let response = thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(Duration::from_millis(500));
            stop.stop();
        });
        Solver::new(long_params())
            .stop(&stop)
            .solve(&hard_model())
            .unwrap()
    });
    assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
    assert_ne!(response.status(), CpSolverStatus::OPTIMAL);
}

#[test]
fn one_handle_stops_concurrent_solves() {
    let stop = StopHandle::new();
    let start = Instant::now();
    let statuses: Vec<CpSolverStatus> = thread::scope(|scope| {
        let solves: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    Solver::new(long_params())
                        .stop(&stop)
                        .solve(&hard_model())
                        .unwrap()
                        .status()
                })
            })
            .collect();
        thread::sleep(Duration::from_millis(500));
        stop.stop();
        solves
            .into_iter()
            .map(|solve| solve.join().unwrap())
            .collect()
    });
    assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
    assert!(statuses.iter().all(|&s| s != CpSolverStatus::OPTIMAL));
}

#[test]
fn stopped_handle_stops_every_later_solve() {
    let stop = StopHandle::new();
    stop.stop();
    for _ in 0..2 {
        let mut solutions = 0;
        let start = Instant::now();
        Solver::new(long_params())
            .on_event(EventSet::SOLUTION, |_| {
                solutions += 1;
                ControlFlow::Continue(())
            })
            .stop(&stop)
            .solve(&hard_model())
            .unwrap();
        assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
        assert!(solutions <= 1, "{solutions} solutions after a stop");
    }
}

#[test]
fn stop_after_solve_is_harmless() {
    let stop = StopHandle::new();
    let mut solver = Solver::default().stop(&stop);
    solver.solve(&tiny_model()).unwrap();
    stop.stop();
    assert!(stop.is_stopped());
}

#[test]
fn callback_panic_stops_and_resumes_after_solve() {
    let start = Instant::now();
    let mut calls = 0;
    let panic = catch_unwind(AssertUnwindSafe(|| {
        Solver::new(long_params())
            .on_event(EventSet::SOLUTION, |_| {
                calls += 1;
                panic!("user callback failed");
            })
            .solve(&hard_model())
    }))
    .unwrap_err();
    assert_eq!(panic.downcast_ref::<&str>(), Some(&"user callback failed"));
    assert_eq!(calls, 1);
    assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
}

#[test]
fn solver_solves_again() {
    let mut solutions = 0;
    let mut solver = Solver::default().on_event(EventSet::ALL, |event| {
        if matches!(event, Event::Solution(_)) {
            solutions += 1;
        }
        ControlFlow::Continue(())
    });
    for _ in 0..20 {
        let response = solver.solve(&tiny_model()).unwrap();
        assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    }
    drop(solver);
    assert!(solutions >= 20);
}
