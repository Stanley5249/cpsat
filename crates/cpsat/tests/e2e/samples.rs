//! Rust versions of the official OR-Tools v9.15 CP-SAT samples in
//! `ortools/sat/samples`. Each test asserts what the Python sample prints,
//! checked by running the samples with `ortools==9.15.6755`.

use std::ops::ControlFlow;

use cpsat::prelude::*;
use cpsat::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};
use cpsat::proto::sat_parameters::SearchBranching;

use crate::common::solve_optimal;

/// The three variables in `[0, 2]` that most samples share.
fn three_vars(model: &mut Model) -> [IntVar; 3] {
    [(); 3].map(|()| model.int_var(0..=2))
}

/// Every solution of `model`, as the values of `exprs`, in the order found.
fn all_solutions<const N: usize>(
    model: &Model,
    parameters: SatParameters,
    exprs: &[LinearExpr; N],
) -> Vec<[i64; N]> {
    let mut solutions = Vec::new();
    let response = Solver::new(&parameters.with_enumerate_all_solutions(true))
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                solutions.push(exprs.each_ref().map(|expr| view.value(expr).unwrap()));
            }
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    solutions
}

/// `simple_sat_program.py`
#[test]
fn simple_sat_program() {
    let mut model = Model::new();
    let [x, y, z] = three_vars(&mut model);
    model.add(x.ne(y));

    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_ne!(response.value(x), response.value(y));
    assert!(response.value(z).is_some_and(|z| (0..=2).contains(&z)));
}

/// `cp_sat_example.py`
#[test]
fn cp_sat_example() {
    let mut model = Model::new();
    let [x, y, z] = [(); 3].map(|()| model.int_var(0..=50));
    model.add((2 * x + 7 * y + 3 * z).le(50));
    model.add((3 * x - 5 * y + 7 * z).le(45));
    model.add((5 * x + 2 * y - 6 * z).le(37));
    model.maximize(2 * x + 2 * y + 3 * z);

    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value - 35.0).abs() < 1e-9);
    assert_eq!(response.solution, [7, 3, 5]);
}

/// `solve_and_print_intermediate_solutions_sample_sat.py`
#[test]
fn solve_and_print_intermediate_solutions() {
    let mut model = Model::new();
    let [x, y, z] = three_vars(&mut model);
    model.add(x.ne(y));
    model.maximize(x + 2 * y + 3 * z);

    let mut objectives = Vec::new();
    let response = Solver::default()
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                objectives.push(view.objective_value);
            }
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_eq!(response.solution, [1, 2, 2]);
    assert_eq!(objectives.last(), Some(&11.0));
    assert!(objectives.is_sorted(), "{objectives:?}");
}

/// `search_for_all_solutions_sample_sat.py`
#[test]
fn search_for_all_solutions() {
    let mut model = Model::new();
    let [x, y, _] = three_vars(&mut model);
    model.add(x.ne(y));
    let parameters = SatParameters::default().with_enumerate_all_solutions(true);

    let mut solutions = Vec::new();
    let response = Solver::new(&parameters)
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                solutions.push(view.solution.to_vec());
            }
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    solutions.sort();
    solutions.dedup();
    assert_eq!(solutions.len(), 18);
    assert!(solutions.iter().all(|solution| solution[0] != solution[1]));
}

/// `stop_after_n_solutions_sample_sat.py`
#[test]
fn stop_after_n_solutions() {
    let mut model = Model::new();
    three_vars(&mut model);
    let parameters = SatParameters::default().with_enumerate_all_solutions(true);

    let mut count = 0;
    let response = Solver::new(&parameters)
        .on_event(EventSet::SOLUTION, |_| {
            count += 1;
            if count >= 5 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(count, 5);
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
}

/// `assumptions_sample_sat.py`
#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "mirrors the names in the official sample"
)]
fn assumptions() {
    let mut model = Model::new();
    let [x, y, z] = [(); 3].map(|()| model.int_var(0..=10));
    let [a, b, c] = [(); 3].map(|()| model.bool_var());
    model.add(x.gt(y).only_if(a));
    model.add(y.gt(z).only_if(b));
    model.add(z.gt(x).only_if(c));
    model.add_assumptions([a, b, c]);

    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::INFEASIBLE);
    let mut core = response.sufficient_assumptions_for_infeasibility.clone();
    core.sort_unstable();
    assert_eq!(core, [a, b, c].map(BoolVar::index));
}

/// `solution_hinting_sample_sat.py`
#[test]
fn solution_hinting() {
    let mut model = Model::new();
    let [x, y, z] = three_vars(&mut model);
    model.add(x.ne(y));
    model.maximize(x + 2 * y + 3 * z);
    model.add_hint(x, 1);
    model.add_hint(y, 2);

    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value - 11.0).abs() < 1e-9);
    assert_eq!(response.solution, [1, 2, 2]);
}

/// `rabbits_and_pheasants_sat.py`
#[test]
fn rabbits_and_pheasants() {
    let mut model = Model::new();
    let rabbits = model.int_var(0..=100);
    let pheasants = model.int_var(0..=100);
    model.add((rabbits + pheasants).eq(20));
    model.add((4 * rabbits + 2 * pheasants).eq(56));

    let response = solve_optimal(&model);
    assert_eq!(response.value(rabbits), Some(8));
    assert_eq!(response.value(pheasants), Some(12));
}

/// `boolean_product_sample_sat.py`
#[test]
fn boolean_product() {
    let mut model = Model::new();
    let [x, y, p] = [(); 3].map(|()| model.bool_var());
    model.add(Constraint::bool_or([!x, !y, p]));
    model.add(Constraint::implication(p, x));
    model.add(Constraint::implication(p, y));

    let solutions = all_solutions(&model, SatParameters::default(), &[x, y, p].map(Into::into));
    assert_eq!(solutions.len(), 4);
    assert!(solutions.iter().all(|&[x, y, p]| p == x * y));
}

/// `channeling_sample_sat.py`
#[test]
fn channeling() {
    let mut model = Model::new();
    let x = model.int_var(0..=10);
    let y = model.int_var(0..=10);
    let b = model.bool_var();
    model.add(x.ge(5).only_if(b));
    model.add(x.lt(5).only_if(!b));
    model.add(y.eq(10 - x).only_if(b));
    model.add(y.eq(0).only_if(!b));
    model.add_decision_strategy(
        [x],
        VariableSelectionStrategy::CHOOSE_FIRST,
        DomainReductionStrategy::SELECT_MIN_VALUE,
    );
    let parameters = SatParameters::default().with_search_branching(SearchBranching::FIXED_SEARCH);

    let solutions = all_solutions(&model, parameters, &[x.into(), y.into(), b.into()]);
    let expected: Vec<[i64; 3]> = (0..=10)
        .map(|x| if x < 5 { [x, 0, 0] } else { [x, 10 - x, 1] })
        .collect();
    assert_eq!(solutions, expected);
}

/// `step_function_sample_sat.py`
#[test]
fn step_function() {
    let mut model = Model::new();
    let x = model.int_var(0..=20);
    let expr = model.int_var(0..=3);
    let b0 = model.bool_var();
    model.add(
        x.in_domain(Domain::from_intervals([(5, 6), (8, 10)]))
            .only_if(b0),
    );
    model.add(expr.eq(0).only_if(b0));
    let b2 = model.bool_var();
    model.add(
        x.in_domain(Domain::from_intervals([(0, 1), (3, 4), (11, 20)]))
            .only_if(b2),
    );
    model.add(expr.eq(2).only_if(b2));
    let b3 = model.bool_var();
    model.add(x.eq(7).only_if(b3));
    model.add(expr.eq(3).only_if(b3));
    model.add(Constraint::bool_or([b0, b2, b3]));
    model.add_decision_strategy(
        [x],
        VariableSelectionStrategy::CHOOSE_FIRST,
        DomainReductionStrategy::SELECT_MIN_VALUE,
    );
    let parameters = SatParameters::default().with_search_branching(SearchBranching::FIXED_SEARCH);

    let solutions = all_solutions(&model, parameters, &[x.into(), expr.into()]);
    let step = |x| match x {
        5 | 6 | 8..=10 => 0,
        7 => 3,
        _ => 2,
    };
    let expected: Vec<[i64; 2]> = (0..=20).filter(|&x| x != 2).map(|x| [x, step(x)]).collect();
    assert_eq!(solutions, expected);
}

/// `non_linear_sat.py`
#[test]
fn non_linear() {
    let perimeter = 20;
    let mut model = Model::new();
    let x = model.int_var(0..=perimeter);
    let y = model.int_var(0..=perimeter);
    model.add((2 * (x + y)).eq(perimeter));
    let area = model.int_var(0..=perimeter * perimeter);
    model.add(Constraint::multiplication_equality(area, [x, y]));
    model.maximize(area);

    let response = solve_optimal(&model);
    assert_eq!(
        [x, y, area].map(|var| response.value(var)),
        [Some(5), Some(5), Some(25)]
    );
}

/// `nqueens_sat.py` with its default board size of 8.
#[test]
fn nqueens() {
    let size = 8;
    let mut model = Model::new();
    let queens: Vec<IntVar> = (0..size).map(|_| model.int_var(0..=size - 1)).collect();
    model.add(Constraint::all_different(queens.iter().copied()));
    model.add(Constraint::all_different(
        (0..).zip(&queens).map(|(i, &q)| q + i),
    ));
    model.add(Constraint::all_different(
        (0..).zip(&queens).map(|(i, &q)| q - i),
    ));

    let mut count = 0;
    Solver::new(&SatParameters::default().with_enumerate_all_solutions(true))
        .on_event(EventSet::SOLUTION, |_| {
            count += 1;
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(count, 92);
}

/// `cp_is_fun_sat.py`: CP + IS + FUN = TRUE with distinct digits.
#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "mirrors the letters of the puzzle"
)]
fn cp_is_fun() {
    let base = 10;
    let mut model = Model::new();
    let mut digit = |lower| model.int_var(lower..=base - 1);
    let [c, p, i, s, f, u, n, t, r, e] = [1, 0, 1, 0, 1, 0, 0, 1, 0, 0].map(&mut digit);
    model.add(Constraint::all_different([c, p, i, s, f, u, n, t, r, e]));
    model.add(
        (c * base + p + i * base + s + f * base * base + u * base + n)
            .eq(t * base * base * base + r * base * base + u * base + e),
    );

    let mut count = 0;
    Solver::new(&SatParameters::default().with_enumerate_all_solutions(true))
        .on_event(EventSet::SOLUTION, |_| {
            count += 1;
            ControlFlow::Continue(())
        })
        .solve(model.proto())
        .unwrap();
    assert_eq!(count, 72);
}

/// `all_different_except_zero_sample_sat.py`
#[test]
fn all_different_except_zero() {
    let mut model = Model::new();
    let x: Vec<IntVar> = (0..5).map(|_| model.int_var(0..=10)).collect();
    let mut literals_per_value = vec![Vec::new(); 11];
    for &var in &x {
        let mut encoding = Vec::new();
        for (value, literals) in (0..).zip(&mut literals_per_value) {
            let literal = model.bool_var();
            model.add(var.eq(value).only_if(literal));
            literals.push(literal);
            encoding.push(literal);
        }
        model.add(Constraint::exactly_one(encoding));
    }
    for literals in literals_per_value.into_iter().skip(1) {
        model.add(Constraint::at_most_one(literals));
    }
    model.add(x[0].eq(0));
    model.add(x[1].eq(0));
    model.maximize(x.iter().copied().sum::<LinearExpr>());

    assert!((solve_optimal(&model).objective_value - 27.0).abs() < 1e-9);
}

/// `no_overlap_sample_sat.py`
#[test]
fn no_overlap() {
    let horizon = 21;
    let mut model = Model::new();
    let mut tasks = Vec::new();
    let mut ends = Vec::new();
    for duration in [2, 4, 3] {
        let start = model.int_var(0..=horizon);
        let end = model.int_var(0..=horizon);
        tasks.push(model.interval_var(start, duration, end));
        ends.push(end);
    }
    let weekends = [5, 12, 19].map(|start| model.interval_var(start, 2, start + 2));
    model.add(Constraint::no_overlap(tasks.into_iter().chain(weekends)));
    let makespan = model.int_var(0..=horizon);
    model.add(Constraint::max_equality(makespan, ends));
    model.minimize(makespan);

    assert_eq!(solve_optimal(&model).value(makespan), Some(11));
}

/// `minimal_jobshop_sat.py`
#[test]
fn minimal_jobshop() {
    // Each job is a sequence of (machine, duration) tasks.
    let jobs: [&[(usize, i64)]; 3] = [
        &[(0, 3), (1, 2), (2, 2)],
        &[(0, 2), (2, 1), (1, 4)],
        &[(1, 4), (2, 3)],
    ];
    let horizon: i64 = jobs
        .iter()
        .flat_map(|job| job.iter().map(|task| task.1))
        .sum();
    let mut model = Model::new();
    let mut machines = vec![Vec::new(); 3];
    let mut last_ends = Vec::new();
    for job in jobs {
        let mut previous_end: Option<IntVar> = None;
        for &(machine, duration) in job {
            let start = model.int_var(0..=horizon);
            let end = model.int_var(0..=horizon);
            machines[machine].push(model.interval_var(start, duration, end));
            if let Some(previous_end) = previous_end {
                model.add(start.ge(previous_end));
            }
            previous_end = Some(end);
        }
        last_ends.extend(previous_end);
    }
    for intervals in machines {
        model.add(Constraint::no_overlap(intervals));
    }
    let makespan = model.int_var(0..=horizon);
    model.add(Constraint::max_equality(makespan, last_ends));
    model.minimize(makespan);

    assert!((solve_optimal(&model).objective_value - 11.0).abs() < 1e-9);
}

/// `ranking_circuit_sample_sat.py`, which ranks optional tasks with a
/// circuit through a dummy node 0.
#[test]
fn ranking_circuit() {
    let horizon = 100;
    let durations = [1, 2, 3, 4];
    let mut model = Model::new();
    let mut starts = Vec::new();
    let mut presences = Vec::new();
    let mut ranks = Vec::new();
    let mut intervals = Vec::new();
    for (task, duration) in durations.into_iter().enumerate() {
        let start = model.int_var(0..=horizon);
        let presence = model.bool_var();
        intervals.push(model.optional_fixed_size_interval_var(start, duration, presence));
        if task < 2 {
            model.add(presence.eq(1));
        }
        starts.push(start);
        presences.push(presence);
        ranks.push(model.int_var(-1..=3));
    }
    model.add(Constraint::no_overlap(intervals));

    let mut arcs = Vec::new();
    for (i, node) in (1..=4).enumerate() {
        let first = model.bool_var();
        arcs.push((0, node, first));
        model.add(ranks[i].eq(0).only_if(first));
        model.add(starts[i].eq(0).only_if(first));
        arcs.push((node, 0, model.bool_var()));
        for (j, next) in (1..=4).enumerate() {
            if i == j {
                arcs.push((node, node, !presences[i]));
                model.add(ranks[i].eq(-1).only_if(!presences[i]));
            } else {
                let literal = model.bool_var();
                arcs.push((node, next, literal));
                model.add(ranks[j].eq(ranks[i] + 1).only_if(literal));
                model.add(starts[j].ge(starts[i] + durations[i]).only_if(literal));
            }
        }
    }
    let empty = model.bool_var();
    arcs.push((0, 0, empty));
    for &presence in &presences {
        model.add(Constraint::implication(empty, !presence));
    }
    model.add(Constraint::circuit(arcs));
    model.add(ranks[0].lt(ranks[1]));

    let makespan = model.int_var(0..=horizon);
    for ((&start, duration), &presence) in starts.iter().zip(durations).zip(&presences) {
        model.add((start + duration).le(makespan).only_if(presence));
    }
    model.minimize(2 * makespan - 7 * presences.iter().copied().sum::<LinearExpr>());

    let response = solve_optimal(&model);
    assert!((response.objective_value + 9.0).abs() < 1e-9);
    assert_eq!(response.value(makespan), Some(6));
    let performed: Vec<_> = presences.iter().map(|&p| response.value(p)).collect();
    assert_eq!(performed, [Some(true), Some(true), Some(true), Some(false)]);
}
