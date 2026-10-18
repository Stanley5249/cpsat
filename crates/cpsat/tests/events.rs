//! Callbacks and cancellation, mirroring Python's solution, best-bound, and
//! log callbacks and `stop_search`.

mod common;

use std::ops::ControlFlow;
use std::thread;
use std::time::{Duration, Instant};

use buffa::MessageView;
use common::{hard_model, small_model, var};
use cpsat::proto::{CpModelProto, CpSolverStatus, SatParameters};
use cpsat::{Event, Events, StopHandle};

fn long_params() -> SatParameters {
    SatParameters {
        max_time_in_seconds: Some(60.0),
        num_workers: Some(4),
        ..Default::default()
    }
}

#[test]
fn enumerates_all_solutions() {
    // x + y == 2 with x, y in [0, 2] has three solutions.
    let mut model = CpModelProto::default();
    let x = var(&mut model, 0, 2);
    let y = var(&mut model, 0, 2);
    common::linear(&mut model, &[(x, 1), (y, 1)], 2, 2);
    let params = SatParameters {
        enumerate_all_solutions: Some(true),
        num_workers: Some(1),
        ..Default::default()
    };
    let mut found = Vec::new();
    let response = cpsat::solve_with(&model, &params, Events::SOLUTION, None, |event| {
        if let Event::Solution(view) = event {
            found.push(view.solution.iter().copied().collect::<Vec<i64>>());
        }
        ControlFlow::Continue(())
    })
    .unwrap();
    assert_eq!(response.status, CpSolverStatus::OPTIMAL);
    found.sort();
    assert_eq!(found, [vec![0, 2], vec![1, 1], vec![2, 0]]);
}

#[test]
fn solution_view_converts_to_owned() {
    let mut last = None;
    cpsat::solve_with(
        &small_model(),
        &common::single_worker(),
        Events::SOLUTION,
        None,
        |event| {
            if let Event::Solution(view) = event {
                last = Some(view.to_owned_message().unwrap());
            }
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert_eq!(last.unwrap().solution, [3, 1]);
}

#[test]
fn reports_best_bounds() {
    let mut bounds = Vec::new();
    let params = SatParameters {
        max_time_in_seconds: Some(2.0),
        num_workers: Some(8),
        ..Default::default()
    };
    cpsat::solve_with(&hard_model(), &params, Events::BEST_BOUND, None, |event| {
        if let Event::BestBound(bound) = event {
            bounds.push(bound);
        }
        ControlFlow::Continue(())
    })
    .unwrap();
    assert_ne!(bounds, Vec::<f64>::new());
    // The model maximizes, so each new bound is no larger than the last.
    assert!(
        bounds.windows(2).all(|pair| pair[1] <= pair[0]),
        "{bounds:?}"
    );
}

#[test]
fn forwards_logs_and_parameters() {
    let params = SatParameters {
        log_search_progress: Some(true),
        log_to_stdout: Some(false),
        num_workers: Some(3),
        ..Default::default()
    };
    let mut log = String::new();
    cpsat::solve_with(&small_model(), &params, Events::LOG, None, |event| {
        if let Event::Log(line) = event {
            log.push_str(line);
            log.push('\n');
        }
        ControlFlow::Continue(())
    })
    .unwrap();
    assert!(log.contains("CP-SAT solver"), "{log}");
    // The log echoes non-default parameters, which proves they reached C++.
    assert!(log.contains("num_workers: 3"), "{log}");
}

#[test]
fn callback_stops_search() {
    let start = Instant::now();
    let response = cpsat::solve_with(
        &hard_model(),
        &long_params(),
        Events::SOLUTION,
        None,
        |event| {
            if matches!(event, Event::Solution(_)) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap();
    assert_eq!(response.status, CpSolverStatus::FEASIBLE);
    assert!(
        start.elapsed() < Duration::from_secs(20),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn stop_handle_works_inside_callback() {
    let stop = StopHandle::new();
    let response = cpsat::solve_with(
        &hard_model(),
        &long_params(),
        Events::SOLUTION,
        Some(&stop),
        |_| {
            stop.stop();
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert_eq!(response.status, CpSolverStatus::FEASIBLE);
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
        cpsat::solve_with(
            &hard_model(),
            &long_params(),
            Events::NONE,
            Some(&stop),
            |_| ControlFlow::Continue(()),
        )
        .unwrap()
    });
    assert!(
        start.elapsed() < Duration::from_secs(20),
        "{:?}",
        start.elapsed()
    );
    assert_ne!(response.status, CpSolverStatus::OPTIMAL);
}

#[test]
fn stop_before_solve_returns_at_once() {
    let stop = StopHandle::new();
    stop.stop();
    let start = Instant::now();
    let response = cpsat::solve_with(
        &hard_model(),
        &long_params(),
        Events::NONE,
        Some(&stop),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(stop.is_stopped());
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
    assert_ne!(response.status, CpSolverStatus::OPTIMAL);
}

#[test]
fn stop_after_solve_is_harmless() {
    let stop = StopHandle::new();
    cpsat::solve_with(
        &small_model(),
        &SatParameters::default(),
        Events::NONE,
        Some(&stop),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    stop.stop();
    assert!(stop.is_stopped());
}

#[test]
#[should_panic(expected = "user callback failed")]
fn callback_panic_resumes_after_solve() {
    let _ = cpsat::solve_with(
        &hard_model(),
        &long_params(),
        Events::SOLUTION,
        None,
        |_| {
            panic!("user callback failed");
        },
    );
}

#[test]
fn repeated_sessions_are_independent() {
    for _ in 0..50 {
        let response = cpsat::solve_with(
            &small_model(),
            &common::single_worker(),
            Events::ALL,
            None,
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
        assert_eq!(response.status, CpSolverStatus::OPTIMAL);
    }
}
