//! Integration tests for Phase 40: Symbolic Linear Algebra & Bareiss Algorithm.
//!
//! Mirrors Maxima's `newdet.lisp` and ADR-0010.

use maxima_rs::prelude::*;
use sym_core::matrix::{
    Matrix, MatrixError, adjoint, charpoly, determinant, eigenvalues, invert, trace,
};

#[test]
fn test_matrix_construction_and_validation() {
    // Valid 2x2 matrix
    let m = Matrix::new(vec![
        vec![Expr::int(1), Expr::int(2)],
        vec![Expr::int(3), Expr::int(4)],
    ])
    .expect("valid matrix");
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
    assert!(m.is_square());

    // Ragged matrix rejection
    let ragged = Matrix::new(vec![vec![Expr::int(1), Expr::int(2)], vec![Expr::int(3)]]);
    assert!(matches!(
        ragged,
        Err(MatrixError::RaggedMatrix {
            row: 1,
            len: 1,
            expected: 2
        })
    ));
}

#[test]
fn test_matrix_arithmetic() {
    let a = Matrix::new(vec![
        vec![Expr::int(1), Expr::int(2)],
        vec![Expr::int(3), Expr::int(4)],
    ])
    .unwrap();

    let b = Matrix::new(vec![
        vec![Expr::int(5), Expr::int(6)],
        vec![Expr::int(7), Expr::int(8)],
    ])
    .unwrap();

    // A + B
    let sum = a.add(&b).unwrap();
    assert_eq!(sum.data[0][0], Expr::int(6));
    assert_eq!(sum.data[0][1], Expr::int(8));
    assert_eq!(sum.data[1][0], Expr::int(10));
    assert_eq!(sum.data[1][1], Expr::int(12));

    // A - B
    let diff = a.sub(&b).unwrap();
    assert_eq!(diff.data[0][0], Expr::int(-4));

    // A * B
    // [1 2] * [5 6] = [1*5+2*7  1*6+2*8] = [19 22]
    // [3 4]   [7 8]   [3*5+4*7  3*6+4*8]   [43 50]
    let prod = a.mul(&b).unwrap();
    assert_eq!(prod.data[0][0], Expr::int(19));
    assert_eq!(prod.data[0][1], Expr::int(22));
    assert_eq!(prod.data[1][0], Expr::int(43));
    assert_eq!(prod.data[1][1], Expr::int(50));

    // Transpose
    let at = a.transpose();
    assert_eq!(at.data[0][1], Expr::int(3));
    assert_eq!(at.data[1][0], Expr::int(2));
}

#[test]
fn test_bareiss_determinant_numeric() {
    // 1x1
    let m1 = Matrix::new(vec![vec![Expr::int(42)]]).unwrap();
    assert_eq!(determinant(&m1).unwrap(), Expr::int(42));

    // 2x2
    let m2 = Matrix::new(vec![
        vec![Expr::int(3), Expr::int(8)],
        vec![Expr::int(4), Expr::int(6)],
    ])
    .unwrap();
    // 3*6 - 8*4 = 18 - 32 = -14
    assert_eq!(determinant(&m2).unwrap(), Expr::int(-14));

    // 3x3 tridiagonal Toeplitz
    // [ 2 -1  0]
    // [-1  2 -1]
    // [ 0 -1  2]
    // det = 2*(4-1) - (-1)*(-2-0) = 6 - 2 = 4
    let m3 = Matrix::new(vec![
        vec![Expr::int(2), Expr::int(-1), Expr::int(0)],
        vec![Expr::int(-1), Expr::int(2), Expr::int(-1)],
        vec![Expr::int(0), Expr::int(-1), Expr::int(2)],
    ])
    .unwrap();
    assert_eq!(determinant(&m3).unwrap(), Expr::int(4));

    // 3x3 singular (rows linearly dependent)
    let m3_sing = Matrix::new(vec![
        vec![Expr::int(1), Expr::int(2), Expr::int(3)],
        vec![Expr::int(4), Expr::int(5), Expr::int(6)],
        vec![Expr::int(7), Expr::int(8), Expr::int(9)],
    ])
    .unwrap();
    assert_eq!(determinant(&m3_sing).unwrap(), Expr::int(0));
}

#[test]
fn test_bareiss_determinant_pivoting_zero_diagonal() {
    // Matrix starting with 0 on diagonal requires row exchange
    // [0 1]
    // [1 0]
    // det = -1
    let m = Matrix::new(vec![
        vec![Expr::int(0), Expr::int(1)],
        vec![Expr::int(1), Expr::int(0)],
    ])
    .unwrap();
    assert_eq!(determinant(&m).unwrap(), Expr::int(-1));

    // 3x3 with 0 at (0,0)
    // [0 1 2]
    // [1 0 3]
    // [4 -3 8]
    // det = -1*(8 - 12) + 2*(-3 - 0) = 4 - 6 = -2
    let m3 = Matrix::new(vec![
        vec![Expr::int(0), Expr::int(1), Expr::int(2)],
        vec![Expr::int(1), Expr::int(0), Expr::int(3)],
        vec![Expr::int(4), Expr::int(-3), Expr::int(8)],
    ])
    .unwrap();
    assert_eq!(determinant(&m3).unwrap(), Expr::int(-2));
}

#[test]
fn test_bareiss_determinant_symbolic() {
    // 2x2 with symbolic variable x:
    // [x 1]
    // [2 x]
    // det = x^2 - 2
    let x = Expr::var("x");
    let m = Matrix::new(vec![
        vec![x.clone(), Expr::int(1)],
        vec![Expr::int(2), x.clone()],
    ])
    .unwrap();
    let det = determinant(&m).unwrap();
    let expected = Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(2));
    let diff = expand(&Expr::sub(det, expected));
    assert_eq!(diff, Expr::int(0));

    // 3x3 circulant with x:
    // [x 1 0]
    // [0 x 1]
    // [1 0 x]
    // det = x*(x^2) - 1*(-1) = x^3 - 1 (or -1 + x^3)
    let m3 = Matrix::new(vec![
        vec![x.clone(), Expr::int(1), Expr::int(0)],
        vec![Expr::int(0), x.clone(), Expr::int(1)],
        vec![Expr::int(1), Expr::int(0), x.clone()],
    ])
    .unwrap();
    let det3 = determinant(&m3).unwrap();
    let diff = expand(&Expr::sub(
        det3,
        Expr::add(Expr::pow(x, Expr::int(3)), Expr::int(1)),
    ));
    assert_eq!(diff, Expr::int(0));
}

#[test]
fn test_matrix_trace() {
    let m = Matrix::new(vec![
        vec![Expr::int(5), Expr::int(1), Expr::int(2)],
        vec![Expr::int(3), Expr::int(7), Expr::int(4)],
        vec![Expr::int(6), Expr::int(8), Expr::int(9)],
    ])
    .unwrap();
    assert_eq!(trace(&m).unwrap(), Expr::int(21));

    let x = Expr::var("x");
    let y = Expr::var("y");
    let m_sym = Matrix::new(vec![
        vec![x.clone(), Expr::int(1)],
        vec![Expr::int(2), y.clone()],
    ])
    .unwrap();
    let tr = trace(&m_sym).unwrap();
    assert_eq!(tr, Expr::add(x, y));
}

#[test]
fn test_matrix_charpoly() {
    // M = [2 1]
    //     [1 2]
    // charpoly(M, lambda) = det(M - lambda*I) = (2 - lambda)^2 - 1 = lambda^2 - 4*lambda + 3
    let m = Matrix::new(vec![
        vec![Expr::int(2), Expr::int(1)],
        vec![Expr::int(1), Expr::int(2)],
    ])
    .unwrap();
    let cp = charpoly(&m, "L").unwrap();
    let l = Expr::var("L");
    let expected = expand(&Expr::sub(
        Expr::pow(Expr::sub(Expr::int(2), l.clone()), Expr::int(2)),
        Expr::int(1),
    ));
    let diff = expand(&Expr::sub(cp, expected));
    assert_eq!(diff, Expr::int(0));
}

#[test]
fn test_matrix_adjoint_and_inverse() {
    // 2x2 matrix:
    // [4 7]
    // [2 6]
    // det = 4*6 - 7*2 = 24 - 14 = 10
    // adj = [ 6 -7]
    //       [-2  4]
    // inv = [ 3/5 -7/10]
    //       [-1/5   2/5]
    let m = Matrix::new(vec![
        vec![Expr::int(4), Expr::int(7)],
        vec![Expr::int(2), Expr::int(6)],
    ])
    .unwrap();

    let adj = adjoint(&m).unwrap();
    assert_eq!(adj.data[0][0], Expr::int(6));
    assert_eq!(adj.data[0][1], Expr::int(-7));
    assert_eq!(adj.data[1][0], Expr::int(-2));
    assert_eq!(adj.data[1][1], Expr::int(4));

    let inv = invert(&m).unwrap();
    assert_eq!(inv.data[0][0], Expr::frac(3, 5));
    assert_eq!(inv.data[0][1], Expr::frac(-7, 10));
    assert_eq!(inv.data[1][0], Expr::frac(-1, 5));
    assert_eq!(inv.data[1][1], Expr::frac(2, 5));

    // Check M * M^(-1) == I
    let identity_check = m.mul(&inv).unwrap();
    assert_eq!(identity_check.data[0][0], Expr::int(1));
    assert_eq!(identity_check.data[0][1], Expr::int(0));
    assert_eq!(identity_check.data[1][0], Expr::int(0));
    assert_eq!(identity_check.data[1][1], Expr::int(1));

    // Singular matrix should fail inversion
    let sing = Matrix::new(vec![
        vec![Expr::int(1), Expr::int(2)],
        vec![Expr::int(2), Expr::int(4)],
    ])
    .unwrap();
    assert!(matches!(invert(&sing), Err(MatrixError::SingularMatrix)));
}

#[test]
fn test_matrix_eigenvalues() {
    // M = [2 1]
    //     [1 2]
    // charpoly = (2-L)^2 - 1 = L^2 - 4L + 3 = (L - 1)(L - 3) = 0 => L = 1, 3
    let m = Matrix::new(vec![
        vec![Expr::int(2), Expr::int(1)],
        vec![Expr::int(1), Expr::int(2)],
    ])
    .unwrap();
    let evs = eigenvalues(&m).unwrap();
    assert!(evs.contains(&Expr::int(1)));
    assert!(evs.contains(&Expr::int(3)));
}

#[test]
fn test_matrix_web_service_dispatch() {
    use maxima_web::api::ComputeRequest;
    use maxima_web::service::compute;
    use std::collections::HashMap;

    // Test determinant via web compute
    let req = ComputeRequest {
        operation: "determinant".into(),
        expression: "matrix([1, 2], [3, 4])".into(),
        input_format: "maxima".into(),
        variable: String::new(),
        variables: Vec::new(),
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
    };
    let resp = compute(&req);
    assert!(resp.success);
    assert_eq!(resp.result, "(-2)");

    // Test trace via web compute
    let req_tr = ComputeRequest {
        operation: "trace".into(),
        expression: "matrix([1, 2], [3, 4])".into(),
        input_format: "maxima".into(),
        variable: String::new(),
        variables: Vec::new(),
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
    };
    let resp_tr = compute(&req_tr);
    assert!(resp_tr.success);
    assert_eq!(resp_tr.result, "5");
}

#[test]
fn test_phase50_multivar_3x3_bareiss_complex_eigenvalues_and_decode_guard() {
    use sym_core::matrix::decode_matrix_symbol;

    let x = Expr::var("x");
    let y = Expr::var("y");
    let m3 = Matrix::new(vec![
        vec![x.clone(), y.clone(), Expr::int(0)],
        vec![Expr::int(1), x.clone(), y.clone()],
        vec![Expr::int(0), Expr::int(1), x.clone()],
    ])
    .unwrap();
    let det = determinant(&m3).unwrap();
    let adj = adjoint(&m3).unwrap();
    let prod = m3.mul(&adj).unwrap();
    // (M * adj(M))[0][0] == det(M)
    let mut env = std::collections::HashMap::new();
    env.insert("x".to_string(), 2.5);
    env.insert("y".to_string(), -1.5);
    assert!((prod[(0, 0)].eval(&env).unwrap() - det.eval(&env).unwrap()).abs() < 1e-9);

    // Complex eigenvalues of 90-deg rotation [[0, -1], [1, 0]]: ±%i
    let rot = Matrix::new(vec![
        vec![Expr::int(0), Expr::int(-1)],
        vec![Expr::int(1), Expr::int(0)],
    ])
    .unwrap();
    let evs = eigenvalues(&rot).unwrap();
    assert_eq!(evs.len(), 2);

    // den == 0 guard in decode_matrix_symbol
    assert!(decode_matrix_symbol("MX 1 1 C 1/0").is_none());
}
