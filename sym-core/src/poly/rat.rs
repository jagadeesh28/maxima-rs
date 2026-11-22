//! Canonical rational simplification (`ratsimp`).
//!
//! An [`Expr`] is converted into a quotient of two [`Poly`] values, reduced by the
//! polynomial GCD, normalized so the denominator is monic, and converted back.

use super::gcd::{poly_div_exact, poly_gcd};
use super::{Monomial, Poly, PolyError, VarContext};
use crate::expr::Expr;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

/// A quotient of two polynomials, `num / den`.
#[derive(Clone, Debug)]
pub struct Fraction {
    /// Numerator polynomial.
    pub num: Poly,
    /// Denominator polynomial (non-zero).
    pub den: Poly,
}

impl Fraction {
    /// Wrap a polynomial as a fraction over `1`.
    pub fn from_poly(p: Poly) -> Self {
        let den = Poly::constant(BigRational::one(), p.ctx.clone());
        Fraction { num: p, den }
    }

    fn add(&self, other: &Fraction) -> Fraction {
        Fraction {
            num: self.num.mul(&other.den).add(&other.num.mul(&self.den)),
            den: self.den.mul(&other.den),
        }
    }

    fn sub(&self, other: &Fraction) -> Fraction {
        self.add(&other.neg())
    }

    fn mul(&self, other: &Fraction) -> Fraction {
        Fraction {
            num: self.num.mul(&other.num),
            den: self.den.mul(&other.den),
        }
    }

    fn div(&self, other: &Fraction) -> Fraction {
        Fraction {
            num: self.num.mul(&other.den),
            den: self.den.mul(&other.num),
        }
    }

    fn neg(&self) -> Fraction {
        Fraction {
            num: self.num.neg(),
            den: self.den.clone(),
        }
    }

    /// Reduce to lowest terms with a monic denominator.
    pub fn reduce(self) -> Fraction {
        let ctx = self.den.ctx.clone();
        if self.num.is_zero() {
            return Fraction {
                num: Poly::zero(ctx.clone()),
                den: Poly::constant(BigRational::one(), ctx),
            };
        }
        let g = poly_gcd(&self.num, &self.den);
        let (num, den) = if g.degree() > 0 {
            (
                poly_div_exact(&self.num, &g).unwrap_or(self.num),
                poly_div_exact(&self.den, &g).unwrap_or(self.den),
            )
        } else {
            (self.num, self.den)
        };
        match den.leading_term() {
            Some(lt) if !lt.coeff.is_zero() => {
                let scale = BigRational::one() / lt.coeff.clone();
                Fraction {
                    num: num.scale(&scale),
                    den: den.scale(&scale),
                }
            }
            _ => Fraction { num, den },
        }
    }

    /// Convert to an [`Expr`], folding a constant denominator into the numerator.
    pub fn to_expr(self) -> Expr {
        let f = self.reduce();
        match f.den.constant_value() {
            Some(c) if c.is_zero() => Expr::int(0),
            Some(c) => f.num.scale(&(BigRational::one() / c)).to_expr(),
            None => Expr::div(f.num.to_expr(), f.den.to_expr()),
        }
    }
}

fn variable_fraction(name: &str, ctx: &VarContext) -> Result<Fraction, PolyError> {
    let idx = ctx
        .index_of(name)
        .ok_or(PolyError::NotPolynomial("unknown variable"))?;
    let mut exps = vec![0u32; ctx.len()];
    exps[idx] = 1;
    Ok(Fraction::from_poly(Poly::monomial(
        BigRational::one(),
        Monomial::from_exps(exps),
        ctx.clone(),
    )))
}

/// Convert an expression into a polynomial fraction over a fixed context.
pub fn to_fraction(expr: &Expr, ctx: &VarContext) -> Result<Fraction, PolyError> {
    match expr {
        Expr::Const(r) => Ok(Fraction::from_poly(Poly::constant(r.clone(), ctx.clone()))),
        Expr::Var(name) => variable_fraction(name, ctx),
        Expr::Add(a, b) => Ok(to_fraction(a, ctx)?.add(&to_fraction(b, ctx)?)),
        Expr::Sub(a, b) => Ok(to_fraction(a, ctx)?.sub(&to_fraction(b, ctx)?)),
        Expr::Mul(a, b) => Ok(to_fraction(a, ctx)?.mul(&to_fraction(b, ctx)?)),
        Expr::Div(a, b) => Ok(to_fraction(a, ctx)?.div(&to_fraction(b, ctx)?)),
        Expr::Neg(a) => Ok(to_fraction(a, ctx)?.neg()),
        Expr::Pow(base, exp) => {
            let f = to_fraction(base, ctx)?;
            match exp.as_ref() {
                Expr::Const(r) if Rat::is_integer(r) => {
                    let n = r.numer().to_i64().ok_or(PolyError::ExponentOverflow)?;
                    let n_abs =
                        u32::try_from(n.unsigned_abs()).map_err(|_| PolyError::ExponentOverflow)?;
                    if n >= 0 {
                        Ok(Fraction {
                            num: f.num.pow(n_abs),
                            den: f.den.pow(n_abs),
                        })
                    } else {
                        Ok(Fraction {
                            num: f.den.pow(n_abs),
                            den: f.num.pow(n_abs),
                        })
                    }
                }
                _ => Err(PolyError::NotPolynomial("non-integer exponent")),
            }
        }
        _ => Err(PolyError::NotPolynomial("non-polynomial expression")),
    }
}

/// Canonically simplify an expression to a single reduced fraction.
pub fn ratsimp(expr: &Expr) -> Result<Expr, PolyError> {
    let mut names = expr.variables();
    names.sort();
    let mut ctx = VarContext::new();
    for n in &names {
        ctx.add(n);
    }
    Ok(to_fraction(expr, &ctx)?.to_expr())
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
    fn ratsimp_cancels_common_factor() {
        let x = Expr::var("x");
        let e = Expr::div(
            Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
            Expr::sub(x, Expr::int(1)),
        );
        let out = ratsimp(&e).unwrap();
        assert!((eval_at(&out, 3.0) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn ratsimp_combines_fractions() {
        let x = Expr::var("x");
        let e = Expr::add(
            Expr::div(Expr::int(1), x.clone()),
            Expr::div(Expr::int(1), x),
        );
        let out = ratsimp(&e).unwrap();
        assert!((eval_at(&out, 2.0) - 1.0).abs() < 1e-12);
    }
}
