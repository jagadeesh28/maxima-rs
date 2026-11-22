//! Golden integration tests for asymptotic limits (Phase 27).

use maxima_rs::prelude::*;
use sym_core::limit::limit;

fn at_zero() -> Expr {
    Expr::int(0)
}

fn inf() -> Expr {
    Expr::var("inf")
}

#[test]
fn test_limit_sin_over_x() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::sin(x.clone()), x);
    assert_eq!(limit(&f, "x", &at_zero()), Some(Expr::int(1)));
}

#[test]
fn test_limit_one_plus_inv_x_pow_x_is_e() {
    let x = Expr::var("x");
    let base = Expr::add(Expr::int(1), Expr::div(Expr::int(1), x.clone()));
    let f = Expr::pow(base, x);
    assert_eq!(limit(&f, "x", &inf()), Some(Expr::e()));
}

#[test]
fn test_limit_exp_tower_difference() {
    let x = Expr::var("x");
    let e_x = Expr::exp(x.clone());
    let inside = Expr::add(x.clone(), Expr::exp(Expr::neg(x.clone())));
    let f = Expr::sub(Expr::exp(inside), e_x);
    assert_eq!(limit(&f, "x", &inf()), Some(Expr::int(1)));
}

#[test]
fn test_limit_reciprocal_at_infinity() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::int(1), x);
    assert_eq!(limit(&f, "x", &inf()), Some(Expr::int(0)));
}

#[test]
fn test_limit_removable_rational_singularity() {
    let x = Expr::var("x");
    let f = Expr::div(
        Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        Expr::sub(x, Expr::int(1)),
    );
    assert_eq!(limit(&f, "x", &Expr::int(1)), Some(Expr::int(2)));
}

#[test]
fn test_limit_polynomial_at_finite_point() {
    let x = Expr::var("x");
    let f = Expr::pow(x, Expr::int(2));
    assert_eq!(limit(&f, "x", &Expr::int(3)), Some(Expr::int(9)));
}
