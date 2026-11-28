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

#[test]
fn test_phase49_limits_directional_poles_oscillation_gruntz_lhopital() {
    use sym_core::limit::zero::is_zero_equivalent;
    use sym_core::limit::{LimitDir, limit_dir};

    let x = Expr::var("x");
    let t = Expr::var("t");

    // 1. zero equivalence must NOT return true for `x`
    assert!(!is_zero_equivalent(&x, "x"));
    assert!(is_zero_equivalent(
        &Expr::sub(Expr::sin(x.clone()), Expr::sin(x.clone())),
        "x"
    ));

    // 2. Directional poles of 1/x at 0: + -> inf, - -> minf, Both -> und
    let inv_x = Expr::div(Expr::int(1), x.clone());
    assert_eq!(
        limit_dir(&inv_x, "x", &at_zero(), LimitDir::Plus),
        Some(Expr::var("inf"))
    );
    assert_eq!(
        limit_dir(&inv_x, "x", &at_zero(), LimitDir::Minus),
        Some(Expr::var("minf"))
    );
    assert_eq!(limit(&inv_x, "x", &at_zero()), Some(Expr::var("und")));

    // 3. Even pole 1/x^2 at 0 -> inf (two-sided)
    let inv_x2 = Expr::div(Expr::int(1), Expr::pow(x.clone(), Expr::int(2)));
    assert_eq!(limit(&inv_x2, "x", &at_zero()), Some(Expr::var("inf")));

    // 4. Gensym avoids collision when expression contains variable "t"
    let t_div_x = Expr::div(t.clone(), x.clone());
    assert_eq!(limit(&t_div_x, "x", &inf()), Some(Expr::int(0)));

    // 5. Oscillatory sin(x) as x -> inf -> ind
    assert_eq!(
        limit(&Expr::sin(x.clone()), "x", &inf()),
        Some(Expr::var("ind"))
    );

    // 6. L'Hopital: ln(x) / x as x -> inf -> 0
    let ln_over_x = Expr::div(Expr::ln(x.clone()), x.clone());
    assert_eq!(limit(&ln_over_x, "x", &inf()), Some(Expr::int(0)));
}
