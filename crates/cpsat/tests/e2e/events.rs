//! Events and cancellation, mirroring Python's solution, best-bound, and log
//! callbacks and `stop_search`.

use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;
use std::time::{Duration, Instant};

use buffa::MessageView;
use cpsat::prelude::*;
use cpsat::proto::CpModelProto;

use crate::common::{hard_model, long_parameters};

/// Generous, because CI runners are slow and run tests in parallel. A stop
/// that fails runs for the full 60 seconds of `long_parameters`.
const STOPPED_WITHIN: Duration = Duration::from_secs(30);

/// `x` in `[0, 3]` maximized, with `x` at index 0.
fn tiny_model() -> CpModelProto {
    let mut model = Model::new();
    let x = model.int_var(0..=3);
    model.maximize(x);
    model.into_proto()
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
    let model = Model::from(tiny_model());
    let x = model.int_var_from_index(0).unwrap();
    let mut owned = None;
    let mut seen = None;
    Solver::default()
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                seen = view.value(x);
                owned = Some(view.to_owned_message().unwrap());
            }
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(seen, Some(3));
    assert_eq!(owned.unwrap().value(x), Some(3));
}

#[test]
fn reports_best_bounds() {
    let mut bounds = Vec::new();
    // A busy machine reports bounds late, so stop after two instead of
    // after a fixed time.
    let parameters = SatParameters::default()
        .with_max_time_in_seconds(30.0)
        .with_num_workers(8);
    Solver::new(&parameters)
        .on_event(EventSet::BEST_BOUND, |event| {
            if let Event::BestBound(bound) = event {
                bounds.push(bound);
            }
            if bounds.len() < 2 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        })
        .solve(hard_model())
        .unwrap();
    assert_ne!(bounds, Vec::<f64>::new());
    // The model maximizes, so each new bound is no larger than the last.
    assert!(bounds.is_sorted_by(|a, b| a >= b), "{bounds:?}");
}

#[test]
fn forwards_logs_and_parameters() {
    let parameters = SatParameters::default()
        .with_log_search_progress(true)
        .with_log_to_stdout(false)
        .with_num_workers(3);
    let mut log = String::new();
    let mut solver = Solver::new(&parameters).on_event(EventSet::LOG, |event| {
        if let Event::Log(line) = event {
            log.push_str(line);
            log.push('\n');
        }
        ControlFlow::Continue(())
    });
    assert_eq!(solver.parameters().num_workers, Some(3));
    solver.solve(tiny_model()).unwrap();
    // OR-Tools 9.15 echoes non-default parameters, which proves they reached
    // C++. The wording may change with the pinned version.
    assert!(log.contains("num_workers: 3"), "{log}");
}

#[test]
fn callback_break_stops_search() {
    let start = Instant::now();
    let response = Solver::new(&long_parameters())
        .on_event(EventSet::SOLUTION, |_| ControlFlow::Break(()))
        .solve(hard_model())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
    assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
}

#[test]
fn stop_handle_works_inside_callback() {
    let stop = StopHandle::new();
    let response = Solver::new(&long_parameters())
        .on_event(EventSet::SOLUTION, |_| {
            stop.stop();
            ControlFlow::Continue(())
        })
        .with_stop_handle(&stop)
        .solve(hard_model())
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
        Solver::new(&long_parameters())
            .with_stop_handle(&stop)
            .solve(hard_model())
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
                    Solver::new(&long_parameters())
                        .with_stop_handle(&stop)
                        .solve(hard_model())
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
        Solver::new(&long_parameters())
            .on_event(EventSet::SOLUTION, |_| {
                solutions += 1;
                ControlFlow::Continue(())
            })
            .with_stop_handle(&stop)
            .solve(hard_model())
            .unwrap();
        assert!(start.elapsed() < STOPPED_WITHIN, "{:?}", start.elapsed());
        assert!(solutions <= 1, "{solutions} solutions after a stop");
    }
}

#[test]
fn stop_after_solve_is_harmless() {
    let stop = StopHandle::new();
    let mut solver = Solver::default().with_stop_handle(&stop);
    solver.solve(tiny_model()).unwrap();
    stop.stop();
    assert!(stop.is_stopped());
}

#[test]
fn callback_panic_stops_and_resumes_after_solve() {
    let start = Instant::now();
    let mut calls = 0;
    let panic = catch_unwind(AssertUnwindSafe(|| {
        Solver::new(&long_parameters())
            .on_event(EventSet::SOLUTION, |_| {
                calls += 1;
                panic!("user callback failed");
            })
            .solve(hard_model())
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
        let response = solver.solve(tiny_model()).unwrap();
        assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    }
    assert!(solutions >= 20);
}

/// Every accessor on the solution view agrees with the owned response, as
/// Python's callback and `CpSolver` accessors agree.
#[test]
fn solution_view_accessors_match_the_owned_response() {
    let mut model = Model::new();
    let x = model.int_var(0..=3);
    let b = model.bool_var();
    let c = model.bool_var();
    model.add(b.eq(1));
    model.add(c.eq(0));
    model.maximize(x);
    // Handles from a larger model are out of range here.
    let mut larger = Model::new();
    let far = [(); 4].map(|()| larger.int_var(0..=1))[3];
    let far_literal = larger.bool_var();

    let read = |value: &dyn Fn(LinearExpr) -> Option<i64>,
                boolean: &dyn Fn(BoolVar) -> Option<bool>,
                float: &dyn Fn(FloatExpr) -> Option<f64>| {
        (
            [value(x.into()), value(2 * x - 1), value(far.into())],
            [
                boolean(b),
                boolean(!b),
                boolean(c),
                boolean(!c),
                boolean(far_literal),
            ],
            [float(0.5 * x + 1.25), float(x + 0.5 * !b), float(0.5 * far)],
        )
    };
    let mut seen = None;
    let response = Solver::new(&SatParameters::default().with_num_workers(1))
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                seen = Some(read(
                    &|expr| view.value(expr),
                    &|literal| view.boolean_value(literal),
                    &|expr| view.float_value(expr),
                ));
            }
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    let owned = read(
        &|expr| response.value(expr),
        &|literal| response.boolean_value(literal),
        &|expr| response.float_value(expr),
    );
    assert_eq!(seen, Some(owned));
    assert_eq!(
        owned,
        (
            [Some(3), Some(5), None],
            [Some(true), Some(false), Some(false), Some(true), None],
            [Some(2.75), Some(3.0), None],
        )
    );
}

#[test]
fn solution_view_carries_search_statistics() {
    let model = Model::from(tiny_model());
    let x = model.int_var_from_index(0).unwrap();
    let mut checked = false;
    Solver::new(&SatParameters::default().with_num_workers(1))
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                // The objective is x, maximized.
                let objective = f64::from(i32::try_from(view.value(x).unwrap()).unwrap());
                assert!((view.objective_value - objective).abs() < 1e-9);
                assert!(view.best_objective_bound >= view.objective_value);
                assert!(
                    view.num_booleans >= 0 && view.num_conflicts >= 0 && view.num_branches >= 0
                );
                for time in [view.wall_time, view.user_time, view.deterministic_time] {
                    assert!(time.is_finite() && time >= 0.0, "{time}");
                }
                checked = true;
            }
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert!(checked);
}
