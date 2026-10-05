//! Golden integration tests for sym_fn! and sym_eval! procedural macros.

use maxima_rs::prelude::*;

sym_fn! {
    pub fn simple_linear(x: f64) -> f64 {
        expr: x * 1.0 + 0.0,
    }
}

sym_fn! {
    pub fn rosenbrock_test(x: f64, y: f64) -> (f64, [f64; 2]) {
        expr: (1.0 - x).powi(2) + 100.0 * (y - x.powi(2)).powi(2),
        gradient: [x, y],
    }
}

#[test]
fn test_sym_fn_simple() {
    assert_eq!(simple_linear(5.0), 5.0);
    assert_eq!(simple_linear(-2.5), -2.5);
}

#[test]
fn test_sym_fn_rosenbrock() {
    let (val, grad) = rosenbrock_test(1.0, 1.0);
    assert_eq!(val, 0.0);
    assert_eq!(grad, [0.0, 0.0]);
}

#[test]
fn test_sym_eval_inline() {
    let x = 42.0f64;
    let res = sym_eval!(x * 1.0 + 0.0);
    assert_eq!(res, 42.0);
}
