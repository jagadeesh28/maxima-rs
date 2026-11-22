//! Transcendental Risch u-substitution patterns (Phase 25).
//!
//! Recognises derivative-divides substitutions for the common transcendental towers:
//!
//! ```text
//! ∫ k·g'·exp(g) dx = k·exp(g)
//! ∫ k·g'·g^n dx    = k·g^(n+1)/(n+1)   (n ≠ -1),   = k·ln(g)  (n = -1)
//! ∫ k·g'/g dx      = k·ln(g)
//! ```
//!
//! These cover integrals such as `∫ x·e^{x²} dx`, `∫ e^x/(e^x+1) dx`, and
//! `∫ 2x/(x²+1) dx`. Non-elementary integrals (e.g. `∫ e^{-x²} dx`) fail every
//! pattern and fall through to the `Expr::integral` noun form.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::poly::Poly;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Try to integrate a transcendental derivative-divides pattern.
pub(crate) fn try_risch(expr: &Expr, var: &str) -> Option<Expr> {
    if let Some(r) = try_exp_times_derivative(expr, var) {
        return Some(r);
    }
    if let Some(r) = try_power_times_derivative(expr, var) {
        return Some(r);
    }
    if let Some(r) = try_derivative_over_function(expr, var) {
        return Some(r);
    }
    None
}

/// If `a = k·b` for a rational constant `k`, return `k`.
fn proportional_constant(a: &Expr, b: &Expr) -> Option<BigRational> {
    if a == b {
        return Some(BigRational::one());
    }
    if let Expr::Mul(x, y) = a {
        if let Expr::Const(c) = x.as_ref()
            && y.as_ref() == b
        {
            return Some(c.clone());
        }
        if let Expr::Const(c) = y.as_ref()
            && x.as_ref() == b
        {
            return Some(c.clone());
        }
    }
    let pa = Poly::from_expr(a).ok()?;
    let pb = Poly::from_expr(b).ok()?;
    if pa.is_zero() || pb.is_zero() {
        return None;
    }
    let la = pa.leading_term()?.coeff.clone();
    let lb = pb.leading_term()?.coeff.clone();
    if la.is_zero() || lb.is_zero() {
        return None;
    }
    let pa_n = pa.scale(&(BigRational::one() / la.clone()));
    let pb_n = pb.scale(&(BigRational::one() / lb.clone()));
    if pa_n == pb_n { Some(la / lb) } else { None }
}

/// `∫ k·g'·exp(g) dx = k·exp(g)`.
fn try_exp_times_derivative(expr: &Expr, var: &str) -> Option<Expr> {
    let Expr::Mul(a, b) = expr else {
        return None;
    };
    for (node, rest) in [(a, b), (b, a)] {
        if let Expr::Exp(arg) = node.as_ref() {
            let gp = differentiate(arg.as_ref(), var);
            if let Some(k) = proportional_constant(rest.as_ref(), &gp) {
                return Some(Expr::mul(
                    Expr::constant(k),
                    Expr::exp(arg.as_ref().clone()),
                ));
            }
        }
    }
    None
}

/// `∫ k·g'·g^n dx = k·g^(n+1)/(n+1)` (or `k·ln(g)` when `n = -1`).
fn try_power_times_derivative(expr: &Expr, var: &str) -> Option<Expr> {
    let Expr::Mul(a, b) = expr else {
        return None;
    };
    for (node, rest) in [(a, b), (b, a)] {
        let Expr::Pow(base, exp) = node.as_ref() else {
            continue;
        };
        let Some(n) = super::as_const(exp) else {
            continue;
        };
        if !Rat::is_integer(&n) {
            continue;
        }
        let gp = differentiate(base.as_ref(), var);
        if let Some(k) = proportional_constant(rest.as_ref(), &gp) {
            if n == Rat::from_int(-1) {
                return Some(Expr::mul(
                    Expr::constant(k),
                    Expr::ln(base.as_ref().clone()),
                ));
            }
            let ne = &n + BigRational::one();
            let pow = Expr::pow(base.as_ref().clone(), Expr::constant(ne.clone()));
            return Some(Expr::div(
                Expr::mul(Expr::constant(k), pow),
                Expr::constant(ne),
            ));
        }
    }
    None
}

/// `∫ k·g'/g dx = k·ln(g)`.
fn try_derivative_over_function(expr: &Expr, var: &str) -> Option<Expr> {
    let Expr::Div(num, den) = expr else {
        return None;
    };
    let dp = differentiate(den.as_ref(), var);
    if let Some(k) = proportional_constant(num.as_ref(), &dp) {
        return Some(Expr::mul(Expr::constant(k), Expr::ln(den.as_ref().clone())));
    }
    None
}
