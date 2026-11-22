//! Golden tests: matrices must survive saturation/extraction losslessly.

use maxima_rs::prelude::*;
use sym_core::saturate::SaturationConfig;

#[test]
fn test_matrix_survives_optimization() {
    let mat = Expr::matrix(vec![
        vec![Expr::var("a"), Expr::var("b")],
        vec![Expr::var("c"), Expr::var("d")],
    ]);
    let out = optimize(&mat, &SaturationConfig::default());
    assert_eq!(out, mat);
}

#[test]
fn test_matrix_with_expressions_survives_optimization() {
    let mat = Expr::matrix(vec![
        vec![Expr::sin(Expr::var("x")), Expr::frac(-1, 2)],
        vec![Expr::pi(), Expr::factorial(Expr::int(5))],
    ]);
    let out = optimize(&mat, &SaturationConfig::default());
    assert_eq!(out, mat);
}

#[test]
fn test_nested_matrix_survives_optimization() {
    let inner = Expr::matrix(vec![vec![Expr::int(1), Expr::int(2)]]);
    let outer = Expr::matrix(vec![vec![inner.clone()]]);
    let out = optimize(&outer, &SaturationConfig::default());
    assert_eq!(out, outer);
}
