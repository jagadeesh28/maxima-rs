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
                Expr::Neg(inner) => *inner,
                other => Expr::Neg(Box::new(other)),
            }
        }
        Expr::Add(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x + y),
                (Expr::Const(x), other) if x.is_zero() => other.clone(),
                (other, Expr::Const(y)) if y.is_zero() => other.clone(),
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x - y),
                (Expr::Const(x), other) if x.is_zero() => match other {
                    Expr::Neg(inner) => *inner.clone(),
                    _ => Expr::Neg(Box::new(other.clone())),
                },
                (other, Expr::Const(y)) if y.is_zero() => other.clone(),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            let one = num_rational::BigRational::from_integer(num_bigint::BigInt::from(1));
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x * y),
                (Expr::Const(x), _) if x.is_zero() => Expr::int(0),
                (_, Expr::Const(y)) if y.is_zero() => Expr::int(0),
                (Expr::Const(x), other) if *x == one => other.clone(),
                (other, Expr::Const(y)) if *y == one => other.clone(),
                _ => Expr::Mul(Box::new(a), Box::new(b)),
            }
        }
        Expr::Div(a, b) => {
            let a = fold_const(*a);
            let b = fold_const(*b);
            let one = num_rational::BigRational::from_integer(num_bigint::BigInt::from(1));
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if !y.is_zero() => Expr::constant(x / y),
                (other, Expr::Const(y)) if *y == one => other.clone(),
                _ => Expr::Div(Box::new(a), Box::new(b)),
            }
        }
        other => other,
    }
}

fn flip_ln_if_negative(e: Expr, var: &str, test_val: f64) -> Expr {
    match e {
        Expr::Ln(inner) => {
            let mut env = std::collections::HashMap::new();
            env.insert(var.to_string(), test_val);
            if let Some(val) = inner.eval(&env)
                && val < 0.0
            {
                return Expr::ln(Expr::neg(*inner));
            }
            Expr::Ln(inner)
        }
        Expr::Add(a, b) => Expr::add(
            flip_ln_if_negative(*a, var, test_val),
            flip_ln_if_negative(*b, var, test_val),
        ),
        Expr::Sub(a, b) => Expr::sub(
            flip_ln_if_negative(*a, var, test_val),
            flip_ln_if_negative(*b, var, test_val),
        ),
        Expr::Mul(a, b) => Expr::mul(
            flip_ln_if_negative(*a, var, test_val),
            flip_ln_if_negative(*b, var, test_val),
        ),
        Expr::Div(a, b) => Expr::div(
            flip_ln_if_negative(*a, var, test_val),
            flip_ln_if_negative(*b, var, test_val),
        ),
        Expr::Neg(a) => Expr::neg(flip_ln_if_negative(*a, var, test_val)),
        Expr::Pow(a, b) => Expr::pow(
            flip_ln_if_negative(*a, var, test_val),
            flip_ln_if_negative(*b, var, test_val),
        ),
        other => other,
    }
}

/// Compute the definite integral of `expr` with respect to `var` over `[a, b]`.
pub fn defint(expr: &Expr, var: &str, a: &Expr, b: &Expr) -> DefiniteResult {
    if has_interior_pole(expr, var, a, b) {
        return DefiniteResult::Divergent;
    }
    let mut f = integrate(expr, var);
    if matches!(f, Expr::Integral(_, _)) {
        return DefiniteResult::Unevaluated;
    }

    if let (Some(af), Some(bf)) = (point_f64(a), point_f64(b))
        && af < 0.0
        && bf <= 0.0
    {
        let mid = (af + bf) / 2.0;
        f = flip_ln_if_negative(f, var, mid);
    }

    let ub = match crate::limit::limit(&f, var, b) {
        Some(v) => v,
        None => return DefiniteResult::Unevaluated,
    };
    let lb = match crate::limit::limit(&f, var, a) {
        Some(v) => v,
        None => return DefiniteResult::Unevaluated,
    };
    if matches!(&ub, Expr::Var(v) if v == "inf" || v == "minf")
        || matches!(&lb, Expr::Var(v) if v == "inf" || v == "minf")
    {
        return DefiniteResult::Divergent;
    }
    DefiniteResult::Value(fold_const(Expr::sub(ub, lb)))
}
