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
