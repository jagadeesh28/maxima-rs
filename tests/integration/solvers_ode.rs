//! Golden integration tests for ODE solvers (Phase 30).
//!
//! Each solution is verified by substituting it back: the residual `y'' + ...`
//! evaluates to ~0 at sample points.

use maxima_rs::prelude::*;
use std::collections::HashMap;
use sym_core::diff::differentiate;
use sym_core::solve::ode2;

fn yprime() -> Expr {
    Expr::derivative(Expr::var("y"), Expr::var("x"))
}

fn ysecond() -> Expr {
    Expr::derivative(yprime(), Expr::var("x"))
}

fn eval_with(e: &Expr, vars: &[(&str, f64)]) -> f64 {
    let mut env = HashMap::new();
    for (k, v) in vars {
        env.insert((*k).to_string(), *v);
    }
    e.eval(&env).expect("evaluation failed")
}

#[test]
fn test_first_order_linear() {
    // y' + y = 0
    let ode = Expr::add(yprime(), Expr::var("y"));
    let sol = ode2(&ode, "y", "x").expect("solution");
    assert_eq!(sol.constants, vec!["_C1".to_string()]);
    let residual = Expr::add(differentiate(&sol.solution, "x"), sol.solution.clone());
    let v = eval_with(&residual, &[("x", 0.5), ("_C1", 1.0)]);
    assert!(v.abs() < 1e-9, "residual {v}");
}

#[test]
fn test_first_order_linear_with_distinct_coefficients() {
    // y' + 2*y = 0 -> y = _C1 * exp(-2*x)
    let ode = Expr::add(yprime(), Expr::mul(Expr::int(2), Expr::var("y")));
    let sol = ode2(&ode, "y", "x").expect("solution");
    assert_eq!(sol.constants, vec!["_C1".to_string()]);
    let residual = Expr::add(
        differentiate(&sol.solution, "x"),
        Expr::mul(Expr::int(2), sol.solution.clone()),
    );
    let v = eval_with(&residual, &[("x", 0.5), ("_C1", 1.0)]);
    assert!(v.abs() < 1e-9, "residual {v}");

    // 2*y' + 5*y = 0 -> y = _C1 * exp(-5/2*x)
    let ode2_expr = Expr::add(
        Expr::mul(Expr::int(2), yprime()),
        Expr::mul(Expr::int(5), Expr::var("y")),
    );
    let sol2 = ode2(&ode2_expr, "y", "x").expect("solution");
    let residual2 = Expr::add(
        Expr::mul(Expr::int(2), differentiate(&sol2.solution, "x")),
        Expr::mul(Expr::int(5), sol2.solution.clone()),
    );
    let v2 = eval_with(&residual2, &[("x", 0.3), ("_C1", 1.0)]);
    assert!(v2.abs() < 1e-9, "residual {v2}");
}

#[test]
fn test_second_order_oscillator() {
    // y'' + w^2 y = 0
    let w = Expr::var("w");
    let ode = Expr::add(
        ysecond(),
        Expr::mul(Expr::pow(w.clone(), Expr::int(2)), Expr::var("y")),
    );
    let sol = ode2(&ode, "y", "x").expect("solution");
    assert_eq!(sol.constants, vec!["_C1".to_string(), "_C2".to_string()]);
    let d1 = differentiate(&sol.solution, "x");
    let d2 = differentiate(&d1, "x");
    let residual = Expr::add(
        d2,
        Expr::mul(Expr::pow(w, Expr::int(2)), sol.solution.clone()),
    );
    let v = eval_with(
        &residual,
        &[("x", 0.4), ("w", 1.7), ("_C1", 1.0), ("_C2", 1.0)],
    );
    assert!(v.abs() < 1e-9, "residual {v}");
}

#[test]
fn test_second_order_real_roots() {
    // y'' - y = 0
    let ode = Expr::sub(ysecond(), Expr::var("y"));
    let sol = ode2(&ode, "y", "x").expect("solution");
    let d1 = differentiate(&sol.solution, "x");
    let d2 = differentiate(&d1, "x");
    let residual = Expr::sub(d2, sol.solution.clone());
    let v = eval_with(&residual, &[("x", 0.7), ("_C1", 1.0), ("_C2", 1.0)]);
    assert!(v.abs() < 1e-9, "residual {v}");
}

#[test]
fn test_phase50_ode_b_nonzero_c_zero_var_coeff_and_nonhomogeneous() {
    // 1. y'' + 3*y' + 2*y = 0 (b != 0, distinct real roots -1, -2)
    let ode_real = Expr::add(
        ysecond(),
        Expr::add(
            Expr::mul(Expr::int(3), yprime()),
            Expr::mul(Expr::int(2), Expr::var("y")),
        ),
    );
    let sol_real = ode2(&ode_real, "y", "x").expect("y'' + 3y' + 2y = 0");
    let d1 = differentiate(&sol_real.solution, "x");
    let d2 = differentiate(&d1, "x");
    let res_real = Expr::add(
        d2,
        Expr::add(
            Expr::mul(Expr::int(3), d1),
            Expr::mul(Expr::int(2), sol_real.solution.clone()),
        ),
    );
    assert!(eval_with(&res_real, &[("x", 0.4), ("_C1", 1.5), ("_C2", -0.7)]).abs() < 1e-9);

    // 2. y'' + 2*y' + y = 0 (b != 0, repeated root -1)
    let ode_rep = Expr::add(
        ysecond(),
        Expr::add(Expr::mul(Expr::int(2), yprime()), Expr::var("y")),
    );
    let sol_rep = ode2(&ode_rep, "y", "x").expect("y'' + 2y' + y = 0");
    let d1r = differentiate(&sol_rep.solution, "x");
    let d2r = differentiate(&d1r, "x");
    let res_rep = Expr::add(
        d2r,
        Expr::add(Expr::mul(Expr::int(2), d1r), sol_rep.solution.clone()),
    );
    assert!(eval_with(&res_rep, &[("x", 0.6), ("_C1", 1.0), ("_C2", 2.0)]).abs() < 1e-9);

    // 3. y'' + 2*y' + 5*y = 0 (b != 0, complex conjugate roots -1 ± 2i)
    let ode_cplx = Expr::add(
        ysecond(),
        Expr::add(
            Expr::mul(Expr::int(2), yprime()),
            Expr::mul(Expr::int(5), Expr::var("y")),
        ),
    );
    let sol_cplx = ode2(&ode_cplx, "y", "x").expect("y'' + 2y' + 5y = 0");
    let d1c = differentiate(&sol_cplx.solution, "x");
    let d2c = differentiate(&d1c, "x");
    let res_cplx = Expr::add(
        d2c,
        Expr::add(
            Expr::mul(Expr::int(2), d1c),
            Expr::mul(Expr::int(5), sol_cplx.solution.clone()),
        ),
    );
    assert!(eval_with(&res_cplx, &[("x", 0.3), ("_C1", 1.0), ("_C2", -1.0)]).abs() < 1e-9);

    // 4. y'' = 0 (c == 0, b == 0 -> _C1 + _C2 * x)
    let sol_zero = ode2(&ysecond(), "y", "x").expect("y'' = 0");
    let d1z = differentiate(&sol_zero.solution, "x");
    let d2z = differentiate(&d1z, "x");
    assert!(eval_with(&d2z, &[("x", 1.2), ("_C1", 3.0), ("_C2", 4.0)]).abs() < 1e-9);
    assert!(
        (eval_with(
            &sol_zero.solution,
            &[("x", 2.0), ("_C1", 3.0), ("_C2", 4.0)]
        ) - 11.0)
            .abs()
            < 1e-9
    );

    // 5. Variable-coefficient 1st-order: y' + (1/x)*y - 1 = 0
    let x = Expr::var("x");
    let ode_var = Expr::sub(
        Expr::add(
            yprime(),
            Expr::mul(Expr::div(Expr::int(1), x.clone()), Expr::var("y")),
        ),
        Expr::int(1),
    );
    let sol_var = ode2(&ode_var, "y", "x").expect("y' + y/x = 1");
    let res_var = Expr::sub(
        Expr::add(
            differentiate(&sol_var.solution, "x"),
            Expr::mul(Expr::div(Expr::int(1), x), sol_var.solution.clone()),
        ),
        Expr::int(1),
    );
    assert!(eval_with(&res_var, &[("x", 1.5), ("_C1", 2.0)]).abs() < 1e-9);

    // 6. Non-homogeneous 2nd-order: y'' + y - sin(2*x) = 0
    let ode_nh = Expr::sub(
        Expr::add(ysecond(), Expr::var("y")),
        Expr::sin(Expr::mul(Expr::int(2), Expr::var("x"))),
    );
    let sol_nh = ode2(&ode_nh, "y", "x").expect("y'' + y = sin(2x)");
    let d1n = differentiate(&sol_nh.solution, "x");
    let d2n = differentiate(&d1n, "x");
    let res_nh = Expr::sub(
        Expr::add(d2n, sol_nh.solution.clone()),
        Expr::sin(Expr::mul(Expr::int(2), Expr::var("x"))),
    );
    assert!(eval_with(&res_nh, &[("x", 0.5), ("_C1", 1.0), ("_C2", -0.5)]).abs() < 1e-8);
}
