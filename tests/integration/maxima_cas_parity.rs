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

#[test]
fn parity_phase52_cas_functions_and_directional_limits() {
    use maxima_rs::parser::maxima::{execute_cas, parse_maxima_cas};

    // 1. trigexpand(sin(x + y))
    let te = execute_cas(&parse_maxima_cas("trigexpand(sin(x + y))").unwrap()).unwrap();
    assert!(te.to_string().contains("sin") && te.to_string().contains("cos"));

    // 2. trigreduce(sin(x)*cos(x)) -> 1/2 * sin(2*x)
    let tr = execute_cas(&parse_maxima_cas("trigreduce(sin(x)*cos(x))").unwrap()).unwrap();
    let mut env = HashMap::new();
    env.insert("x".to_string(), 0.7);
    let diff_tr = (tr.eval(&env).unwrap() - (0.7_f64.sin() * 0.7_f64.cos())).abs();
    assert!(diff_tr < 1e-10, "trigreduce mismatch: {tr}");

    // 3. trigsimp(tan(x)*cos(x)) -> sin(x)
    let ts = execute_cas(&parse_maxima_cas("trigsimp(tan(x)*cos(x))").unwrap()).unwrap();
    let diff_ts = (ts.eval(&env).unwrap() - 0.7_f64.sin()).abs();
    assert!(diff_ts < 1e-10, "trigsimp mismatch: {ts}");

    // 4. radcan(ln(exp(x))) -> x
    let rc = execute_cas(&parse_maxima_cas("radcan(ln(exp(x)))").unwrap()).unwrap();
    assert_eq!(rc, Expr::var("x"));

    // 5. demoivre(exp(%i*x)) -> cos(x) + %i*sin(x) and exponentialize(cos(x))
    let dm = execute_cas(&parse_maxima_cas("demoivre(exp(%i*x))").unwrap()).unwrap();
    assert!(dm.to_string().contains("cos") && dm.to_string().contains("sin"));
    let ex = execute_cas(&parse_maxima_cas("exponentialize(cos(x))").unwrap()).unwrap();
    assert!(ex.to_string().contains("exp"));

    // 6. subst(3, x, x^2 + 1) -> 10 (or 3^2 + 1)
    let sb = execute_cas(&parse_maxima_cas("subst(3, x, x^2 + 1)").unwrap()).unwrap();
    assert_eq!(sb.eval(&HashMap::new()), Some(10.0));

    // 7. float(%pi)
    let fl = execute_cas(&parse_maxima_cas("float(%pi)").unwrap()).unwrap();
    assert!((fl.eval(&HashMap::new()).unwrap() - std::f64::consts::PI).abs() < 1e-12);

    // 8. divide, quotient, remainder, num, denom
    let q = execute_cas(&parse_maxima_cas("quotient(x^2 - 1, x - 1, x)").unwrap()).unwrap();
    assert!((q.eval(&env).unwrap() - 1.7).abs() < 1e-10);
    let r = execute_cas(&parse_maxima_cas("remainder(x^2 + 1, x - 1, x)").unwrap()).unwrap();
    assert_eq!(r.eval(&env), Some(2.0));
    let div = execute_cas(&parse_maxima_cas("divide(x^2 - 1, x - 1, x)").unwrap()).unwrap();
    assert!(matches!(div, Expr::Matrix(_)));
    let n = execute_cas(&parse_maxima_cas("num((x^2 - 1)/(x + 2))").unwrap()).unwrap();
    let d = execute_cas(&parse_maxima_cas("denom((x^2 - 1)/(x + 2))").unwrap()).unwrap();
    assert!((n.eval(&env).unwrap() - (0.49 - 1.0)).abs() < 1e-10);
    assert!((d.eval(&env).unwrap() - 2.7).abs() < 1e-10);

    // 9. finite sum & product
    let s = execute_cas(&parse_maxima_cas("sum(k, k, 1, 5)").unwrap()).unwrap();
    assert_eq!(s.eval(&HashMap::new()), Some(15.0));
    let p = execute_cas(&parse_maxima_cas("product(k, k, 1, 5)").unwrap()).unwrap();
    assert_eq!(p.eval(&HashMap::new()), Some(120.0));

    // 10. directional limits: limit(exp(1/x), x, 0, plus) = inf, minus = 0
    let lp = execute_cas(&parse_maxima_cas("limit(exp(1/x), x, 0, plus)").unwrap()).unwrap();
    assert_eq!(lp, Expr::var("inf"));
    let lm = execute_cas(&parse_maxima_cas("limit(exp(1/x), x, 0, minus)").unwrap()).unwrap();
    assert_eq!(lm, Expr::int(0));

    // 11. diff(x^3, x, 2) -> 6*x
    let dn = execute_cas(&parse_maxima_cas("diff(x^3, x, 2)").unwrap()).unwrap();
    assert!((dn.eval(&env).unwrap() - 4.2).abs() < 1e-10);
}
