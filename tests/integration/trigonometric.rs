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

#[test]
fn test_hyperbolic_zero_values() {
    let config = SaturationConfig::default();
    assert_eq!(optimize(&Expr::sinh(Expr::int(0)), &config), Expr::int(0));
    assert_eq!(optimize(&Expr::cosh(Expr::int(0)), &config), Expr::int(1));
    assert_eq!(optimize(&Expr::tanh(Expr::int(0)), &config), Expr::int(0));
    // sinh(-0) -> -sinh(0) -> 0
    assert_eq!(
        optimize(&Expr::sinh(Expr::neg(Expr::int(0))), &config),
        Expr::int(0)
    );
}

#[test]
fn test_hyperbolic_pythagorean_identity() {
    let x = Expr::var("x");
    // cosh(x)^2 - sinh(x)^2 -> 1
    let expr = Expr::sub(
        Expr::pow(Expr::cosh(x.clone()), Expr::int(2)),
        Expr::pow(Expr::sinh(x), Expr::int(2)),
    );
    assert_eq!(optimize(&expr, &SaturationConfig::default()), Expr::int(1));
}

#[test]
fn test_inverse_trig_special_values() {
    let config = SaturationConfig::default();
    assert_eq!(optimize(&Expr::asin(Expr::int(0)), &config), Expr::int(0));
    assert_eq!(optimize(&Expr::acos(Expr::int(1)), &config), Expr::int(0));
    assert_eq!(optimize(&Expr::atan(Expr::int(0)), &config), Expr::int(0));
}
