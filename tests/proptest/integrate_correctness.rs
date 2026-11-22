//! Proptest invariant: integrating then differentiating recovers the integrand (Phase 32).

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::integrate::integrate;

fn poly_expr(coeffs: &[i64]) -> Expr {
    let x = Expr::var("x");
    let mut e = Expr::int(0);
    for (k, &c) in coeffs.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let term = Expr::mul(Expr::int(c), Expr::pow(x.clone(), Expr::int(k as i64)));
        e = Expr::add(e, term);
    }
    e
}

fn assert_antiderivative(f: &Expr) {
    let big_f = integrate(f, "x");
    assert!(
        !matches!(big_f, Expr::Integral(_, _)),
        "unexpected noun integrating {f}"
    );
    let d = differentiate(&big_f, "x");
    for xv in [0.37_f64, 1.3, -0.8] {
        let mut env = HashMap::new();
        env.insert("x".to_string(), xv);
        if let (Some(a), Some(b)) = (f.eval(&env), d.eval(&env)) {
            assert!((a - b).abs() < 1e-6, "at {xv}: {a} vs {b} for {f}");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn prop_integrate_then_differentiate(coeffs in prop::collection::vec(-4i64..=4, 0..6)) {
        let f = poly_expr(&coeffs);
        if f.variables().is_empty() {
            return Ok(());
        }
        assert_antiderivative(&f);
    }
}

#[test]
fn deterministic_integrate_1000_polynomials() {
    let mut state = 123_456_789u64;
    for _ in 0..1000 {
        let mut coeffs = Vec::new();
        for _ in 0..5 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            coeffs.push(((state >> 33) % 11) as i64 - 5);
        }
        let f = poly_expr(&coeffs);
        if f.variables().is_empty() {
            continue;
        }
        assert_antiderivative(&f);
    }
}
