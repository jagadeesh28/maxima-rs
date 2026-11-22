//! Golden integration tests for transcendental Risch u-substitution (Phase 25).

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::integrate::integrate;

fn assert_antiderivative(f: &Expr, var: &str, points: &[f64]) {
    let big_f = integrate(f, var);
    assert!(
        !matches!(big_f, Expr::Integral(_, _)),
        "unexpected noun form integrating {f}"
    );
    let d = differentiate(&big_f, var);
    for &xv in points {
        let mut env = HashMap::new();
        env.insert(var.to_string(), xv);
        if let (Some(fa), Some(db)) = (f.eval(&env), d.eval(&env)) {
            assert!(
                (fa - db).abs() < 1e-6,
                "at {var}={xv}: d/d{var}(F) = {db} vs f = {fa} (F = {big_f})"
            );
        }
    }
}

#[test]
fn test_integrate_x_exp_x_squared() {
    let x = Expr::var("x");
    let f = Expr::mul(x.clone(), Expr::exp(Expr::pow(x, Expr::int(2))));
    assert_antiderivative(&f, "x", &[-1.5, -0.5, 0.0, 0.7, 1.2]);
}

#[test]
fn test_integrate_x_squared_exp_x_cubed() {
    let x = Expr::var("x");
    let f = Expr::mul(
        Expr::pow(x.clone(), Expr::int(2)),
        Expr::exp(Expr::pow(x, Expr::int(3))),
    );
    assert_antiderivative(&f, "x", &[-1.0, -0.5, 0.0, 0.6, 1.1]);
}

#[test]
fn test_integrate_exp_over_exp_plus_one() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::exp(x.clone()), Expr::add(Expr::exp(x), Expr::int(1)));
    assert_antiderivative(&f, "x", &[-2.0, -0.5, 0.0, 1.0, 2.5]);
}

#[test]
fn test_integrate_derivative_over_function() {
    let x = Expr::var("x");
    // ∫ 2x / (x^2 + 1) dx = ln(x^2 + 1)
    let f = Expr::div(
        Expr::mul(Expr::int(2), x.clone()),
        Expr::add(Expr::pow(x, Expr::int(2)), Expr::int(1)),
    );
    assert_antiderivative(&f, "x", &[-2.0, -0.5, 0.0, 1.0, 3.0]);
}

#[test]
fn test_integrate_log_is_elementary() {
    let x = Expr::var("x");
    assert_antiderivative(&Expr::ln(x), "x", &[0.5, 1.0, 3.0]);
}

#[test]
fn test_non_elementary_gaussian_returns_noun() {
    let x = Expr::var("x");
    let f = Expr::exp(Expr::neg(Expr::pow(x, Expr::int(2))));
    let out = integrate(&f, "x");
    assert!(matches!(out, Expr::Integral(_, _)));
}
