//! Proptest invariant: every returned root satisfies the equation and root-count is complete (Phase 32 / 53).

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;
use sym_core::solve::solve;

fn quadratic(r1: i64, r2: i64) -> Expr {
    let x = Expr::var("x");
    Expr::sub(
        Expr::add(Expr::pow(x.clone(), Expr::int(2)), Expr::int(r1 * r2)),
        Expr::mul(Expr::int(r1 + r2), x),
    )
}

fn cubic(r1: i64, r2: i64, r3: i64) -> Expr {
    let x = Expr::var("x");
    Expr::mul(
        Expr::mul(
            Expr::sub(x.clone(), Expr::int(r1)),
            Expr::sub(x.clone(), Expr::int(r2)),
        ),
        Expr::sub(x, Expr::int(r3)),
    )
}

fn eval_root(r: &Expr) -> Option<f64> {
    let empty: HashMap<String, f64> = HashMap::new();
    r.eval(&empty)
}

fn root_valid(e: &Expr, r: &Expr) -> bool {
    let Some(rv) = eval_root(r) else {
        return false;
    };
    let mut env = HashMap::new();
    env.insert("x".to_string(), rv);
    matches!(e.eval(&env), Some(v) if v.abs() < 1e-6)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn prop_quadratic_roots_complete_and_valid(r1 in -12i64..=12, r2 in -12i64..=12) {
        let e = quadratic(r1, r2);
        let sols = solve(&e, "x");
        if r1 == r2 {
            prop_assert!(!sols.is_empty());
        } else {
            prop_assert_eq!(sols.0.len(), 2, "expected 2 distinct roots for ({}, {}), got {:?}", r1, r2, sols.0);
        }
        let vals: Vec<f64> = sols.0.iter().filter_map(eval_root).collect();
        prop_assert_eq!(vals.len(), sols.0.len(), "non-numeric root in {:?}", sols.0);
        prop_assert!(vals.iter().any(|&v| (v - r1 as f64).abs() < 1e-6), "missing root {r1} in {vals:?}");
        prop_assert!(vals.iter().any(|&v| (v - r2 as f64).abs() < 1e-6), "missing root {r2} in {vals:?}");
        for r in &sols.0 {
            prop_assert!(root_valid(&e, r));
        }
    }

    #[test]
    fn prop_cubic_distinct_roots_complete(
        r1 in -6i64..=6,
        r2 in -6i64..=6,
        r3 in -6i64..=6
    ) {
        prop_assume!(r1 != r2 && r2 != r3 && r1 != r3);
        let e = cubic(r1, r2, r3);
        let sols = solve(&e, "x");
        prop_assert_eq!(
            sols.0.len(),
            3,
            "expected 3 roots for ({}, {}, {}), got {:?}",
            r1,
            r2,
            r3,
            sols.0
        );
        for r in &sols.0 {
            prop_assert!(root_valid(&e, r));
        }
    }
}
