//! Symbolic integration (Phase 24).
//!
//! `integrate(expr, var)` computes an antiderivative using linearity, the power
//! rule, elementary-function rules, rational integration via partial fractions, and
//! guarded integration by parts. When no rule applies it returns the unevaluated
//! `Expr::integral` noun form. Results are exact (no floats).

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

/// Integrate `expr` with respect to `var`, returning an antiderivative or noun.
pub fn integrate(expr: &Expr, var: &str) -> Expr {
    let result = integrate_core(expr, var);
    if matches!(&result, Expr::Integral(f, v) if **f == *expr && **v == Expr::var(var))
        && let Some(sub) = risch::try_risch(expr, var)
    {
        return sub;
    }
    result
}

fn integrate_core(expr: &Expr, var: &str) -> Expr {
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
        Expr::Mul(a, b) => integrate_product(expr, a, b, var),
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

fn integrate_product(expr: &Expr, a: &Expr, b: &Expr, var: &str) -> Expr {
    if is_constant_wrt(a, var) {
        return Expr::mul(a.clone(), integrate(b, var));
    }
    if is_constant_wrt(b, var) {
        return Expr::mul(b.clone(), integrate(a, var));
    }
    if let Some(r) = ibp::try_ibp(expr, var) {
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
    if n < 1 {
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

fn integrate_pow(expr: &Expr, base: &Expr, exp: &Expr, var: &str) -> Expr {
    if is_constant_wrt(exp, var)
        && let Some(n) = as_const(exp)
        && let Some((coef, _)) = linear_in(base, var)
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
