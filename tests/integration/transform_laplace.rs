//! Integration test suite for Phase 43: Operational Laplace Transforms & ODE Initial Values (`transform`).
//!
//! Citations:
//! - Maxima 5.50: `src/laplac.lisp`, `src/ode2.lisp`.
//! - ADR-0011: Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture.

use maxima_rs::prelude::*;

#[test]
fn test_forward_laplace_constants_and_powers() {
    let t = "t";
    let s = "s";

    // L{0} = 0
    assert_eq!(laplace(&Expr::int(0), t, s), Expr::int(0));

    // L{1} = 1/s
    assert_eq!(
        laplace(&Expr::int(1), t, s),
        Expr::div(Expr::int(1), Expr::var(s))
    );

    // L{7} = 7/s
    assert_eq!(
        laplace(&Expr::int(7), t, s),
        Expr::div(Expr::int(7), Expr::var(s))
    );

    // L{t} = 1/s^2
    assert_eq!(
        laplace(&Expr::var(t), t, s),
        Expr::div(Expr::int(1), Expr::pow(Expr::var(s), Expr::int(2)))
    );

    // L{t^2} = 2! / s^3 = 2 / s^3
    let t2 = Expr::pow(Expr::var(t), Expr::int(2));
    assert_eq!(
        laplace(&t2, t, s),
        Expr::div(Expr::int(2), Expr::pow(Expr::var(s), Expr::int(3)))
    );

    // L{t^3} = 3! / s^4 = 6 / s^4
    let t3 = Expr::pow(Expr::var(t), Expr::int(3));
    assert_eq!(
        laplace(&t3, t, s),
        Expr::div(Expr::int(6), Expr::pow(Expr::var(s), Expr::int(4)))
    );
}

#[test]
fn test_forward_laplace_transcendentals() {
    let t = "t";
    let s = "s";

    // L{exp(3*t)} = 1 / (s - 3)
    let exp_3t = Expr::exp(Expr::mul(Expr::int(3), Expr::var(t)));
    assert_eq!(
        laplace(&exp_3t, t, s),
        Expr::div(Expr::int(1), Expr::sub(Expr::var(s), Expr::int(3)))
    );

    // L{exp(-5*t)} = 1 / (s - (-5)) = 1 / (s - (-5))
    let exp_neg5t = Expr::exp(Expr::mul(Expr::int(-5), Expr::var(t)));
    assert_eq!(
        laplace(&exp_neg5t, t, s),
        Expr::div(Expr::int(1), Expr::sub(Expr::var(s), Expr::int(-5)))
    );

    // L{sin(4*t)} = 4 / (s^2 + 16)
    let sin_4t = Expr::sin(Expr::mul(Expr::int(4), Expr::var(t)));
    let s_sq = Expr::pow(Expr::var(s), Expr::int(2));
    let expected_sin = Expr::div(
        Expr::int(4),
        Expr::add(s_sq.clone(), Expr::pow(Expr::int(4), Expr::int(2))),
    );
    assert_eq!(laplace(&sin_4t, t, s), expected_sin);

    // L{cos(4*t)} = s / (s^2 + 16)
    let cos_4t = Expr::cos(Expr::mul(Expr::int(4), Expr::var(t)));
    let expected_cos = Expr::div(
        Expr::var(s),
        Expr::add(s_sq.clone(), Expr::pow(Expr::int(4), Expr::int(2))),
    );
    assert_eq!(laplace(&cos_4t, t, s), expected_cos);

    // L{sinh(2*t)} = 2 / (s^2 - 4)
    let sinh_2t = Expr::sinh(Expr::mul(Expr::int(2), Expr::var(t)));
    let expected_sinh = Expr::div(
        Expr::int(2),
        Expr::sub(s_sq.clone(), Expr::pow(Expr::int(2), Expr::int(2))),
    );
    assert_eq!(laplace(&sinh_2t, t, s), expected_sinh);

    // L{cosh(2*t)} = s / (s^2 - 4)
    let cosh_2t = Expr::cosh(Expr::mul(Expr::int(2), Expr::var(t)));
    let expected_cosh = Expr::div(
        Expr::var(s),
        Expr::sub(s_sq, Expr::pow(Expr::int(2), Expr::int(2))),
    );
    assert_eq!(laplace(&cosh_2t, t, s), expected_cosh);
}

#[test]
fn test_forward_laplace_operational_rules() {
    let t = "t";
    let s = "s";

    // Frequency shift: L{exp(2*t) * sin(3*t)} = 3 / ((s - 2)^2 + 9)
    let exp_sin = Expr::mul(
        Expr::exp(Expr::mul(Expr::int(2), Expr::var(t))),
        Expr::sin(Expr::mul(Expr::int(3), Expr::var(t))),
    );
    let shifted_s = Expr::sub(Expr::var(s), Expr::int(2));
    let expected_shift = Expr::div(
        Expr::int(3),
        Expr::add(
            Expr::pow(shifted_s, Expr::int(2)),
            Expr::pow(Expr::int(3), Expr::int(2)),
        ),
    );
    assert_eq!(laplace(&exp_sin, t, s), expected_shift);

    // Multiplication by t: L{t * exp(-a*t)} = - d/ds (1 / (s - (-a)))
    let exp_neg_at = Expr::exp(Expr::mul(Expr::neg(Expr::var("a")), Expr::var(t)));
    let t_exp = Expr::mul(Expr::var(t), exp_neg_at);
    let l_t_exp = laplace(&t_exp, t, s);
    // Should evaluate correctly at s = 2, a = 1
    let env = [("s".to_string(), 2.0_f64), ("a".to_string(), 1.0_f64)]
        .into_iter()
        .collect();
    // 1 / (2 + 1)^2 = 1/9 ≈ 0.1111
    let num_val = l_t_exp.eval(&env).expect("eval");
    assert!((num_val - 1.0 / 9.0).abs() < 1e-6);

    // Derivatives: L{y'(t)} = s*Y(s) - y(0)
    let dy = Expr::derivative(Expr::var("y"), Expr::var(t));
    let l_dy = laplace(&dy, t, s);
    assert_eq!(
        l_dy,
        Expr::sub(Expr::mul(Expr::var(s), Expr::var("y_s")), Expr::var("y_0"))
    );
}

#[test]
fn test_inverse_laplace_rational() {
    let s = "s";
    let t = "t";

    // ilt{1/s} = 1
    let f1 = Expr::div(Expr::int(1), Expr::var(s));
    assert_eq!(ilt(&f1, s, t), Expr::int(1));

    // ilt{1/s^2} = t
    let f2 = Expr::div(Expr::int(1), Expr::pow(Expr::var(s), Expr::int(2)));
    assert_eq!(ilt(&f2, s, t), Expr::var(t));

    // ilt{1/s^3} = t^2 / 2
    let f3 = Expr::div(Expr::int(1), Expr::pow(Expr::var(s), Expr::int(3)));
    assert_eq!(
        ilt(&f3, s, t),
        Expr::div(Expr::pow(Expr::var(t), Expr::int(2)), Expr::int(2))
    );

    // ilt{1 / (s - 3)} = exp(3*t)
    let f_exp = Expr::div(Expr::int(1), Expr::sub(Expr::var(s), Expr::int(3)));
    assert_eq!(
        ilt(&f_exp, s, t),
        Expr::exp(Expr::mul(Expr::int(3), Expr::var(t)))
    );

    // ilt{1 / (s + 4)} = exp(-4*t)
    let f_exp_neg = Expr::div(Expr::int(1), Expr::add(Expr::var(s), Expr::int(4)));
    assert_eq!(
        ilt(&f_exp_neg, s, t),
        Expr::exp(Expr::mul(Expr::int(-4), Expr::var(t)))
    );

    // ilt{1 / (s^2 + 4)} = (1/2)*sin(2*t)
    let f_sin = Expr::div(
        Expr::int(1),
        Expr::add(Expr::pow(Expr::var(s), Expr::int(2)), Expr::int(4)),
    );
    assert_eq!(
        ilt(&f_sin, s, t),
        Expr::div(
            Expr::sin(Expr::mul(Expr::int(2), Expr::var(t))),
            Expr::int(2)
        )
    );

    // ilt{s / (s^2 + 9)} = cos(3*t)
    let f_cos = Expr::div(
        Expr::var(s),
        Expr::add(Expr::pow(Expr::var(s), Expr::int(2)), Expr::int(9)),
    );
    assert_eq!(
        ilt(&f_cos, s, t),
        Expr::cos(Expr::mul(Expr::int(3), Expr::var(t)))
    );
}

#[test]
fn test_initial_value_problem_ic1() {
    // General solution: y(x) = C * exp(-x)
    let c = "_C1";
    let x = "x";
    let general_sol = Expr::mul(Expr::var(c), Expr::exp(Expr::neg(Expr::var(x))));

    // Initial condition: y(0) = 5
    let particular = ic1(&general_sol, x, c, &Expr::int(0), &Expr::int(5)).expect("solve ic1");

    // At x = 0, y should be 5
    let env0 = [("x".to_string(), 0.0_f64)].into_iter().collect();
    assert_eq!(particular.eval(&env0), Some(5.0));

    // At x = 1, y should be 5 * e^(-1)
    let env1 = [("x".to_string(), 1.0_f64)].into_iter().collect();
    let expected = 5.0 * (-1.0_f64).exp();
    let actual = particular.eval(&env1).expect("eval");
    assert!((actual - expected).abs() < 1e-6);
}

#[test]
fn test_initial_value_problem_ic2() {
    // 2nd-order harmonic oscillator: y(x) = C1 * cos(2x) + C2 * sin(2x)
    let x = "x";
    let c1 = "_C1";
    let c2 = "_C2";
    let two_x = Expr::mul(Expr::int(2), Expr::var(x));
    let general_sol = Expr::add(
        Expr::mul(Expr::var(c1), Expr::cos(two_x.clone())),
        Expr::mul(Expr::var(c2), Expr::sin(two_x)),
    );

    // Initial conditions: y(0) = 3, y'(0) = 4
    // Since y(0) = C1 = 3, and y'(x) = -2 C1 sin(2x) + 2 C2 cos(2x) => y'(0) = 2 C2 = 4 => C2 = 2
    let particular = ic2(
        &general_sol,
        x,
        c1,
        c2,
        &Expr::int(0),
        &Expr::int(3),
        &Expr::int(4),
    )
    .expect("solve ic2");

    // Verify y(0) = 3
    let env0 = [("x".to_string(), 0.0_f64)].into_iter().collect();
    assert_eq!(particular.eval(&env0), Some(3.0));

    // Verify y'(0) = 4
    let dy = differentiate(&particular, x);
    assert_eq!(dy.eval(&env0), Some(4.0));
}

#[test]
fn test_boundary_value_problem_bc2() {
    // Boundary value problem: y(x) = C1 * cos(x) + C2 * sin(x)
    let x = "x";
    let c1 = "_C1";
    let c2 = "_C2";
    let general_sol = Expr::add(
        Expr::mul(Expr::var(c1), Expr::cos(Expr::var(x))),
        Expr::mul(Expr::var(c2), Expr::sin(Expr::var(x))),
    );

    // Boundary conditions: y(0) = 0, y(pi/2) = 1
    // => C1 = 0, C2 = 1 => y(x) = sin(x)
    let half_pi = Expr::div(Expr::pi(), Expr::int(2));
    let particular = bc2(
        &general_sol,
        x,
        c1,
        c2,
        (&Expr::int(0), &Expr::int(0)),
        (&half_pi, &Expr::int(1)),
    )
    .expect("solve bc2");

    // Verify y(0) = 0
    let env0 = [("x".to_string(), 0.0_f64)].into_iter().collect();
    let val0 = particular.eval(&env0).expect("eval at 0");
    assert!(val0.abs() < 1e-6);

    // Verify y(pi/2) = 1
    let env_pi_2 = [("x".to_string(), std::f64::consts::PI / 2.0)]
        .into_iter()
        .collect();
    let val_pi_2 = particular.eval(&env_pi_2).expect("eval at pi/2");
    assert!((val_pi_2 - 1.0).abs() < 1e-6);
}

#[test]
fn test_ic1_and_ic2_ode_integration() {
    // Solve y' + y = 0
    let eq = Expr::add(
        Expr::derivative(Expr::var("y"), Expr::var("x")),
        Expr::var("y"),
    );
    let ode_sol = ode2(&eq, "y", "x").expect("solve ode");

    // Apply IC: y(0) = 2
    let particular = ic1_ode(&ode_sol, "x", &Expr::int(0), &Expr::int(2)).expect("ic1_ode");
    let env0 = [("x".to_string(), 0.0_f64)].into_iter().collect();
    assert_eq!(particular.eval(&env0), Some(2.0));
}
