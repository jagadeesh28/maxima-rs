//! Polynomial root finding via rational-root factoring plus linear/quadratic forms.

use crate::expr::Expr;
use crate::poly::factor::distinct_rational_roots;
use crate::poly::gcd::poly_div_exact;
use crate::poly::{Monomial, Poly, VarContext};
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Solve a univariate polynomial equation `expr = 0`.
/// Returns the found roots and whether the factorization was complete (`cur.degree() == 0`).
pub(crate) fn solve_polynomial(expr: &Expr, var: &str) -> Option<(Vec<Expr>, bool)> {
    let p = Poly::from_expr(expr).ok()?;
    if p.vars().iter().any(|v| v != var) {
        return None;
    }
    if p.is_zero() {
        return Some((Vec::new(), true));
    }
    let mut cur = p.clone();
    let mut roots: Vec<Expr> = Vec::new();

    if let Some(rationals) = distinct_rational_roots(&p) {
        for r in rationals {
            if cur.degree() == 0 {
                break;
            }
            let lin = linear_poly(&r, &cur.ctx);
            while let Some(q) = poly_div_exact(&cur, &lin) {
                cur = q;
                roots.push(Expr::constant(r.clone()));
            }
        }
    }

    match cur.degree() {
        0 => {}
        1 => {
            let a = coeff(&cur, 1);
            let b = coeff(&cur, 0);
            if !a.is_zero() {
                roots.push(Expr::constant(-&b / &a));
                cur = Poly::constant(BigRational::one(), cur.ctx.clone());
            }
        }
        2 => {
            let q_roots = quadratic_roots(&cur);
            if !q_roots.is_empty() {
                roots.extend(q_roots);
                cur = Poly::constant(BigRational::one(), cur.ctx.clone());
            }
        }
        _ => {}
    }

    let is_complete = cur.degree() == 0;
    Some((roots, is_complete))
}

fn coeff(p: &Poly, exp: u32) -> BigRational {
    for t in p.terms() {
        if t.mono.exponent(0) == exp {
            return t.coeff.clone();
        }
    }
    BigRational::zero()
}

fn linear_poly(r: &BigRational, ctx: &VarContext) -> Poly {
    let mut exps = vec![0u32; ctx.len()];
    exps[0] = 1;
    let x = Poly::monomial(BigRational::one(), Monomial::from_exps(exps), ctx.clone());
    x.add(&Poly::constant(-r.clone(), ctx.clone()))
}

fn quadratic_roots(p: &Poly) -> Vec<Expr> {
    let a = coeff(p, 2);
    let b = coeff(p, 1);
    let c = coeff(p, 0);
    let four = Rat::from_int(4);
    let disc = &b * &b - &four * &a * &c;
    if disc < BigRational::zero() {
        return Vec::new();
    }
    let sqrt = Expr::sqrt(Expr::constant(disc));
    let two_a = Expr::constant(&a * Rat::from_int(2));
    let neg_b = Expr::neg(Expr::constant(b));
    vec![
        Expr::div(Expr::add(neg_b.clone(), sqrt.clone()), two_a.clone()),
        Expr::div(Expr::sub(neg_b, sqrt), two_a),
    ]
}
