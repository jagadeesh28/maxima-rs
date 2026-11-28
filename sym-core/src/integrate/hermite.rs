//! Rational-function integration via Hermite reduction and LRT (Phase 24 / Phase 49).
//!
//! Performs polynomial long division, Hermite reduction on repeated factors
//! (Ostrogradsky-Hermite algorithm), and delegates the remaining square-free
//! proper rational part to [`super::lrt::integrate_squarefree_rational`].

use super::lrt::integrate_squarefree_rational;
use crate::expr::Expr;
use crate::poly::gcd::{poly_derivative, poly_div_exact, poly_divrem};
use crate::poly::partfrac::poly_ext_gcd;
use crate::poly::rat::to_fraction;
use crate::poly::{Poly, VarContext};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::One;

/// Integrate a rational expression in `var` via polynomial division, Hermite
/// reduction for repeated factors, and LRT for the square-free remainder.
pub(crate) fn integrate_rational(expr: &Expr, var: &str) -> Option<Expr> {
    let mut names = expr.variables();
    if !names.iter().any(|v| v == var) {
        return None;
    }
    // Only univariate in `var`
    if names.iter().any(|v| v != var) {
        return None;
    }
    names.sort();
    let mut ctx = VarContext::new();
    for n in &names {
        ctx.add(n);
    }
    let frac = to_fraction(expr, &ctx).ok()?.reduce();
    if frac.den.is_zero() {
        return None;
    }
    if frac.num.is_zero() {
        return Some(Expr::int(0));
    }

    let (quot, rem) = poly_divrem(&frac.num, &frac.den)?;
    let poly_part = if quot.is_zero() {
        None
    } else {
        Some(super::integrate(&quot.to_expr(), var))
    };

    if rem.is_zero() {
        return poly_part.or_else(|| Some(Expr::int(0)));
    }

    // Hermite reduction: reduce repeated factors in `frac.den`
    let (rat_part, sf_num, sf_den) = hermite_reduce(&rem, &frac.den)?;
    let log_part = if sf_num.is_zero() {
        None
    } else {
        Some(integrate_squarefree_rational(&sf_num, &sf_den, var)?)
    };

    let mut acc: Option<Expr> = poly_part;
    if let Some(rp) = rat_part {
        acc = Some(match acc {
            Some(prev) => Expr::add(prev, rp),
            None => rp,
        });
    }
    if let Some(lp) = log_part {
        acc = Some(match acc {
            Some(prev) => Expr::add(prev, lp),
            None => lp,
        });
    }
    acc.or_else(|| Some(Expr::int(0)))
}

/// Hermite reduction on proper fraction `a / d`:
/// returns `(rational_part, sf_num, sf_den)` where `sf_den` is square-free and
/// `∫ (a/d) dx = rational_part + ∫ (sf_num/sf_den) dx`.
fn hermite_reduce(a: &Poly, d: &Poly) -> Option<(Option<Expr>, Poly, Poly)> {
    let num = a.clone();
    let den = d.clone();
    let mut rat_acc: Option<Expr> = None;

    // Use Yun's square-free decomposition: den = c * prod_{i=1}^m f_i^i
    let sqf = crate::poly::factor::yun_square_free(&den);
    if sqf.iter().all(|(_, mult)| *mult <= 1) {
        return Some((None, num, den));
    }

    // Split num / den into sum_i num_i / f_i^i via Bezout, then reduce each f_i^i (i >= 2)
    let mut parts: Vec<(Poly, Poly, u32)> = Vec::new();
    let mut rem_num = num;
    let mut rem_den = den.clone();
    for (f_i, mult) in sqf {
        let mut f_pow = Poly::constant(BigRational::one(), den.ctx.clone());
        for _ in 0..mult {
            f_pow = f_pow.mul(&f_i);
        }
        let other = poly_div_exact(&rem_den, &f_pow)?;
        if other.is_constant() {
            parts.push((rem_num.clone(), f_i, mult));
            break;
        }
        let (g, s, t) = poly_ext_gcd(&f_pow, &other);
        let g_inv = g.leading_term()?.coeff.clone();
        let s_norm = s.scale(&(BigRational::one() / g_inv.clone()));
        let t_norm = t.scale(&(BigRational::one() / g_inv));
        let (_, n_i) = poly_divrem(&rem_num.mul(&t_norm), &f_pow)?;
        let (_, n_rest) = poly_divrem(&rem_num.mul(&s_norm), &other)?;
        parts.push((n_i, f_i, mult));
        rem_num = n_rest;
        rem_den = other;
    }

    // Reduce each (n_i, f_i, mult) down to multiplicity 1 using Bezout(f_i, f_i')
    let mut sf_num = Poly::zero(den.ctx.clone());
    let mut sf_den = Poly::constant(BigRational::one(), den.ctx.clone());

    for (mut cur_n, f_i, mut m) in parts {
        if cur_n.is_zero() {
            continue;
        }
        let f_prime = poly_derivative(&f_i);
        let (g, s, t) = poly_ext_gcd(&f_i, &f_prime);
        let g_inv = g.leading_term()?.coeff.clone();
        let s_norm = s.scale(&(BigRational::one() / g_inv.clone()));
        let t_norm = t.scale(&(BigRational::one() / g_inv));

        while m > 1 {
            // cur_n = (cur_n * s_norm) * f_i + (cur_n * t_norm) * f_i'
            // Let b = (cur_n * t_norm) mod f_i, a = (cur_n - b * f_i') / f_i
            let (_, b) = poly_divrem(&cur_n.mul(&t_norm), &f_i)?;
            let scale_m = BigRational::from_integer(BigInt::from(1 - i64::from(m)));
            let b_scaled = b.scale(&(BigRational::one() / scale_m));
            // d/dx (b_scaled / f_i^(m-1)) = b_scaled' / f_i^(m-1) + b * f_i' / f_i^m
            let mut f_pow_prev = Poly::constant(BigRational::one(), den.ctx.clone());
            for _ in 0..(m - 1) {
                f_pow_prev = f_pow_prev.mul(&f_i);
            }
            if !b_scaled.is_zero() {
                let term = Expr::div(b_scaled.to_expr(), f_pow_prev.to_expr());
                rat_acc = Some(match rat_acc {
                    Some(prev) => Expr::add(prev, term),
                    None => term,
                });
            }
            let diff = cur_n.sub(&b.mul(&f_prime));
            let a_part = poly_div_exact(&diff, &f_i)?;
            let b_prime = poly_derivative(&b_scaled);
            cur_n = a_part.sub(&b_prime);
            m -= 1;
            let _ = &s_norm;
        }

        if !cur_n.is_zero() {
            // Add cur_n / f_i into sf_num / sf_den
            sf_num = sf_num.mul(&f_i).add(&cur_n.mul(&sf_den));
            sf_den = sf_den.mul(&f_i);
        }
    }

    Some((rat_acc, sf_num, sf_den))
}
