//! Golden integration tests for exact rational algebra (Phase 23).

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::poly::{expand, factor, partfrac, ratsimp};

fn eval_at(e: &Expr, x: f64) -> f64 {
    let mut env = HashMap::new();
    env.insert("x".to_string(), x);
    e.eval(&env).expect("evaluation failed")
}

#[test]
fn test_ratsimp_cancels_common_factor() {
    let x = Expr::var("x");
    let e = Expr::div(
        Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        Expr::sub(x, Expr::int(1)),
    );
    let out = ratsimp(&e).expect("ratsimp");
    for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
        assert!((eval_at(&out, xv) - (xv + 1.0)).abs() < 1e-9);
    }
}

#[test]
fn test_ratsimp_combines_sum_of_reciprocals() {
    let x = Expr::var("x");
    let e = Expr::add(
        Expr::div(Expr::int(1), x.clone()),
        Expr::div(Expr::int(1), x),
    );
    let out = ratsimp(&e).expect("ratsimp");
    for xv in [1.0_f64, -2.0, 3.0] {
        assert!((eval_at(&out, xv) - 2.0 / xv).abs() < 1e-9);
    }
}

#[test]
fn test_factor_roundtrip_difference_of_squares() {
    let x = Expr::var("x");
    let e = Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1));
    let factored = factor(&e);
    let back = expand(&factored);
    for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
        assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
    }
}

#[test]
fn test_partfrac_one_over_x_squared_minus_one() {
    let x = Expr::var("x");
    let e = Expr::div(
        Expr::int(1),
        Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1)),
    );
    let out = partfrac(&e, "x").expect("partfrac");
    for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
        assert!((eval_at(&out, xv) - eval_at(&e, xv)).abs() < 1e-9);
    }
}
