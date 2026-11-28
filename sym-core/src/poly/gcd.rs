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

/// Greatest common divisor of univariate or multivariate polynomials over Q.
///
/// For univariate polynomials, returns the monic GCD.
/// For multivariate polynomials, computes the primitive recursive PRS GCD over `Z[x_0, ..., x_{n-1}]`.
pub fn poly_gcd(a: &Poly, b: &Poly) -> Poly {
    let ctx = canonical_ctx(&a.ctx, &b.ctx);
    let x = remap_poly(a, &ctx);
    let y = remap_poly(b, &ctx);
    if x.is_zero() {
        return monic(&y);
    }
    if y.is_zero() {
        return monic(&x);
    }
    if x.is_constant() || y.is_constant() {
        return Poly::constant(BigRational::one(), ctx);
    }
    if ctx.len() <= 1 {
        let mut u = x;
        let mut v = y;
        while !v.is_zero() {
            match poly_divrem(&u, &v) {
                Some((_, r)) => {
                    u = v;
                    v = r;
                }
                None => break,
            }
        }
        return monic(&u);
    }

    // Multivariate primitive PRS in variable index 0
    let g = multivar_gcd_var(&x, &y, 0);
    monic(&g)
}

fn degree_in_var(p: &Poly, var_idx: usize) -> u32 {
    p.terms()
        .iter()
        .map(|t| t.mono.exponent(var_idx))
        .max()
        .unwrap_or(0)
}

/// View `p` as a univariate polynomial in `var_idx` with coefficients in the remaining variables.
fn split_by_var(p: &Poly, var_idx: usize) -> Vec<Poly> {
    let d = degree_in_var(p, var_idx) as usize;
    let mut coeffs = vec![Vec::<Term>::new(); d + 1];
    for t in p.terms() {
        let e = t.mono.exponent(var_idx) as usize;
        let mut exps = t.mono.exps().to_vec();
        exps[var_idx] = 0;
        coeffs[e].push(Term {
            coeff: t.coeff.clone(),
            mono: Monomial::from_exps(exps),
        });
    }
    coeffs
        .into_iter()
        .map(|ts| Poly::from_terms(ts, p.ctx.clone()))
        .collect()
}

fn assemble_from_var(coeffs: &[Poly], var_idx: usize, ctx: &VarContext) -> Poly {
    let mut all_terms = Vec::new();
    for (deg, c) in coeffs.iter().enumerate() {
        for t in c.terms() {
            let mut exps = t.mono.exps().to_vec();
            exps[var_idx] += deg as u32;
            all_terms.push(Term {
                coeff: t.coeff.clone(),
                mono: Monomial::from_exps(exps),
            });
        }
    }
    Poly::from_terms(all_terms, ctx.clone())
}

fn content_in_var(p: &Poly, var_idx: usize) -> Poly {
    let coeffs = split_by_var(p, var_idx);
    let mut g: Option<Poly> = None;
    for c in coeffs {
        if c.is_zero() {
            continue;
        }
        g = Some(match g {
            None => c,
            Some(acc) => {
                if var_idx + 1 < p.ctx.len() {
                    multivar_gcd_var(&acc, &c, var_idx + 1)
                } else {
                    Poly::constant(BigRational::one(), p.ctx.clone())
                }
            }
        });
        if g.as_ref().is_some_and(|poly| poly.is_constant()) {
            return Poly::constant(BigRational::one(), p.ctx.clone());
        }
    }
    g.unwrap_or_else(|| Poly::constant(BigRational::one(), p.ctx.clone()))
}

fn primitive_part_var(p: &Poly, var_idx: usize) -> (Poly, Poly) {
    if p.is_zero() {
        return (Poly::constant(BigRational::one(), p.ctx.clone()), p.clone());
    }
    let cont = content_in_var(p, var_idx);
    if cont.is_constant() {
        return (Poly::constant(BigRational::one(), p.ctx.clone()), monic(p));
    }
    match poly_div_exact(p, &cont) {
        Some(pp) => (cont, monic(&pp)),
        None => (Poly::constant(BigRational::one(), p.ctx.clone()), monic(p)),
    }
}

fn pseudo_rem_var(a: &Poly, b: &Poly, var_idx: usize) -> Poly {
    let deg_b = degree_in_var(b, var_idx);
    if b.is_zero() {
        return a.clone();
    }
    let b_coeffs = split_by_var(b, var_idx);
    let lc_b = &b_coeffs[deg_b as usize];

    let mut r = a.clone();
    let mut guard = 0usize;
    while !r.is_zero() {
        let deg_r = degree_in_var(&r, var_idx);
        if deg_r < deg_b {
            break;
        }
        let r_coeffs = split_by_var(&r, var_idx);
        let lc_r = &r_coeffs[deg_r as usize];
        let diff_deg = deg_r - deg_b;

        let mut shift_exps = vec![0u32; r.ctx.len()];
        shift_exps[var_idx] = diff_deg;
        let x_pow = Poly::monomial(
            BigRational::one(),
            Monomial::from_exps(shift_exps),
            r.ctx.clone(),
        );

        // r = lc_b * r - lc_r * x^(deg_r - deg_b) * b
        let lhs = lc_b.mul(&r);
        let rhs = lc_r.mul(&x_pow).mul(b);
        r = lhs.sub(&rhs);
        let _ = assemble_from_var; // keep helper used if needed

        guard += 1;
        if guard > 1_000 {
            break;
        }
    }
    r
}

fn multivar_gcd_var(a: &Poly, b: &Poly, var_idx: usize) -> Poly {
    if a.is_zero() {
        return monic(b);
    }
    if b.is_zero() {
        return monic(a);
    }
    if a.is_constant() || b.is_constant() {
        return Poly::constant(BigRational::one(), a.ctx.clone());
    }
    if var_idx >= a.ctx.len() {
        return Poly::constant(BigRational::one(), a.ctx.clone());
    }

    let (cont_a, mut u) = primitive_part_var(a, var_idx);
    let (cont_b, mut v) = primitive_part_var(b, var_idx);

    let g_cont = if var_idx + 1 < a.ctx.len() {
        multivar_gcd_var(&cont_a, &cont_b, var_idx + 1)
    } else {
        Poly::constant(BigRational::one(), a.ctx.clone())
    };

    if degree_in_var(&u, var_idx) == 0 || degree_in_var(&v, var_idx) == 0 {
        return g_cont;
    }
    if degree_in_var(&u, var_idx) < degree_in_var(&v, var_idx) {
        std::mem::swap(&mut u, &mut v);
    }

    while !v.is_zero() {
        if degree_in_var(&v, var_idx) == 0 {
            return g_cont;
        }
        let r = pseudo_rem_var(&u, &v, var_idx);
        if r.is_zero() {
            u = v;
            break;
        }
        let (_, pp_r) = primitive_part_var(&r, var_idx);
        u = v;
        v = pp_r;
    }

    g_cont.mul(&u)
}

/// The formal derivative of a univariate polynomial (with respect to variable 0).
pub fn poly_derivative(p: &Poly) -> Poly {
    poly_derivative_var(p, 0)
}

/// The formal derivative of a multivariate polynomial with respect to `var_idx`.
pub fn poly_derivative_var(p: &Poly, var_idx: usize) -> Poly {
    let terms: Vec<Term> = p
        .terms()
        .iter()
        .filter_map(|t| {
            let e = t.mono.exponent(var_idx);
            if e == 0 {
                None
            } else {
                let mut exps = t.mono.exps().to_vec();
                exps[var_idx] = e - 1;
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
