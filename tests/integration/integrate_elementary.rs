//! Golden integration tests for symbolic integration (Phase 24).
//!
//! Every successful antiderivative is verified by differentiating it back and
//! comparing numerically against the integrand.

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::integrate::integrate;

fn assert_antiderivative(f: &Expr, var: &str, points: &[f64]) {
    let big_f = integrate(f, var);
    assert!(
        !matches!(big_f, Expr::Integral(_, _)),
        "unexpected noun form integrating {f}"
    );
    let d = differentiate(&big_f, var);
    for &xv in points {
        let mut env = HashMap::new();
        env.insert(var.to_string(), xv);
        if let (Some(fa), Some(db)) = (f.eval(&env), d.eval(&env)) {
            assert!(
                (fa - db).abs() < 1e-6,
                "at {var}={xv}: d/d{var}(F) = {db} vs f = {fa} (F = {big_f})"
            );
        }
    }
}

#[test]
fn test_integrate_polynomial() {
    let x = Expr::var("x");
    assert_antiderivative(&Expr::pow(x, Expr::int(2)), "x", &[-2.0, -0.5, 1.0, 3.0]);
}

#[test]
fn test_integrate_sine() {
    let x = Expr::var("x");
    assert_antiderivative(&Expr::sin(x), "x", &[-2.0, 0.0, 1.0, 2.5]);
}

#[test]
fn test_integrate_cosine_scaled() {
    let x = Expr::var("x");
    let f = Expr::cos(Expr::mul(Expr::int(2), x));
    assert_antiderivative(&f, "x", &[-1.5, 0.0, 0.5, 1.2]);
}

#[test]
fn test_integrate_exponential() {
    let x = Expr::var("x");
    assert_antiderivative(&Expr::exp(x.clone()), "x", &[-1.0, 0.0, 1.5]);
    assert_antiderivative(
        &Expr::exp(Expr::mul(Expr::int(2), x)),
        "x",
        &[-1.0, 0.0, 1.0],
    );
}

#[test]
fn test_integrate_x_times_exponential() {
    let x = Expr::var("x");
    let f = Expr::mul(x, Expr::exp(Expr::var("x")));
    assert_antiderivative(&f, "x", &[-1.0, 0.0, 2.0]);
}

#[test]
fn test_integrate_logarithm() {
    let x = Expr::var("x");
    assert_antiderivative(&Expr::ln(x), "x", &[0.5, 1.0, 3.0]);
}

#[test]
fn test_integrate_reciprocal() {
    let x = Expr::var("x");
    assert_antiderivative(&Expr::div(Expr::int(1), x), "x", &[0.5, 1.0, 3.0]);
}

#[test]
fn test_integrate_rational_partial_fractions() {
    let x = Expr::var("x");
    let f = Expr::div(
        Expr::int(1),
        Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1)),
    );
    assert_antiderivative(&f, "x", &[-3.0, -0.5, 0.0, 2.0, 4.0]);
}

#[test]
fn test_integrate_repeated_linear_pole() {
    let x = Expr::var("x");
    let f = Expr::div(
        Expr::int(1),
        Expr::pow(Expr::sub(x, Expr::int(1)), Expr::int(2)),
    );
    assert_antiderivative(&f, "x", &[-2.0, 0.0, 2.0, 3.0]);
}

#[test]
fn test_non_elementary_returns_noun() {
    let x = Expr::var("x");
    let f = Expr::exp(Expr::pow(x, Expr::int(2)));
    let out = integrate(&f, "x");
    assert!(matches!(out, Expr::Integral(_, _)));
}

#[test]
fn test_phase49_integration_hermite_lrt_sin_and_non_elementary_guard() {
    use sym_core::integrate::is_proven_non_elementary;

    let x = Expr::var("x");

    // 1. 1/(x^2 + 1) -> atan(x)
    let inv_x2_p1 = Expr::div(
        Expr::int(1),
        Expr::add(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
    );
    assert_antiderivative(&inv_x2_p1, "x", &[-2.0, -0.5, 0.0, 1.0, 3.0]);

    // 2. Repeated irreducible quadratic 1/(x^2 + 1)^2 via Hermite reduction
    let rep_quad = Expr::div(
        Expr::int(1),
        Expr::pow(
            Expr::add(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
            Expr::int(2),
        ),
    );
    assert_antiderivative(&rep_quad, "x", &[-1.5, 0.0, 0.7, 2.0]);

    // 3. Product x*(x + 1)
    let prod = Expr::mul(x.clone(), Expr::add(x.clone(), Expr::int(1)));
    assert_antiderivative(&prod, "x", &[-2.0, 0.0, 1.5, 3.0]);

    // 4. Elementary sin.rs additions: tan, tanh, sqrt, asin, acos, atan, 2^x
    assert_antiderivative(&Expr::tan(x.clone()), "x", &[-0.8, 0.0, 0.7]);
    assert_antiderivative(&Expr::tanh(x.clone()), "x", &[-1.2, 0.0, 1.2]);
    assert_antiderivative(
        &Expr::sqrt(Expr::add(Expr::mul(Expr::int(2), x.clone()), Expr::int(1))),
        "x",
        &[0.0, 0.5, 2.0],
    );
    assert_antiderivative(&Expr::asin(x.clone()), "x", &[-0.5, 0.0, 0.6]);
    assert_antiderivative(&Expr::acos(x.clone()), "x", &[-0.5, 0.0, 0.6]);
    assert_antiderivative(&Expr::atan(x.clone()), "x", &[-1.5, 0.0, 1.5]);
    assert_antiderivative(&Expr::pow(Expr::int(2), x.clone()), "x", &[-1.0, 0.0, 2.0]);

    // 5. exp(0 * x^2) must NOT be proven non-elementary
    let zero_gauss = Expr::exp(Expr::mul(Expr::int(0), Expr::pow(x, Expr::int(2))));
    assert!(!is_proven_non_elementary(&zero_gauss, "x"));
}
