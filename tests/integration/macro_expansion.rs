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

sym_fn! {
    #[fast_math = false]
    #[node_limit = 5000]
    #[time_limit_ms = 2000]
    pub fn strict_f32_neg_pow(x: f32) -> f32 {
        expr: (-x).powi(2) + (-x).cos(),
    }
}

sym_fn! {
    pub fn f32_grad_test(x: f32, y: f32) -> (f32, [f32; 2]) {
        expr: x.powi(2) + 2.0 * y,
        gradient: [x, y],
    }
}

#[test]
fn test_sym_fn_f32_strict_and_neg_precedence() {
    let v = strict_f32_neg_pow(3.0_f32);
    let expected = 9.0_f32 + (-3.0_f32).cos();
    assert!((v - expected).abs() < 1e-5, "got {v}, expected {expected}");

    let (val, grad) = f32_grad_test(3.0_f32, 4.0_f32);
    assert!((val - 17.0_f32).abs() < 1e-5);
    assert!((grad[0] - 6.0_f32).abs() < 1e-5);
    assert!((grad[1] - 2.0_f32).abs() < 1e-5);
}

#[test]
fn test_sym_eval_inline() {
    let x = 42.0f64;
    let res = sym_eval!(x * 1.0 + 0.0);
    assert_eq!(res, 42.0);
}
