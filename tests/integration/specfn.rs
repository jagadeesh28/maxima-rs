//! Integration test suite for Phase 42: Special Functions & Table Dispatch (`specfn`).
//!
//! Citations:
//! - Maxima 5.50: `src/specfn.lisp`, `src/bessel.lisp`, `src/airy.lisp`, `share/orthopoly/orthopoly.lisp`.
//! - ADR-0011: Special Functions Table Dispatch.

use maxima_rs::prelude::*;
use sym_core::specfn::SpecialFnKind;
use sym_core::specfn::orthopoly::{
    chebyshev_t_poly, chebyshev_u_poly, hermite_h_poly, laguerre_l_poly, legendre_p_poly,
};

#[test]
fn test_gamma_exact_evaluations() {
    // Gamma(1) = 1
    let g1 = Expr::gamma(Expr::int(1));
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(1)]),
        Some(Expr::int(1))
    );
    // Gamma(2) = 1
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(2)]),
        Some(Expr::int(1))
    );
    // Gamma(3) = 2! = 2
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(3)]),
        Some(Expr::int(2))
    );
    // Gamma(4) = 3! = 6
    assert_eq!(
        SpecialFnKind::Gamma.eval_exact(&[Expr::int(4)]),
        Some(Expr::int(6))
    );

    // Gamma(1/2) = sqrt(pi)
    let half = Expr::frac(1, 2);
    let g_half = SpecialFnKind::Gamma.eval_exact(&[half]);
    assert_eq!(g_half, Some(Expr::sqrt(Expr::pi())));

    // Gamma(3/2) = (1/2)*sqrt(pi)
    let three_halves = Expr::frac(3, 2);
    let g_three_halves = SpecialFnKind::Gamma.eval_exact(&[three_halves]);
    assert_eq!(
        g_three_halves,
        Some(Expr::mul(Expr::frac(1, 2), Expr::sqrt(Expr::pi())))
    );

    let _ = g1;
}

#[test]
fn test_beta_and_erf_evaluations() {
    // Beta(1, 1) = Gamma(1)*Gamma(1)/Gamma(2) = 1*1/1 = 1
    let b11 = SpecialFnKind::Beta.eval_exact(&[Expr::int(1), Expr::int(1)]);
    assert_eq!(b11, Some(Expr::int(1)));

    // erf(0) = 0
    assert_eq!(
        SpecialFnKind::Erf.eval_exact(&[Expr::int(0)]),
        Some(Expr::int(0))
    );
    // erfc(0) = 1
    assert_eq!(
        SpecialFnKind::Erfc.eval_exact(&[Expr::int(0)]),
        Some(Expr::int(1))
    );

    // Bessel J0(0) = 1, J1(0) = 0
    assert_eq!(
        SpecialFnKind::BesselJ.eval_exact(&[Expr::int(0), Expr::int(0)]),
        Some(Expr::int(1))
    );
    assert_eq!(
        SpecialFnKind::BesselJ.eval_exact(&[Expr::int(1), Expr::int(0)]),
        Some(Expr::int(0))
    );

    // Bessel I0(0) = 1, I1(0) = 0
    assert_eq!(
        SpecialFnKind::BesselI.eval_exact(&[Expr::int(0), Expr::int(0)]),
        Some(Expr::int(1))
    );
    assert_eq!(
        SpecialFnKind::BesselI.eval_exact(&[Expr::int(1), Expr::int(0)]),
        Some(Expr::int(0))
    );
}

#[test]
fn test_gamma_and_erf_differentiation() {
    let x = Expr::var("x");

    // d/dx Gamma(x) = Gamma(x) * psi[0](x)
    let d_gamma = differentiate(&Expr::gamma(x.clone()), "x");
    let expected_gamma = Expr::mul(
        Expr::gamma(x.clone()),
        Expr::special_fn(SpecialFnKind::Psi, vec![Expr::int(0), x.clone()]),
    );
    assert_eq!(d_gamma, expected_gamma);

    // d/dx erf(x) = (2/sqrt(pi)) * exp(-x^2)
    let d_erf = differentiate(&Expr::erf(x.clone()), "x");
    let expected_erf = Expr::mul(
        Expr::div(Expr::int(2), Expr::sqrt(Expr::pi())),
        Expr::exp(Expr::neg(Expr::pow(x.clone(), Expr::int(2)))),
    );
    assert_eq!(d_erf, expected_erf);

    // d/dx erfc(x) = - (2/sqrt(pi)) * exp(-x^2)
    let d_erfc = differentiate(&Expr::erfc(x.clone()), "x");
    assert_eq!(d_erfc, Expr::neg(expected_erf));

    // Chain rule: d/dx erf(x^2) = (2/sqrt(pi)) * exp(-(x^2)^2) * 2x
    let x_sq = Expr::pow(x.clone(), Expr::int(2));
    let d_erf_sq = differentiate(&Expr::erf(x_sq.clone()), "x");
    let inner_exp = Expr::exp(Expr::neg(Expr::pow(x_sq, Expr::int(2))));
    let expected_chain = Expr::mul(
        Expr::mul(Expr::div(Expr::int(2), Expr::sqrt(Expr::pi())), inner_exp),
        Expr::mul(Expr::int(2), x.clone()),
    );
    assert_eq!(d_erf_sq, expected_chain);

    // Differentiating constant returns 0
    assert_eq!(differentiate(&Expr::gamma(Expr::int(5)), "x"), Expr::int(0));
    assert_eq!(differentiate(&Expr::erf(Expr::int(2)), "x"), Expr::int(0));
}

#[test]
fn test_bessel_differentiation() {
    let x = Expr::var("x");
    let n = Expr::var("n");

    // d/dx J_n(x) = (J_{n-1}(x) - J_{n+1}(x)) / 2
    let d_jn = differentiate(&Expr::bessel_j(n.clone(), x.clone()), "x");
    let expected_jn = Expr::div(
        Expr::sub(
            Expr::special_fn(
                SpecialFnKind::BesselJ,
                vec![Expr::sub(n.clone(), Expr::int(1)), x.clone()],
            ),
            Expr::special_fn(
                SpecialFnKind::BesselJ,
                vec![Expr::add(n.clone(), Expr::int(1)), x.clone()],
            ),
        ),
        Expr::int(2),
    );
    assert_eq!(d_jn, expected_jn);

    // d/dx Y_n(x) = (Y_{n-1}(x) - Y_{n+1}(x)) / 2
    let d_yn = differentiate(&Expr::bessel_y(n.clone(), x.clone()), "x");
    let expected_yn = Expr::div(
        Expr::sub(
            Expr::special_fn(
                SpecialFnKind::BesselY,
                vec![Expr::sub(n.clone(), Expr::int(1)), x.clone()],
            ),
            Expr::special_fn(
                SpecialFnKind::BesselY,
                vec![Expr::add(n.clone(), Expr::int(1)), x.clone()],
            ),
        ),
        Expr::int(2),
    );
    assert_eq!(d_yn, expected_yn);

    // d/dx I_n(x) = (I_{n-1}(x) + I_{n+1}(x)) / 2
    let d_in = differentiate(&Expr::bessel_i(n.clone(), x.clone()), "x");
    let expected_in = Expr::div(
        Expr::add(
            Expr::special_fn(
                SpecialFnKind::BesselI,
                vec![Expr::sub(n.clone(), Expr::int(1)), x.clone()],
            ),
            Expr::special_fn(
                SpecialFnKind::BesselI,
                vec![Expr::add(n.clone(), Expr::int(1)), x.clone()],
            ),
        ),
        Expr::int(2),
    );
    assert_eq!(d_in, expected_in);

    // d/dx K_n(x) = - (K_{n-1}(x) + K_{n+1}(x)) / 2
    let d_kn = differentiate(&Expr::bessel_k(n.clone(), x.clone()), "x");
    let expected_kn = Expr::neg(Expr::div(
        Expr::add(
            Expr::special_fn(
                SpecialFnKind::BesselK,
                vec![Expr::sub(n.clone(), Expr::int(1)), x.clone()],
            ),
            Expr::special_fn(
                SpecialFnKind::BesselK,
                vec![Expr::add(n.clone(), Expr::int(1)), x.clone()],
            ),
        ),
        Expr::int(2),
    ));
    assert_eq!(d_kn, expected_kn);
}

#[test]
fn test_airy_differentiation_and_de() {
    let x = Expr::var("x");

    // d/dx Ai(x) = Ai'(x)
    let d_ai = differentiate(&Expr::airy_ai(x.clone()), "x");
    let dai_node = Expr::special_fn(SpecialFnKind::AiryAiPrime, vec![x.clone()]);
    assert_eq!(d_ai, dai_node);

    // d^2/dx^2 Ai(x) = x * Ai(x) (Airy differential equation: w'' - x*w = 0)
    let d2_ai = differentiate(&d_ai, "x");
    assert_eq!(d2_ai, Expr::mul(x.clone(), Expr::airy_ai(x.clone())));

    // d/dx Bi(x) = Bi'(x)
    let d_bi = differentiate(&Expr::airy_bi(x.clone()), "x");
    let dbi_node = Expr::special_fn(SpecialFnKind::AiryBiPrime, vec![x.clone()]);
    assert_eq!(d_bi, dbi_node);

    // d^2/dx^2 Bi(x) = x * Bi(x)
    let d2_bi = differentiate(&d_bi, "x");
    assert_eq!(d2_bi, Expr::mul(x.clone(), Expr::airy_bi(x.clone())));
}

#[test]
fn test_orthogonal_polynomials_generation() {
    let x = Expr::var("x");

    // Legendre P_0 = 1, P_1 = x
    assert_eq!(legendre_p_poly(0, &x), Expr::int(1));
    assert_eq!(legendre_p_poly(1, &x), x);

    // Chebyshev T_0 = 1, T_1 = x, T_2 = 2x^2 - 1
    assert_eq!(chebyshev_t_poly(0, &x), Expr::int(1));
    assert_eq!(chebyshev_t_poly(1, &x), x);
    let t2 = chebyshev_t_poly(2, &x);
    assert_eq!(
        t2,
        Expr::sub(
            Expr::mul(Expr::mul(Expr::int(2), x.clone()), x.clone()),
            Expr::int(1)
        )
    );

    // Chebyshev U_0 = 1, U_1 = 2x
    assert_eq!(chebyshev_u_poly(0, &x), Expr::int(1));
    assert_eq!(chebyshev_u_poly(1, &x), Expr::mul(Expr::int(2), x.clone()));

    // Hermite H_0 = 1, H_1 = 2x, H_2 = 4x^2 - 2
    assert_eq!(hermite_h_poly(0, &x), Expr::int(1));
    assert_eq!(hermite_h_poly(1, &x), Expr::mul(Expr::int(2), x.clone()));
    let h2 = hermite_h_poly(2, &x);
    assert_eq!(
        h2,
        Expr::sub(
            Expr::mul(
                Expr::mul(Expr::int(2), x.clone()),
                Expr::mul(Expr::int(2), x.clone())
            ),
            Expr::int(2)
        )
    );

    // Laguerre L_0 = 1, L_1 = 1 - x
    assert_eq!(laguerre_l_poly(0, &x), Expr::int(1));
    assert_eq!(laguerre_l_poly(1, &x), Expr::sub(Expr::int(1), x.clone()));
}

#[test]
fn test_orthogonal_polynomials_differentiation() {
    let x = Expr::var("x");

    // d/dx H_2(x) using explicit polynomial:
    // H_2(x) = 2x*(2x) - 2 = 4x^2 - 2 => derivative is 8x
    let dh2 = differentiate(&Expr::hermite_h(Expr::int(2), x.clone()), "x");
    // Should differentiate to an expression equivalent to 8x
    let env = [("x".to_string(), 3.0_f64)].into_iter().collect();
    assert_eq!(dh2.eval(&env), Some(24.0)); // 8 * 3 = 24

    // d/dx T_2(x): T_2 = 2x^2 - 1 => derivative is 4x
    let dt2 = differentiate(&Expr::chebyshev_t(Expr::int(2), x.clone()), "x");
    assert_eq!(dt2.eval(&env), Some(12.0)); // 4 * 3 = 12

    // d/dx P_2(x): P_2 = (3x^2 - 1)/2 => derivative is 3x
    let dp2 = differentiate(&Expr::legendre_p(Expr::int(2), x.clone()), "x");
    assert_eq!(dp2.eval(&env), Some(9.0)); // 3 * 3 = 9
}

#[test]
fn test_parser_and_latex_roundtrip() {
    let p_gamma = parse_maxima("gamma(x)").expect("parse gamma");
    assert_eq!(p_gamma, Expr::gamma(Expr::var("x")));
    assert_eq!(to_latex(&p_gamma), r"\Gamma\left(x\right)");

    let p_erf = parse_maxima("erf(x)").expect("parse erf");
    assert_eq!(p_erf, Expr::erf(Expr::var("x")));
    assert_eq!(to_latex(&p_erf), r"\mathrm{erf}\left(x\right)");

    let p_bj = parse_maxima("bessel_j(0, x)").expect("parse bessel_j");
    assert_eq!(p_bj, Expr::bessel_j(Expr::int(0), Expr::var("x")));
    assert_eq!(to_latex(&p_bj), r"J_{0}\left(x\right)");

    let p_ai = parse_maxima("airy_ai(x)").expect("parse airy_ai");
    assert_eq!(p_ai, Expr::airy_ai(Expr::var("x")));
    assert_eq!(to_latex(&p_ai), r"\mathrm{Ai}\left(x\right)");

    let p_leg = parse_maxima("legendre_p(2, x)").expect("parse legendre_p");
    assert_eq!(p_leg, Expr::legendre_p(Expr::int(2), Expr::var("x")));
    assert_eq!(to_latex(&p_leg), r"P_{2}\left(x\right)");

    let p_cheb = parse_maxima("chebyshev_t(3, x)").expect("parse chebyshev_t");
    assert_eq!(p_cheb, Expr::chebyshev_t(Expr::int(3), Expr::var("x")));
    assert_eq!(to_latex(&p_cheb), r"T_{3}\left(x\right)");

    let p_herm = parse_maxima("hermite(2, x)").expect("parse hermite");
    assert_eq!(p_herm, Expr::hermite_h(Expr::int(2), Expr::var("x")));
    assert_eq!(to_latex(&p_herm), r"H_{2}\left(x\right)");
}

#[test]
fn test_matrix_lossless_with_special_functions() {
    let m = Expr::matrix(vec![
        vec![Expr::gamma(Expr::var("x")), Expr::erf(Expr::var("y"))],
        vec![
            Expr::airy_ai(Expr::var("z")),
            Expr::bessel_j(Expr::int(0), Expr::var("t")),
        ],
    ]);
    let rows = match &m {
        Expr::Matrix(r) => r.clone(),
        _ => unreachable!(),
    };
    let encoded = sym_core::matrix::encode_matrix_symbol(&rows);
    let decoded = sym_core::matrix::decode_matrix_symbol(&encoded).expect("decode matrix");
    assert_eq!(Expr::Matrix(decoded), m);
}
