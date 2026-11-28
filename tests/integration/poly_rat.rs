//! Golden integration tests for exact rational algebra (Phase 23).

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::poly::{expand, factor, partfrac, ratsimp};

fn eval_at(e: &Expr, x: f64) -> f64 {
    let mut env = HashMap::new();
    env.insert("x".to_string(), x);
    e.eval(&env).expect("evaluation failed")
}

#[test]
fn test_ratsimp_cancels_common_factor() {
    let x = Expr::var("x");
    let e = Expr::div(
        Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        Expr::sub(x, Expr::int(1)),
    );
    let out = ratsimp(&e).expect("ratsimp");
    for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
        assert!((eval_at(&out, xv) - (xv + 1.0)).abs() < 1e-9);
    }
}

#[test]
fn test_ratsimp_combines_sum_of_reciprocals() {
    let x = Expr::var("x");
    let e = Expr::add(
        Expr::div(Expr::int(1), x.clone()),
        Expr::div(Expr::int(1), x),
    );
    let out = ratsimp(&e).expect("ratsimp");
    for xv in [1.0_f64, -2.0, 3.0] {
        assert!((eval_at(&out, xv) - 2.0 / xv).abs() < 1e-9);
    }
}

#[test]
fn test_factor_roundtrip_difference_of_squares() {
    let x = Expr::var("x");
    let e = Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1));
    let factored = factor(&e);
    let back = expand(&factored);
    for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
        assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
    }
}

#[test]
fn test_partfrac_one_over_x_squared_minus_one() {
    let x = Expr::var("x");
    let e = Expr::div(
        Expr::int(1),
        Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1)),
    );
    let out = partfrac(&e, "x").expect("partfrac");
    for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
        assert!((eval_at(&out, xv) - eval_at(&e, xv)).abs() < 1e-9);
    }
}

#[test]
fn test_phase48_division_by_zero_multivar_gcd_kernels_factor_and_partfrac() {
    use sym_core::poly::{Poly, PolyError, poly_gcd};

    let x = Expr::var("x");
    let y = Expr::var("y");

    // 1. ratsimp(1 / (x - x)) -> Err(PolyError::DivisionByZero)
    let div_zero = Expr::div(Expr::int(1), Expr::sub(x.clone(), x.clone()));
    assert!(matches!(ratsimp(&div_zero), Err(PolyError::DivisionByZero)));

    // 2. Multivariate poly_gcd(x^2 - y^2, x - y) == x - y (up to sign)
    let p1 = Poly::from_expr(&Expr::sub(
        Expr::pow(x.clone(), Expr::int(2)),
        Expr::pow(y.clone(), Expr::int(2)),
    ))
    .expect("p1");
    let p2 = Poly::from_expr(&Expr::sub(x.clone(), y.clone())).expect("p2");
    let g = poly_gcd(&p1, &p2);
    assert_eq!(g.degree(), 1);
    let mut env = std::collections::HashMap::new();
    for (xv, yv) in [(3.0, 1.0), (-2.0, 5.0)] {
        env.insert("x".to_string(), xv);
        env.insert("y".to_string(), yv);
        let gv = g.to_expr().eval(&env).expect("gcd eval");
        assert!((gv.abs() - (xv - yv).abs()).abs() < 1e-9);
    }

    // 3. ratsimp((sin(x)^2 - 1)/(sin(x) - 1)) -> sin(x) + 1
    let sin_x = Expr::sin(x.clone());
    let trig_rat = Expr::div(
        Expr::sub(Expr::pow(sin_x.clone(), Expr::int(2)), Expr::int(1)),
        Expr::sub(sin_x, Expr::int(1)),
    );
    let trig_simp = ratsimp(&trig_rat).expect("trig_rat");
    for xv in [0.2_f64, 0.7, -1.1] {
        assert!((eval_at(&trig_simp, xv) - (xv.sin() + 1.0)).abs() < 1e-9);
    }

    // 4. factor(x^4 + 4) -> (x^2 - 2x + 2)(x^2 + 2x + 2)
    let sophie_germain = Expr::add(Expr::pow(x.clone(), Expr::int(4)), Expr::int(4));
    let factored = factor(&sophie_germain);
    assert!(matches!(factored, Expr::Mul(_, _)));
    let back = expand(&factored);
    for xv in [-2.0_f64, 0.0, 1.5] {
        assert!((eval_at(&back, xv) - eval_at(&sophie_germain, xv)).abs() < 1e-9);
    }

    // 5. partfrac(1/((x-1)^2 * (x^2+1)), "x")
    let pf_in = Expr::div(
        Expr::int(1),
        Expr::mul(
            Expr::pow(Expr::sub(x.clone(), Expr::int(1)), Expr::int(2)),
            Expr::add(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        ),
    );
    let pf_out = partfrac(&pf_in, "x").expect("partfrac repeated + quad");
    for xv in [-2.0_f64, -0.5, 0.0, 2.0, 3.5] {
        assert!((eval_at(&pf_out, xv) - eval_at(&pf_in, xv)).abs() < 1e-9);
    }

    // 6. improper partfrac(x^3 / (x - 1), "x")
    let imp_in = Expr::div(
        Expr::pow(x.clone(), Expr::int(3)),
        Expr::sub(x, Expr::int(1)),
    );
    let imp_out = partfrac(&imp_in, "x").expect("improper partfrac");
    for xv in [-2.0_f64, 0.0, 2.0, 4.0] {
        assert!((eval_at(&imp_out, xv) - eval_at(&imp_in, xv)).abs() < 1e-9);
    }
}
