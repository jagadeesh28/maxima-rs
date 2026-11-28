//! Real logarithmic and arctangent antiderivative terms (Lazard-Rioboo-Trager).

use crate::expr::Expr;
use crate::poly::Poly;
use crate::poly::factor::factor_poly_list;
use crate::poly::gcd::poly_derivative;
use crate::poly::partfrac::partfrac;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

/// Build `∫ num_const / (a*x + b) dx = (num_const / a) * ln(a*x + b)`.
pub(crate) fn log_over_linear(num_const: &Expr, den_linear: &Expr, a: &BigRational) -> Expr {
    Expr::div(
        Expr::mul(num_const.clone(), Expr::ln(den_linear.clone())),
        Expr::constant(a.clone()),
    )
}

/// Integrate a proper square-free rational function `num / den` in `var` over `R`
/// using Lazard-Rioboo-Trager / real partial-fraction splitting into linear `ln`
/// and irreducible quadratic `ln + atan` terms.
pub(crate) fn integrate_squarefree_rational(num: &Poly, den: &Poly, var: &str) -> Option<Expr> {
    if num.is_zero() {
        return Some(Expr::int(0));
    }
    if den.degree() == 0 {
        let c = den.leading_term()?.coeff.clone();
        return Some(super::integrate(
            &num.scale(&(BigRational::one() / c)).to_expr(),
            var,
        ));
    }

    let (_, factors) = factor_poly_list(den);
    // All factors must be irreducible of degree <= 2 (or have multiplicity 1 and degree <= 2)
    // For higher-degree square-free factors, try derivative-divides (num = k * den')
    if factors.iter().any(|(f, _)| f.degree() > 2) {
        let dp = poly_derivative(den);
        if !dp.is_zero()
            && let (Some(ln), Some(ld)) = (num.leading_term(), dp.leading_term())
        {
            let k = &ln.coeff / &ld.coeff;
            if &dp.scale(&k) == num {
                return Some(Expr::mul(Expr::constant(k), Expr::ln(den.to_expr())));
            }
        }
        return None;
    }

    let frac_expr = Expr::div(num.to_expr(), den.to_expr());
    let pf = if factors.len() == 1 && factors[0].1 == 1 {
        frac_expr
    } else {
        partfrac(&frac_expr, var).ok()?
    };
    integrate_pf_terms(&pf, var)
}

fn integrate_pf_terms(expr: &Expr, var: &str) -> Option<Expr> {
    match expr {
        Expr::Add(a, b) => Some(Expr::add(
            integrate_pf_terms(a, var)?,
            integrate_pf_terms(b, var)?,
        )),
        Expr::Sub(a, b) => Some(Expr::sub(
            integrate_pf_terms(a, var)?,
            integrate_pf_terms(b, var)?,
        )),
        Expr::Neg(a) => Some(Expr::neg(integrate_pf_terms(a, var)?)),
        Expr::Div(num, den) => integrate_single_term(num, den, var),
        other => {
            if other.is_zero() {
                Some(Expr::int(0))
            } else {
                let res = super::integrate(other, var);
                if super::contains_integral(&res) {
                    None
                } else {
                    Some(res)
                }
            }
        }
    }
}

fn univariate_coeffs(p: &Poly, var: &str) -> Option<(BigRational, BigRational, BigRational)> {
    if p.vars().iter().any(|v| v != var) || p.degree() > 2 {
        return None;
    }
    let mut c2 = BigRational::zero();
    let mut c1 = BigRational::zero();
    let mut c0 = BigRational::zero();
    for t in p.terms() {
        match t.mono.total_degree() {
            0 => c0 = t.coeff.clone(),
            1 => c1 = t.coeff.clone(),
            2 => c2 = t.coeff.clone(),
            _ => return None,
        }
    }
    Some((c2, c1, c0))
}

fn integrate_single_term(num: &Expr, den: &Expr, var: &str) -> Option<Expr> {
    let pn = Poly::from_expr(num).ok()?;
    let pd = Poly::from_expr(den).ok()?;
    if pn.is_zero() {
        return Some(Expr::int(0));
    }
    if pd.degree() == 1 {
        let (_, a, _) = univariate_coeffs(&pd, var)?;
        if a.is_zero() {
            return None;
        }
        return Some(log_over_linear(num, den, &a));
    }
    if pd.degree() == 2 {
        let (_, a_num, b_num) = univariate_coeffs(&pn, var)?;
        let (p, q, r) = univariate_coeffs(&pd, var)?;
        let two = Rat::from_int(2);
        let four = Rat::from_int(4);
        // Split (A*x + B) / (p*x^2 + q*x + r) = (A/(2p))*(2px + q)/(px^2+qx+r) + C/(px^2+qx+r)
        let mut acc: Option<Expr> = None;
        if !a_num.is_zero() {
            let log_coeff = &a_num / (&two * &p);
            let log_term = Expr::mul(Expr::constant(log_coeff), Expr::ln(den.clone()));
            acc = Some(log_term);
        }
        let c_rem = &b_num - (&a_num * &q) / (&two * &p);
        if !c_rem.is_zero() {
            let disc = &q * &q - &four * &p * &r;
            if disc.is_negative() {
                // D = (4*p*r - q^2) / (4*p^2) > 0
                let d_rat = (-&disc) / (&four * &p * &p);
                let sqrt_d = exact_sqrt_expr(&d_rat);
                let shift = &q / (&two * &p);
                let arg_num = if shift.is_zero() {
                    Expr::var(var)
                } else {
                    Expr::add(Expr::var(var), Expr::constant(shift))
                };
                let atan_arg = Expr::div(arg_num, sqrt_d.clone());
                let atan_coeff =
                    Expr::div(Expr::constant(c_rem), Expr::mul(Expr::constant(p), sqrt_d));
                let atan_term = Expr::mul(atan_coeff, Expr::atan(atan_arg));
                acc = Some(match acc {
                    Some(prev) => Expr::add(prev, atan_term),
                    None => atan_term,
                });
            } else {
                return None;
            }
        }
        return acc.or_else(|| Some(Expr::int(0)));
    }
    None
}

fn exact_sqrt_expr(r: &BigRational) -> Expr {
    if r.is_one() {
        return Expr::int(1);
    }
    let ns = r.numer().sqrt();
    let ds = r.denom().sqrt();
    if &ns * &ns == *r.numer() && &ds * &ds == *r.denom() {
        Expr::constant(BigRational::new(ns, ds))
    } else {
        Expr::sqrt(Expr::constant(r.clone()))
    }
}
