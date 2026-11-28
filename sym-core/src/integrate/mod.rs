//! Symbolic integration (Phase 24).
//!
//! `integrate(expr, var)` computes an antiderivative using linearity, the power
//! rule, elementary-function rules, rational integration via partial fractions, and
//! guarded integration by parts. When no rule applies it returns the unevaluated
//! `Expr::integral` noun form. Results are exact (no floats).

pub mod defint;
pub mod hermite;
pub mod ibp;
pub mod lrt;
pub mod risch;
pub mod risch_de;
pub mod sin;

use crate::expr::Expr;
use crate::poly::Poly;
use crate::rational::Rat;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

/// True when `expr` does not mention `var`.
pub(crate) fn is_constant_wrt(expr: &Expr, var: &str) -> bool {
    !expr.variables().iter().any(|v| v == var)
}

/// Extract a constant rational value, if `expr` is a numeric constant.
pub(crate) fn as_const(expr: &Expr) -> Option<BigRational> {
    match expr {
        Expr::Const(r) => Some(r.clone()),
        _ => None,
    }
}

/// Interpret `expr` as a linear form `a*var + b`; returns `(a, b_expr)`.
pub(crate) fn linear_in(expr: &Expr, var: &str) -> Option<(BigRational, Expr)> {
    let p = Poly::from_expr(expr).ok()?;
    if p.vars().iter().any(|v| v != var) || p.degree() > 1 {
        return None;
    }
    let mut a = BigRational::zero();
    let mut b = BigRational::zero();
    for t in p.terms() {
        match t.mono.total_degree() {
            0 => b = t.coeff.clone(),
            1 => a = t.coeff.clone(),
            _ => return None,
        }
    }
    Some((a, Expr::constant(b)))
}

/// Build the unevaluated integral noun form `∫ expr d(var)`.
pub(crate) fn noun(expr: &Expr, var: &str) -> Expr {
    Expr::integral(expr.clone(), Expr::var(var))
}

/// Outcome of an analytical integration attempt according to ADR-0012.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationResult {
    /// An exact analytical closed-form antiderivative was found.
    Solved(Expr),
    /// Formally proven to have no elementary antiderivative (Liouville's theorem / Risch).
    ProvenNonElementary,
    /// The integrand falls outside currently implemented algorithms; returns unevaluated noun form with reason.
    Unsupported(String),
}

/// Check if an expression contains an unevaluated integral noun form.
pub fn contains_integral(e: &Expr) -> bool {
    match e {
        Expr::Integral(_, _) => true,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            contains_integral(a) || contains_integral(b)
        }
        Expr::Neg(a)
        | Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::Exp(a)
        | Expr::Ln(a)
        | Expr::Sqrt(a)
        | Expr::Sinh(a)
        | Expr::Cosh(a)
        | Expr::Tanh(a)
        | Expr::Asin(a)
        | Expr::Acos(a)
        | Expr::Atan(a)
        | Expr::Abs(a)
        | Expr::Factorial(a) => contains_integral(a),
        Expr::Atan2(y, x) => contains_integral(y) || contains_integral(x),
        _ => false,
    }
}

fn is_var_exact(e: &Expr, var: &str) -> bool {
    matches!(e, Expr::Var(v) if v == var)
}

fn is_linear_in_var(e: &Expr, var: &str) -> bool {
    match e {
        Expr::Var(v) if v == var => true,
        Expr::Mul(a, b) => {
            (is_constant_wrt(a, var) && is_var_exact(b, var))
                || (is_constant_wrt(b, var) && is_var_exact(a, var))
        }
        Expr::Add(a, b) | Expr::Sub(a, b) => {
            (is_linear_in_var(a, var) && is_constant_wrt(b, var))
                || (is_linear_in_var(b, var) && is_constant_wrt(a, var))
        }
        _ => false,
    }
}

fn is_gaussian_like(arg: &Expr, var: &str) -> bool {
    match arg {
        Expr::Neg(inner) => is_gaussian_like(inner, var),
        Expr::Pow(base, exp) => {
            if is_var_exact(base, var)
                && let Expr::Const(r) = exp.as_ref()
            {
                return r.numer() >= &BigInt::from(2) && r.denom() == &BigInt::from(1);
            }
            false
        }
        Expr::Mul(a, b) => {
            (is_constant_wrt(a, var) && !a.is_zero() && is_gaussian_like(b, var))
                || (is_constant_wrt(b, var) && !b.is_zero() && is_gaussian_like(a, var))
        }
        _ => false,
    }
}

/// Check if an integrand is formally proven to have no elementary antiderivative (Liouville's theorem).
pub fn is_proven_non_elementary(expr: &Expr, var: &str) -> bool {
    // Reject constant-folded zero expressions such as exp(0*x^2)
    if let Expr::Exp(arg) = expr
        && let Ok(p) = Poly::from_expr(arg)
        && p.is_constant()
    {
        return false;
    }
    match expr {
        Expr::Exp(arg) => is_gaussian_like(arg, var),
        Expr::Div(num, den) => {
            if is_constant_wrt(num, var)
                && let Expr::Ln(inner) = den.as_ref()
                && is_linear_in_var(inner, var)
            {
                return true;
            }
            if is_var_exact(den, var) {
                match num.as_ref() {
                    Expr::Sin(arg) | Expr::Cos(arg) => is_linear_in_var(arg, var),
                    Expr::Exp(arg) => is_linear_in_var(arg, var),
                    _ => false,
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Integrate `expr` with respect to `var`, returning a structured [`IntegrationResult`].
pub fn integrate_result(expr: &Expr, var: &str) -> IntegrationResult {
    if is_proven_non_elementary(expr, var) {
        return IntegrationResult::ProvenNonElementary;
    }
    let res = integrate(expr, var);
    if contains_integral(&res) {
        IntegrationResult::Unsupported("no elementary antiderivative rule matched".to_string())
    } else {
        IntegrationResult::Solved(res)
    }
}

#[cfg(feature = "assume")]
use crate::assume::{Assumptions, Comparison, PredicateResult};

/// Integrate `expr` with respect to `var`, returning an antiderivative or noun.
pub fn integrate(expr: &Expr, var: &str) -> Expr {
    #[cfg(feature = "assume")]
    {
        integrate_with_assumptions(expr, var, None)
    }
    #[cfg(not(feature = "assume"))]
    {
        let result = integrate_core(expr, var, None);
        if matches!(&result, Expr::Integral(f, v) if **f == *expr && **v == Expr::var(var))
            && let Some(sub) = risch::try_risch(expr, var)
        {
            return sub;
        }
        result
    }
}

/// Integrate `expr` with respect to `var` with an optional [`Assumptions`] context.
#[cfg(feature = "assume")]
pub fn integrate_with_assumptions(
    expr: &Expr,
    var: &str,
    assumptions: Option<&Assumptions>,
) -> Expr {
    let result = integrate_core(expr, var, assumptions);
    if matches!(&result, Expr::Integral(f, v) if **f == *expr && **v == Expr::var(var))
        && let Some(sub) = risch::try_risch(expr, var)
    {
        return sub;
    }
    result
}

#[cfg(feature = "assume")]
fn integrate_core(expr: &Expr, var: &str, assumptions: Option<&Assumptions>) -> Expr {
    if is_constant_wrt(expr, var) {
        return Expr::mul(expr.clone(), Expr::var(var));
    }
    match expr {
        Expr::Var(name) if name == var => {
            Expr::div(Expr::pow(expr.clone(), Expr::int(2)), Expr::int(2))
        }
        Expr::Add(a, b) => Expr::add(
            integrate_with_assumptions(a, var, assumptions),
            integrate_with_assumptions(b, var, assumptions),
        ),
        Expr::Sub(a, b) => Expr::sub(
            integrate_with_assumptions(a, var, assumptions),
            integrate_with_assumptions(b, var, assumptions),
        ),
        Expr::Neg(a) => Expr::neg(integrate_with_assumptions(a, var, assumptions)),
        Expr::Mul(a, b) => integrate_product(expr, a, b, var, assumptions),
        Expr::Div(a, b) => integrate_quotient(expr, a, b, var),
        Expr::Pow(base, exp) => integrate_pow_with_assumptions(expr, base, exp, var, assumptions),
        _ => {
            if let Some(r) = sin::integrate_elementary(expr, var) {
                return r;
            }
            if let Some(r) = hermite::integrate_rational(expr, var) {
                return r;
            }
            if let Some(r) = ibp::try_ibp(expr, var) {
                return r;
            }
            noun(expr, var)
        }
    }
}

#[cfg(not(feature = "assume"))]
fn integrate_core(expr: &Expr, var: &str, _assumptions: Option<()>) -> Expr {
    if is_constant_wrt(expr, var) {
        return Expr::mul(expr.clone(), Expr::var(var));
    }
    match expr {
        Expr::Var(name) if name == var => {
            Expr::div(Expr::pow(expr.clone(), Expr::int(2)), Expr::int(2))
        }
        Expr::Add(a, b) => Expr::add(integrate(a, var), integrate(b, var)),
        Expr::Sub(a, b) => Expr::sub(integrate(a, var), integrate(b, var)),
        Expr::Neg(a) => Expr::neg(integrate(a, var)),
        Expr::Mul(a, b) => integrate_product(expr, a, b, var, assumptions),
        Expr::Div(a, b) => integrate_quotient(expr, a, b, var),
        Expr::Pow(base, exp) => integrate_pow(expr, base, exp, var),
        _ => {
            if let Some(r) = sin::integrate_elementary(expr, var) {
                return r;
            }
            if let Some(r) = hermite::integrate_rational(expr, var) {
                return r;
            }
            if let Some(r) = ibp::try_ibp(expr, var) {
                return r;
            }
            noun(expr, var)
        }
    }
}

#[cfg(feature = "assume")]
fn integrate_product(
    expr: &Expr,
    a: &Expr,
    b: &Expr,
    var: &str,
    assumptions: Option<&Assumptions>,
) -> Expr {
    if is_constant_wrt(a, var) {
        return Expr::mul(a.clone(), integrate_with_assumptions(b, var, assumptions));
    }
    if is_constant_wrt(b, var) {
        return Expr::mul(b.clone(), integrate_with_assumptions(a, var, assumptions));
    }
    if let Some(r) = ibp::try_ibp(expr, var) {
        return r;
    }
    // Expand polynomial/distributive products such as x*(x + 1)
    let expanded = crate::poly::expand(expr);
    if expanded != *expr {
        let res = integrate_core(&expanded, var, assumptions);
        if !contains_integral(&res) {
            return res;
        }
    }
    if let Some(r) = sin::integrate_elementary(expr, var) {
        return r;
    }
    noun(expr, var)
}

#[cfg(not(feature = "assume"))]
fn integrate_product(expr: &Expr, a: &Expr, b: &Expr, var: &str, _assumptions: Option<()>) -> Expr {
    if is_constant_wrt(a, var) {
        return Expr::mul(a.clone(), integrate(b, var));
    }
    if is_constant_wrt(b, var) {
        return Expr::mul(b.clone(), integrate(a, var));
    }
    if let Some(r) = ibp::try_ibp(expr, var) {
        return r;
    }
    let expanded = crate::poly::expand(expr);
    if expanded != *expr {
        let res = integrate_core(&expanded, var, None);
        if !contains_integral(&res) {
            return res;
        }
    }
    if let Some(r) = sin::integrate_elementary(expr, var) {
        return r;
    }
    noun(expr, var)
}

fn integrate_quotient(expr: &Expr, a: &Expr, b: &Expr, var: &str) -> Expr {
    if is_constant_wrt(b, var) {
        return Expr::div(integrate(a, var), b.clone());
    }
    if is_constant_wrt(a, var) {
        if let Some((coef, _)) = linear_in(b, var)
            && !coef.is_zero()
        {
            return lrt::log_over_linear(a, b, &coef);
        }
        if let Some(r) = integrate_power_denominator(a, b, var) {
            return r;
        }
    }
    if let Some(r) = hermite::integrate_rational(expr, var) {
        return r;
    }
    if let Some(r) = sin::integrate_elementary(expr, var) {
        return r;
    }
    noun(expr, var)
}

/// Handle `c / (a x + b)^n` for integer `n >= 2`.
fn integrate_power_denominator(a: &Expr, b: &Expr, var: &str) -> Option<Expr> {
    let Expr::Pow(base, exp) = b else {
        return None;
    };
    if !is_constant_wrt(exp, var) {
        return None;
    }
    let r = as_const(exp)?;
    if !Rat::is_integer(&r) {
        return None;
    }
    let n = r.numer().to_i64()?;
    if n < 2 {
        return None;
    }
    let (coef, _) = linear_in(base, var)?;
    if coef.is_zero() {
        return None;
    }
    let e1 = 1 - n;
    let scale = &coef * BigRational::from_integer(BigInt::from(e1));
    Some(Expr::mul(
        Expr::div(a.clone(), Expr::constant(scale)),
        Expr::pow((**base).clone(), Expr::int(e1)),
    ))
}

#[cfg(feature = "assume")]
fn integrate_pow_with_assumptions(
    expr: &Expr,
    base: &Expr,
    exp: &Expr,
    var: &str,
    assumptions: Option<&Assumptions>,
) -> Expr {
    if is_constant_wrt(exp, var)
        && let Some(ctx) = assumptions
        && matches!(base, Expr::Var(v) if v == var)
        && (ctx.is(exp, Comparison::NotEqual, &Expr::int(-1)) == PredicateResult::True
            || ctx.is(exp, Comparison::GreaterThan, &Expr::int(0)) == PredicateResult::True)
    {
        let new_exp = Expr::add(exp.clone(), Expr::int(1));
        return Expr::div(Expr::pow(base.clone(), new_exp.clone()), new_exp);
    }
    integrate_pow(expr, base, exp, var)
}

fn integrate_pow(expr: &Expr, base: &Expr, exp: &Expr, var: &str) -> Expr {
    if is_constant_wrt(exp, var)
        && let Some(n) = as_const(exp)
    {
        if n == Rat::from_int(2) {
            match base {
                Expr::Sin(u) => {
                    if let Some((coef, _)) = linear_in(u, var)
                        && !coef.is_zero()
                    {
                        // ∫ sin(a*x + b)^2 dx = x/2 - sin(2*(a*x + b))/(4*a)
                        let two_u = Expr::mul(Expr::int(2), (**u).clone());
                        let term1 = Expr::div(Expr::var(var), Expr::int(2));
                        let term2 =
                            Expr::div(Expr::sin(two_u), Expr::constant(&coef * Rat::from_int(4)));
                        return Expr::sub(term1, term2);
                    }
                }
                Expr::Cos(u) => {
                    if let Some((coef, _)) = linear_in(u, var)
                        && !coef.is_zero()
                    {
                        // ∫ cos(a*x + b)^2 dx = x/2 + sin(2*(a*x + b))/(4*a)
                        let two_u = Expr::mul(Expr::int(2), (**u).clone());
                        let term1 = Expr::div(Expr::var(var), Expr::int(2));
                        let term2 =
                            Expr::div(Expr::sin(two_u), Expr::constant(&coef * Rat::from_int(4)));
                        return Expr::add(term1, term2);
                    }
                }
                _ => {}
            }
        }
        if let Some((coef, _)) = linear_in(base, var)
            && !coef.is_zero()
        {
            if n == Rat::from_int(-1) {
                return lrt::log_over_linear(&Expr::int(1), base, &coef);
            }
            let new_exp = &n + BigRational::one();
            let scale = &coef * &new_exp;
            return Expr::div(
                Expr::pow(base.clone(), Expr::constant(new_exp)),
                Expr::constant(scale),
            );
        }
    }
    if let Some(r) = sin::integrate_elementary(expr, var) {
        return r;
    }
    noun(expr, var)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrates_variable_to_half_square() {
        let x = Expr::var("x");
        assert_eq!(
            integrate(&x, "x"),
            Expr::div(Expr::pow(x, Expr::int(2)), Expr::int(2))
        );
    }

    #[test]
    fn integrates_independent_of_var() {
        let a = Expr::var("a");
        assert_eq!(integrate(&a, "x"), Expr::mul(a, Expr::var("x")));
    }
}
