//! Proptest invariant: every returned root satisfies the equation (Phase 32).

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

fn root_valid(e: &Expr, r: &Expr) -> bool {
    let empty: HashMap<String, f64> = HashMap::new();
    let Some(rv) = r.eval(&empty) else {
        return false;
    };
    let mut env = HashMap::new();
    env.insert("x".to_string(), rv);
    matches!(e.eval(&env), Some(v) if v.abs() < 1e-6)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn prop_quadratic_roots_back_substitute(r1 in -6i64..=6, r2 in -6i64..=6) {
        let e = quadratic(r1, r2);
        let sols = solve(&e, "x");
        prop_assert!(!sols.is_empty());
        for r in &sols.0 {
            prop_assert!(root_valid(&e, r));
        }
    }
}

#[test]
fn deterministic_1000_quadratics() {
    let mut checked = 0;
    for r1 in -16..=16 {
        for r2 in -16..=16 {
            let e = quadratic(r1, r2);
            let sols = solve(&e, "x");
            assert!(!sols.is_empty(), "no roots for ({r1}, {r2})");
            for r in &sols.0 {
                assert!(root_valid(&e, r), "invalid root for ({r1}, {r2})");
            }
            checked += 1;
        }
    }
    assert!(checked >= 1000, "expected >= 1000 cases, got {checked}");
}
