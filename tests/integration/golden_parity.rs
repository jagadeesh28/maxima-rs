//! Maxima 5.50 Golden Parity Verification Suite (Phase 45).
//!
//! Citations:
//! - Maxima 5.50 golden test suites:
//!   - `tests/rtest_integrate.mac`
//!   - `tests/rtest_limit.mac`
//!   - `tests/rtest_taylor.mac`
//!   - `tests/rtest_solve.mac`
//!   - `tests/rtest_laplace.mac`
//!   - `tests/rtest_bessel.mac`

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::series::series_expand;
use sym_core::specfn::SpecialFnKind;
use sym_core::specfn::orthopoly::{chebyshev_t_poly, hermite_h_poly, legendre_p_poly};

fn eval_at(expr: &Expr, var: &str, val: f64) -> f64 {
    let mut env = HashMap::new();
    env.insert(var.to_string(), val);
    expr.eval(&env).expect("eval")
}

// =========================================================================
// 1. Upstream Maxima: tests/rtest_integrate.mac
// =========================================================================
#[test]
fn golden_maxima_rtest_integrate() {
    let x = Expr::var("x");

    // Case 1: Polynomial integration ∫(3x^2 + 4x + 5) dx = x^3 + 2x^2 + 5x
    let poly_integrand = Expr::add(
        Expr::add(
            Expr::mul(Expr::int(3), Expr::pow(x.clone(), Expr::int(2))),
            Expr::mul(Expr::int(4), x.clone()),
        ),
        Expr::int(5),
    );
    let poly_antideriv = integrate(&poly_integrand, "x");
    let d_poly = differentiate(&poly_antideriv, "x");
    for &xv in &[0.5_f64, 1.0, 2.5] {
        assert!((eval_at(&poly_integrand, "x", xv) - eval_at(&d_poly, "x", xv)).abs() < 1e-6);
    }

    // Case 2: Transcendental elementary forms: ∫ sin(x) dx, ∫ cos(x) dx, ∫ exp(x) dx
    let sin_x = Expr::sin(x.clone());
    let int_sin = integrate(&sin_x, "x");
    let d_sin = differentiate(&int_sin, "x");
    assert!((eval_at(&sin_x, "x", 0.5) - eval_at(&d_sin, "x", 0.5)).abs() < 1e-6);

    let cos_x = Expr::cos(x.clone());
    let int_cos = integrate(&cos_x, "x");
    let d_cos = differentiate(&int_cos, "x");
    assert!((eval_at(&cos_x, "x", 0.5) - eval_at(&d_cos, "x", 0.5)).abs() < 1e-6);

    let exp_x = Expr::exp(x.clone());
    let int_exp = integrate(&exp_x, "x");
    let d_exp = differentiate(&int_exp, "x");
    assert!((eval_at(&exp_x, "x", 0.5) - eval_at(&d_exp, "x", 0.5)).abs() < 1e-6);

    // Case 3: Rational pole ∫ 1/(x - 2) dx
    let inv_lin = Expr::div(Expr::int(1), Expr::sub(x.clone(), Expr::int(2)));
    let int_inv = integrate(&inv_lin, "x");
    let d_inv = differentiate(&int_inv, "x");
    for &xv in &[3.0_f64, 4.5, 7.2] {
        assert!((eval_at(&inv_lin, "x", xv) - eval_at(&d_inv, "x", xv)).abs() < 1e-6);
    }

    // Case 4: Power of sine ∫ sin^2(x) dx
    let sin2 = Expr::pow(Expr::sin(x.clone()), Expr::int(2));
    let int_sin2 = integrate(&sin2, "x");
    let d_sin2 = differentiate(&int_sin2, "x");
    for &xv in &[0.3_f64, 0.7, 1.2] {
        assert!((eval_at(&sin2, "x", xv) - eval_at(&d_sin2, "x", xv)).abs() < 1e-5);
    }

    // Case 5: Derivative-divides u-substitution ∫ 2x * exp(x^2) dx = exp(x^2)
    let sub_integrand = Expr::mul(
        Expr::mul(Expr::int(2), x.clone()),
        Expr::exp(Expr::pow(x.clone(), Expr::int(2))),
    );
    let int_sub = integrate(&sub_integrand, "x");
    let d_sub = differentiate(&int_sub, "x");
    for &xv in &[0.4_f64, 0.8, 1.2] {
        assert!((eval_at(&sub_integrand, "x", xv) - eval_at(&d_sub, "x", xv)).abs() < 1e-5);
    }
}

// =========================================================================
// 2. Upstream Maxima: tests/rtest_limit.mac
// =========================================================================
#[test]
fn golden_maxima_rtest_limit() {
    let x = Expr::var("x");

    // Case 1: lim_{x -> 0} sin(x)/x = 1
    let sinc = Expr::div(Expr::sin(x.clone()), x.clone());
    assert_eq!(limit(&sinc, "x", &Expr::int(0)), Some(Expr::int(1)));

    // Case 2: lim_{x -> inf} (1 + 1/x)^x = e
    let euler = Expr::pow(
        Expr::add(Expr::int(1), Expr::div(Expr::int(1), x.clone())),
        x.clone(),
    );
    assert_eq!(limit(&euler, "x", &Expr::var("inf")), Some(Expr::e()));

    // Case 3: Polynomial cancellation lim_{x -> 2} (x^2 - 4) / (x - 2) = 4
    let frac = Expr::div(
        Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(4)),
        Expr::sub(x.clone(), Expr::int(2)),
    );
    assert_eq!(limit(&frac, "x", &Expr::int(2)), Some(Expr::int(4)));

    // Case 4: Asymptotic power ratio lim_{x -> inf} (3x^2 + 1) / (2x^2 + 5) = 3/2
    let rational_inf = Expr::div(
        Expr::add(
            Expr::mul(Expr::int(3), Expr::pow(x.clone(), Expr::int(2))),
            Expr::int(1),
        ),
        Expr::add(
            Expr::mul(Expr::int(2), Expr::pow(x.clone(), Expr::int(2))),
            Expr::int(5),
        ),
    );
    assert_eq!(
        limit(&rational_inf, "x", &Expr::var("inf")),
        Some(Expr::frac(3, 2))
    );
}

// =========================================================================
// 3. Upstream Maxima: tests/rtest_taylor.mac
// =========================================================================
#[test]
fn golden_maxima_rtest_taylor() {
    // Taylor expansion of exp(x) around 0 to order 4:
    // exp(x) = 1 + x + (1/2)*x^2 + (1/6)*x^3 + (1/24)*x^4
    let exp_x = Expr::exp(Expr::var("x"));
    let s_exp = series_expand(&exp_x, "x", &Expr::int(0), 5).expect("expand exp(x)");

    assert_eq!(s_exp.coeff_at(0), Expr::int(1));
    assert_eq!(s_exp.coeff_at(1), Expr::int(1));
    assert_eq!(s_exp.coeff_at(2), Expr::frac(1, 2));
    assert_eq!(s_exp.coeff_at(3), Expr::frac(1, 6));
    assert_eq!(s_exp.coeff_at(4), Expr::frac(1, 24));

    // Taylor expansion of sin(x) around 0: sin(x) = x - (1/6)*x^3 + O(x^5)
    let sin_x = Expr::sin(Expr::var("x"));
    let s_sin = series_expand(&sin_x, "x", &Expr::int(0), 5).expect("expand sin(x)");

    assert_eq!(s_sin.coeff_at(0), Expr::int(0));
    assert_eq!(s_sin.coeff_at(1), Expr::int(1));
    assert_eq!(s_sin.coeff_at(2), Expr::int(0));
    assert_eq!(s_sin.coeff_at(3), Expr::frac(-1, 6));

    // Taylor expansion of 1 / (1 - x): 1 + x + x^2 + x^3 + x^4
    let geom = Expr::div(Expr::int(1), Expr::sub(Expr::int(1), Expr::var("x")));
    let s_geom = series_expand(&geom, "x", &Expr::int(0), 4).expect("expand 1/(1-x)");

    assert_eq!(s_geom.coeff_at(0), Expr::int(1));
    assert_eq!(s_geom.coeff_at(1), Expr::int(1));
    assert_eq!(s_geom.coeff_at(2), Expr::int(1));
    assert_eq!(s_geom.coeff_at(3), Expr::int(1));
}

// =========================================================================
// 4. Upstream Maxima: tests/rtest_solve.mac
// =========================================================================
#[test]
fn golden_maxima_rtest_solve() {
    let x = Expr::var("x");

    // Case 1: Quadratic equation x^2 - 5x + 6 = 0 => {2, 3}
    let eq_quad = Expr::add(
        Expr::sub(
            Expr::pow(x.clone(), Expr::int(2)),
            Expr::mul(Expr::int(5), x.clone()),
        ),
        Expr::int(6),
    );
    let sols_quad = solve(&eq_quad, "x");
    let mut quad_roots: Vec<i64> = sols_quad
        .0
        .iter()
        .filter_map(|r| r.eval(&HashMap::new()).map(|v| v.round() as i64))
        .collect();
    quad_roots.sort_unstable();
    assert_eq!(quad_roots, vec![2, 3]);

    // Case 2: Linear equation 4x - 12 = 0 => {3}
    let eq_lin = Expr::sub(Expr::mul(Expr::int(4), x.clone()), Expr::int(12));
    let sols_lin = solve(&eq_lin, "x");
    assert_eq!(sols_lin.len(), 1);
    assert_eq!(sols_lin.0[0].eval(&HashMap::new()), Some(3.0));

    // Case 3: Pure quadratic x^2 - 16 = 0 => {-4, 4}
    let eq_diff_sq = Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(16));
    let sols_diff = solve(&eq_diff_sq, "x");
    let mut diff_roots: Vec<i64> = sols_diff
        .0
        .iter()
        .filter_map(|r| r.eval(&HashMap::new()).map(|v| v.round() as i64))
        .collect();
    diff_roots.sort_unstable();
    assert_eq!(diff_roots, vec![-4, 4]);
}

// =========================================================================
// 5. Upstream Maxima: tests/rtest_laplace.mac
// =========================================================================
#[test]
fn golden_maxima_rtest_laplace() {
    let t = "t";
    let s = "s";

    // Case 1: Forward transform of constants and monomials
    // L{7} = 7/s
    assert_eq!(
        laplace(&Expr::int(7), t, s),
        Expr::div(Expr::int(7), Expr::var(s))
    );
    // L{t} = 1/s^2
    assert_eq!(
        laplace(&Expr::var(t), t, s),
        Expr::div(Expr::int(1), Expr::pow(Expr::var(s), Expr::int(2)))
    );
    // L{t^3} = 6/s^4
    assert_eq!(
        laplace(&Expr::pow(Expr::var(t), Expr::int(3)), t, s),
        Expr::div(Expr::int(6), Expr::pow(Expr::var(s), Expr::int(4)))
    );

    // Case 2: Forward transform of transcendentals
    // L{exp(5t)} = 1 / (s - 5)
    let exp_5t = Expr::exp(Expr::mul(Expr::int(5), Expr::var(t)));
    assert_eq!(
        laplace(&exp_5t, t, s),
        Expr::div(Expr::int(1), Expr::sub(Expr::var(s), Expr::int(5)))
    );
    // L{sin(4t)} = 4 / (s^2 + 16)
    let sin_4t = Expr::sin(Expr::mul(Expr::int(4), Expr::var(t)));
    assert_eq!(
        laplace(&sin_4t, t, s),
        Expr::div(
            Expr::int(4),
            Expr::add(
                Expr::pow(Expr::var(s), Expr::int(2)),
                Expr::pow(Expr::int(4), Expr::int(2)),
            )
        )
    );

    // Case 3: Inverse transform
    // ilt{1/s} = 1
    assert_eq!(
        ilt(&Expr::div(Expr::int(1), Expr::var(s)), s, t),
        Expr::int(1)
    );
    // ilt{1/s^2} = t
    assert_eq!(
        ilt(
            &Expr::div(Expr::int(1), Expr::pow(Expr::var(s), Expr::int(2))),
            s,
            t
        ),
        Expr::var(t)
    );
    // ilt{1/(s-3)} = exp(3t)
    assert_eq!(
        ilt(
            &Expr::div(Expr::int(1), Expr::sub(Expr::var(s), Expr::int(3))),
            s,
            t
        ),
        Expr::exp(Expr::mul(Expr::int(3), Expr::var(t)))
    );
}

// =========================================================================
// 6. Upstream Maxima: tests/rtest_bessel.mac & Special Functions
// =========================================================================
#[test]
fn golden_maxima_rtest_bessel_and_specfn() {
    // Case 1: Bessel J values at zero
    // J_0(0) = 1, J_1(0) = 0, J_2(0) = 0
    assert_eq!(
        SpecialFnKind::BesselJ.eval_exact(&[Expr::int(0), Expr::int(0)]),
        Some(Expr::int(1))
    );
    assert_eq!(
        SpecialFnKind::BesselJ.eval_exact(&[Expr::int(1), Expr::int(0)]),
        Some(Expr::int(0))
    );
    assert_eq!(
        SpecialFnKind::BesselJ.eval_exact(&[Expr::int(2), Expr::int(0)]),
        Some(Expr::int(0))
    );

    // Case 2: Modified Bessel I values at zero
    // I_0(0) = 1, I_1(0) = 0
    assert_eq!(
        SpecialFnKind::BesselI.eval_exact(&[Expr::int(0), Expr::int(0)]),
        Some(Expr::int(1))
    );
    assert_eq!(
        SpecialFnKind::BesselI.eval_exact(&[Expr::int(1), Expr::int(0)]),
        Some(Expr::int(0))
    );

    // Case 3: Gamma function exact integer values
    // Gamma(1) = 1, Gamma(2) = 1, Gamma(3) = 2, Gamma(4) = 6, Gamma(5) = 24
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(1)]),
        Some(Expr::int(1))
    );
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(2)]),
        Some(Expr::int(1))
    );
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(3)]),
        Some(Expr::int(2))
    );
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(4)]),
        Some(Expr::int(6))
    );
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(5)]),
        Some(Expr::int(24))
    );

    // Case 4: Error function exact values
    // erf(0) = 0, erfc(0) = 1
    assert_eq!(
        SpecialFnKind::Erf.eval_exact(&[Expr::int(0)]),
        Some(Expr::int(0))
    );
    assert_eq!(
        SpecialFnKind::Erfc.eval_exact(&[Expr::int(0)]),
        Some(Expr::int(1))
    );

    // Case 5: Orthogonal polynomials
    // Legendre P_0(x) = 1, P_1(x) = x
    let x = Expr::var("x");
    assert_eq!(legendre_p_poly(0, &x), Expr::int(1));
    assert_eq!(legendre_p_poly(1, &x), x);

    // Hermite H_0(x) = 1, H_1(x) = 2x
    assert_eq!(hermite_h_poly(0, &x), Expr::int(1));
    assert_eq!(hermite_h_poly(1, &x), Expr::mul(Expr::int(2), x.clone()));

    // Chebyshev T_0(x) = 1, T_1(x) = x
    assert_eq!(chebyshev_t_poly(0, &x), Expr::int(1));
    assert_eq!(chebyshev_t_poly(1, &x), x);
}
