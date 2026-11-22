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
