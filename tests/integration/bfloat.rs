//! Integration tests for Phase 39: Pure-Rust Arbitrary-Precision Float (`bfloat`).
//!
//! Mirrors Maxima's `float.lisp` `bfloat` and `fpprec` precision control,
//! backed by pure-Rust [`dashu_float::DBig`].

use maxima_rs::prelude::*;
use sym_core::bfloat::{BfloatContext, BfloatWrapper, DEFAULT_FPPREC, bfloat, bfloat_from_str};

#[test]
fn test_bfloat_parse_basic() {
    let bf = BfloatWrapper::parse("1.2345678901234567890123456789", 32).expect("parse failed");
    assert_eq!(bf.prec, 32);
    assert!(!bf.is_zero());
    assert!(!bf.is_one());

    // Check machine f64 approximation is close
    let f = bf.to_f64();
    assert!((f - 1.2345678901234567).abs() < 1e-12);
}

#[test]
fn test_bfloat_parse_maxima_exponent_notation() {
    // Maxima uses 'b' or 'B' for bigfloat exponent indicator (e.g. 1.25b-3)
    let bf1 = BfloatWrapper::parse("1.25b-3", 16).expect("parse b failed");
    let bf2 = BfloatWrapper::parse("1.25B-3", 16).expect("parse B failed");
    let bf3 = BfloatWrapper::parse("0.00125", 16).expect("parse decimal failed");

    assert_eq!(bf1, bf2);
    assert_eq!(bf1, bf3);
}

#[test]
fn test_bfloat_from_rational() {
    let r = BigRational::new(1.into(), 3.into());
    let bf = BfloatWrapper::from_rational(&r, 16).expect("conversion failed");
    let s = bf.to_maxima_string();
    assert!(s.starts_with("3.333333333333333"), "got string: {s}");

    let r_neg = BigRational::new((-5).into(), 2.into());
    let bf_neg = BfloatWrapper::from_rational(&r_neg, 16).expect("neg conversion failed");
    assert_eq!(bf_neg.to_f64(), -2.5);
}

#[test]
fn test_bfloat_constants_precision() {
    let ctx16 = BfloatContext::new(16);
    let pi16 = ctx16.pi();
    assert_eq!(ctx16.fpprec, DEFAULT_FPPREC);
    assert!(format!("{pi16}").starts_with("3.141592653589793"));

    let ctx32 = BfloatContext::new(32);
    let pi32 = ctx32.pi();
    assert!(format!("{pi32}").starts_with("3.1415926535897932384626433832795"));

    let e32 = ctx32.e();
    assert!(format!("{e32}").starts_with("2.7182818284590452353602874713527"));
}

#[test]
fn test_bfloat_arithmetic() {
    let a = BfloatWrapper::parse("2.5", 20).unwrap();
    let b = BfloatWrapper::parse("4.0", 20).unwrap();

    let sum = a.add(&b);
    assert_eq!(sum.to_f64(), 6.5);

    let diff = b.sub(&a);
    assert_eq!(diff.to_f64(), 1.5);

    let prod = a.mul(&b);
    assert_eq!(prod.to_f64(), 10.0);

    let quot = b.div(&a).unwrap();
    assert_eq!(quot.to_f64(), 1.6);

    let neg = a.neg();
    assert_eq!(neg.to_f64(), -2.5);

    let sqrt_b = b.sqrt().unwrap();
    assert_eq!(sqrt_b.to_f64(), 2.0);
}

#[test]
fn test_bfloat_transcendentals() {
    let zero = BfloatWrapper::parse("0.0", 25).unwrap();
    assert_eq!(zero.sin().unwrap().to_f64(), 0.0);
    assert_eq!(zero.cos().unwrap().to_f64(), 1.0);
    assert_eq!(zero.tan().unwrap().to_f64(), 0.0);
    assert_eq!(zero.exp().to_f64(), 1.0);
    assert_eq!(zero.sinh().to_f64(), 0.0);
    assert_eq!(zero.cosh().to_f64(), 1.0);

    let one = BfloatWrapper::parse("1.0", 25).unwrap();
    assert_eq!(one.ln().unwrap().to_f64(), 0.0);
}

#[test]
fn test_bfloat_expression_eval() {
    // bfloat(1/3)
    let expr = Expr::div(Expr::int(1), Expr::int(3));
    let bf_expr = bfloat(&expr, Some(16));
    if let Expr::Bfloat(bw) = bf_expr {
        assert!(bw.to_maxima_string().starts_with("3.333333333333333"));
    } else {
        panic!("expected Expr::Bfloat, got {:?}", bf_expr);
    }

    // bfloat(sin(0) + cos(0)) -> 1.0
    let expr2 = Expr::add(Expr::sin(Expr::int(0)), Expr::cos(Expr::int(0)));
    let bf_expr2 = bfloat(&expr2, Some(20));
    if let Expr::Bfloat(bw) = bf_expr2 {
        assert_eq!(bw.to_f64(), 1.0);
    } else {
        panic!("expected Expr::Bfloat, got {:?}", bf_expr2);
    }

    // bfloat with variable: x + 1/2 -> x + 0.5b0
    let expr3 = Expr::add(Expr::var("x"), Expr::frac(1, 2));
    let bf_expr3 = bfloat(&expr3, Some(16));
    match bf_expr3 {
        Expr::Add(lhs, rhs) => {
            assert_eq!(*lhs, Expr::var("x"));
            if let Expr::Bfloat(bw) = *rhs {
                assert_eq!(bw.to_f64(), 0.5);
            } else {
                panic!("expected Expr::Bfloat on rhs, got {:?}", rhs);
            }
        }
        other => panic!("expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_bfloat_string_and_latex_formatting() {
    let bf = BfloatWrapper::parse("1.25b3", 16).unwrap();
    let s = bf.to_maxima_string();
    assert!(s.contains('b'), "expected 'b' in maxima string, got {s}");

    let latex = bf.to_latex();
    assert!(latex.contains("10^{") || !latex.is_empty());
}

#[test]
fn test_bfloat_from_str_helper() {
    let expr = bfloat_from_str("1.7320508b0", Some(16)).expect("bfloat_from_str failed");
    if let Expr::Bfloat(bw) = expr {
        assert_eq!(bw.prec, 16);
        assert!((bw.to_f64() - 1.7320508).abs() < 1e-6);
    } else {
        panic!("expected Expr::Bfloat");
    }
}

#[test]
fn test_phase47_bfloat_edge_cases() {
    // BfloatContext::new(0) must clamp to >= 1 without panicking
    let ctx0 = BfloatContext::new(0);
    assert_eq!(ctx0.fpprec, 1);

    // sinh(1b-10) must not suffer catastrophic cancellation
    let small = BfloatWrapper::parse("1b-10", 20).unwrap();
    let s = small.sinh();
    let diff = (s.to_f64() - 1e-10).abs();
    assert!(
        diff < 1e-20,
        "sinh(1b-10) cancellation: got {}, diff={diff}",
        s.to_f64()
    );

    // 0^0 and 0^(-1) return Err
    let zero = BfloatWrapper::parse("0", 16).unwrap();
    let neg1 = BfloatWrapper::parse("-1", 16).unwrap();
    assert!(zero.pow(&zero).is_err());
    assert!(zero.pow(&neg1).is_err());
}
