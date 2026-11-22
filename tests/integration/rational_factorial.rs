//! Golden tests for rational normalization (`nrat4.lisp`) and factorial (`sfacts.lisp`) rules.

use maxima_rs::prelude::*;
use sym_core::saturate::SaturationConfig;

#[test]
fn test_factorial_zero_and_one() {
    let cfg = SaturationConfig::default();
    assert_eq!(optimize(&Expr::factorial(Expr::int(0)), &cfg), Expr::int(1));
    assert_eq!(optimize(&Expr::factorial(Expr::int(1)), &cfg), Expr::int(1));
}

#[test]
fn test_factorial_constant_folding_via_analysis() {
    let cfg = SaturationConfig::default();
    // The e-graph analysis folds 5! to 120 without needing a rewrite.
    assert_eq!(
        optimize(&Expr::factorial(Expr::int(5)), &cfg),
        Expr::int(120)
    );
}

#[test]
fn test_factorial_quotient() {
    let cfg = SaturationConfig::default();
    let n = Expr::var("n");
    let expr = Expr::div(
        Expr::factorial(n.clone()),
        Expr::factorial(Expr::sub(n.clone(), Expr::int(1))),
    );
    assert_eq!(optimize(&expr, &cfg), n);
}

#[test]
fn test_rational_double_reciprocal() {
    let cfg = SaturationConfig::default();
    let x = Expr::var("x");
    let expr = Expr::div(Expr::int(1), Expr::div(Expr::int(1), x.clone()));
    assert_eq!(optimize(&expr, &cfg), x);
}

#[test]
fn test_rational_neg_over_neg() {
    let cfg = SaturationConfig::default();
    let a = Expr::var("a");
    let b = Expr::var("b");
    let expr = Expr::div(Expr::neg(a.clone()), Expr::neg(b.clone()));
    assert_eq!(optimize(&expr, &cfg), Expr::div(a, b));
}

#[test]
fn test_rational_same_denominator_addition() {
    let cfg = SaturationConfig::default();
    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");
    let expr = Expr::add(
        Expr::div(a.clone(), c.clone()),
        Expr::div(b.clone(), c.clone()),
    );
    // (a+b)/c is cheaper than two separate divisions.
    assert_eq!(optimize(&expr, &cfg), Expr::div(Expr::add(a, b), c));
}
