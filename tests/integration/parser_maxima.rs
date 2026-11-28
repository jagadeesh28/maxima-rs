//! Golden integration tests for Maxima string syntax parser.

use maxima_rs::prelude::*;

#[test]
fn test_parse_arithmetic_precedence() {
    // 2 + 3 * 4 should parse as 2 + (3 * 4) = 14
    let e = parse_maxima("2 + 3 * 4").expect("parse arithmetic");
    assert_eq!(
        e,
        Expr::add(Expr::int(2), Expr::mul(Expr::int(3), Expr::int(4)))
    );

    let val = e.eval(&Default::default()).expect("eval");
    assert_eq!(val, 14.0);
}

#[test]
fn test_parse_nested_parentheses() {
    let e = parse_maxima("((x + 1) * (y - 2)) / 3").expect("parse nested");
    let expected = Expr::div(
        Expr::mul(
            Expr::add(Expr::var("x"), Expr::int(1)),
            Expr::sub(Expr::var("y"), Expr::int(2)),
        ),
        Expr::int(3),
    );
    assert_eq!(e, expected);
}

#[test]
fn test_parse_functions_and_powers() {
    let e = parse_maxima("sin(x)^2 + cos(x)^2").expect("parse trig powers");
    let expected = Expr::add(
        Expr::pow(Expr::sin(Expr::var("x")), Expr::int(2)),
        Expr::pow(Expr::cos(Expr::var("x")), Expr::int(2)),
    );
    assert_eq!(e, expected);

    // Optimize it!
    let opt = Pipeline::new().with_expr(e).optimize();
    assert_eq!(opt.expressions(), &[Expr::int(1)]);
}

#[test]
fn test_parse_exponential_and_log() {
    let e = parse_maxima("exp(x) * ln(y)").expect("parse exp and ln");
    let expected = Expr::mul(Expr::exp(Expr::var("x")), Expr::ln(Expr::var("y")));
    assert_eq!(e, expected);
}

#[test]
fn test_parse_constants_and_factorial() {
    let half_pi = parse_maxima("%pi / 2").expect("parse %pi");
    assert_eq!(half_pi, Expr::div(Expr::pi(), Expr::int(2)));

    let fac = parse_maxima("5!").expect("parse factorial");
    assert_eq!(fac, Expr::factorial(Expr::int(5)));
    assert_eq!(fac.eval(&Default::default()), Some(120.0));
}

#[test]
fn test_parse_hyperbolic_and_inverse_trig() {
    let e = parse_maxima("sinh(x) + atan(y)").expect("parse hyperbolic/inverse trig");
    assert_eq!(
        e,
        Expr::add(Expr::sinh(Expr::var("x")), Expr::atan(Expr::var("y")))
    );
}

#[test]
fn test_parse_calculus_and_matrix() {
    let d = parse_maxima("diff(x^2, x)").expect("parse diff");
    assert_eq!(
        d,
        Expr::derivative(Expr::pow(Expr::var("x"), Expr::int(2)), Expr::var("x"))
    );

    let m = parse_maxima("matrix([1, 2], [3, 4])").expect("parse matrix");
    match m {
        Expr::Matrix(rows) => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][1], Expr::int(2));
            assert_eq!(rows[1][0], Expr::int(3));
        }
        other => panic!("expected matrix, got {other}"),
    }
}

#[test]
fn test_parse_maxima_display_roundtrip() {
    for s in ["%pi + %e", "sinh(x)^2", "3!", "2 * x!"] {
        let parsed = parse_maxima(s).expect("initial parse");
        let reparsed = parse_maxima(&parsed.to_string());
        assert!(reparsed.is_ok(), "display roundtrip failed for {s}");
    }
}

#[test]
fn test_parse_operator_precedence_negative_power() {
    // -x^2 at x=3 must equal -9 (never 9)
    let e = parse_maxima("-x^2").expect("parse -x^2");
    assert_eq!(e, Expr::neg(Expr::pow(Expr::var("x"), Expr::int(2))));
    let mut env = std::collections::HashMap::new();
    env.insert("x".to_string(), 3.0);
    assert_eq!(e.eval(&env), Some(-9.0));

    // -2^2 == -4
    let e2 = parse_maxima("-2^2").expect("parse -2^2");
    assert_eq!(e2, Expr::neg(Expr::pow(Expr::int(2), Expr::int(2))));
    assert_eq!(e2.eval(&Default::default()), Some(-4.0));

    // 2^-1 == 0.5
    let e3 = parse_maxima("2^-1").expect("parse 2^-1");
    assert_eq!(e3, Expr::pow(Expr::int(2), Expr::neg(Expr::int(1))));
    assert_eq!(e3.eval(&Default::default()), Some(0.5));

    // -x! == -(x!)
    let e4 = parse_maxima("-x!").expect("parse -x!");
    assert_eq!(e4, Expr::neg(Expr::factorial(Expr::var("x"))));

    // (-x)^2 == x^2 at x=3 is 9
    let e5 = parse_maxima("(-x)^2").expect("parse (-x)^2");
    assert_eq!(e5, Expr::pow(Expr::neg(Expr::var("x")), Expr::int(2)));
    assert_eq!(e5.eval(&env), Some(9.0));

    // exp(-x^2) == Exp(Neg(Pow(x, 2)))
    let e6 = parse_maxima("exp(-x^2)").expect("parse exp(-x^2)");
    assert_eq!(
        e6,
        Expr::exp(Expr::neg(Expr::pow(Expr::var("x"), Expr::int(2))))
    );
}

#[test]
fn test_parse_scientific_notation() {
    let e1 = parse_maxima("1e-3").expect("parse 1e-3");
    assert_eq!(e1.eval(&Default::default()), Some(0.001));

    let e2 = parse_maxima("2.5e+4").expect("parse 2.5e+4");
    assert_eq!(e2.eval(&Default::default()), Some(25000.0));
}

#[test]
fn test_parse_recursion_depth_guard() {
    let deep_expr = format!("{}x{}", "(".repeat(3000), ")".repeat(3000));
    let res = parse_maxima(&deep_expr);
    assert_eq!(
        res,
        Err(maxima_rs::parser::ParseError::RecursionLimitExceeded)
    );
}

#[test]
fn test_phase52_maxima_550_syntax_parity() {
    let mut env = std::collections::HashMap::new();
    env.insert("x".to_string(), 2.0);
    env.insert("y".to_string(), 3.0);

    // 1. `**` power operator (same precedence & right-associativity as `^`)
    let p1 = parse_maxima("2**3").expect("2**3");
    assert_eq!(p1.eval(&env), Some(8.0));
    let p2 = parse_maxima("-x**2").expect("-x**2");
    assert_eq!(p2.eval(&env), Some(-4.0));

    // 2. `lhs = rhs` equation -> `lhs - rhs`
    let eq = parse_maxima("x^2 = 4").expect("x^2 = 4");
    assert_eq!(eq.eval(&env), Some(0.0));

    // 3. `[a, b, c]` list literal
    let lst = parse_maxima("[1, x, y]").expect("[1, x, y]");
    assert_eq!(
        lst,
        Expr::matrix(vec![vec![Expr::int(1), Expr::var("x"), Expr::var("y")]])
    );

    // 4. Reciprocal & inverse trig/hyperbolic + abs + atan2
    for s in [
        "sec(0)",
        "csc(%pi/2)",
        "cot(%pi/4)",
        "asec(2)",
        "acsc(2)",
        "acot(1)",
        "asinh(0)",
        "acosh(1)",
        "atanh(0)",
        "atan2(1, 1)",
        "abs(-5)",
    ] {
        let parsed = parse_maxima(s).unwrap_or_else(|e| panic!("failed to parse {s}: {e}"));
        assert!(parsed.eval(&env).is_some(), "eval failed for {s}");
    }
    assert_eq!(parse_maxima("abs(-5)").unwrap().eval(&env), Some(5.0));

    // 5. Higher-order diff(expr, x, n)
    let d2 = parse_maxima("diff(x^3, x, 2)").expect("diff(x^3, x, 2)");
    assert_eq!(
        d2,
        Expr::derivative(
            Expr::derivative(Expr::pow(Expr::var("x"), Expr::int(3)), Expr::var("x")),
            Expr::var("x")
        )
    );

    // 6. Noun quote `'`
    let quoted = parse_maxima("'diff(y, x) + y = 0").expect("'diff(y, x) + y = 0");
    assert_eq!(
        quoted,
        Expr::sub(
            Expr::add(
                Expr::derivative(Expr::var("y"), Expr::var("x")),
                Expr::var("y")
            ),
            Expr::int(0)
        )
    );
}
