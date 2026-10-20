//! Callbacks and cancellation, mirroring Python's solution, best-bound, and log
//! callbacks and `stop_search`.

use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use cpsat::prelude::*;
use cpsat::proto::{CpModelProto, CpSolverStatus, SatParameters};

use crate::common::{hard_model, long_parameters};

/// Generous, because CI runners are slow and run tests in parallel. A stop
/// that fails runs for the full 60 seconds of `long_parameters`.
const STOPPED_WITHIN: Duration = Duration::from_secs(30);

/// `x` in `[0, 3]` maximized, with `x` at index 0.
fn tiny_model() -> CpModelProto {
    let mut model = CpModel::new();
    let x = model.new_var(0..=3);
    model.maximize(x);
    model.into_proto()
}

#[test]
fn callbacks_of_one_kind_run_in_order() {
    let calls = Mutex::new(Vec::new());
    let calls_ref = &calls;
    CpSolver::new(&SatParameters::default().with_enumerate_all_solutions(true))
        .add_solution_callback(|_| {
            calls_ref.lock().unwrap().push(1);
            ControlFlow::Continue(())
        })
        .add_solution_callback(|_| {
            calls_ref.lock().unwrap().push(2);
            ControlFlow::Continue(())
        })
        .solve(&tiny_model())
        .unwrap();
    let calls = calls.into_inner().unwrap();
    assert_ne!(calls, Vec::<i32>::new());
    assert!(calls.chunks(2).all(|pair| pair == [1, 2]), "{calls:?}");
}

#[test]
fn solution_ref_reads_values_and_converts_to_owned() {
    let model = CpModel::from(tiny_model());
    let x = model.int_var_from_index(0).unwrap();
    let mut owned = None;
    let mut seen = None;
    CpSolver::default()
        .add_solution_callback(|response| {
            seen = response.solution().map(|solution| solution.value(x));
            owned = Some(response.to_owned_message().unwrap());
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(seen, Some(3));
    let owned = owned.unwrap();
    assert_eq!(owned.solution().map(|solution| solution.value(x)), Some(3));
}

#[test]
fn reports_best_bounds() {
    let mut bounds = Vec::new();
    // A busy machine reports bounds late, so stop after the first instead
    // of after a fixed time.
    let parameters = SatParameters::default()
        .with_max_time_in_seconds(30.0)
        .with_num_workers(8);
    CpSolver::new(&parameters)
        .add_best_bound_callback(|bound| {
            bounds.push(bound);
            if bounds.is_empty() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        })
        .solve(&hard_model())
        .unwrap();
    assert_ne!(bounds, Vec::<f64>::new());
}

#[test]
fn forwards_logs_and_parameters() {
    let parameters = SatParameters::default()
        .with_log_search_progress(true)
        .with_log_to_stdout(false)
        .with_num_workers(3);
    let mut log = String::new();
    let mut solver = CpSolver::new(&parameters).add_log_callback(|line| {
        log.push_str(line);
        log.push('\n');
        ControlFlow::Continue(())
    });
    assert_eq!(solver.parameters().num_workers, Some(3));
    solver.solve(&tiny_model()).unwrap();
    // OR-Tools 9.15 echoes non-default parameters, which proves they reached
    // C++. The wording may change with the pinned version.
    assert!(log.contains("num_workers: 3"), "{log}");
}

#[test]
fn callback_break_stops_search() {
    let start = Instant::now();
    let response = CpSolver::new(&long_parameters())
        .add_solution_callback(|_| ControlFlow::Break(()))
        .solve(&hard_model())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
    assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
}

#[test]
fn stop_handle_works_inside_callback() {
    let stop = StopHandle::new();
    let response = CpSolver::new(&long_parameters())
        .add_solution_callback(|_| {
            stop.stop();
            ControlFlow::Continue(())
        })
        .with_stop_handle(&stop)
        .solve(&hard_model())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
}

#[test]
fn stops_from_another_thread() {
    let stop = StopHandle::new();
    let start = Instant::now();
    let stop = &stop;
    let (found, first_solutions) = mpsc::channel();
    let response = thread::scope(|scope| {
        // Stopping after the first solution proves that the stop reaches a
        // running solve, not only one that has yet to start.
        scope.spawn(move || {
            first_solutions.recv().unwrap();
            stop.stop();
        });
        CpSolver::new(&long_parameters())
            .add_solution_callback(|_| {
                // The receiver leaves after the first solution.
                let _ = found.send(());
                ControlFlow::Continue(())
            })
            .with_stop_handle(stop)
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
    let stop = &stop;
    let (found, first_solutions) = mpsc::channel();
    let statuses: Vec<CpSolverStatus> = thread::scope(|scope| {
        let solves: Vec<_> = (0..2)
            .map(|_| {
                let found = found.clone();
                scope.spawn(move || {
                    CpSolver::new(&long_parameters())
                        .add_solution_callback(move |_| {
                            // The receiver leaves after both first solutions.
                            let _ = found.send(());
                            ControlFlow::Continue(())
                        })
                        .with_stop_handle(stop)
                        .solve(&hard_model())
                        .unwrap()
                        .status()
                })
            })
            .collect();
        first_solutions.recv().unwrap();
        first_solutions.recv().unwrap();
        drop(first_solutions);
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
        CpSolver::new(&long_parameters())
            .add_solution_callback(|_| {
                solutions += 1;
                ControlFlow::Continue(())
            })
            .with_stop_handle(&stop)
            .solve(&hard_model())
            .unwrap();
        assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
        assert!(solutions <= 1, "{solutions} solutions after a stop");
    }
}

#[test]
fn stop_after_solve_is_harmless() {
    let stop = StopHandle::new();
    let mut solver = CpSolver::default().with_stop_handle(&stop);
    solver.solve(&tiny_model()).unwrap();
    stop.stop();
    assert!(stop.is_stopped());
}

#[test]
fn callback_panic_stops_and_resumes_after_solve() {
    let start = Instant::now();
    let mut calls = 0;
    let panic = catch_unwind(AssertUnwindSafe(|| {
        CpSolver::new(&long_parameters())
            .add_solution_callback(|_| {
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
    let mut solver = CpSolver::default()
        .add_solution_callback(|_| {
            solutions += 1;
            ControlFlow::Continue(())
        })
        .add_best_bound_callback(|_| ControlFlow::Continue(()))
        .add_log_callback(|_| ControlFlow::Continue(()));
    for _ in 0..20 {
        let response = solver.solve(&tiny_model()).unwrap();
        assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    }
    assert!(solutions >= 20);
}

/// Every accessor on the borrowed solution agrees with the owned response, as
/// Python's callback and `CpSolver` accessors agree.
#[test]
fn solution_ref_accessors_match_the_owned_response() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=3);
    let b = model.new_var(Bool);
    let c = model.new_var(Bool);
    model.add(b.eq(1));
    model.add(c.eq(0));
    model.maximize(x);
    let read = |solution: Solution<'_>| {
        (
            [solution.value(x), solution.value(2 * x - 1)],
            [
                solution.value(b),
                solution.value(!b),
                solution.value(c),
                solution.value(!c),
            ],
            [solution.value(0.5 * x + 1.25), solution.value(x + 0.5 * !b)],
        )
    };
    let mut seen = None;
    let response = CpSolver::new(&SatParameters::default().with_num_workers(1))
        .add_solution_callback(|response| {
            seen = response.solution().map(read);
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    let owned = read(response.solution().unwrap());
    assert_eq!(seen, Some(owned));
    assert_eq!(owned, ([3, 5], [true, false, false, true], [2.75, 3.0]));
}

#[test]
fn solution_ref_carries_search_statistics() {
    let model = CpModel::from(tiny_model());
    let x = model.int_var_from_index(0).unwrap();
    let mut checked = false;
    CpSolver::new(&SatParameters::default().with_num_workers(1))
        .add_solution_callback(|response| {
            // The objective is x, maximized.
            let value = response.solution().unwrap().value(x);
            let objective = f64::from(i32::try_from(value).unwrap());
            assert!((response.objective_value() - objective).abs() < 1e-9);
            assert!(
                response.num_booleans() >= 0
                    && response.num_conflicts() >= 0
                    && response.num_branches() >= 0
            );
            for time in [
                response.wall_time(),
                response.user_time(),
                response.deterministic_time(),
            ] {
                assert!(time.is_finite() && time >= 0.0, "{time}");
            }
            checked = true;
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert!(checked);
}
