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
