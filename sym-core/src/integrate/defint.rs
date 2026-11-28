//! Definite integration via the Fundamental Theorem of Calculus (Phase 28).
//!
//! `defint(f, x, a, b)` computes `F(b) - F(a)` where `F = ∫ f dx`, but first scans
//! for rational poles inside the integration interval and refuses to apply FTC
//! across them (returning [`DefiniteResult::Divergent`]). Improper bounds use the
//! [`crate::limit`] engine.

use super::{contains_integral, integrate};
use crate::expr::Expr;
use crate::limit::{LimitDir, limit_dir};
use crate::poly::VarContext;
use crate::poly::factor::{distinct_rational_roots, factor_poly_list};
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

fn rational_poles(expr: &Expr, var: &str) -> Option<(Vec<BigRational>, Vec<f64>)> {
    let mut names = expr.variables();
    names.sort();
    let mut ctx = VarContext::new();
    for n in &names {
        ctx.add(n);
    }
    let _ = var;
    let f = to_fraction(expr, &ctx).ok()?.reduce();
    let rat_roots = distinct_rational_roots(&f.den).unwrap_or_default();
    let mut irrat_roots = Vec::new();
    for (fac, _) in factor_poly_list(&f.den).1 {
        if fac.degree() == 2 {
            // Check quadratic a*x^2 + b*x + c real roots
            let mut a = 0.0_f64;
            let mut b = 0.0_f64;
            let mut c = 0.0_f64;
            for t in fac.terms() {
                match t.mono.total_degree() {
                    2 => a = Rat::to_f64(&t.coeff),
                    1 => b = Rat::to_f64(&t.coeff),
                    0 => c = Rat::to_f64(&t.coeff),
                    _ => {}
                }
            }
            let disc = b * b - 4.0 * a * c;
            if disc > 0.0 && a != 0.0 {
                let sq = disc.sqrt();
                irrat_roots.push((-b - sq) / (2.0 * a));
                irrat_roots.push((-b + sq) / (2.0 * a));
            }
        }
    }
    Some((rat_roots, irrat_roots))
}

fn has_transcendental_pole(expr: &Expr, var: &str, lo: f64, hi: f64) -> bool {
    match expr {
        Expr::Tan(arg) => {
            if matches!(arg.as_ref(), Expr::Var(v) if v == var) {
                let pi = std::f64::consts::PI;
                let k_min = ((lo - pi / 2.0) / pi).ceil() as i64;
                let k_max = ((hi - pi / 2.0) / pi).floor() as i64;
                if k_min <= k_max {
                    for k in k_min..=k_max {
                        let pole = (k as f64 + 0.5) * pi;
                        if pole > lo && pole < hi {
                            return true;
                        }
                    }
                }
            }
            has_transcendental_pole(arg, var, lo, hi)
        }
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            has_transcendental_pole(a, var, lo, hi) || has_transcendental_pole(b, var, lo, hi)
        }
        Expr::Neg(a) | Expr::Sin(a) | Expr::Cos(a) | Expr::Exp(a) | Expr::Ln(a) | Expr::Sqrt(a) => {
            has_transcendental_pole(a, var, lo, hi)
        }
        _ => false,
    }
}

fn has_interior_pole(expr: &Expr, var: &str, a: &Expr, b: &Expr) -> bool {
    let (Some(af), Some(bf)) = (point_f64(a), point_f64(b)) else {
        return false;
    };
    let lo = af.min(bf);
    let hi = af.max(bf);
    if has_transcendental_pole(expr, var, lo, hi) {
        return true;
    }
    let Some((rat_roots, irrat_roots)) = rational_poles(expr, var) else {
        return false;
    };
    rat_roots.iter().any(|r| {
        let rf = Rat::to_f64(r);
        rf > lo && rf < hi
    }) || irrat_roots.iter().any(|&rf| rf > lo && rf < hi)
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
    if contains_integral(&f) {
        return DefiniteResult::Unevaluated;
    }

    if let (Some(af), Some(bf)) = (point_f64(a), point_f64(b))
        && af < 0.0
        && bf <= 0.0
    {
        let mid = (af + bf) / 2.0;
        f = flip_ln_if_negative(f, var, mid);
    }

    // Use directional limits at finite endpoints (handles endpoint poles like 1/x on [0, 1])
    let dir_a = match (point_f64(a), point_f64(b)) {
        (Some(af), Some(bf)) if af.is_finite() && bf.is_finite() && af <= bf => LimitDir::Plus,
        (Some(af), Some(bf)) if af.is_finite() && bf.is_finite() => LimitDir::Minus,
        _ => LimitDir::Both,
    };
    let dir_b = match (point_f64(a), point_f64(b)) {
        (Some(af), Some(bf)) if af.is_finite() && bf.is_finite() && af <= bf => LimitDir::Minus,
        (Some(af), Some(bf)) if af.is_finite() && bf.is_finite() => LimitDir::Plus,
        _ => LimitDir::Both,
    };

    let ub = match limit_dir(&f, var, b, dir_b) {
        Some(v) => v,
        None => {
            // Check if endpoint b is a pole of the integrand
            if let (Some(bf), Some((rat_roots, irrat_roots))) =
                (point_f64(b), rational_poles(expr, var))
                && (rat_roots
                    .iter()
                    .any(|r| (Rat::to_f64(r) - bf).abs() < 1e-12)
                    || irrat_roots.iter().any(|&rf| (rf - bf).abs() < 1e-12))
            {
                return DefiniteResult::Divergent;
            }
            return DefiniteResult::Unevaluated;
        }
    };
    let lb = match limit_dir(&f, var, a, dir_a) {
        Some(v) => v,
        None => {
            if let (Some(af), Some((rat_roots, irrat_roots))) =
                (point_f64(a), rational_poles(expr, var))
                && (rat_roots
                    .iter()
                    .any(|r| (Rat::to_f64(r) - af).abs() < 1e-12)
                    || irrat_roots.iter().any(|&rf| (rf - af).abs() < 1e-12))
            {
                return DefiniteResult::Divergent;
            }
            return DefiniteResult::Unevaluated;
        }
    };
    if matches!(&ub, Expr::Var(v) if v == "inf" || v == "minf" || v == "und" || v == "ind")
        || matches!(&lb, Expr::Var(v) if v == "inf" || v == "minf" || v == "und" || v == "ind")
    {
        return DefiniteResult::Divergent;
    }
    DefiniteResult::Value(fold_const(Expr::sub(ub, lb)))
}
