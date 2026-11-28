//! Golden integration tests for Laurent/Taylor series (Phase 26).

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::series::{Series, series_expand};

fn coeff(ser: &Series, exp: i64) -> f64 {
    let empty: HashMap<String, f64> = HashMap::new();
    ser.coeff_at(exp).eval(&empty).expect("numeric coefficient")
}

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} vs {b}");
}

fn at_zero() -> Expr {
    Expr::int(0)
}

#[test]
fn test_sin_series() {
    let s = series_expand(&Expr::sin(Expr::var("x")), "x", &at_zero(), 6).unwrap();
    assert_eq!(s.valuation, 1);
    approx(coeff(&s, 0), 0.0);
    approx(coeff(&s, 1), 1.0);
    approx(coeff(&s, 2), 0.0);
    approx(coeff(&s, 3), -1.0 / 6.0);
    approx(coeff(&s, 5), 1.0 / 120.0);
}

#[test]
fn test_cos_series() {
    let c = series_expand(&Expr::cos(Expr::var("x")), "x", &at_zero(), 6).unwrap();
    approx(coeff(&c, 0), 1.0);
    approx(coeff(&c, 2), -1.0 / 2.0);
    approx(coeff(&c, 4), 1.0 / 24.0);
}

#[test]
fn test_exp_series() {
    let e = series_expand(&Expr::exp(Expr::var("x")), "x", &at_zero(), 5).unwrap();
    approx(coeff(&e, 0), 1.0);
    approx(coeff(&e, 1), 1.0);
    approx(coeff(&e, 2), 1.0 / 2.0);
    approx(coeff(&e, 3), 1.0 / 6.0);
    approx(coeff(&e, 4), 1.0 / 24.0);
}

#[test]
fn test_geometric_series() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::int(1), Expr::sub(Expr::int(1), x));
    let g = series_expand(&f, "x", &at_zero(), 5).unwrap();
    for e in 0..5 {
        approx(coeff(&g, e), 1.0);
    }
}

#[test]
fn test_cosecant_is_laurent_with_pole() {
    let x = Expr::var("x");
    let f = Expr::div(Expr::int(1), Expr::sin(x));
    let c = series_expand(&f, "x", &at_zero(), 6).unwrap();
    assert_eq!(c.valuation, -1);
    approx(coeff(&c, -1), 1.0);
    approx(coeff(&c, 1), 1.0 / 6.0);
    approx(coeff(&c, 3), 7.0 / 360.0);
}

#[test]
fn test_pythagorean_identity_from_series() {
    let s = series_expand(&Expr::sin(Expr::var("x")), "x", &at_zero(), 8).unwrap();
    let c = series_expand(&Expr::cos(Expr::var("x")), "x", &at_zero(), 8).unwrap();
    let sum = s.mul(&s).add(&c.mul(&c));
    approx(coeff(&sum, 0), 1.0);
    approx(coeff(&sum, 2), 0.0);
    approx(coeff(&sum, 4), 0.0);
    approx(coeff(&sum, 6), 0.0);
}

#[test]
fn test_inverse_multiplication() {
    let x = Expr::var("x");
    let s = series_expand(&Expr::sin(x.clone()), "x", &at_zero(), 6).unwrap();
    let csc = series_expand(&Expr::div(Expr::int(1), Expr::sin(x)), "x", &at_zero(), 6).unwrap();
    let prod = s.mul(&csc);
    approx(coeff(&prod, 0), 1.0);
    approx(coeff(&prod, 1), 0.0);
    approx(coeff(&prod, 2), 0.0);
}

#[test]
fn test_derivative_of_sin_is_cos() {
    let s = series_expand(&Expr::sin(Expr::var("x")), "x", &at_zero(), 6).unwrap();
    let d = s.diff();
    approx(coeff(&d, 0), 1.0);
    approx(coeff(&d, 2), -1.0 / 2.0);
    approx(coeff(&d, 4), 1.0 / 24.0);
}

#[test]
fn test_composition_sin_of_x_squared() {
    let f = Expr::sin(Expr::pow(Expr::var("x"), Expr::int(2)));
    let s = series_expand(&f, "x", &at_zero(), 8).unwrap();
    assert_eq!(s.valuation, 2);
    approx(coeff(&s, 2), 1.0);
    approx(coeff(&s, 6), -1.0 / 6.0);
}

#[test]
fn test_phase49_series_symbolic_center_and_new_functions() {
    use sym_core::series::taylor;
    let x = Expr::var("x");

    // 1. sin(x) at %pi = -(x - pi) + (x - pi)^3 / 6 - ...
    let s_pi = series_expand(&Expr::sin(x.clone()), "x", &Expr::pi(), 4).unwrap();
    approx(coeff(&s_pi, 0), 0.0);
    approx(coeff(&s_pi, 1), -1.0);
    approx(coeff(&s_pi, 2), 0.0);
    approx(coeff(&s_pi, 3), 1.0 / 6.0);

    // 2. tan(x) at 0 = x + x^3/3 + 2x^5/15
    let t0 = series_expand(&Expr::tan(x.clone()), "x", &at_zero(), 5).unwrap();
    approx(coeff(&t0, 1), 1.0);
    approx(coeff(&t0, 3), 1.0 / 3.0);
    approx(coeff(&t0, 5), 2.0 / 15.0);

    // 3. tan(x) at %pi/2 has simple pole -1/(x - pi/2)
    let pi_2 = Expr::div(Expr::pi(), Expr::int(2));
    let t_pi2 = series_expand(&Expr::tan(x.clone()), "x", &pi_2, 2).unwrap();
    assert_eq!(t_pi2.valuation, -1);
    approx(coeff(&t_pi2, -1), -1.0);

    // 4. sqrt(1 + x) at 0 = 1 + x/2 - x^2/8 + x^3/16
    let sq = series_expand(
        &Expr::sqrt(Expr::add(Expr::int(1), x.clone())),
        "x",
        &at_zero(),
        3,
    )
    .unwrap();
    approx(coeff(&sq, 0), 1.0);
    approx(coeff(&sq, 1), 0.5);
    approx(coeff(&sq, 2), -0.125);
    approx(coeff(&sq, 3), 1.0 / 16.0);

    // 5. Symbolic exponent (1 + x)^a at 0
    let a = Expr::var("a");
    let sym_p = series_expand(
        &Expr::pow(Expr::add(Expr::int(1), x.clone()), a),
        "x",
        &at_zero(),
        2,
    )
    .unwrap();
    let mut env = HashMap::new();
    env.insert("a".to_string(), 3.0);
    approx(sym_p.coeff_at(0).eval(&env).unwrap(), 1.0);
    approx(sym_p.coeff_at(1).eval(&env).unwrap(), 3.0);
    approx(sym_p.coeff_at(2).eval(&env).unwrap(), 3.0);

    // 6. tanh(x), atan(x), asin(x) at 0
    let th = series_expand(&Expr::tanh(x.clone()), "x", &at_zero(), 3).unwrap();
    approx(coeff(&th, 1), 1.0);
    approx(coeff(&th, 3), -1.0 / 3.0);

    let at = series_expand(&Expr::atan(x.clone()), "x", &at_zero(), 3).unwrap();
    approx(coeff(&at, 1), 1.0);
    approx(coeff(&at, 3), -1.0 / 3.0);

    let asn = series_expand(&Expr::asin(x.clone()), "x", &at_zero(), 3).unwrap();
    approx(coeff(&asn, 1), 1.0);
    approx(coeff(&asn, 3), 1.0 / 6.0);

    // 7. Puiseux / logarithmic taylor at 0
    assert!(taylor(&Expr::sqrt(x.clone()), "x", &at_zero(), 3).is_some());
    assert!(
        taylor(
            &Expr::ln(Expr::mul(x.clone(), Expr::add(Expr::int(1), x))),
            "x",
            &at_zero(),
            3
        )
        .is_some()
    );

    // 8. Truncation order math: diff reduces order by 1, inv shifts by -2*v
    let s = series_expand(&Expr::sin(Expr::var("x")), "x", &at_zero(), 4).unwrap();
    assert_eq!(s.order, 5);
    assert_eq!(s.diff().order, 4);
    assert_eq!(s.inv().unwrap().order, 3);
}
