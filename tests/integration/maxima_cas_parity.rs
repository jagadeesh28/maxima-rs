//! Maxima-style CAS parity cases (Phase 32).
//!
//! Representative cases mirroring Maxima's `rtest_integrate.mac`,
//! `rtest_limit.mac`, and `rtest_solve.mac`.

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;

fn eval_x(e: &Expr, x: f64) -> f64 {
    let mut env = HashMap::new();
    env.insert("x".to_string(), x);
    e.eval(&env).expect("eval")
}

#[test]
fn parity_integrate_power_rule() {
    for n in 1..=5 {
        let x = Expr::var("x");
        let f = Expr::pow(x, Expr::int(n));
        let big_f = integrate(&f, "x");
        let d = differentiate(&big_f, "x");
        for xv in [0.5_f64, 1.7] {
            assert!((eval_x(&f, xv) - eval_x(&d, xv)).abs() < 1e-6);
        }
    }
}

#[test]
fn parity_limit_cases() {
    let x = Expr::var("x");
    assert_eq!(
        limit(
            &Expr::div(Expr::sin(x.clone()), x.clone()),
            "x",
            &Expr::int(0)
        ),
        Some(Expr::int(1))
    );
    let base = Expr::add(Expr::int(1), Expr::div(Expr::int(1), x.clone()));
    assert_eq!(
        limit(&Expr::pow(base, x), "x", &Expr::var("inf")),
        Some(Expr::e())
    );
}

#[test]
fn parity_solve_cases() {
    let x = Expr::var("x");
    // x^2 - 5x + 6 = 0 => {2, 3}
    let e = Expr::add(
        Expr::sub(
            Expr::pow(x.clone(), Expr::int(2)),
            Expr::mul(Expr::int(5), x),
        ),
        Expr::int(6),
    );
    let sols = solve(&e, "x");
    assert_eq!(sols.len(), 2);
    let mut values: Vec<i64> = sols
        .0
        .iter()
        .filter_map(|r| r.eval(&HashMap::new()).map(|v| v.round() as i64))
        .collect();
    values.sort_unstable();
    assert_eq!(values, vec![2, 3]);
}

#[test]
fn parity_defint_cases() {
    let x = Expr::var("x");
    assert_eq!(
        defint(
            &Expr::pow(x, Expr::int(2)),
            "x",
            &Expr::int(0),
            &Expr::int(1)
        ),
        DefiniteResult::Value(Expr::frac(1, 3))
    );
}
