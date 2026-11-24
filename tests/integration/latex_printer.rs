//! Integration and snapshot tests for the precedence-aware LaTeX formatter.

use maxima_rs::prelude::*;

#[test]
fn test_precedence_basic_arithmetic() {
    let x = Expr::var("x");
    let y = Expr::var("y");
    let z = Expr::var("z");

    // Addition: x + y + z (no parens)
    let e_add = Expr::add(Expr::add(x.clone(), y.clone()), z.clone());
    assert_eq!(to_latex(&e_add), "x + y + z");

    // Subtraction: x - (y + z) (parens on right)
    let e_sub_add = Expr::sub(x.clone(), Expr::add(y.clone(), z.clone()));
    assert_eq!(to_latex(&e_sub_add), r"x - \left( y + z \right)");

    // Subtraction: x - (y - z) (parens on right)
    let e_sub_sub = Expr::sub(x.clone(), Expr::sub(y.clone(), z.clone()));
    assert_eq!(to_latex(&e_sub_sub), r"x - \left( y - z \right)");

    // Subtraction: (x - y) - z (no parens because left-associative)
    let e_sub_left = Expr::sub(Expr::sub(x.clone(), y.clone()), z.clone());
    assert_eq!(to_latex(&e_sub_left), "x - y - z");

    // Multiplication over addition: (x + y) * z
    let e_mul_add = Expr::mul(Expr::add(x.clone(), y.clone()), z.clone());
    assert_eq!(to_latex(&e_mul_add), r"\left( x + y \right) \cdot z");

    // Addition over multiplication: x * y + z (no parens)
    let e_add_mul = Expr::add(Expr::mul(x.clone(), y.clone()), z.clone());
    assert_eq!(to_latex(&e_add_mul), r"x \cdot y + z");
}

#[test]
fn test_precedence_powers_and_fractions() {
    let x = Expr::var("x");
    let y = Expr::var("y");

    // Power with compound base: (x + 1)^2
    let p_add = Expr::pow(Expr::add(x.clone(), Expr::int(1)), Expr::int(2));
    assert_eq!(to_latex(&p_add), r"{\left( x + 1 \right)}^{2}");

    // Power with product base: (x * y)^2
    let p_mul = Expr::pow(Expr::mul(x.clone(), y.clone()), Expr::int(2));
    assert_eq!(to_latex(&p_mul), r"{\left( x \cdot y \right)}^{2}");

    // Power with fraction base: (1/2)^3
    let p_frac = Expr::pow(Expr::frac(1, 2), Expr::int(3));
    assert_eq!(to_latex(&p_frac), r"{\left( \frac{1}{2} \right)}^{3}");

    // Power with negative base: (-2)^2
    let p_neg = Expr::pow(Expr::int(-2), Expr::int(2));
    assert_eq!(to_latex(&p_neg), r"{\left( -2 \right)}^{2}");

    // Power with simple variable base: x^2 (no parens)
    let p_simple = Expr::pow(x.clone(), Expr::int(2));
    assert_eq!(to_latex(&p_simple), r"{x}^{2}");

    // Power inside fraction: (x^2 + 1) / (y^3 - 1)
    let frac = Expr::div(
        Expr::add(p_simple, Expr::int(1)),
        Expr::sub(Expr::pow(y.clone(), Expr::int(3)), Expr::int(1)),
    );
    assert_eq!(to_latex(&frac), r"\frac{{x}^{2} + 1}{{y}^{3} - 1}");
}

#[test]
fn test_greek_letters_and_subscripts() {
    let alpha = Expr::var("alpha");
    let beta = Expr::var("beta");
    let theta_1 = Expr::var("theta_1");
    let c1 = Expr::var("_C1");
    let c2 = Expr::var("_C2");
    let x_0 = Expr::var("x_0");

    assert_eq!(to_latex(&alpha), r"\alpha");
    assert_eq!(to_latex(&beta), r"\beta");
    assert_eq!(to_latex(&theta_1), r"\theta_{1}");
    assert_eq!(to_latex(&c1), r"C_{1}");
    assert_eq!(to_latex(&c2), r"C_{2}");
    assert_eq!(to_latex(&x_0), r"x_{0}");

    // Formula: alpha * x_0 + beta
    let expr = Expr::add(Expr::mul(alpha, x_0), beta);
    assert_eq!(to_latex(&expr), r"\alpha \cdot x_{0} + \beta");
}

#[test]
fn test_matrices_and_calculus() {
    // 2x2 Matrix
    let mat = Expr::Matrix(vec![
        vec![Expr::int(1), Expr::var("x")],
        vec![Expr::var("y"), Expr::int(2)],
    ]);
    assert_eq!(
        to_latex(&mat),
        r"\begin{pmatrix} 1 & x \\ y & 2 \end{pmatrix}"
    );

    // Summation: sum_{i=1}^{n} (i^2 + 1)
    let summ = Expr::Sum(
        Box::new(Expr::add(
            Expr::pow(Expr::var("i"), Expr::int(2)),
            Expr::int(1),
        )),
        Box::new(Expr::var("i")),
        Box::new(Expr::int(1)),
        Box::new(Expr::var("n")),
    );
    assert_eq!(
        to_latex(&summ),
        r"\sum_{i=1}^{n} \left( {i}^{2} + 1 \right)"
    );

    // Integral: int (x^2 + sin(x)) dx
    let intg = Expr::Integral(
        Box::new(Expr::add(
            Expr::pow(Expr::var("x"), Expr::int(2)),
            Expr::sin(Expr::var("x")),
        )),
        Box::new(Expr::var("x")),
    );
    assert_eq!(
        to_latex(&intg),
        r"\int \left( {x}^{2} + \sin\left( x \right) \right) \, dx"
    );
}

#[test]
fn test_parse_latex_roundtrip() {
    let cases = [
        Expr::add(Expr::var("x"), Expr::int(1)),
        Expr::sub(Expr::var("x"), Expr::var("y")),
        Expr::mul(Expr::int(2), Expr::var("x")),
        Expr::div(Expr::add(Expr::var("x"), Expr::int(1)), Expr::int(2)),
        Expr::add(Expr::var("alpha"), Expr::var("beta")),
        Expr::Matrix(vec![
            vec![Expr::int(1), Expr::var("x")],
            vec![Expr::var("y"), Expr::int(2)],
        ]),
    ];

    for expr in &cases {
        let latex_str = to_latex(expr);
        let parsed = parse_latex(&latex_str)
            .unwrap_or_else(|e| panic!("Failed to parse LaTeX '{latex_str}': {e:?}"));
        assert_eq!(&parsed, expr, "Mismatch for LaTeX '{latex_str}'");
    }
}
