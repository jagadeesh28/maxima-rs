//! Golden integration tests for trigonometric simplification and Maxima trigi.lisp rules.

use maxima_rs::prelude::*;
use sym_core::saturate::SaturationConfig;

#[test]
fn test_trig_zero_values() {
    let config = SaturationConfig::default();

    // sin(0) -> 0
    let e1 = Expr::sin(Expr::int(0));
    assert_eq!(optimize(&e1, &config), Expr::int(0));

    // cos(0) -> 1
    let e2 = Expr::cos(Expr::int(0));
    assert_eq!(optimize(&e2, &config), Expr::int(1));

    // tan(0) -> 0
    let e3 = Expr::tan(Expr::int(0));
    assert_eq!(optimize(&e3, &config), Expr::int(0));
}

#[test]
fn test_pythagorean_identity() {
    let config = SaturationConfig::default();
    let x = Expr::var("x");

    // sin(x)^2 + cos(x)^2 -> 1
    let sin_sq = Expr::pow(Expr::sin(x.clone()), Expr::int(2));
    let cos_sq = Expr::pow(Expr::cos(x.clone()), Expr::int(2));
    let pyth = Expr::add(sin_sq.clone(), cos_sq.clone());
    assert_eq!(optimize(&pyth, &config), Expr::int(1));

    // cos(x)^2 + sin(x)^2 -> 1
    let pyth_comm = Expr::add(cos_sq, sin_sq);
    assert_eq!(optimize(&pyth_comm, &config), Expr::int(1));
}

#[test]
fn test_pythagorean_in_larger_expression() {
    let config = SaturationConfig::default();
    let x = Expr::var("x");

    // (sin(x)^2 + cos(x)^2) * x -> 1 * x -> x
    let expr = Expr::mul(
        Expr::add(
            Expr::pow(Expr::sin(x.clone()), Expr::int(2)),
            Expr::pow(Expr::cos(x.clone()), Expr::int(2)),
        ),
        x.clone(),
    );
    assert_eq!(optimize(&expr, &config), x);
}
