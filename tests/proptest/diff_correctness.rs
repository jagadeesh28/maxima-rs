//! Property-based testing: symbolic differentiation vs numerical central finite differences.

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;

fn diff_leaf_strategy() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (-10i64..=10i64).prop_map(Expr::int),
        Just(Expr::var("x")),
        Just(Expr::var("x")),
    ]
}

fn diff_expr_strategy() -> impl Strategy<Value = Expr> {
    diff_leaf_strategy().prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::add(a, b)),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::sub(a, b)),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::mul(a, b)),
            (inner.clone(), 1i64..=4i64).prop_map(|(a, p)| Expr::pow(a, Expr::int(p))),
            inner.clone().prop_map(Expr::neg),
            inner.clone().prop_map(Expr::sin),
            inner.clone().prop_map(Expr::cos),
            inner.prop_map(Expr::exp),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn prop_diff_matches_finite_difference_recursive(
        f in diff_expr_strategy(),
        x_val in -2.0f64..2.0f64
    ) {
        let df = differentiate(&f, "x");
        let h = 1e-6;
        let mut env_plus = HashMap::new();
        env_plus.insert("x".to_string(), x_val + h);
        let mut env_minus = HashMap::new();
        env_minus.insert("x".to_string(), x_val - h);
        let mut env_at = HashMap::new();
        env_at.insert("x".to_string(), x_val);

        let f_plus = f.eval(&env_plus).expect("eval f_plus");
        let f_minus = f.eval(&env_minus).expect("eval f_minus");
        prop_assume!(f_plus.is_finite() && f_minus.is_finite() && f_plus.abs() < 1e8 && f_minus.abs() < 1e8);
        let num_df = (f_plus - f_minus) / (2.0 * h);

        let sym_df = df.eval(&env_at).expect("eval sym_df");
        prop_assume!(sym_df.is_finite() && sym_df.abs() < 1e8);

        let diff = (sym_df - num_df).abs();
        let rel_err = diff / (sym_df.abs().max(1.0));
        prop_assert!(rel_err < 1e-3, "f={f}, x_val={x_val}: sym={sym_df}, num={num_df}, rel_err={rel_err}");
    }
}
