//! Partial fraction decomposition over distinct linear rational factors.
//!
//! Given `num/den` where `den` splits completely into distinct linear factors with
//! rational roots, the cover-up rule yields `num/den = Σ A_r / (x - r)` with
//! `A_r = num(r) / den'(r)`. Non-splitting denominators are returned unchanged.

use super::factor::{distinct_rational_roots, eval_poly};
use super::gcd::poly_derivative;
use super::rat::to_fraction;
use super::{PolyError, VarContext};
use crate::expr::Expr;
use num_rational::BigRational;
use num_traits::Zero;

/// Decompose `expr` into partial fractions with respect to `var`.
pub fn partfrac(expr: &Expr, var: &str) -> Result<Expr, PolyError> {
    let mut names = expr.variables();
    names.sort();
    let mut ctx = VarContext::new();
    for n in &names {
        ctx.add(n);
    }
    if ctx.index_of(var).is_none() {
        ctx.add(var);
    }

    let f = to_fraction(expr, &ctx)?;
    if f.den.is_zero() {
        return Err(PolyError::DivisionByZero);
    }
    let num = f.num.clone();
    let den = f.den.clone();
    if den.degree() == 0 {
        return Ok(f.to_expr());
    }
    if den.nvars() > 1 || num.degree() >= den.degree() {
        return Ok(expr.clone());
    }

    let Some(roots) = distinct_rational_roots(&den) else {
        return Ok(expr.clone());
    };
    if roots.len() != den.degree() as usize {
        return Ok(expr.clone());
    }

    let dden = poly_derivative(&den);
    let var_expr = Expr::var(var);
    let mut terms: Vec<Expr> = Vec::new();
    for r in roots {
        let den_at = eval_poly(&dden, &r);
        if den_at.is_zero() {
            return Ok(expr.clone());
        }
        let a: BigRational = eval_poly(&num, &r) / den_at;
        if a.is_zero() {
            continue;
        }
        let denom = Expr::sub(var_expr.clone(), Expr::constant(r));
        terms.push(Expr::div(Expr::constant(a), denom));
    }

    let mut it = terms.into_iter();
    match it.next() {
        None => Ok(Expr::int(0)),
        Some(first) => Ok(it.fold(first, Expr::add)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn eval_at(e: &Expr, x: f64) -> f64 {
        let mut env = HashMap::new();
        env.insert("x".to_string(), x);
        e.eval(&env).expect("eval")
    }

    #[test]
    fn partfrac_one_over_x_squared_minus_one() {
        let x = Expr::var("x");
        let e = Expr::div(
            Expr::int(1),
            Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1)),
        );
        let out = partfrac(&e, "x").unwrap();
        for xv in [-3.0_f64, -0.5, 0.0, 2.0, 4.0] {
            assert!((eval_at(&out, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }
}
