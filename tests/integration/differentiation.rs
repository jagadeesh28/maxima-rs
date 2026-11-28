//! Golden integration tests for analytical differentiation, gradient, and Hessian.

use maxima_rs::prelude::*;
use std::collections::HashMap;

#[test]
fn test_polynomial_derivatives() {
    let x = Expr::var("x");

    // f(x) = x^3 + 2*x^2 + 5*x + 7
    let f = Expr::add(
        Expr::add(
            Expr::pow(x.clone(), Expr::int(3)),
            Expr::mul(Expr::int(2), Expr::pow(x.clone(), Expr::int(2))),
        ),
        Expr::add(Expr::mul(Expr::int(5), x.clone()), Expr::int(7)),
    );

    let df = differentiate(&f, "x");

    // Numerical evaluation at x = 2
    // f'(x) = 3*x^2 + 4*x + 5 -> at x=2: 3(4) + 4(2) + 5 = 12 + 8 + 5 = 25
    let mut env = HashMap::new();
    env.insert("x".to_string(), 2.0);

    let val = df.eval(&env).expect("evaluation succeeded");
    assert!((val - 25.0).abs() < 1e-10);
}

#[test]
fn test_product_and_chain_rule() {
    let x = Expr::var("x");

    // f(x) = x * sin(x^2)
    let f = Expr::mul(x.clone(), Expr::sin(Expr::pow(x.clone(), Expr::int(2))));

    let df = differentiate(&f, "x");

    // f'(x) = sin(x^2) + x * cos(x^2) * 2*x = sin(x^2) + 2*x^2 * cos(x^2)
    // At x = 1: sin(1) + 2 * cos(1) ≈ 0.84147098 + 2 * 0.5403023 = 1.92207559
    let mut env = HashMap::new();
    env.insert("x".to_string(), 1.0);

    let val = df.eval(&env).expect("evaluation succeeded");
    let expected = 1.0f64.sin() + 2.0 * 1.0f64.cos();
    assert!((val - expected).abs() < 1e-10);
}

#[test]
fn test_rosenbrock_gradient_and_hessian() {
    let x = Expr::var("x");
    let y = Expr::var("y");

    // Rosenbrock: f(x, y) = (1 - x)^2 + 100*(y - x^2)^2
    let term1 = Expr::pow(Expr::sub(Expr::int(1), x.clone()), Expr::int(2));
    let term2 = Expr::mul(
        Expr::int(100),
        Expr::pow(
            Expr::sub(y.clone(), Expr::pow(x.clone(), Expr::int(2))),
            Expr::int(2),
        ),
    );
    let rosen = Expr::add(term1, term2);

    let grad = gradient(&rosen, &["x", "y"]);
    assert_eq!(grad.len(), 2);

    // At minimum (1, 1), gradient should be [0, 0]
    let mut env = HashMap::new();
    env.insert("x".to_string(), 1.0);
    env.insert("y".to_string(), 1.0);

    let gx = grad[0].eval(&env).expect("eval gx");
    let gy = grad[1].eval(&env).expect("eval gy");
    assert!(gx.abs() < 1e-10, "gx was {gx}");
    assert!(gy.abs() < 1e-10, "gy was {gy}");

    // Test Hessian symmetry: H_01 == H_10
    let hess = hessian(&rosen, &["x", "y"]);
    let h01 = hess[0][1].eval(&env).expect("eval h01");
    let h10 = hess[1][0].eval(&env).expect("eval h10");
    assert!(
        (h01 - h10).abs() < 1e-10,
        "Hessian not symmetric: {h01} vs {h10}"
    );
}

#[test]
fn test_phase49_diff_factorial_ftc_sum_and_zero_pow() {
    let x = Expr::var("x");
    let i = Expr::var("i");

    // 1. diff(x!, x) = x! * psi(0, x + 1)
    let dfac = differentiate(&Expr::factorial(x.clone()), "x");
    let expected_fac = Expr::mul(
        Expr::factorial(x.clone()),
        Expr::special_fn(
            sym_core::specfn::SpecialFnKind::Psi,
            vec![Expr::int(0), Expr::add(x.clone(), Expr::int(1))],
        ),
    );
    assert_eq!(dfac, expected_fac);

    // 2. FTC: d/dx(∫ sin(x) dx) = sin(x)
    let ftc = differentiate(&Expr::integral(Expr::sin(x.clone()), x.clone()), "x");
    assert_eq!(ftc, Expr::sin(x.clone()));

    // 3. Sum with constant bounds: d/dx(sum(i * x^2, i, 1, 5)) = sum(i * (2 * x), i, 1, 5)
    let sum_expr = Expr::sum(
        Expr::mul(i.clone(), Expr::pow(x.clone(), Expr::int(2))),
        i.clone(),
        Expr::int(1),
        Expr::int(5),
    );
    let dsum = differentiate(&sum_expr, "x");
    assert_eq!(
        dsum,
        Expr::sum(
            Expr::mul(i.clone(), Expr::mul(Expr::int(2), x.clone())),
            i.clone(),
            Expr::int(1),
            Expr::int(5),
        )
    );

    // Shadowed summation index: d/di(sum(i * x, i, 1, 5)) = 0
    assert_eq!(differentiate(&sum_expr, "i"), Expr::int(0));

    // 4. 0^0 and 0^(-1) guards in diff
    let d_zero_neg = differentiate(&Expr::pow(x, Expr::int(-1)), "x");
    assert!(!d_zero_neg.is_zero());
}
