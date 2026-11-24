//! Golden integration tests for definite integration & pole scanning (Phase 28).

use maxima_rs::prelude::*;
use sym_core::integrate::defint::{DefiniteResult, defint};

fn inf() -> Expr {
    Expr::var("inf")
}

#[test]
fn test_defint_x_squared_zero_to_one() {
    let x = Expr::var("x");
    let f = Expr::pow(x, Expr::int(2));
    assert_eq!(
        defint(&f, "x", &Expr::int(0), &Expr::int(1)),
        DefiniteResult::Value(Expr::frac(1, 3))
    );
}

#[test]
fn test_defint_x_zero_to_one() {
    let x = Expr::var("x");
    assert_eq!(
        defint(&x, "x", &Expr::int(0), &Expr::int(1)),
        DefiniteResult::Value(Expr::frac(1, 2))
    );
}

#[test]
fn test_defint_exp_neg_x_improper() {
    let x = Expr::var("x");
    let f = Expr::exp(Expr::neg(x));
    assert_eq!(
        defint(&f, "x", &Expr::int(0), &inf()),
        DefiniteResult::Value(Expr::int(1))
    );
}

#[test]
fn test_defint_reciprocal_over_symmetric_pole_is_divergent() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::int(1), x);
    assert_eq!(
        defint(&f, "x", &Expr::int(-1), &Expr::int(1)),
        DefiniteResult::Divergent
    );
}

#[test]
fn test_defint_reciprocal_over_negative_interval() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::int(1), x);
    assert_eq!(
        defint(&f, "x", &Expr::int(-3), &Expr::int(-1)),
        DefiniteResult::Value(Expr::neg(Expr::ln(Expr::int(3))))
    );
}
