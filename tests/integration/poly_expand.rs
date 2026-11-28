//! Golden integration tests for the sparse multivariate polynomial engine (Phase 22).

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::poly::{Poly, expand};

fn eval(e: &Expr, pairs: &[(&str, f64)]) -> f64 {
    let mut env = HashMap::new();
    for (k, v) in pairs {
        env.insert((*k).to_string(), *v);
    }
    e.eval(&env).expect("evaluation failed")
}

#[test]
fn test_expand_binomial_fourth_power() {
    let x = Expr::var("x");
    let y = Expr::var("y");
    let e = Expr::pow(Expr::add(x, y), Expr::int(4));
    let out = expand(&e);
    for (a, b) in [(1.0, 1.0), (2.0, -3.0), (0.5, 1.5)] {
        let before = eval(&e, &[("x", a), ("y", b)]);
        let after = eval(&out, &[("x", a), ("y", b)]);
        assert!((before - after).abs() < 1e-9, "{before} vs {after}");
    }
}

#[test]
fn test_expand_multivariate_product() {
    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");
    let d = Expr::var("d");
    let e = Expr::mul(Expr::add(a, b), Expr::add(c, d));
    let out = expand(&e);
    for (av, bv, cv, dv) in [(1.0, 2.0, 3.0, 4.0), (-1.0, 0.5, 2.0, -3.0)] {
        let before = eval(&e, &[("a", av), ("b", bv), ("c", cv), ("d", dv)]);
        let after = eval(&out, &[("a", av), ("b", bv), ("c", cv), ("d", dv)]);
        assert!((before - after).abs() < 1e-9);
    }
}

#[test]
fn test_expand_difference_of_squares() {
    let x = Expr::var("x");
    let y = Expr::var("y");
    let e = Expr::mul(Expr::add(x.clone(), y.clone()), Expr::sub(x, y));
    let out = expand(&e);
    for (a, b) in [(3.0, 1.0), (-2.0, 5.0)] {
        let before = eval(&e, &[("x", a), ("y", b)]);
        let after = eval(&out, &[("x", a), ("y", b)]);
        assert!((before - after).abs() < 1e-9);
    }
}

#[test]
fn test_expand_integer_power_of_binomial() {
    let x = Expr::var("x");
    let e = Expr::pow(Expr::add(x, Expr::int(1)), Expr::int(5));
    let out = expand(&e);
    for xv in [-2.0_f64, -0.5, 0.0, 1.5, 3.0] {
        let before = eval(&e, &[("x", xv)]);
        let after = eval(&out, &[("x", xv)]);
        assert!((before - after).abs() < 1e-9);
    }
}

#[test]
fn test_expand_constant_folding() {
    assert_eq!(expand(&Expr::add(Expr::int(2), Expr::int(3))), Expr::int(5));
    assert_eq!(expand(&Expr::mul(Expr::int(2), Expr::int(3))), Expr::int(6));
}

#[test]
fn test_poly_roundtrip_is_numerically_exact() {
    let x = Expr::var("x");
    let y = Expr::var("y");
    let e = Expr::add(
        Expr::pow(x, Expr::int(3)),
        Expr::mul(Expr::int(-2), Expr::mul(Expr::var("x"), y)),
    );
    let poly = Poly::from_expr(&e).expect("polynomial");
    let back = poly.to_expr();
    for (a, b) in [(1.0, 2.0), (-1.5, 0.5)] {
        assert!(
            (eval(&e, &[("x", a), ("y", b)]) - eval(&back, &[("x", a), ("y", b)])).abs() < 1e-9
        );
    }
}

#[test]
fn test_phase48_poly_scale_zero_and_structural_expand() {
    let x = Expr::var("x");
    let y = Expr::var("y");
    let p = Poly::from_expr(&Expr::add(x.clone(), y.clone())).expect("poly");
    let scaled_zero = p.scale(&sym_core::Rat::from_int(0));
    assert!(
        scaled_zero.is_zero(),
        "Poly::scale(0) must produce zero poly"
    );

    // expand(-(x + y)) distributes negation
    let neg_sum = expand(&Expr::neg(Expr::add(x.clone(), y.clone())));
    for (xv, yv) in [(1.0, 2.0), (-3.0, 4.5)] {
        assert!((eval(&neg_sum, &[("x", xv), ("y", yv)]) - (-(xv + yv))).abs() < 1e-12);
    }
    assert!(!matches!(neg_sum, Expr::Neg(_)));

    // expand(sin((x + 1)^2)) recurses inside function arguments
    let nested = expand(&Expr::sin(Expr::pow(
        Expr::add(x.clone(), Expr::int(1)),
        Expr::int(2),
    )));
    for xv in [-1.5_f64, 0.0, 2.0] {
        let expected = ((xv + 1.0).powi(2)).sin();
        assert!((eval(&nested, &[("x", xv)]) - expected).abs() < 1e-12);
    }
}
