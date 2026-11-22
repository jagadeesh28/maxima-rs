//! Sparse distributed multivariate polynomials over exact rationals.
//!
//! This module implements the polynomial engine backing `expand`, `ratsimp`,
//! `factor`, and `partfrac` (see `agents.md`, Phases 22–23). Polynomials are stored
//! as a list of terms sorted in descending graded reverse lexicographic (`grevlex`)
//! order, with coefficients in `BigRational` for exact arithmetic.
//!
//! Polynomial manipulation is **procedural** and deliberately bypasses the e-graph:
//! `expand` must not be undone by cost-based extraction (ADR-0006, Guardrail 1).

pub mod expand;
pub mod factor;
pub mod gcd;
pub mod heap;
pub mod monomial;
pub mod partfrac;
pub mod rat;

pub use expand::expand;
pub use factor::factor;
pub use gcd::{poly_derivative, poly_div_exact, poly_gcd};
pub use heap::{heap_mul, pseudo_divmod};
pub use monomial::{Monomial, PACKED_MAX_VARS, VarContext};
pub use partfrac::partfrac;
pub use rat::{Fraction, ratsimp};

use crate::expr::Expr;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::fmt;

/// A single polynomial term: `coeff * mono`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    /// Exact rational coefficient.
    pub coeff: BigRational,
    /// Monomial part.
    pub mono: Monomial,
}

/// Errors that arise when converting an [`Expr`] into a [`Poly`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolyError {
    /// The expression contains a construct that is not polynomial.
    NotPolynomial(&'static str),
    /// Division by an expression that depends on variables.
    DivisionByPolynomial,
    /// Division by zero.
    DivisionByZero,
    /// A negative exponent was used.
    NegativeExponent,
    /// An exponent was too large to represent.
    ExponentOverflow,
}

impl fmt::Display for PolyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolyError::NotPolynomial(msg) => write!(f, "not a polynomial: {msg}"),
            PolyError::DivisionByPolynomial => write!(f, "division by a non-constant polynomial"),
            PolyError::DivisionByZero => write!(f, "division by zero"),
            PolyError::NegativeExponent => write!(f, "negative exponent"),
            PolyError::ExponentOverflow => write!(f, "exponent too large"),
        }
    }
}

impl std::error::Error for PolyError {}

/// A sparse distributed multivariate polynomial.
#[derive(Debug, Clone)]
pub struct Poly {
    pub(crate) terms: Vec<Term>,
    pub(crate) ctx: VarContext,
}

impl Poly {
    /// The zero polynomial over `ctx`.
    pub fn zero(ctx: VarContext) -> Self {
        Poly {
            terms: Vec::new(),
            ctx,
        }
    }

    /// A constant polynomial.
    pub fn constant(coeff: BigRational, ctx: VarContext) -> Self {
        Poly::from_terms(
            vec![Term {
                coeff,
                mono: Monomial::one(ctx.len()),
            }],
            ctx,
        )
    }

    /// A polynomial consisting of a single term.
    pub fn monomial(coeff: BigRational, mono: Monomial, ctx: VarContext) -> Self {
        Poly::from_terms(vec![Term { coeff, mono }], ctx)
    }

    /// Build a polynomial from a raw term list, normalizing it.
    pub fn from_terms(terms: Vec<Term>, ctx: VarContext) -> Self {
        Poly {
            terms: normalize_terms(terms),
            ctx,
        }
    }

    /// Variable names in exponent-vector index order.
    pub fn vars(&self) -> &[String] {
        self.ctx.vars()
    }

    /// Number of variables.
    pub fn nvars(&self) -> usize {
        self.ctx.len()
    }

    /// The terms in descending `grevlex` order.
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// Leading term (largest under `grevlex`), if any.
    pub fn leading_term(&self) -> Option<&Term> {
        self.terms.first()
    }

    /// True when this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// True when this polynomial has no variables (a constant, including zero).
    pub fn is_constant(&self) -> bool {
        self.terms.is_empty() || (self.terms.len() == 1 && self.terms[0].mono.is_one())
    }

    /// The constant value, if this is a constant polynomial (zero included).
    pub fn constant_value(&self) -> Option<BigRational> {
        if self.terms.is_empty() {
            Some(BigRational::zero())
        } else if self.terms.len() == 1 && self.terms[0].mono.is_one() {
            Some(self.terms[0].coeff.clone())
        } else {
            None
        }
    }

    /// Total degree of the polynomial (0 for constants).
    pub fn degree(&self) -> u64 {
        self.terms
            .iter()
            .map(|t| t.mono.total_degree())
            .max()
            .unwrap_or(0)
    }

    /// Scale every coefficient by `c`.
    pub fn scale(&self, c: &BigRational) -> Poly {
        Poly {
            terms: self
                .terms
                .iter()
                .map(|t| Term {
                    coeff: &t.coeff * c,
                    mono: t.mono.clone(),
                })
                .collect(),
            ctx: self.ctx.clone(),
        }
    }

    /// Polynomial addition.
    #[allow(clippy::should_implement_trait)]
    pub fn add(&self, other: &Poly) -> Poly {
        let ctx = canonical_ctx(&self.ctx, &other.ctx);
        let mut terms = remap_terms(self, &ctx);
        terms.extend(remap_terms(other, &ctx));
        Poly {
            terms: normalize_terms(terms),
            ctx,
        }
    }

    /// Polynomial subtraction.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(&self, other: &Poly) -> Poly {
        self.add(&other.neg())
    }

    /// Polynomial negation.
    #[allow(clippy::should_implement_trait)]
    pub fn neg(&self) -> Poly {
        Poly {
            terms: self
                .terms
                .iter()
                .map(|t| Term {
                    coeff: -&t.coeff,
                    mono: t.mono.clone(),
                })
                .collect(),
            ctx: self.ctx.clone(),
        }
    }

    /// Polynomial multiplication (Monagan-Pearce heap merge).
    #[allow(clippy::should_implement_trait)]
    pub fn mul(&self, other: &Poly) -> Poly {
        heap_mul(self, other)
    }

    /// Raise the polynomial to a non-negative integer power.
    pub fn pow(&self, mut n: u32) -> Poly {
        let mut result = Poly::constant(BigRational::one(), self.ctx.clone());
        let mut base = self.clone();
        while n > 0 {
            if n & 1 == 1 {
                result = result.mul(&base);
            }
            n >>= 1;
            if n > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    /// Convert back into an [`Expr`] tree (terms summed with `+`).
    pub fn to_expr(&self) -> Expr {
        if self.terms.is_empty() {
            return Expr::int(0);
        }
        let mut acc: Option<Expr> = None;
        for t in &self.terms {
            let term_expr = self.term_to_expr(t);
            acc = Some(match acc {
                None => term_expr,
                Some(a) => Expr::add(a, term_expr),
            });
        }
        acc.expect("non-empty poly has at least one term")
    }

    fn term_to_expr(&self, t: &Term) -> Expr {
        let mut factors: Vec<Expr> = Vec::new();
        for (i, name) in self.ctx.vars().iter().enumerate() {
            let e = t.mono.exponent(i);
            if e == 0 {
                continue;
            }
            let var = Expr::var(name);
            factors.push(if e == 1 {
                var
            } else {
                Expr::pow(var, Expr::int(i64::from(e)))
            });
        }
        match multiply_all(factors) {
            None => Expr::constant(t.coeff.clone()),
            Some(var_part) => {
                if Rat::is_one(&t.coeff) {
                    var_part
                } else {
                    Expr::mul(Expr::constant(t.coeff.clone()), var_part)
                }
            }
        }
    }
}

/// Multiply a list of factors together (returns `None` when the list is empty).
fn multiply_all(factors: Vec<Expr>) -> Option<Expr> {
    let mut it = factors.into_iter();
    let first = it.next()?;
    Some(it.fold(first, Expr::mul))
}

impl Poly {
    /// Convert an [`Expr`] into a polynomial, or report why it is not one.
    ///
    /// The variable order is derived deterministically from the expression's
    /// variables (sorted lexicographically).
    pub fn from_expr(expr: &Expr) -> Result<Poly, PolyError> {
        let mut names = expr.variables();
        names.sort();
        let mut ctx = VarContext::new();
        for n in &names {
            ctx.add(n);
        }
        Poly::from_expr_with_ctx(expr, &ctx)
    }

    /// Convert an [`Expr`] into a polynomial using a fixed variable context.
    pub fn from_expr_with_ctx(expr: &Expr, ctx: &VarContext) -> Result<Poly, PolyError> {
        match expr {
            Expr::Const(r) => Ok(Poly::constant(r.clone(), ctx.clone())),
            Expr::Var(name) => {
                let idx = ctx
                    .index_of(name)
                    .ok_or(PolyError::NotPolynomial("unknown variable"))?;
                let mut exps = vec![0u32; ctx.len()];
                exps[idx] = 1;
                Ok(Poly::monomial(
                    BigRational::one(),
                    Monomial::from_exps(exps),
                    ctx.clone(),
                ))
            }
            Expr::Add(a, b) => {
                let l = Poly::from_expr_with_ctx(a, ctx)?;
                let r = Poly::from_expr_with_ctx(b, ctx)?;
                Ok(l.add(&r))
            }
            Expr::Sub(a, b) => {
                let l = Poly::from_expr_with_ctx(a, ctx)?;
                let r = Poly::from_expr_with_ctx(b, ctx)?;
                Ok(l.sub(&r))
            }
            Expr::Mul(a, b) => {
                let l = Poly::from_expr_with_ctx(a, ctx)?;
                let r = Poly::from_expr_with_ctx(b, ctx)?;
                Ok(l.mul(&r))
            }
            Expr::Neg(a) => Ok(Poly::from_expr_with_ctx(a, ctx)?.neg()),
            Expr::Pow(base, exp) => {
                let b = Poly::from_expr_with_ctx(base, ctx)?;
                match exp.as_ref() {
                    Expr::Const(r) => {
                        if !Rat::is_integer(r) {
                            return Err(PolyError::NotPolynomial("non-integer exponent"));
                        }
                        let n = r.numer().to_i64().ok_or(PolyError::ExponentOverflow)?;
                        if n < 0 {
                            return Err(PolyError::NegativeExponent);
                        }
                        let n = u32::try_from(n).map_err(|_| PolyError::ExponentOverflow)?;
                        Ok(b.pow(n))
                    }
                    _ => Err(PolyError::NotPolynomial("variable exponent")),
                }
            }
            Expr::Div(a, b) => {
                let num = Poly::from_expr_with_ctx(a, ctx)?;
                let den = Poly::from_expr_with_ctx(b, ctx)?;
                match den.constant_value() {
                    Some(c) if c.is_zero() => Err(PolyError::DivisionByZero),
                    Some(c) => Ok(num.scale(&(BigRational::one() / c))),
                    None => Err(PolyError::DivisionByPolynomial),
                }
            }
            _ => Err(PolyError::NotPolynomial("non-polynomial expression")),
        }
    }
}

/// Normalize a term list: drop zeros, sort descending `grevlex`, combine like terms.
pub(crate) fn normalize_terms(mut terms: Vec<Term>) -> Vec<Term> {
    terms.retain(|t| !t.coeff.is_zero());
    terms.sort_by(|a, b| b.mono.cmp(&a.mono));
    let mut out: Vec<Term> = Vec::with_capacity(terms.len());
    for t in terms {
        if let Some(last) = out.last_mut()
            && last.mono == t.mono
        {
            last.coeff = &last.coeff + &t.coeff;
            continue;
        }
        out.push(t);
    }
    out.retain(|t| !t.coeff.is_zero());
    out
}

/// Build the union variable context of two contexts in sorted name order.
pub(crate) fn canonical_ctx(a: &VarContext, b: &VarContext) -> VarContext {
    let mut names: Vec<String> = a.vars().iter().chain(b.vars().iter()).cloned().collect();
    names.sort();
    names.dedup();
    let mut ctx = VarContext::new();
    for n in names {
        ctx.add(&n);
    }
    ctx
}

/// Re-map a monomial from one variable ordering to another.
pub(crate) fn remap_mono(mono: &Monomial, from: &VarContext, to: &VarContext) -> Monomial {
    if from.vars() == to.vars() {
        return mono.clone();
    }
    let mut exps = vec![0u32; to.len()];
    for (i, name) in from.vars().iter().enumerate() {
        if let Some(j) = to.index_of(name) {
            exps[j] = mono.exponent(i);
        }
    }
    Monomial::from_exps(exps)
}

/// Re-map a polynomial's terms into the target variable ordering.
pub(crate) fn remap_terms(p: &Poly, to: &VarContext) -> Vec<Term> {
    p.terms
        .iter()
        .map(|t| Term {
            coeff: t.coeff.clone(),
            mono: remap_mono(&t.mono, &p.ctx, to),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Expr;
    use crate::rational::Rat;
    use std::collections::HashMap;

    fn eval(e: &Expr, pairs: &[(&str, f64)]) -> f64 {
        let mut env = HashMap::new();
        for (k, v) in pairs {
            env.insert((*k).to_string(), *v);
        }
        e.eval(&env).expect("evaluation failed")
    }

    #[test]
    fn from_expr_roundtrip_polynomial() {
        let x = Expr::var("x");
        let y = Expr::var("y");
        let e = Expr::add(
            Expr::pow(x.clone(), Expr::int(2)),
            Expr::mul(Expr::int(3), Expr::mul(x, y)),
        );
        let back = Poly::from_expr(&e).unwrap().to_expr();
        for (a, b) in [(1.0, 2.0), (-1.5, 0.5)] {
            assert!(
                (eval(&e, &[("x", a), ("y", b)]) - eval(&back, &[("x", a), ("y", b)])).abs() < 1e-9
            );
        }
    }

    #[test]
    fn add_combines_like_terms() {
        let x = Expr::var("x");
        let p = Poly::from_expr(&Expr::add(x.clone(), x)).unwrap();
        assert_eq!(p.terms().len(), 1);
        assert_eq!(p.leading_term().unwrap().coeff, Rat::from_int(2));
    }

    #[test]
    fn multiplication_has_expected_terms() {
        let x = Expr::var("x");
        let a = Poly::from_expr(&Expr::add(x.clone(), Expr::int(1))).unwrap();
        let b = Poly::from_expr(&Expr::sub(x, Expr::int(1))).unwrap();
        let p = a.mul(&b);
        assert_eq!(p.terms().len(), 2);
        assert_eq!(p.degree(), 2);
        assert_eq!(p.terms()[1].coeff, Rat::from_int(-1));
    }

    #[test]
    fn division_by_constant_scales_coefficients() {
        let e = Expr::div(Expr::mul(Expr::int(4), Expr::var("x")), Expr::int(2));
        let p = Poly::from_expr(&e).unwrap();
        assert_eq!(p.leading_term().unwrap().coeff, Rat::from_int(2));
    }

    #[test]
    fn non_polynomial_is_rejected() {
        assert!(Poly::from_expr(&Expr::sin(Expr::var("x"))).is_err());
    }

    #[test]
    fn division_by_polynomial_is_rejected() {
        assert!(Poly::from_expr(&Expr::div(Expr::int(1), Expr::var("x"))).is_err());
    }
}
