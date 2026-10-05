//! Golden integration tests for LaTeX scalar math parser.

use maxima_rs::prelude::*;

#[test]
fn test_parse_latex_fraction() {
    let e = parse_latex(r"\frac{x + 1}{2}").expect("parse frac");
    let expected = Expr::div(Expr::add(Expr::var("x"), Expr::int(1)), Expr::int(2));
    assert_eq!(e, expected);
}

#[test]
fn test_parse_latex_sqrt() {
    let e = parse_latex(r"\sqrt{x^2 + 1}").expect("parse sqrt");
    let expected = Expr::sqrt(Expr::add(
        Expr::pow(Expr::var("x"), Expr::int(2)),
        Expr::int(1),
    ));
    assert_eq!(e, expected);
}

#[test]
fn test_parse_latex_pythagorean_and_optimize() {
    // \frac{\sin(x)^2 + \cos(x)^2}{2} -> 1/2
    let e = parse_latex(r"\frac{\sin(x)^2 + \cos(x)^2}{2}").expect("parse pythagorean latex");
    let opt = Pipeline::new().with_expr(e).optimize();
    assert_eq!(opt.expressions(), &[Expr::frac(1, 2)]);
}

#[test]
fn test_parse_latex_multiplication_symbols() {
    // \cdot and \times
    let e1 = parse_latex(r"2 \cdot x").expect("parse cdot");
    assert_eq!(e1, Expr::mul(Expr::int(2), Expr::var("x")));

    let e2 = parse_latex(r"3 \times y").expect("parse times");
    assert_eq!(e2, Expr::mul(Expr::int(3), Expr::var("y")));
}

#[test]
fn test_parse_latex_advanced_math() {
    // Tensor indices
    let t = parse_latex(r"T_{\mu}").unwrap();
    assert_eq!(
        t,
        Expr::tensor(
            "T",
            vec![sym_core::tensor::Index {
                name: "mu".to_string(),
                idx_type: sym_core::tensor::IndexType::Covariant
            }]
        )
    );

    // Greek letters
    let greek = parse_latex(r"\alpha + \beta").unwrap();
    assert_eq!(greek, Expr::add(Expr::var("alpha"), Expr::var("beta")));

    // Matrix
    let mat = parse_latex(r"\begin{pmatrix} 1 & x \\ y & 2 \end{pmatrix}").unwrap();
    assert_eq!(
        mat,
        Expr::Matrix(vec![
            vec![Expr::int(1), Expr::var("x")],
            vec![Expr::var("y"), Expr::int(2)],
        ])
    );

    // Integral
    let intg = parse_latex(r"\int_{0}^{1} x").unwrap();
    assert_eq!(intg, Expr::integral(Expr::var("x"), Expr::var("x"))); // default integration var is x

    // Summation
    let summ = parse_latex(r"\sum_{i=1}^{n} i").unwrap();
    assert_eq!(
        summ,
        Expr::sum(Expr::var("i"), Expr::var("i"), Expr::int(1), Expr::var("n"))
    );

    // Partial derivative
    let diff1 = parse_latex(r"\frac{\partial f}{\partial x}").unwrap();
    assert_eq!(diff1, Expr::derivative(Expr::var("f"), Expr::var("x")));

    let diff2 = parse_latex(r"\frac{\partial}{\partial y} g").unwrap();
    assert_eq!(diff2, Expr::derivative(Expr::var("g"), Expr::var("y")));
}
