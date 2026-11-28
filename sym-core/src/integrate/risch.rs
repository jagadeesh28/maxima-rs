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
    if let Some(r) = try_trig_or_atan_tower(expr, var) {
        return Some(r);
    }
    None
}

/// Map non-polynomial transcendental kernels (sin, cos, exp, ln, sqrt, etc.) to
/// synthetic variables so `Poly::from_expr` can test proportionality bidirectionally.
fn map_kernels(e: &Expr, kernels: &mut Vec<Expr>) -> Expr {
    match e {
        Expr::Add(a, b) => Expr::add(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Sub(a, b) => Expr::sub(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Mul(a, b) => Expr::mul(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Div(a, b) => Expr::div(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Neg(a) => Expr::neg(map_kernels(a, kernels)),
        Expr::Pow(b, exp) => {
            if let Expr::Const(r) = exp.as_ref()
                && Rat::is_integer(r)
                && *r >= BigRational::zero()
            {
                return Expr::pow(map_kernels(b, kernels), (**exp).clone());
            }
            kernel_var(e, kernels)
        }
        Expr::Var(_) | Expr::Const(_) => e.clone(),
        _ => kernel_var(e, kernels),
    }
}

fn kernel_var(e: &Expr, kernels: &mut Vec<Expr>) -> Expr {
    if let Some(idx) = kernels.iter().position(|k| k == e) {
        return Expr::var(&format!("__rk_{idx}"));
    }
    let idx = kernels.len();
    kernels.push(e.clone());
    Expr::var(&format!("__rk_{idx}"))
}

/// If `a = k·b` for a rational constant `k`, return `k` (bidirectional).
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
    if let Expr::Mul(x, y) = b {
        if let Expr::Const(c) = x.as_ref()
            && y.as_ref() == a
            && !c.is_zero()
        {
            return Some(BigRational::one() / c);
        }
        if let Expr::Const(c) = y.as_ref()
            && x.as_ref() == a
            && !c.is_zero()
        {
            return Some(BigRational::one() / c);
        }
    }
    let mut kernels = Vec::new();
    let ma = map_kernels(a, &mut kernels);
    let mb = map_kernels(b, &mut kernels);
    let pa = Poly::from_expr(&ma).ok()?;
    let pb = Poly::from_expr(&mb).ok()?;
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
    // ∫ k * g' / (1 + g^2) dx = k * atan(g)
    if let Expr::Add(p, q) = den.as_ref() {
        let g_sq = if p.is_one() {
            Some(q.as_ref())
        } else if q.is_one() {
            Some(p.as_ref())
        } else {
            None
        };
        if let Some(Expr::Pow(g, exp)) = g_sq
            && let Some(r) = super::as_const(exp)
            && r == Rat::from_int(2)
        {
            let gp = differentiate(g.as_ref(), var);
            if let Some(k) = proportional_constant(num.as_ref(), &gp) {
                return Some(Expr::mul(Expr::constant(k), Expr::atan(g.as_ref().clone())));
            }
        }
    }
    None
}

/// `∫ k·g'·f(g) dx` for `f in {sin, cos, sinh, cosh, tan, tanh}`.
fn try_trig_or_atan_tower(expr: &Expr, var: &str) -> Option<Expr> {
    let Expr::Mul(a, b) = expr else {
        return None;
    };
    for (node, rest) in [(a, b), (b, a)] {
        let (arg, antideriv) = match node.as_ref() {
            Expr::Sin(g) => (g.as_ref(), Expr::neg(Expr::cos(g.as_ref().clone()))),
            Expr::Cos(g) => (g.as_ref(), Expr::sin(g.as_ref().clone())),
            Expr::Sinh(g) => (g.as_ref(), Expr::cosh(g.as_ref().clone())),
            Expr::Cosh(g) => (g.as_ref(), Expr::sinh(g.as_ref().clone())),
            Expr::Tan(g) => (
                g.as_ref(),
                Expr::neg(Expr::ln(Expr::cos(g.as_ref().clone()))),
            ),
            Expr::Tanh(g) => (g.as_ref(), Expr::ln(Expr::cosh(g.as_ref().clone()))),
            _ => continue,
        };
        let gp = differentiate(arg, var);
        if let Some(k) = proportional_constant(rest.as_ref(), &gp) {
            return Some(Expr::mul(Expr::constant(k), antideriv));
        }
    }
    None
}
