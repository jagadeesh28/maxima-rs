//! Property-based testing for print-parse round-trip invariant.
//!
//! Validates that formatting an [`Expr`] to string or LaTeX and parsing it back
//! preserves numerical and structural equivalence.

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;

fn leaf_strategy() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (-20i64..20i64).prop_map(Expr::int),
        prop_oneof![Just("x"), Just("y"), Just("z")].prop_map(Expr::var),
        Just(Expr::pi()),
        Just(Expr::e()),
    ]
}

fn expr_strategy() -> impl Strategy<Value = Expr> {
    let leaf = leaf_strategy();
    leaf.prop_recursive(
        3,  // max depth
        16, // max total nodes
        4,  // items per collection
        |inner| {
            prop_oneof![
                (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::add(a, b)),
                (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::sub(a, b)),
                (inner.clone(), inner.clone()).prop_map(|(a, b)| Expr::mul(a, b)),
                (inner.clone(), 1i64..4i64).prop_map(|(a, p)| Expr::pow(a, Expr::int(p))),
                inner.clone().prop_map(Expr::neg),
                inner.clone().prop_map(Expr::sin),
                inner.clone().prop_map(Expr::cos),
                inner.clone().prop_map(Expr::exp),
                inner.clone().prop_map(Expr::sinh),
                inner.prop_map(Expr::cosh),
            ]
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn prop_maxima_print_parse_roundtrip(orig in expr_strategy()) {
        let text = orig.to_string();
        let parsed = parse_maxima(&text);
        prop_assert!(parsed.is_ok(), "Failed to parse Maxima string '{}' for expr {:?}", text, orig);

        let parsed = parsed.unwrap();
        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.5);
        env.insert("y".to_string(), 2.0);
        env.insert("z".to_string(), 0.5);

        if let (Some(v1), Some(v2)) = (orig.eval(&env), parsed.eval(&env))
            && v1.is_finite()
            && v2.is_finite()
        {
            prop_assert!(
                (v1 - v2).abs() < 1e-6,
                "Numerical divergence: orig={}, parsed={}, text={}",
                v1,
                v2,
                text
            );
        }
    }

    #[test]
    fn prop_latex_print_parse_roundtrip(orig in expr_strategy()) {
        let latex_text = to_latex(&orig);
        let parsed = parse_latex(&latex_text);
        prop_assert!(parsed.is_ok(), "Failed to parse LaTeX string '{}' for expr {:?}", latex_text, orig);

        let parsed = parsed.unwrap();
        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.5);
        env.insert("y".to_string(), 2.0);
        env.insert("z".to_string(), 0.5);

        if let (Some(v1), Some(v2)) = (orig.eval(&env), parsed.eval(&env))
            && v1.is_finite()
            && v2.is_finite()
        {
            prop_assert!(
                (v1 - v2).abs() < 1e-6,
                "Numerical divergence: orig={}, parsed={}, latex={}",
                v1,
                v2,
                latex_text
            );
        }
    }
}
