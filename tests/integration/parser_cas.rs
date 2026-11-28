//! Golden integration tests for Maxima CAS command parsing + execution (Phase 31).

use maxima_rs::prelude::*;
use std::collections::HashMap;

fn eval_xy(e: &Expr, x: f64, y: f64) -> f64 {
    let mut env = HashMap::new();
    env.insert("x".to_string(), x);
    env.insert("y".to_string(), y);
    e.eval(&env).expect("eval xy")
}

fn eval_x(e: &Expr, x: f64) -> f64 {
    let mut env = HashMap::new();
    env.insert("x".to_string(), x);
    e.eval(&env).expect("eval x")
}

fn run(cmd: &str) -> Expr {
    let parsed = parse_maxima_cas(cmd).expect("parse");
    execute_cas(&parsed).expect("execute")
}

#[test]
fn test_parse_and_execute_expand() {
    let out = run("expand((x + y)^2)");
    assert!((eval_xy(&out, 2.0, 3.0) - 25.0).abs() < 1e-9);
}

#[test]
fn test_parse_and_execute_ratsimp() {
    let out = run("ratsimp((x^2 - 1)/(x - 1))");
    assert!((eval_x(&out, 3.0) - 4.0).abs() < 1e-9);
}

#[test]
fn test_parse_and_execute_factor_roundtrip() {
    let out = run("factor(x^2 - 1)");
    let back = expand(&out);
    assert!((eval_x(&back, 3.0) - 8.0).abs() < 1e-9);
}

#[test]
fn test_parse_and_execute_limit() {
    assert_eq!(
        execute_cas(&parse_maxima_cas("limit(sin(x)/x, x, 0)").unwrap()),
        Some(Expr::int(1))
    );
}

#[test]
fn test_parse_and_execute_integrate() {
    let f = run("integrate(x^2, x)");
    let d = differentiate(&f, "x");
    assert!((eval_x(&d, 2.0) - 4.0).abs() < 1e-9);
}

#[test]
fn test_parse_and_execute_defint() {
    let out = run("defint(x^2, x, 0, 1)");
    assert!((eval_x(&out, 0.0) - 1.0 / 3.0).abs() < 1e-9);
}

#[test]
fn test_parse_and_execute_solve() {
    let out = run("solve(x^2 - 1, x)");
    let e = Expr::sub(Expr::pow(Expr::var("x"), Expr::int(2)), Expr::int(1));
    match out {
        Expr::Matrix(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].len(), 2, "expected both roots of x^2 - 1");
            for r in &rows[0] {
                let rv = eval_x(r, 0.0);
                assert!(eval_x(&e, rv).abs() < 1e-6);
            }
        }
        other => panic!("expected Expr::Matrix of roots, got {other:?}"),
    }
    // Trailing tokens after ')' must be rejected
    assert!(parse_maxima_cas("expand(x) + 1").is_err());
}

#[test]
fn test_parse_and_execute_ode2() {
    let sol = run("ode2(diff(y, x) + y, y, x)");
    let residual = Expr::add(differentiate(&sol, "x"), sol.clone());
    let mut env = HashMap::new();
    env.insert("x".to_string(), 0.3);
    env.insert("_C1".to_string(), 1.0);
    let v = residual.eval(&env).expect("residual eval");
    assert!(v.abs() < 1e-9);
}

#[test]
fn test_parse_and_execute_taylor() {
    let out = run("taylor(sin(x), x, 0, 5)");
    assert!((eval_x(&out, 0.3) - 0.3_f64.sin()).abs() < 1e-4);
}

#[test]
fn test_prelude_exposes_all_cas_functions() {
    let x = Expr::var("x");
    let _ = expand(&Expr::pow(x.clone(), Expr::int(2)));
    let _ = ratsimp(&Expr::div(Expr::int(1), Expr::var("x")));
    let _ = factor(&Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)));
    let _ = partfrac(
        &Expr::div(
            Expr::int(1),
            Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
        ),
        "x",
    );
    let _ = integrate(&Expr::var("x"), "x");
    let _ = defint(&Expr::var("x"), "x", &Expr::int(0), &Expr::int(1));
    let _ = limit(&Expr::var("x"), "x", &Expr::int(0));
    let _ = taylor(&Expr::sin(Expr::var("x")), "x", &Expr::int(0), 5);
    let _ = solve(&Expr::sub(x, Expr::int(1)), "x");
    let ode = Expr::add(
        Expr::derivative(Expr::var("y"), Expr::var("x")),
        Expr::var("y"),
    );
    let _ = ode2(&ode, "y", "x");
}
