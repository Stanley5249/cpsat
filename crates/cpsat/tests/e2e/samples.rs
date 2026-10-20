//! Rust versions of the official OR-Tools v9.15 CP-SAT samples in
//! `ortools/sat/samples`. Each test asserts what the Python sample prints,
//! checked by running the samples with the `ortools` that `just gen-fixtures`
//! pins.

use std::ops::ControlFlow;

use cpsat::prelude::*;
use cpsat::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};
use cpsat::proto::sat_parameters::SearchBranching;
use cpsat::proto::{CpSolverStatus, SatParameters};

use crate::common::solve_optimal;

/// The three variables in `[0, 2]` that most samples share.
fn three_vars(model: &mut CpModel) -> [IntVar; 3] {
    std::array::from_fn(|_| model.new_var(0..=2))
}

/// Every solution of `model`, as the values of `exprs`, in the order found.
fn all_solutions<const N: usize>(
    model: &CpModel,
    parameters: SatParameters,
    exprs: &[LinearExpr; N],
) -> Vec<[i64; N]> {
    let mut solutions = Vec::new();
    let response = CpSolver::new(&parameters.with_enumerate_all_solutions(true))
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
    let mut model = CpModel::new();
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
    let mut model = CpModel::new();
    let [x, y, z] = std::array::from_fn(|_| model.new_var(0..=50));
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
    let mut model = CpModel::new();
    let [x, y, z] = three_vars(&mut model);
    model.add(x.ne(y));
    model.maximize(x + 2 * y + 3 * z);

    let mut objectives = Vec::new();
    let response = CpSolver::default()
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
}

/// `search_for_all_solutions_sample_sat.py`
#[test]
fn search_for_all_solutions() {
    let mut model = CpModel::new();
    let [x, y, _] = three_vars(&mut model);
    model.add(x.ne(y));
    let parameters = SatParameters::default().with_enumerate_all_solutions(true);

    let mut solutions = Vec::new();
    let response = CpSolver::new(&parameters)
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
    let mut model = CpModel::new();
    three_vars(&mut model);
    let parameters = SatParameters::default().with_enumerate_all_solutions(true);

    let mut count = 0;
    let response = CpSolver::new(&parameters)
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
    let mut model = CpModel::new();
    let [x, y, z] = std::array::from_fn(|_| model.new_var(0..=10));
    let [a, b, c] = std::array::from_fn(|_| model.new_var(Bool));
    model.add(x.gt(y).only_enforce_if(a));
    model.add(y.gt(z).only_enforce_if(b));
    model.add(z.gt(x).only_enforce_if(c));
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
    let mut model = CpModel::new();
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
    let mut model = CpModel::new();
    let rabbits = model.new_var(0..=100);
    let pheasants = model.new_var(0..=100);
    model.add((rabbits + pheasants).eq(20));
    model.add((4 * rabbits + 2 * pheasants).eq(56));

    let response = solve_optimal(&model);
    assert_eq!(response.value(rabbits), Some(8));
    assert_eq!(response.value(pheasants), Some(12));
}

/// `boolean_product_sample_sat.py`
#[test]
fn boolean_product() {
    let mut model = CpModel::new();
    let [x, y, p] = std::array::from_fn(|_| model.new_var(Bool));
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
    let mut model = CpModel::new();
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
    let b = model.new_var(Bool);
    model.add(x.ge(5).only_enforce_if(b));
    model.add(x.lt(5).only_enforce_if(!b));
    model.add(y.eq(10 - x).only_enforce_if(b));
    model.add(y.eq(0).only_enforce_if(!b));
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
    let mut model = CpModel::new();
    let x = model.new_var(0..=20);
    let expr = model.new_var(0..=3);
    let b0 = model.new_var(Bool);
    model.add(
        x.in_domain(Domain::from_intervals([(5, 6), (8, 10)]))
            .only_enforce_if(b0),
    );
    model.add(expr.eq(0).only_enforce_if(b0));
    let b2 = model.new_var(Bool);
    model.add(
        x.in_domain(Domain::from_intervals([(0, 1), (3, 4), (11, 20)]))
            .only_enforce_if(b2),
    );
    model.add(expr.eq(2).only_enforce_if(b2));
    let b3 = model.new_var(Bool);
    model.add(x.eq(7).only_enforce_if(b3));
    model.add(expr.eq(3).only_enforce_if(b3));
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
    let mut model = CpModel::new();
    let x = model.new_var(0..=perimeter);
    let y = model.new_var(0..=perimeter);
    model.add((2 * (x + y)).eq(perimeter));
    let area = model.new_var(0..=perimeter * perimeter);
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
    let mut model = CpModel::new();
    let queens: Vec<IntVar> = (0..size).map(|_| model.new_var(0..=size - 1)).collect();
    model.add(Constraint::all_different(queens.iter().copied()));
    model.add(Constraint::all_different(
        (0..).zip(&queens).map(|(i, &q)| q + i),
    ));
    model.add(Constraint::all_different(
        (0..).zip(&queens).map(|(i, &q)| q - i),
    ));

    let mut count = 0;
    CpSolver::new(&SatParameters::default().with_enumerate_all_solutions(true))
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
    let mut model = CpModel::new();
    let mut digit = |lower| model.new_var(lower..=base - 1);
    let [c, p, i, s, f, u, n, t, r, e] = [1, 0, 1, 0, 1, 0, 0, 1, 0, 0].map(&mut digit);
    model.add(Constraint::all_different([c, p, i, s, f, u, n, t, r, e]));
    model.add(
        (c * base + p + i * base + s + f * base * base + u * base + n)
            .eq(t * base * base * base + r * base * base + u * base + e),
    );

    let mut count = 0;
    CpSolver::new(&SatParameters::default().with_enumerate_all_solutions(true))
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
    let mut model = CpModel::new();
    let x: Vec<IntVar> = (0..5).map(|_| model.new_var(0..=10)).collect();
    let mut literals_per_value = vec![Vec::new(); 11];
    for &var in &x {
        let mut encoding = Vec::new();
        for (value, literals) in (0..).zip(&mut literals_per_value) {
            let literal = model.new_var(Bool);
            model.add(var.eq(value).only_enforce_if(literal));
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
    let mut model = CpModel::new();
    let mut tasks = Vec::new();
    let mut ends = Vec::new();
    for duration in [2, 4, 3] {
        let start = model.new_var(0..=horizon);
        let end = model.new_var(0..=horizon);
        tasks.push(model.new_var(Interval::new(start, duration, end)));
        ends.push(end);
    }
    let weekends = [5, 12, 19].map(|start| model.new_var(Interval::new(start, 2, start + 2)));
    model.add(Constraint::no_overlap(tasks.into_iter().chain(weekends)));
    let makespan = model.new_var(0..=horizon);
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
    let mut model = CpModel::new();
    let mut machines = vec![Vec::new(); 3];
    let mut last_ends = Vec::new();
    for job in jobs {
        let mut previous_end: Option<IntVar> = None;
        for &(machine, duration) in job {
            let start = model.new_var(0..=horizon);
            let end = model.new_var(0..=horizon);
            machines[machine].push(model.new_var(Interval::new(start, duration, end)));
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
    let makespan = model.new_var(0..=horizon);
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
    let mut model = CpModel::new();
    let mut starts = Vec::new();
    let mut presences = Vec::new();
    let mut ranks = Vec::new();
    let mut intervals = Vec::new();
    for (task, duration) in durations.into_iter().enumerate() {
        let start = model.new_var(0..=horizon);
        let presence = model.new_var(Bool);
        intervals
            .push(model.new_var(Interval::fixed_size(start, duration).only_enforce_if(presence)));
        if task < 2 {
            model.add(presence.eq(1));
        }
        starts.push(start);
        presences.push(presence);
        ranks.push(model.new_var(-1..=3));
    }
    model.add(Constraint::no_overlap(intervals));

    let mut arcs = Vec::new();
    for (i, node) in (1..=4).enumerate() {
        let first = model.new_var(Bool);
        arcs.push((0, node, first));
        model.add(ranks[i].eq(0).only_enforce_if(first));
        model.add(starts[i].eq(0).only_enforce_if(first));
        arcs.push((node, 0, model.new_var(Bool)));
        for (j, next) in (1..=4).enumerate() {
            if i == j {
                arcs.push((node, node, !presences[i]));
                model.add(ranks[i].eq(-1).only_enforce_if(!presences[i]));
            } else {
                let literal = model.new_var(Bool);
                arcs.push((node, next, literal));
                model.add(ranks[j].eq(ranks[i] + 1).only_enforce_if(literal));
                model.add(
                    starts[j]
                        .ge(starts[i] + durations[i])
                        .only_enforce_if(literal),
                );
            }
        }
    }
    let empty = model.new_var(Bool);
    arcs.push((0, 0, empty));
    for &presence in &presences {
        model.add(Constraint::implication(empty, !presence));
    }
    model.add(Constraint::circuit(arcs));
    model.add(ranks[0].lt(ranks[1]));

    let makespan = model.new_var(0..=horizon);
    for ((&start, duration), &presence) in starts.iter().zip(durations).zip(&presences) {
        model.add((start + duration).le(makespan).only_enforce_if(presence));
    }
    model.minimize(2 * makespan - 7 * presences.iter().copied().sum::<LinearExpr>());

    let response = solve_optimal(&model);
    assert!((response.objective_value + 9.0).abs() < 1e-9);
    assert_eq!(response.value(makespan), Some(6));
    let performed: Vec<_> = presences.iter().map(|&p| response.value(p)).collect();
    assert_eq!(performed, [Some(true), Some(true), Some(true), Some(false)]);
}

/// `assignment_groups_sat.py`, which limits which workers work with
/// `allowed_assignments`.
#[test]
fn assignment_groups() {
    let costs: [[i64; 6]; 12] = [
        [90, 76, 75, 70, 50, 74],
        [35, 85, 55, 65, 48, 101],
        [125, 95, 90, 105, 59, 120],
        [45, 110, 95, 115, 104, 83],
        [60, 105, 80, 75, 59, 62],
        [45, 65, 110, 95, 47, 31],
        [38, 51, 107, 41, 69, 99],
        [47, 85, 57, 71, 92, 77],
        [39, 63, 97, 49, 118, 56],
        [47, 101, 71, 60, 88, 109],
        [17, 39, 103, 64, 61, 92],
        [101, 45, 83, 59, 92, 27],
    ];
    let group1 = [
        [0, 0, 1, 1],
        [0, 1, 0, 1],
        [0, 1, 1, 0],
        [1, 1, 0, 0],
        [1, 0, 1, 0],
    ];
    let group2 = [
        [0, 0, 1, 1],
        [0, 1, 0, 1],
        [0, 1, 1, 0],
        [1, 1, 0, 0],
        [1, 0, 0, 1],
    ];
    let group3 = [
        [0, 0, 1, 1],
        [0, 1, 0, 1],
        [0, 1, 1, 0],
        [1, 0, 1, 0],
        [1, 0, 0, 1],
    ];

    let mut model = CpModel::new();
    let x: Vec<Vec<BoolVar>> = costs
        .iter()
        .map(|row| row.iter().map(|_| model.new_var(Bool)).collect())
        .collect();
    for worker in &x {
        model.add(Constraint::at_most_one(worker.iter().copied()));
    }
    for task in 0..6 {
        model.add(Constraint::exactly_one(x.iter().map(|worker| worker[task])));
    }
    let work: Vec<BoolVar> = x
        .iter()
        .map(|worker| {
            let work = model.new_var(Bool);
            model.add(work.eq(worker.iter().copied().sum::<LinearExpr>()));
            work
        })
        .collect();
    model.add(Constraint::allowed_assignments(
        work[0..4].iter().copied(),
        group1,
    ));
    model.add(Constraint::allowed_assignments(
        work[4..8].iter().copied(),
        group2,
    ));
    model.add(Constraint::allowed_assignments(
        work[8..12].iter().copied(),
        group3,
    ));
    let total: LinearExpr = x
        .iter()
        .zip(&costs)
        .flat_map(|(worker, row)| worker.iter().zip(row).map(|(&var, &cost)| var * cost))
        .sum();
    model.minimize(total);

    assert!((solve_optimal(&model).objective_value - 239.0).abs() < 1e-9);
}

/// `index_first_boolvar_true_sample_sat.py`, which channels an index to the
/// first true Boolean variable with `min_equality`.
#[test]
fn index_first_boolvar_true() {
    let count = 5;
    let mut model = CpModel::new();
    let bools: Vec<BoolVar> = (0..count).map(|_| model.new_var(Bool)).collect();
    let index = model.new_var(0..=count);
    model.add(Constraint::min_equality(
        index,
        (0..).zip(&bools).map(|(i, &b)| count - b * (count - i)),
    ));
    model.add_decision_strategy(
        bools.iter().copied(),
        VariableSelectionStrategy::CHOOSE_FIRST,
        DomainReductionStrategy::SELECT_MIN_VALUE,
    );
    let parameters = SatParameters::default().with_search_branching(SearchBranching::FIXED_SEARCH);

    let mut exprs: Vec<LinearExpr> = bools.iter().map(|&b| b.into()).collect();
    exprs.push(index.into());
    let exprs: [LinearExpr; 6] = exprs.try_into().unwrap();
    let solutions = all_solutions(&model, parameters, &exprs);
    // The sample prints "00000 -> 5" first and "11111 -> 0" last.
    let expected: Vec<[i64; 6]> = (0..32)
        .map(|bits: i64| {
            let b = |i: i64| (bits >> (4 - i)) & 1;
            let first = (0..5).find(|&i| b(i) == 1).unwrap_or(5);
            [b(0), b(1), b(2), b(3), b(4), first]
        })
        .collect();
    assert_eq!(solutions, expected);
}

/// `solve_with_time_limit_sample_sat.py`
#[test]
fn solve_with_time_limit() {
    let mut model = CpModel::new();
    let [x, y, _] = three_vars(&mut model);
    model.add(x.ne(y));
    let parameters = SatParameters::default().with_max_time_in_seconds(10.0);

    let response = model.solve(&parameters).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_ne!(response.value(x), response.value(y));
}

/// The (duration, load, priority) of t1 to t30 in `cumulative_variable_profile`.
const CUMULATIVE_TASKS: [(i64, i64, i64); 30] = [
    (60, 3, 2),
    (180, 2, 1),
    (240, 5, 3),
    (90, 4, 2),
    (120, 3, 1),
    (300, 3, 3),
    (120, 1, 2),
    (100, 5, 2),
    (110, 2, 1),
    (300, 5, 3),
    (90, 4, 2),
    (120, 3, 1),
    (250, 3, 3),
    (120, 1, 2),
    (40, 5, 3),
    (70, 4, 2),
    (90, 8, 1),
    (40, 3, 3),
    (120, 5, 2),
    (60, 3, 2),
    (180, 2, 1),
    (240, 5, 3),
    (90, 4, 2),
    (120, 3, 1),
    (300, 3, 3),
    (120, 1, 2),
    (100, 5, 2),
    (110, 2, 1),
    (300, 5, 3),
    (90, 4, 2),
];

/// `cumulative_variable_profile_sample_sat.py`, which schedules optional tasks
/// between a minimum and a maximum load profile. The sample asserts its own
/// `check_solution` and prints no objective.
#[test]
fn cumulative_variable_profile() {
    // Per two-hour period starting at hour 0, 2, ..., 22.
    let max_loads: [i64; 12] = [0, 0, 3, 6, 8, 12, 8, 12, 10, 6, 4, 0];
    let min_loads: [i64; 12] = [0, 0, 0, 0, 3, 3, 1, 3, 3, 1, 1, 0];
    let tasks = CUMULATIVE_TASKS;
    let minutes_per_hour = 60;
    let period_length = 2 * minutes_per_hour;
    let horizon = 24 * minutes_per_hour;
    let max_load = max_loads.into_iter().max().unwrap();

    let mut model = CpModel::new();
    let starts: Vec<IntVar> = tasks
        .iter()
        .map(|&(duration, ..)| model.new_var(0..=horizon - duration))
        .collect();
    let performed: Vec<BoolVar> = tasks.iter().map(|_| model.new_var(Bool)).collect();

    // The max profile: fixed intervals fill the gap up to the max capacity.
    let mut max_tasks = Vec::new();
    for (&start, (&(duration, load, _), &presence)) in
        starts.iter().zip(tasks.iter().zip(&performed))
    {
        let interval = Interval::fixed_size(start, duration).only_enforce_if(presence);
        max_tasks.push((model.new_var(interval), load));
    }
    for (hour, load) in (0..).step_by(2).zip(max_loads) {
        let interval = Interval::fixed_size(hour * minutes_per_hour, period_length);
        max_tasks.push((model.new_var(interval), max_load - load));
    }
    model.add(Constraint::cumulative(max_load, max_tasks));

    // The min profile: the complement of each task plus fixed intervals.
    let mut min_tasks = Vec::new();
    for (&start, (&(duration, load, _), &presence)) in
        starts.iter().zip(tasks.iter().zip(&performed))
    {
        let prefix = Interval::new(0, start, start).only_enforce_if(presence);
        let suffix = Interval::new(start + duration, horizon - start - duration, horizon)
            .only_enforce_if(presence);
        min_tasks.push((model.new_var(prefix), load));
        min_tasks.push((model.new_var(suffix), load));
    }
    for (hour, load) in (0..).step_by(2).zip(min_loads) {
        let interval = Interval::fixed_size(hour * minutes_per_hour, period_length);
        min_tasks.push((model.new_var(interval), load));
    }
    let total_demand: i64 = tasks.iter().map(|task| task.1).sum();
    let complement_capacity = model.new_var(0..=total_demand);
    model.add(
        complement_capacity.eq(tasks
            .iter()
            .zip(&performed)
            .map(|(task, &presence)| presence * task.1)
            .sum::<LinearExpr>()),
    );
    model.add(Constraint::cumulative(complement_capacity, min_tasks));

    let max_priority = tasks.iter().map(|task| task.2).max().unwrap();
    model.maximize(
        tasks
            .iter()
            .zip(&performed)
            .map(|(task, &presence)| presence * (max_priority + 1 - task.2))
            .sum::<LinearExpr>(),
    );

    let response = model.solve(&SatParameters::default()).unwrap();
    assert!(response.status().has_solution(), "{}", response.status());

    // The sample's `check_solution`.
    let mut load_profile = vec![0; usize::try_from(horizon).unwrap()];
    for (&start, (&(duration, load, _), &presence)) in
        starts.iter().zip(tasks.iter().zip(&performed))
    {
        if response.value(presence) == Some(true) {
            let start = usize::try_from(response.value(start).unwrap()).unwrap();
            for slot in &mut load_profile[start..start + usize::try_from(duration).unwrap()] {
                *slot += load;
            }
        }
    }
    for (minute, &actual) in (0..).zip(&load_profile) {
        let period = usize::try_from(minute / period_length).unwrap();
        assert!(
            (min_loads[period]..=max_loads[period]).contains(&actual),
            "load {actual} at minute {minute}"
        );
    }
}
