//! Integration tests for Phase 41: Complex Analysis, Rectangular Form & Residues.
//!
//! Validates:
//! - Exact arithmetic of imaginary unit `Expr::I` and cyclic powers $i^k$
//! - Rectangular decomposition `rectform(expr) = (u, v)` citing Maxima 5.50 `rpart.lisp`
//! - Euler's identity $\exp(i\pi) = -1$ and trigonometric/hyperbolic decompositions
//! - Complex logarithm branch cut $\ln(-1) = i\pi$
//! - Modulus `cabs`, argument `carg`, `polarform`, and `conjugate`
//! - Laurent series residue calculation $\operatorname{Res}(f(z), z=z_0)$
//! - Web compute API integration (`rectform`, `polarform`, `realpart`, `imagpart`, `conjugate`, `carg`, `cabs`, `residue`)

use maxima_rs::prelude::*;
use sym_core::complex::{
    cabs, carg, conjugate, imagpart, polarform, polarform_expr, realpart, rectform, rectform_expr,
    residue,
};

#[test]
fn test_i_powers() {
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(0))),
        Expr::int(1)
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(1))),
        Expr::i()
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(2))),
        Expr::int(-1)
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(3))),
        Expr::neg(Expr::i())
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(4))),
        Expr::int(1)
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(-1))),
        Expr::neg(Expr::i())
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(-2))),
        Expr::int(-1)
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(-3))),
        Expr::i()
    );
    assert_eq!(
        rectform_expr(&Expr::pow(Expr::i(), Expr::int(-4))),
        Expr::int(1)
    );
}

#[test]
fn test_rectform_arithmetic() {
    // (3 + 4i) + (1 - 2i) = 4 + 2i
    let z1 = Expr::add(Expr::int(3), Expr::mul(Expr::int(4), Expr::i()));
    let z2 = Expr::sub(Expr::int(1), Expr::mul(Expr::int(2), Expr::i()));
    let sum = Expr::add(z1.clone(), z2.clone());
    let (u_sum, v_sum) = rectform(&sum);
    assert_eq!(u_sum, Expr::int(4));
    assert_eq!(v_sum, Expr::int(2));

    // (3 + 4i) - (1 - 2i) = 2 + 6i
    let diff = Expr::sub(z1.clone(), z2.clone());
    let (u_diff, v_diff) = rectform(&diff);
    assert_eq!(u_diff, Expr::int(2));
    assert_eq!(v_diff, Expr::int(6));

    // (3 + 4i)(1 - 2i) = 3 - 6i + 4i - 8i^2 = 11 - 2i
    let prod = Expr::mul(z1.clone(), z2.clone());
    let (u_prod, v_prod) = rectform(&prod);
    assert_eq!(u_prod, Expr::int(11));
    assert_eq!(v_prod, Expr::int(-2));

    // (1 + i) / (1 - i) = (1+i)^2 / 2 = 2i / 2 = i
    let num = Expr::add(Expr::int(1), Expr::i());
    let den = Expr::sub(Expr::int(1), Expr::i());
    let quot = Expr::div(num, den);
    let (u_quot, v_quot) = rectform(&quot);
    assert_eq!(u_quot, Expr::int(0));
    assert_eq!(v_quot, Expr::int(1));
}

#[test]
fn test_euler_identity() {
    // exp(i * pi) = -1
    let e_ipi = Expr::exp(Expr::mul(Expr::i(), Expr::pi()));
    assert_eq!(rectform_expr(&e_ipi), Expr::int(-1));

    // exp(i * pi / 2) = i
    let e_ipi2 = Expr::exp(Expr::mul(Expr::i(), Expr::div(Expr::pi(), Expr::int(2))));
    assert_eq!(rectform_expr(&e_ipi2), Expr::i());

    // exp(-i * pi / 2) = -i
    let e_neg_ipi2 = Expr::exp(Expr::mul(
        Expr::neg(Expr::i()),
        Expr::div(Expr::pi(), Expr::int(2)),
    ));
    assert_eq!(rectform_expr(&e_neg_ipi2), Expr::neg(Expr::i()));

    // exp(2 * i * pi) = 1
    let e_2ipi = Expr::exp(Expr::mul(Expr::mul(Expr::int(2), Expr::i()), Expr::pi()));
    assert_eq!(rectform_expr(&e_2ipi), Expr::int(1));
}

#[test]
fn test_complex_logarithm() {
    // ln(-1) = i * pi
    let ln_neg1 = Expr::ln(Expr::int(-1));
    let (u, v) = rectform(&ln_neg1);
    assert_eq!(u, Expr::int(0));
    assert_eq!(v, Expr::pi());

    // ln(i) = i * pi / 2
    let ln_i = Expr::ln(Expr::i());
    let (u, v) = rectform(&ln_i);
    assert_eq!(u, Expr::int(0));
    assert_eq!(v, Expr::frac(1, 2) * Expr::pi());
}

#[test]
fn test_trig_and_hyperbolic_identities() {
    let x = Expr::var("x");

    // sin(i * x) = i * sinh(x) => real = 0, imag = sinh(x)
    let sin_ix = Expr::sin(Expr::mul(Expr::i(), x.clone()));
    let (u_sin, v_sin) = rectform(&sin_ix);
    assert_eq!(u_sin, Expr::int(0));
    assert_eq!(v_sin, Expr::sinh(x.clone()));

    // cos(i * x) = cosh(x) => real = cosh(x), imag = 0
    let cos_ix = Expr::cos(Expr::mul(Expr::i(), x.clone()));
    let (u_cos, v_cos) = rectform(&cos_ix);
    assert_eq!(u_cos, Expr::cosh(x.clone()));
    assert_eq!(v_cos, Expr::int(0));

    // sinh(i * x) = i * sin(x) => real = 0, imag = sin(x)
    let sinh_ix = Expr::sinh(Expr::mul(Expr::i(), x.clone()));
    let (u_sh, v_sh) = rectform(&sinh_ix);
    assert_eq!(u_sh, Expr::int(0));
    assert_eq!(v_sh, Expr::sin(x.clone()));

    // cosh(i * x) = cos(x) => real = cos(x), imag = 0
    let cosh_ix = Expr::cosh(Expr::mul(Expr::i(), x));
    let (u_ch, v_ch) = rectform(&cosh_ix);
    assert_eq!(u_ch, Expr::cos(Expr::var("x")));
    assert_eq!(v_ch, Expr::int(0));
}

#[test]
fn test_cabs_and_carg() {
    // |3 + 4i| = 5
    let z = Expr::add(Expr::int(3), Expr::mul(Expr::int(4), Expr::i()));
    assert_eq!(cabs(&z), Expr::int(5));

    // |-5| = 5
    assert_eq!(cabs(&Expr::int(-5)), Expr::int(5));

    // |5i| = 5
    assert_eq!(cabs(&Expr::mul(Expr::int(5), Expr::i())), Expr::int(5));

    // carg(5) = 0
    assert_eq!(carg(&Expr::int(5)), Expr::int(0));

    // carg(-5) = pi
    assert_eq!(carg(&Expr::int(-5)), Expr::pi());

    // carg(5i) = pi / 2
    assert_eq!(
        carg(&Expr::mul(Expr::int(5), Expr::i())),
        Expr::frac(1, 2) * Expr::pi()
    );

    // carg(-5i) = -pi / 2
    assert_eq!(
        carg(&Expr::mul(Expr::int(-5), Expr::i())),
        Expr::neg(Expr::frac(1, 2) * Expr::pi())
    );
}

#[test]
fn test_conjugate_and_polarform() {
    // realpart and imagpart
    let z_test = Expr::add(Expr::int(3), Expr::mul(Expr::int(4), Expr::i()));
    assert_eq!(realpart(&z_test), Expr::int(3));
    assert_eq!(imagpart(&z_test), Expr::int(4));

    // conjugate(3 + 4i) = 3 - 4i
    let z = Expr::add(Expr::int(3), Expr::mul(Expr::int(4), Expr::i()));
    let conj = conjugate(&z);
    let (u, v) = rectform(&conj);
    assert_eq!(u, Expr::int(3));
    assert_eq!(v, Expr::int(-4));

    // conjugate(i) = -i
    assert_eq!(conjugate(&Expr::i()), Expr::neg(Expr::i()));

    // conjugate(42) = 42
    assert_eq!(conjugate(&Expr::int(42)), Expr::int(42));

    // polarform(i) => (r, theta) = (1, pi/2)
    let (r_i, th_i) = polarform(&Expr::i());
    assert_eq!(r_i, Expr::int(1));
    assert_eq!(th_i, Expr::frac(1, 2) * Expr::pi());

    // polarform_expr(i) => exp(i * pi / 2)
    let p = polarform_expr(&Expr::i());
    assert_eq!(
        p,
        Expr::exp(Expr::mul(Expr::i(), Expr::frac(1, 2) * Expr::pi()))
    );
}

#[test]
fn test_laurent_residues() {
    let z = Expr::var("z");

    // Res(1/z, z=0) = 1
    let f1 = Expr::div(Expr::int(1), z.clone());
    let r1 = residue(&f1, "z", &Expr::int(0)).unwrap();
    assert_eq!(r1, Expr::int(1));

    // Res(1/z^2, z=0) = 0
    let f2 = Expr::div(Expr::int(1), Expr::pow(z.clone(), Expr::int(2)));
    let r2 = residue(&f2, "z", &Expr::int(0)).unwrap();
    assert_eq!(r2, Expr::int(0));

    // Res(exp(z)/z^3, z=0) = 1/2
    let f3 = Expr::div(Expr::exp(z.clone()), Expr::pow(z.clone(), Expr::int(3)));
    let r3 = residue(&f3, "z", &Expr::int(0)).unwrap();
    assert_eq!(r3, Expr::frac(1, 2));

    // Res(1/(z-3), z=3) = 1
    let f4 = Expr::div(Expr::int(1), Expr::sub(z, Expr::int(3)));
    let r4 = residue(&f4, "z", &Expr::int(3)).unwrap();
    assert_eq!(r4, Expr::int(1));
}

#[test]
fn test_maxima_parser_complex() {
    // Parse %i
    let parsed_i = parse_maxima("%i").unwrap();
    assert_eq!(parsed_i, Expr::i());

    // Parse 3 + 4*%i
    let parsed_z = parse_maxima("3 + 4 * %i").unwrap();
    assert_eq!(
        parsed_z,
        Expr::add(Expr::int(3), Expr::mul(Expr::int(4), Expr::i()))
    );

    // Parse exp(%i * %pi)
    let parsed_e = parse_maxima("exp(%i * %pi)").unwrap();
    assert_eq!(rectform_expr(&parsed_e), Expr::int(-1));
}

#[test]
fn test_complex_web_service_dispatch() {
    use maxima_web::api::ComputeRequest;
    use maxima_web::service::compute;
    use std::collections::HashMap;

    // 1. rectform via web compute
    let req = ComputeRequest {
        operation: "rectform".into(),
        expression: "(1 + %i) / (1 - %i)".into(),
        input_format: "maxima".into(),
        variable: String::new(),
        variables: Vec::new(),
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
        session_context: None,
    };
    let resp = compute(&req);
    assert!(resp.success);
    assert_eq!(resp.result, "%i");

    // 2. realpart via web compute
    let req_re = ComputeRequest {
        operation: "realpart".into(),
        expression: "3 + 4*%i".into(),
        input_format: "maxima".into(),
        variable: String::new(),
        variables: Vec::new(),
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
        session_context: None,
    };
    let resp_re = compute(&req_re);
    assert!(resp_re.success);
    assert_eq!(resp_re.result, "3");

    // 3. imagpart via web compute
    let req_im = ComputeRequest {
        operation: "imagpart".into(),
        expression: "3 + 4*%i".into(),
        input_format: "maxima".into(),
        variable: String::new(),
        variables: Vec::new(),
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
        session_context: None,
    };
    let resp_im = compute(&req_im);
    assert!(resp_im.success);
    assert_eq!(resp_im.result, "4");

    // 4. cabs via web compute
    let req_abs = ComputeRequest {
        operation: "cabs".into(),
        expression: "3 + 4*%i".into(),
        input_format: "maxima".into(),
        variable: String::new(),
        variables: Vec::new(),
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
        session_context: None,
    };
    let resp_abs = compute(&req_abs);
    assert!(resp_abs.success);
    assert_eq!(resp_abs.result, "5");

    // 5. residue via web compute: Res(exp(z)/z^3, z=0) = 1/2
    let mut params_res = HashMap::new();
    params_res.insert("variable".into(), "z".into());
    params_res.insert("point".into(), "0".into());
    let req_res = ComputeRequest {
        operation: "residue".into(),
        expression: "exp(z)/z^3".into(),
        input_format: "maxima".into(),
        variable: "z".into(),
        variables: vec!["z".into()],
        float_type: "f64".into(),
        fast_math: true,
        env: HashMap::new(),
        params: params_res,
        session_context: None,
    };
    let resp_res = compute(&req_res);
    assert!(resp_res.success);
    assert_eq!(resp_res.result, "1/2");
}

#[test]
fn test_phase48_carg_quadrants_cabs_and_rectform() {
    use std::collections::HashMap;
    let empty = HashMap::new();

    // All 4 quadrants of carg(±1 ± %i)
    let q1 = carg(&Expr::add(Expr::int(1), Expr::i()));
    let q2 = carg(&Expr::add(Expr::int(-1), Expr::i()));
    let q3 = carg(&Expr::sub(Expr::int(-1), Expr::i()));
    let q4 = carg(&Expr::sub(Expr::int(1), Expr::i()));

    let pi = std::f64::consts::PI;
    assert!((q1.eval(&empty).expect("q1") - pi / 4.0).abs() < 1e-12);
    assert!((q2.eval(&empty).expect("q2") - 3.0 * pi / 4.0).abs() < 1e-12);
    assert!((q3.eval(&empty).expect("q3") + 3.0 * pi / 4.0).abs() < 1e-12);
    assert!((q4.eval(&empty).expect("q4") + pi / 4.0).abs() < 1e-12);

    // cabs(x) preserves abs(x) for real symbolic x
    let x = Expr::var("x");
    assert_eq!(cabs(&x), Expr::abs(x));

    // rectform(2^%i) = cos(ln(2)) + %i * sin(ln(2))
    let (u_pow, v_pow) = rectform(&Expr::pow(Expr::int(2), Expr::i()));
    let ln2 = 2.0_f64.ln();
    assert!((u_pow.eval(&empty).expect("u_pow") - ln2.cos()).abs() < 1e-12);
    assert!((v_pow.eval(&empty).expect("v_pow") - ln2.sin()).abs() < 1e-12);

    // rectform(atan(1 + %i)): Re = (pi/2 - atan2(0, 1 - 2) ...), verify against analytic atan(1+i)
    // atan(z) = (i/2) * ln((1 - iz)/(1 + iz))
    // For z = 1 + i: Re(atan(1+i)) = 1/2 * atan2(2, -1) = 1/2 * (pi - atan(2)) ≈ 1.0172219678978514
    // Im(atan(1+i)) = 1/4 * ln((1 + 1 + 2)/(1 + 1 - 2)?) -> actually 1/4 * ln(((1)^2 + (1+1)^2) / ((1)^2 + (1-1)^2)) = 1/4 * ln(5) ≈ 0.40235947810852507
    let (u_atan, v_atan) = rectform(&Expr::atan(Expr::add(Expr::int(1), Expr::i())));
    let expected_u = 0.5 * (pi - 2.0_f64.atan());
    let expected_v = 0.25 * 5.0_f64.ln();
    assert!((u_atan.eval(&empty).expect("u_atan") - expected_u).abs() < 1e-12);
    assert!((v_atan.eval(&empty).expect("v_atan") - expected_v).abs() < 1e-12);
}
