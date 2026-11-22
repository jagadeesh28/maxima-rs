//! Golden integration tests for ODE solvers (Phase 30).
//!
//! Each solution is verified by substituting it back: the residual `y'' + ...`
//! evaluates to ~0 at sample points.

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::solve::ode2;

fn yprime() -> Expr {
    Expr::derivative(Expr::var("y"), Expr::var("x"))
}

fn ysecond() -> Expr {
    Expr::derivative(yprime(), Expr::var("x"))
}

fn eval_with(e: &Expr, vars: &[(&str, f64)]) -> f64 {
    let mut env = HashMap::new();
    for (k, v) in vars {
        env.insert((*k).to_string(), *v);
    }
    e.eval(&env).expect("evaluation failed")
}

#[test]
fn test_first_order_linear() {
    // y' + y = 0
    let ode = Expr::add(yprime(), Expr::var("y"));
    let sol = ode2(&ode, "y", "x").expect("solution");
    assert_eq!(sol.constants, vec!["_C1".to_string()]);
    let residual = Expr::add(differentiate(&sol.solution, "x"), sol.solution.clone());
    let v = eval_with(&residual, &[("x", 0.5), ("_C1", 1.0)]);
    assert!(v.abs() < 1e-9, "residual {v}");
}

#[test]
fn test_second_order_oscillator() {
    // y'' + w^2 y = 0
    let w = Expr::var("w");
    let ode = Expr::add(
        ysecond(),
        Expr::mul(Expr::pow(w.clone(), Expr::int(2)), Expr::var("y")),
    );
    let sol = ode2(&ode, "y", "x").expect("solution");
    assert_eq!(sol.constants, vec!["_C1".to_string(), "_C2".to_string()]);
    let d1 = differentiate(&sol.solution, "x");
    let d2 = differentiate(&d1, "x");
    let residual = Expr::add(
        d2,
        Expr::mul(Expr::pow(w, Expr::int(2)), sol.solution.clone()),
    );
    let v = eval_with(
        &residual,
        &[("x", 0.4), ("w", 1.7), ("_C1", 1.0), ("_C2", 1.0)],
    );
    assert!(v.abs() < 1e-9, "residual {v}");
}

#[test]
fn test_second_order_real_roots() {
    // y'' - y = 0
    let ode = Expr::sub(ysecond(), Expr::var("y"));
    let sol = ode2(&ode, "y", "x").expect("solution");
    let d1 = differentiate(&sol.solution, "x");
    let d2 = differentiate(&d1, "x");
    let residual = Expr::sub(d2, sol.solution.clone());
    let v = eval_with(&residual, &[("x", 0.7), ("_C1", 1.0), ("_C2", 1.0)]);
    assert!(v.abs() < 1e-9, "residual {v}");
}
