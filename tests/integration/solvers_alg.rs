//! Golden integration tests for closed-form algebraic solvers (Phase 29).

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::solve::solve;

fn root_is_valid(expr: &Expr, var: &str, root: &Expr) -> bool {
    let empty: HashMap<String, f64> = HashMap::new();
    let Some(rv) = root.eval(&empty) else {
        return false;
    };
    let mut env = HashMap::new();
    env.insert(var.to_string(), rv);
    matches!(expr.eval(&env), Some(v) if v.abs() < 1e-6)
}

#[test]
fn test_linear_equation() {
    let x = Expr::var("x");
    let e = Expr::sub(Expr::mul(Expr::int(2), x), Expr::int(4));
    let sols = solve(&e, "x");
    assert_eq!(sols.0, vec![Expr::int(2)]);
}

#[test]
fn test_quadratic_with_rational_roots() {
    let x = Expr::var("x");
    let e = Expr::add(
        Expr::sub(
            Expr::pow(x.clone(), Expr::int(2)),
            Expr::mul(Expr::int(5), x),
        ),
        Expr::int(6),
    );
    let sols = solve(&e, "x");
    assert_eq!(sols.0.len(), 2);
    for r in &sols.0 {
        assert!(root_is_valid(&e, "x", r), "invalid root {r}");
    }
}

#[test]
fn test_quadratic_with_irrational_roots() {
    let x = Expr::var("x");
    let e = Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(2));
    let sols = solve(&e, "x");
    assert_eq!(sols.0.len(), 2);
    for r in &sols.0 {
        assert!(root_is_valid(&e, "x", r), "invalid root {r}");
    }
}

#[test]
fn test_cubic_with_rational_roots() {
    let x = Expr::var("x");
    let e = Expr::add(
        Expr::add(
            Expr::sub(
                Expr::pow(x.clone(), Expr::int(3)),
                Expr::mul(Expr::int(6), Expr::pow(x.clone(), Expr::int(2))),
            ),
            Expr::mul(Expr::int(11), x),
        ),
        Expr::neg(Expr::int(6)),
    );
    let sols = solve(&e, "x");
    assert_eq!(sols.0.len(), 3);
    for r in &sols.0 {
        assert!(root_is_valid(&e, "x", r), "invalid root {r}");
    }
}

#[test]
fn test_quartic_with_rational_roots() {
    let x = Expr::var("x");
    let e = Expr::add(
        Expr::sub(
            Expr::pow(x.clone(), Expr::int(4)),
            Expr::mul(Expr::int(5), Expr::pow(x, Expr::int(2))),
        ),
        Expr::int(4),
    );
    let sols = solve(&e, "x");
    assert_eq!(sols.0.len(), 4);
    for r in &sols.0 {
        assert!(root_is_valid(&e, "x", r), "invalid root {r}");
    }
}

#[test]
fn test_transcendental_isolated() {
    // exp(x) - 2 = 0  =>  x = ln(2)
    let x = Expr::var("x");
    let e = Expr::sub(Expr::exp(x), Expr::int(2));
    let sols = solve(&e, "x");
    assert_eq!(sols.0, vec![Expr::ln(Expr::int(2))]);
}

#[test]
fn test_phase50_complex_parametric_status_linsolve_and_isolate() {
    use sym_core::solve::{SolveStatus, linsolve, solve_classified};

    let x = Expr::var("x");
    let y = Expr::var("y");
    let a = Expr::var("a");
    let b = Expr::var("b");

    // 1. Complex roots: x^2 + 1 = 0 -> [%i, -%i]
    let cplx = solve(
        &Expr::add(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        "x",
    );
    assert_eq!(cplx.0.len(), 2);
    assert!(cplx.is_complete());

    // 2. Parametric linear: a*x + b = 0 -> [-b/a]
    let param_sol = solve(&Expr::add(Expr::mul(a, x.clone()), b), "x");
    assert_eq!(param_sol.0.len(), 1);
    assert!(param_sol.is_complete());

    // 3. 0 = 0 (All) vs 1 = 0 (Empty)
    assert_eq!(solve_classified(&Expr::int(0), "x").1, SolveStatus::All);
    assert_eq!(solve_classified(&Expr::int(1), "x").1, SolveStatus::Empty);

    // 4. linsolve: 2x + y - 5 = 0, x - y - 1 = 0 => x = 2, y = 1
    let eq1 = Expr::sub(
        Expr::add(Expr::mul(Expr::int(2), x.clone()), y.clone()),
        Expr::int(5),
    );
    let eq2 = Expr::sub(Expr::sub(x.clone(), y), Expr::int(1));
    let lin = linsolve(&[eq1, eq2], &["x", "y"]).expect("linsolve");
    assert_eq!(lin, vec![Expr::int(2), Expr::int(1)]);

    // 5. isolate: 5 - x = 0 -> 5, and exp(x) - x = 0 must NOT return circular x = ln(x)
    assert_eq!(
        solve(&Expr::sub(Expr::int(5), x.clone()), "x").0,
        vec![Expr::int(5)]
    );
    assert!(solve(&Expr::sub(Expr::exp(x.clone()), x), "x").0.is_empty());
}
