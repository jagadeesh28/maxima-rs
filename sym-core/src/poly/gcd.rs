//! Polynomial GCD, exact division, and derivative utilities (Phase 23).
//!
//! Exact division and the Euclidean GCD operate over `BigRational`, so results are
//! always exact. Univariate polynomials are the supported focus; multivariate GCDs
//! fall back to a constant when the leading monomials do not divide.

use super::monomial::Monomial;
use super::{Poly, Term, VarContext, canonical_ctx, normalize_terms, remap_terms};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Re-map a polynomial into a target context and normalize its term order.
fn remap_poly(p: &Poly, ctx: &VarContext) -> Poly {
    let mut t = remap_terms(p, ctx);
    t.sort_by(|a, b| b.mono.cmp(&a.mono));
    Poly {
        terms: normalize_terms(t),
        ctx: ctx.clone(),
    }
}

/// Long-divide `a` by `b`, returning `(quotient, remainder)`.
///
/// The division stops when the leading monomial of the remainder is not divisible
/// by the leading monomial of `b` (which for univariate polynomials means the
/// remainder degree is below the divisor degree). Returns `None` for a zero divisor.
pub fn poly_divrem(a: &Poly, b: &Poly) -> Option<(Poly, Poly)> {
    if b.is_zero() {
        return None;
    }
    let ctx = canonical_ctx(&a.ctx, &b.ctx);
    let mut r = remap_terms(a, &ctx);
    r.sort_by(|x, y| y.mono.cmp(&x.mono));
    let mut bb = remap_terms(b, &ctx);
    bb.sort_by(|x, y| y.mono.cmp(&x.mono));
    if bb.is_empty() {
        return None;
    }

    let mut q: Vec<Term> = Vec::new();
    let mut guard = 0usize;
    while let Some(lt) = r.first().cloned() {
        let bt = &bb[0];
        let Some(mono) = lt.mono.div(&bt.mono) else {
            break;
        };
        let coeff = &lt.coeff / &bt.coeff;
        let qt = Term {
            coeff: coeff.clone(),
            mono: mono.clone(),
        };
        q.push(qt.clone());

        let sub: Vec<Term> = bb
            .iter()
            .map(|t| Term {
                coeff: &qt.coeff * &t.coeff,
                mono: t.mono.mul(&qt.mono),
            })
            .collect();
        let mut nr = r;
        nr.extend(sub.into_iter().map(|t| Term {
            coeff: -t.coeff,
            mono: t.mono,
        }));
        r = normalize_terms(nr);

        guard += 1;
        if guard > 100_000 {
            break;
        }
    }

    Some((
        Poly {
            terms: normalize_terms(q),
            ctx: ctx.clone(),
        },
        Poly {
            terms: normalize_terms(r),
            ctx,
        },
    ))
}

/// Exact quotient `a / b`, or `None` if the division leaves a non-zero remainder.
pub fn poly_div_exact(a: &Poly, b: &Poly) -> Option<Poly> {
    let (q, r) = poly_divrem(a, b)?;
    if r.is_zero() { Some(q) } else { None }
}

/// Scale a non-zero polynomial so its leading coefficient is `1`.
pub fn monic(p: &Poly) -> Poly {
    match p.leading_term() {
        Some(t) if !t.coeff.is_zero() => p.scale(&(BigRational::one() / t.coeff.clone())),
        _ => p.clone(),
    }
}

/// Greatest common divisor of univariate polynomials over Q (monic result).
///
/// Multivariate inputs degrade to the constant `1` (a documented limitation of the
/// univariate Euclidean engine).
pub fn poly_gcd(a: &Poly, b: &Poly) -> Poly {
    let ctx = canonical_ctx(&a.ctx, &b.ctx);
    if ctx.len() > 1 {
        return Poly::constant(BigRational::one(), ctx);
    }
    let mut x = remap_poly(a, &ctx);
    let mut y = remap_poly(b, &ctx);
    while !y.is_zero() {
        match poly_divrem(&x, &y) {
            Some((_, r)) => {
                x = y;
                y = r;
            }
            None => break,
        }
    }
    monic(&x)
}

/// The formal derivative of a univariate polynomial.
pub fn poly_derivative(p: &Poly) -> Poly {
    let terms: Vec<Term> = p
        .terms()
        .iter()
        .filter_map(|t| {
            let e = t.mono.exponent(0);
            if e == 0 {
                None
            } else {
                let mut exps = t.mono.exps().to_vec();
                exps[0] = e - 1;
                Some(Term {
                    coeff: &t.coeff * BigRational::from_integer(BigInt::from(e)),
                    mono: Monomial::from_exps(exps),
                })
            }
        })
        .collect();
    Poly::from_terms(terms, p.ctx.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Expr;

    #[test]
    fn gcd_of_x_squared_minus_one_and_x_minus_one() {
        let x = Expr::var("x");
        let a =
            Poly::from_expr(&Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1))).unwrap();
        let b = Poly::from_expr(&Expr::sub(x, Expr::int(1))).unwrap();
        let g = poly_gcd(&a, &b);
        assert_eq!(g.degree(), 1);
    }

    #[test]
    fn exact_division_recovers_factor() {
        let x = Expr::var("x");
        let a =
            Poly::from_expr(&Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1))).unwrap();
        let b = Poly::from_expr(&Expr::sub(x, Expr::int(1))).unwrap();
        let q = poly_div_exact(&a, &b).unwrap();
        assert_eq!(q.degree(), 1);
    }

    #[test]
    fn derivative_of_x_cubed() {
        let x = Expr::var("x");
        let p = Poly::from_expr(&Expr::pow(x, Expr::int(3))).unwrap();
        let d = poly_derivative(&p);
        assert_eq!(d.degree(), 2);
    }
}
