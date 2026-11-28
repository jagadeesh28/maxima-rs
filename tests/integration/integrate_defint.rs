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

#[test]
fn test_phase49_defint_poles_and_reduction() {
    let x = Expr::var("x");

    // 1. Removable singularity (x^2 - 1)/(x - 1) on [0, 2] -> 4
    let rem = Expr::div(
        Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        Expr::sub(x.clone(), Expr::int(1)),
    );
    assert_eq!(
        defint(&rem, "x", &Expr::int(0), &Expr::int(2)),
        DefiniteResult::Value(Expr::int(4))
    );

    // 2. Irrational interior pole 1/(x^2 - 2) on [0, 2] -> Divergent
    let irrat = Expr::div(
        Expr::int(1),
        Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(2)),
    );
    assert_eq!(
        defint(&irrat, "x", &Expr::int(0), &Expr::int(2)),
        DefiniteResult::Divergent
    );

    // 3. Transcendental pole tan(x) on [0, 2] -> Divergent
    assert_eq!(
        defint(&Expr::tan(x.clone()), "x", &Expr::int(0), &Expr::int(2)),
        DefiniteResult::Divergent
    );

    // 4. Endpoint pole 1/x on [0, 1] -> Divergent
    let inv_x = Expr::div(Expr::int(1), x);
    assert_eq!(
        defint(&inv_x, "x", &Expr::int(0), &Expr::int(1)),
        DefiniteResult::Divergent
    );
}
