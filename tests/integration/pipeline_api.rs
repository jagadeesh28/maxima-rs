//! Golden integration tests for high-level Pipeline builder and API ergonomics.

use maxima_rs::prelude::*;
use sym_core::saturate::SaturationConfig;

#[test]
fn test_fluent_pipeline_latex_differentiation_optimize() {
    let pipeline = Pipeline::new()
        .from_latex(r"\frac{\sin(x)^2 + \cos(x)^2}{x}")
        .expect("valid LaTeX")
        .optimize();

    // (sin(x)^2 + cos(x)^2) / x -> 1 / x
    assert_eq!(
        pipeline.expressions(),
        &[Expr::div(Expr::int(1), Expr::var("x"))]
    );
}

#[test]
fn test_fluent_pipeline_maxima_differentiate() {
    let pipeline = Pipeline::new()
        .from_maxima("x^2 + 3*x")
        .expect("valid Maxima")
        .differentiate(&["x"])
        .optimize();

    assert_eq!(pipeline.expressions().len(), 2);
    // original expr: x^2 + 3*x
    // derivative: 2*x + 3
    let diff_str = format!("{}", pipeline.expressions()[1]);
    assert!(diff_str.contains('x'));
}

#[test]
fn test_optimize_gradient_free_function() {
    let expr = Expr::pow(Expr::var("x"), Expr::int(2));
    let (opt_val, opt_grad) = optimize_gradient(&expr, &["x"], &SaturationConfig::default());

    assert_eq!(opt_val, Expr::pow(Expr::var("x"), Expr::int(2)));
    assert_eq!(opt_grad.len(), 1);
    let grad_str = format!("{}", opt_grad[0]);
    assert!(grad_str.contains('x'));
}

#[test]
fn test_pipeline_to_rust_fn_codegen() {
    let pipeline = Pipeline::new()
        .with_expr(Expr::add(Expr::var("x"), Expr::int(0)))
        .optimize();

    let ts = pipeline.to_rust_fn("eval_x", FloatType::F64);
    let s = ts.to_string();
    assert!(s.contains("pub fn eval_x (x : f64) -> f64"), "got: {s}");
}
