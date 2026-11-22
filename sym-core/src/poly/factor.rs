//! Univariate polynomial factorization via the rational-root theorem.
//!
//! Extracts linear factors `(x - r)` for rational roots `r` and returns any
//! irreducible residual factor unchanged. Full Berlekamp/Hensel lifting is a planned
//! refinement; this pragmatic engine covers the common CAS cases.

use super::gcd::poly_div_exact;
use super::{Monomial, Poly, VarContext};
use crate::expr::Expr;
use crate::rational::Rat;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

/// Factor an expression over Q when it is a univariate integer polynomial.
pub fn factor(expr: &Expr) -> Expr {
    match Poly::from_expr(expr) {
        Ok(p) => factor_poly(&p),
        Err(_) => expr.clone(),
    }
}

fn factor_poly(p: &Poly) -> Expr {
    if p.degree() == 0 || p.nvars() != 1 {
        return p.to_expr();
    }
    if univariate_i64_coeffs(p).is_none() {
        return p.to_expr();
    }
    let mut current = p.clone();
    let mut factors: Vec<Expr> = Vec::new();
    loop {
        if current.degree() == 0 {
            break;
        }
        let Some(coeffs) = univariate_i64_coeffs(&current) else {
            break;
        };
        let Some(root) = find_root(&current, &coeffs) else {
            break;
        };
        let lin = linear_poly(&root, &current.ctx);
        let Some(q) = poly_div_exact(&current, &lin) else {
            break;
        };
        factors.push(lin.to_expr());
        current = q;
    }

    if current.degree() > 0 {
        factors.push(current.to_expr());
    } else if let Some(c) = current.constant_value()
        && !Rat::is_one(&c)
    {
        factors.push(Expr::constant(c));
    }
    multiply_exprs(factors)
}

fn multiply_exprs(parts: Vec<Expr>) -> Expr {
    let mut it = parts.into_iter();
    match it.next() {
        None => Expr::int(1),
        Some(first) => it.fold(first, Expr::mul),
    }
}

/// Integer coefficients (descending degree) of a univariate polynomial, if exact.
pub(crate) fn univariate_i64_coeffs(p: &Poly) -> Option<Vec<i64>> {
    if p.nvars() != 1 {
        return None;
    }
    let deg = p.degree() as usize;
    let mut coeffs = vec![0i64; deg + 1];
    for t in p.terms() {
        if !Rat::is_integer(&t.coeff) {
            return None;
        }
        let v = t.coeff.numer().to_i64()?;
        let e = t.mono.exponent(0) as usize;
        coeffs[deg - e] = coeffs[deg - e].checked_add(v)?;
    }
    Some(coeffs)
}

fn divisors(n: i64) -> Vec<i64> {
    let m = n.abs();
    if m == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut d = 1i64;
    while d * d <= m {
        if m % d == 0 {
            out.push(d);
            if d != m / d {
                out.push(m / d);
            }
        }
        d += 1;
    }
    out.sort_unstable();
    out
}

/// Candidate rational roots `p/q` of an integer polynomial (descending coeffs).
fn rational_root_candidates(coeffs: &[i64]) -> Vec<BigRational> {
    let deg = coeffs.len() - 1;
    let constant = coeffs[deg];
    if constant == 0 {
        return vec![BigRational::zero()];
    }
    let leading = coeffs[0];
    if leading == 0 {
        return Vec::new();
    }
    let mut out: Vec<BigRational> = Vec::new();
    for pn in divisors(constant) {
        for qn in divisors(leading) {
            if qn == 0 {
                continue;
            }
            let r = BigRational::new(BigInt::from(pn), BigInt::from(qn));
            out.push(r.clone());
            out.push(-r);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn find_root(p: &Poly, coeffs: &[i64]) -> Option<BigRational> {
    rational_root_candidates(coeffs)
        .into_iter()
        .find(|r| eval_poly(p, r).is_zero())
}

/// Evaluate a univariate polynomial at a rational point.
pub(crate) fn eval_poly(p: &Poly, r: &BigRational) -> BigRational {
    let mut acc = BigRational::zero();
    for t in p.terms() {
        let e = t.mono.exponent(0);
        let mut pw = BigRational::one();
        for _ in 0..e {
            pw = &pw * r;
        }
        acc = &acc + &(&t.coeff * &pw);
    }
    acc
}

fn linear_poly(r: &BigRational, ctx: &VarContext) -> Poly {
    let mut exps = vec![0u32; ctx.len()];
    exps[0] = 1;
    let x = Poly::monomial(BigRational::one(), Monomial::from_exps(exps), ctx.clone());
    let c = Poly::constant(-r.clone(), ctx.clone());
    x.add(&c)
}

/// All rational roots of a univariate polynomial (with multiplicity).
pub(crate) fn distinct_rational_roots(p: &Poly) -> Option<Vec<BigRational>> {
    let mut cur = p.clone();
    let mut roots = Vec::new();
    loop {
        if cur.degree() == 0 {
            break;
        }
        let coeffs = univariate_i64_coeffs(&cur)?;
        let Some(root) = find_root(&cur, &coeffs) else {
            break;
        };
        let lin = linear_poly(&root, &cur.ctx);
        let Some(q) = poly_div_exact(&cur, &lin) else {
            break;
        };
        roots.push(root);
        cur = q;
    }
    Some(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::expand::expand;
    use std::collections::HashMap;

    fn eval_at(e: &Expr, x: f64) -> f64 {
        let mut env = HashMap::new();
        env.insert("x".to_string(), x);
        e.eval(&env).expect("eval")
    }

    #[test]
    fn factor_difference_of_squares() {
        let x = Expr::var("x");
        let e = Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1));
        let f = factor(&e);
        let back = expand(&f);
        for xv in [-3.0_f64, -0.5, 0.0, 2.0] {
            assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }

    #[test]
    fn factor_cubic_with_zero_root() {
        let x = Expr::var("x");
        let e = Expr::sub(Expr::pow(x.clone(), Expr::int(3)), x);
        let f = factor(&e);
        let back = expand(&f);
        for xv in [-2.0_f64, 0.0, 1.5, 3.0] {
            assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }
}
