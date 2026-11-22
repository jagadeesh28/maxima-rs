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
