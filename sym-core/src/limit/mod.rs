//! Asymptotic limits (Phase 27).
//!
//! `limit(expr, var, point)` reduces the request to a series expansion about the
//! limit point (via [`crate::series`]) for exact analytic cases, then falls back to
//! a bounded numeric convergence check that recognises common exact constants
//! (`0`, integers, small rationals, `e`, `π`). Infinite points (`"inf"`, `"minf"`)
//! are handled with the substitution `x = ±1/t`.

pub mod mrv;
pub mod zero;

use crate::expr::Expr;
use crate::rational::Rat;
use crate::series::series_expand;
use std::collections::HashMap;

fn is_pos_inf(point: &Expr) -> bool {
    matches!(point, Expr::Var(n) if n.as_str() == "inf")
}

fn is_neg_inf(point: &Expr) -> bool {
    matches!(point, Expr::Var(n) if n.as_str() == "minf")
}

/// Replace every occurrence of variable `name` with `r`.
fn subst_var(e: &Expr, name: &str, r: &Expr) -> Expr {
    if let Expr::Var(n) = e
        && n.as_str() == name
    {
        return r.clone();
    }
    match e {
        Expr::Var(_) | Expr::Const(_) | Expr::Pi | Expr::E => e.clone(),
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(_) => e.clone(),
        #[cfg(feature = "complex")]
        Expr::I => e.clone(),
        Expr::Add(a, b) => Expr::add(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Sub(a, b) => Expr::sub(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Mul(a, b) => Expr::mul(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Div(a, b) => Expr::div(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Pow(a, b) => Expr::pow(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Neg(a) => Expr::neg(subst_var(a, name, r)),
        Expr::Sin(a) => Expr::sin(subst_var(a, name, r)),
        Expr::Cos(a) => Expr::cos(subst_var(a, name, r)),
        Expr::Tan(a) => Expr::tan(subst_var(a, name, r)),
        Expr::Exp(a) => Expr::exp(subst_var(a, name, r)),
        Expr::Ln(a) => Expr::ln(subst_var(a, name, r)),
        Expr::Sqrt(a) => Expr::sqrt(subst_var(a, name, r)),
        Expr::Sinh(a) => Expr::sinh(subst_var(a, name, r)),
        Expr::Cosh(a) => Expr::cosh(subst_var(a, name, r)),
        Expr::Tanh(a) => Expr::tanh(subst_var(a, name, r)),
        Expr::Asin(a) => Expr::asin(subst_var(a, name, r)),
        Expr::Acos(a) => Expr::acos(subst_var(a, name, r)),
        Expr::Atan(a) => Expr::atan(subst_var(a, name, r)),
        Expr::Abs(a) => Expr::abs(subst_var(a, name, r)),
        Expr::Atan2(y, x) => Expr::atan2(subst_var(y, name, r), subst_var(x, name, r)),
        Expr::Factorial(a) => Expr::factorial(subst_var(a, name, r)),
        Expr::Tensor(n, idx) => Expr::Tensor(n.clone(), idx.clone()),
        Expr::Derivative(a, b) => Expr::derivative(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Integral(a, b) => Expr::integral(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Sum(a, b, c, d) => Expr::sum(
            subst_var(a, name, r),
            subst_var(b, name, r),
            subst_var(c, name, r),
            subst_var(d, name, r),
        ),
        Expr::Matrix(m) => Expr::matrix(
            m.iter()
                .map(|row| row.iter().map(|x| subst_var(x, name, r)).collect())
                .collect(),
        ),
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => {
            Expr::SpecialFn(*kind, args.iter().map(|x| subst_var(x, name, r)).collect())
        }
    }
}

#[cfg(feature = "assume")]
use crate::assume::{Assumptions, Sign};

/// Compute the two-sided limit of `expr` as `var` approaches `point`.
///
/// `point` is an [`Expr`] constant for finite limits, or `Expr::var("inf")` /
/// `Expr::var("minf")` for infinite limits. Returns `None` when the limit cannot
/// be determined.
pub fn limit(expr: &Expr, var: &str, point: &Expr) -> Option<Expr> {
    #[cfg(feature = "assume")]
    {
        limit_with_assumptions(expr, var, point, None)
    }
    #[cfg(not(feature = "assume"))]
    {
        limit_core(expr, var, point)
    }
}

/// Compute limit with optional [`Assumptions`] context.
#[cfg(feature = "assume")]
pub fn limit_with_assumptions(
    expr: &Expr,
    var: &str,
    point: &Expr,
    assumptions: Option<&Assumptions>,
) -> Option<Expr> {
    if !expr.variables().iter().any(|v| v == var) {
        return Some(expr.clone());
    }
    // Check monomial power with assumptions as point -> inf
    if let Expr::Pow(base, exp) = expr
        && matches!(base.as_ref(), Expr::Var(v) if v == var)
        && is_pos_inf(point)
        && !exp.variables().contains(&var.to_string())
        && let Some(ctx) = assumptions
    {
        match ctx.csign(exp) {
            Sign::Pos => return Some(Expr::var("inf")),
            Sign::Neg => return Some(Expr::int(0)),
            Sign::Zero => return Some(Expr::int(1)),
            _ => {}
        }
    }
    limit_core(expr, var, point)
}

fn limit_core(expr: &Expr, var: &str, point: &Expr) -> Option<Expr> {
    if !expr.variables().iter().any(|v| v == var) {
        return Some(expr.clone());
    }
    if let Some(r) = symbolic_limit(expr, var, point) {
        return Some(r);
    }
    if let Expr::Pow(base, exp) = expr {
        let e_arg = Expr::mul((**exp).clone(), Expr::ln((**base).clone()));
        if let Some(l) = limit(&e_arg, var, point) {
            return Some(exp_of(&l));
        }
    }
    if let Some(r) = limit_exp_difference(expr, var, point) {
        return Some(r);
    }
    numeric_limit(expr, var, point)
}

fn exp_of(l: &Expr) -> Expr {
    if l.is_zero() {
        Expr::int(1)
    } else if l.is_one() {
        Expr::e()
    } else {
        Expr::exp(l.clone())
    }
}

/// Light algebraic simplification focused on cancellation of common subterms.
fn simplify_light(e: Expr) -> Expr {
    match e {
        Expr::Sub(a, b) => {
            let a = simplify_light(*a);
            let b = simplify_light(*b);
            if a == b {
                return Expr::int(0);
            }
            if let Expr::Add(p, q) = &a {
                if **p == b {
                    return (**q).clone();
                }
                if **q == b {
                    return (**p).clone();
                }
            }
            Expr::Sub(Box::new(a), Box::new(b))
        }
        Expr::Add(a, b) => {
            let a = simplify_light(*a);
            let b = simplify_light(*b);
            if a.is_zero() {
                return b;
            }
            if b.is_zero() {
                return a;
            }
            Expr::Add(Box::new(a), Box::new(b))
        }
        other => other,
    }
}

/// `e^A - e^B ≈ e^B·(A-B)` when `A - B → 0` (first-order leading term).
fn limit_exp_difference(expr: &Expr, var: &str, point: &Expr) -> Option<Expr> {
    let Expr::Sub(a, b) = expr else {
        return None;
    };
    let (Expr::Exp(aa), Expr::Exp(bb)) = (a.as_ref(), b.as_ref()) else {
        return None;
    };
    let diff = simplify_light(Expr::sub((**aa).clone(), (**bb).clone()));
    let ld = limit(&diff, var, point)?;
    if !ld.is_zero() {
        return None;
    }
    let approx = Expr::mul(Expr::exp((**bb).clone()), diff);
    limit(&approx, var, point)
}

fn symbolic_limit(expr: &Expr, var: &str, point: &Expr) -> Option<Expr> {
    if is_pos_inf(point) {
        let sub = subst_var(expr, var, &Expr::div(Expr::int(1), Expr::var("t")));
        let s = series_expand(&sub, "t", &Expr::int(0), 8)?;
        return if s.valuation >= 0 {
            s.constant_term()
        } else {
            None
        };
    }
    if is_neg_inf(point) {
        let sub = subst_var(
            expr,
            var,
            &Expr::neg(Expr::div(Expr::int(1), Expr::var("t"))),
        );
        let s = series_expand(&sub, "t", &Expr::int(0), 8)?;
        return if s.valuation >= 0 {
            s.constant_term()
        } else {
            None
        };
    }
    let s = series_expand(expr, var, point, 8)?;
    if s.valuation >= 0 {
        s.constant_term()
    } else {
        None
    }
}

fn converge(expr: &Expr, var: &str, samples: &[f64]) -> Option<Expr> {
    let mut vals: Vec<f64> = Vec::new();
    let mut env: HashMap<String, f64> = HashMap::new();
    for &x in samples {
        env.insert(var.to_string(), x);
        if let Some(v) = expr.eval(&env)
            && v.is_finite()
        {
            vals.push(v);
        }
    }
    if vals.len() < 4 {
        return None;
    }
    let start = vals.len().saturating_sub(5);
    let tail = &vals[start..];
    let min = tail.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = tail.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() > 1e-6 {
        return None;
    }
    let v = tail.iter().sum::<f64>() / tail.len() as f64;
    recognize(v)
}

/// Recognise a converged numeric value as an exact constant.
fn recognize(v: f64) -> Option<Expr> {
    if !v.is_finite() {
        return None;
    }
    let tol = 1e-6;
    if v.abs() < tol {
        return Some(Expr::int(0));
    }
    if (v - std::f64::consts::E).abs() < tol {
        return Some(Expr::e());
    }
    if (v + std::f64::consts::E).abs() < tol {
        return Some(Expr::neg(Expr::e()));
    }
    if (v - std::f64::consts::PI).abs() < tol {
        return Some(Expr::pi());
    }
    let r = v.round();
    if (v - r).abs() < tol && r.abs() <= 100.0 {
        return Some(Expr::int(r as i64));
    }
    for q in 1..=12i64 {
        let p = (v * q as f64).round() as i64;
        if (v - (p as f64) / (q as f64)).abs() < tol {
            return Some(Expr::frac(p, q));
        }
    }
    None
}

fn inf_sequences() -> Vec<Vec<f64>> {
    vec![
        (6..=12).map(|k| 10f64.powi(k)).collect(),
        (5..=40).map(|k| 10.0 * k as f64).collect(),
        (2..=8).map(|k| 10f64.powi(k)).collect(),
    ]
}

fn numeric_limit(expr: &Expr, var: &str, point: &Expr) -> Option<Expr> {
    if is_pos_inf(point) {
        for seq in inf_sequences() {
            if let Some(r) = converge(expr, var, &seq) {
                return Some(r);
            }
        }
        return None;
    }
    if is_neg_inf(point) {
        for seq in inf_sequences() {
            let neg: Vec<f64> = seq.iter().map(|v| -v).collect();
            if let Some(r) = converge(expr, var, &neg) {
                return Some(r);
            }
        }
        return None;
    }
    let a = match point {
        Expr::Const(r) => Rat::to_f64(r),
        _ => return None,
    };
    let right: Vec<f64> = (1..=10).map(|k| a + 10f64.powi(-k)).collect();
    let left: Vec<f64> = (1..=10).map(|k| a - 10f64.powi(-k)).collect();
    match (converge(expr, var, &right), converge(expr, var, &left)) {
        (Some(x), Some(y)) if x == y => Some(x),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        _ => None,
    }
}
