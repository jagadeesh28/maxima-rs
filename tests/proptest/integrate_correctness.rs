//! Proptest invariant: integrating then differentiating recovers the integrand (Phase 32 / 53).

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::integrate::integrate;

fn integrable_atom_strategy() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (-5i64..=5i64).prop_map(Expr::int),
        (1i64..=4i64).prop_map(|p| Expr::pow(Expr::var("x"), Expr::int(p))),
        (1i64..=3i64).prop_map(|k| Expr::sin(Expr::mul(Expr::int(k), Expr::var("x")))),
        (1i64..=3i64).prop_map(|k| Expr::cos(Expr::mul(Expr::int(k), Expr::var("x")))),
        (1i64..=2i64).prop_map(|k| Expr::exp(Expr::mul(Expr::int(k), Expr::var("x")))),
    ]
}

fn integrable_expr_strategy() -> impl Strategy<Value = Expr> {
    integrable_atom_strategy().prop_recursive(2, 10, 3, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::add(a, b)),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::sub(a, b)),
            (-4i64..=4i64, inner.clone()).prop_map(|(c, e)| Expr::mul(Expr::int(c), e)),
            inner.prop_map(Expr::neg),
        ]
    })
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
        let a = f.eval(&env).expect("integrand eval must succeed");
        let b = d
            .eval(&env)
            .expect("derivative of antiderivative eval must succeed");
        assert!((a - b).abs() < 1e-5, "at {xv}: {a} vs {b} for {f}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    fn prop_integrate_then_differentiate(f in integrable_expr_strategy()) {
        assert_antiderivative(&f);
    }
}
