//! Property-based testing: numerical equivalence between original and e-graph-optimized expressions.

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;
use sym_core::saturate::SaturationConfig;

fn opt_leaf_strategy() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (-10i64..=10i64).prop_map(Expr::int),
        prop_oneof![Just("x"), Just("y")].prop_map(Expr::var),
    ]
}

fn opt_expr_strategy() -> impl Strategy<Value = Expr> {
    opt_leaf_strategy().prop_recursive(3, 14, 4, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::add(a, b)),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::sub(a, b)),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::mul(a, b)),
            (inner.clone(), 1i64..=3i64).prop_map(|(a, p)| Expr::pow(a, Expr::int(p))),
            inner.clone().prop_map(Expr::neg),
            inner.clone().prop_map(Expr::sin),
            inner.prop_map(Expr::cos),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn prop_recursive_optimize_numeric_equiv(
        orig in opt_expr_strategy(),
        x_val in -3.0f64..3.0f64,
        y_val in -3.0f64..3.0f64
    ) {
        let cfg = SaturationConfig {
            node_limit: 4_000,
            iter_limit: 12,
            time_limit_secs: 0.25,
            ..SaturationConfig::default()
        };
        let opt = optimize(&orig, &cfg);

        let mut env = HashMap::new();
        env.insert("x".to_string(), x_val);
        env.insert("y".to_string(), y_val);

        let val_orig = orig.eval(&env).expect("eval orig");
        prop_assume!(val_orig.is_finite() && val_orig.abs() < 1e9);
        let val_opt = opt.eval(&env).expect("eval opt");
        prop_assert!(val_opt.is_finite(), "opt non-finite for {orig} -> {opt}");
        let tol = 1e-6 * (1.0 + val_orig.abs());
        prop_assert!(
            (val_orig - val_opt).abs() <= tol,
            "orig={orig} ({val_orig}) vs opt={opt} ({val_opt})"
        );
    }
}
