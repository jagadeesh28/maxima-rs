//! Definite integration via the Fundamental Theorem of Calculus (Phase 28).
//!
//! `defint(f, x, a, b)` computes `F(b) - F(a)` where `F = ∫ f dx`, but first scans
//! for rational poles inside the integration interval and refuses to apply FTC
//! across them (returning [`DefiniteResult::Divergent`]). Improper bounds use the
//! [`crate::limit`] engine.

use super::integrate;
use crate::expr::Expr;
use crate::poly::VarContext;
use crate::poly::factor::distinct_rational_roots;
use crate::poly::rat::to_fraction;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::Zero;

/// Outcome of a definite integration request.
#[derive(Debug, Clone, PartialEq)]
pub enum DefiniteResult {
    /// A closed-form exact value.
    Value(Expr),
    /// The integral diverges (e.g. a real pole lies inside the interval).
    Divergent,
    /// The antiderivative or a limit could not be determined.
    Unevaluated,
}

fn point_f64(p: &Expr) -> Option<f64> {
    match p {
        Expr::Const(r) => Some(Rat::to_f64(r)),
        Expr::Var(n) if n.as_str() == "inf" => Some(f64::INFINITY),
        Expr::Var(n) if n.as_str() == "minf" => Some(f64::NEG_INFINITY),
        _ => None,
    }
}

fn rational_poles(expr: &Expr, var: &str) -> Option<Vec<BigRational>> {
    let mut names = expr.variables();
    names.sort();
    let mut ctx = VarContext::new();
    for n in &names {
        ctx.add(n);
    }
    let _ = var;
    let f = to_fraction(expr, &ctx).ok()?;
    distinct_rational_roots(&f.den)
}

fn has_interior_pole(expr: &Expr, var: &str, a: &Expr, b: &Expr) -> bool {
    let (Some(af), Some(bf)) = (point_f64(a), point_f64(b)) else {
        return false;
    };
    let Some(roots) = rational_poles(expr, var) else {
        return false;
    };
    roots.iter().any(|r| {
        let rf = Rat::to_f64(r);
        rf > af.min(bf) && rf < af.max(bf)
    })
}

/// Fold an expression made only of constants into a single rational (when possible).
fn fold_const(e: Expr) -> Expr {
    match e {
        Expr::Const(_) => e,
        Expr::Neg(a) => {
            let a = fold_const(*a);
            match a {
                Expr::Const(r) => Expr::constant(-r),
                other => Expr::Neg(Box::new(other)),
            }
        }
        Expr::Add(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x + y),
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x - y),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x * y),
                _ => Expr::Mul(Box::new(a), Box::new(b)),
            }
        }
        Expr::Div(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if !y.is_zero() => Expr::constant(x / y),
                _ => Expr::Div(Box::new(a), Box::new(b)),
            }
        }
        other => other,
    }
}

/// Compute the definite integral of `expr` with respect to `var` over `[a, b]`.
pub fn defint(expr: &Expr, var: &str, a: &Expr, b: &Expr) -> DefiniteResult {
    if has_interior_pole(expr, var, a, b) {
        return DefiniteResult::Divergent;
    }
    let f = integrate(expr, var);
    if matches!(f, Expr::Integral(_, _)) {
        return DefiniteResult::Unevaluated;
    }
    let ub = match crate::limit::limit(&f, var, b) {
        Some(v) => v,
        None => return DefiniteResult::Unevaluated,
    };
    let lb = match crate::limit::limit(&f, var, a) {
        Some(v) => v,
        None => return DefiniteResult::Unevaluated,
    };
    DefiniteResult::Value(fold_const(Expr::sub(ub, lb)))
}
