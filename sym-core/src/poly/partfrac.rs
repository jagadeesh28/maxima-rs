//! Partial fraction decomposition over Q(x).
//!
//! Handles unreduced fractions, improper rational functions (polynomial long
//! division), repeated linear poles, and irreducible quadratic/higher-degree
//! factors via Yun square-free + Cantor-Zassenhaus factorization and extended
//! Euclidean splitting.

use super::factor::factor_poly_list;
use super::gcd::{poly_div_exact, poly_divrem};
use super::rat::to_fraction;
use super::{Poly, PolyError, VarContext};
use crate::expr::Expr;
use num_rational::BigRational;
use num_traits::{One, Zero};

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

    let f = to_fraction(expr, &ctx)?.reduce();
    if f.den.is_zero() {
        return Err(PolyError::DivisionByZero);
    }
    if f.num.is_zero() {
        return Ok(Expr::int(0));
    }
    if f.den.degree() == 0 {
        return Ok(f.to_expr());
    }
    if f.den.nvars() > 1 {
        return Ok(f.to_expr());
    }

    // 1. Long division if improper: num = q * den + rem
    let (quot, rem) = if f.num.degree() >= f.den.degree() {
        poly_divrem(&f.num, &f.den).unwrap_or_else(|| (Poly::zero(ctx.clone()), f.num.clone()))
    } else {
        (Poly::zero(ctx.clone()), f.num.clone())
    };

    let mut terms: Vec<Expr> = Vec::new();
    if !quot.is_zero() {
        terms.push(quot.to_expr());
    }
    if rem.is_zero() {
        return Ok(sum_exprs(terms));
    }

    // 2. Factor denominator: den = lead * prod(f_i ^ e_i)
    let (lead, factors) = factor_poly_list(&f.den);
    if factors.is_empty() {
        terms.push(Expr::div(rem.to_expr(), f.den.to_expr()));
        return Ok(sum_exprs(terms));
    }

    let rem_scaled = if Rat::is_one(&lead) {
        rem
    } else {
        rem.scale(&(BigRational::one() / lead))
    };

    // 3. Split rem_scaled / prod(f_i^e_i) into sum of U_i / f_i^e_i via Bezout
    let mut power_factors: Vec<(Poly, Poly, u32)> = Vec::with_capacity(factors.len());
    for (fac, mult) in factors {
        let mut p_pow = fac.clone();
        for _ in 1..mult {
            p_pow = p_pow.mul(&fac);
        }
        power_factors.push((fac, p_pow, mult));
    }

    let split_parts = split_coprime_powers(&rem_scaled, &power_factors);
    for ((fac, _, mult), u_i) in power_factors.into_iter().zip(split_parts) {
        if u_i.is_zero() {
            continue;
        }
        // Expand U_i in base `fac` to get sum_{k=1..=mult} R_k / fac^k
        let mut cur = u_i;
        let mut remainders = vec![Poly::zero(ctx.clone()); mult as usize];
        for slot in remainders.iter_mut().rev() {
            if cur.is_zero() {
                break;
            }
            if let Some((q, r)) = poly_divrem(&cur, &fac) {
                *slot = r;
                cur = q;
            } else {
                *slot = cur;
                break;
            }
        }
        for (idx, r_k) in remainders.into_iter().enumerate() {
            if r_k.is_zero() {
                continue;
            }
            let power = (idx + 1) as u32;
            let denom_expr = if power == 1 {
                fac.to_expr()
            } else {
                Expr::pow(fac.to_expr(), Expr::int(i64::from(power)))
            };
            terms.push(Expr::div(r_k.to_expr(), denom_expr));
        }
    }

    Ok(sum_exprs(terms))
}

use crate::rational::Rat;

fn sum_exprs(terms: Vec<Expr>) -> Expr {
    let mut it = terms.into_iter();
    match it.next() {
        None => Expr::int(0),
        Some(first) => it.fold(first, Expr::add),
    }
}

/// Extended Euclidean algorithm for univariate `Poly` over Q: returns `(g, s, t)` with `s*a + t*b = g`.
fn poly_ext_gcd(a: &Poly, b: &Poly) -> (Poly, Poly, Poly) {
    let ctx = a.ctx.clone();
    let mut old_r = a.clone();
    let mut r = b.clone();
    let mut old_s = Poly::constant(BigRational::one(), ctx.clone());
    let mut s = Poly::zero(ctx.clone());
    let mut old_t = Poly::zero(ctx.clone());
    let mut t = Poly::constant(BigRational::one(), ctx);

    while !r.is_zero() {
        let Some((q, rem)) = poly_divrem(&old_r, &r) else {
            break;
        };
        old_r = r;
        r = rem;
        let ns = old_s.sub(&q.mul(&s));
        old_s = s;
        s = ns;
        let nt = old_t.sub(&q.mul(&t));
        old_t = t;
        t = nt;
    }

    if let Some(lt) = old_r.leading_term()
        && !lt.coeff.is_zero()
    {
        let inv = BigRational::one() / lt.coeff.clone();
        old_r = old_r.scale(&inv);
        old_s = old_s.scale(&inv);
        old_t = old_t.scale(&inv);
    }
    (old_r, old_s, old_t)
}

fn split_coprime_powers(num: &Poly, factors: &[(Poly, Poly, u32)]) -> Vec<Poly> {
    if factors.len() == 1 {
        return vec![num.clone()];
    }
    let ctx = num.ctx.clone();
    let mut result = Vec::with_capacity(factors.len());
    let mut cur_num = num.clone();

    for i in 0..(factors.len() - 1) {
        let a = &factors[i].1;
        let mut b = Poly::constant(BigRational::one(), ctx.clone());
        for f in &factors[(i + 1)..] {
            b = b.mul(&f.1);
        }
        let (g, s, t) = poly_ext_gcd(a, &b);
        // s*a + t*b = g (g should be 1)
        let cur_scaled = if g.degree() == 0 && !g.is_zero() {
            let c = g.constant_value().unwrap_or_else(BigRational::one);
            cur_num.scale(&(BigRational::one() / c))
        } else {
            poly_div_exact(&cur_num, &g).unwrap_or(cur_num)
        };
        let (_, u_i) = poly_divrem(&cur_scaled.mul(&t), a)
            .unwrap_or_else(|| (Poly::zero(ctx.clone()), cur_scaled.clone()));
        let (_, next_num) = poly_divrem(&cur_scaled.mul(&s), &b)
            .unwrap_or_else(|| (Poly::zero(ctx.clone()), Poly::zero(ctx.clone())));
        result.push(u_i);
        cur_num = next_num;
    }
    result.push(cur_num);
    result
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

    #[test]
    fn partfrac_repeated_and_irreducible_quadratic() {
        // 1 / ((x - 1)^2 * (x^2 + 1))
        let x = Expr::var("x");
        let e = Expr::div(
            Expr::int(1),
            Expr::mul(
                Expr::pow(Expr::sub(x.clone(), Expr::int(1)), Expr::int(2)),
                Expr::add(Expr::pow(x, Expr::int(2)), Expr::int(1)),
            ),
        );
        let out = partfrac(&e, "x").unwrap();
        assert!(
            matches!(out, Expr::Add(_, _)),
            "expected decomposed sum, got {out}"
        );
        for xv in [-2.0_f64, -0.5, 0.0, 2.0, 3.0] {
            assert!((eval_at(&out, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }
}
