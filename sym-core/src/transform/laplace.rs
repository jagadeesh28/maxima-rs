//! Symbolic forward and inverse Laplace transform engine.
//!
//! Citations:
//! - Maxima 5.50: `src/laplac.lisp` (`laplace`, `laplus`, `laptimes`, `lapexpt`, `lapsin`, `lapsinh`, `lapdiff`, `ilt0`).
//! - ADR-0011: Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::poly::partfrac::partfrac;
use crate::rational::{BigRational, Rat};
use num_bigint::BigInt;
use num_traits::{One, Signed};

/// Strongly typed error for symbolic Laplace / inverse Laplace transforms.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum TransformError {
    #[error("unsupported forward Laplace transform for: {0}")]
    UnsupportedLaplace(String),
    #[error("unsupported inverse Laplace transform for: {0}")]
    UnsupportedIlt(String),
}

/// Compute the forward Laplace transform $\mathcal{L}\{f(t)\} = F(s)$.
///
/// Maxima `src/laplac.lisp:60` (`defun laplace`).
/// Returns the formal Laplace integral noun $\int f(t) e^{-st}\,dt$ when unmatched.
pub fn laplace(f: &Expr, t: &str, s: &str) -> Expr {
    try_laplace(f, t, s).unwrap_or_else(|_| {
        Expr::integral(
            Expr::mul(
                f.clone(),
                Expr::exp(Expr::neg(Expr::mul(Expr::var(s), Expr::var(t)))),
            ),
            Expr::var(t),
        )
    })
}

/// Compute the forward Laplace transform, returning `Err(TransformError)` on unsupported inputs.
pub fn try_laplace(f: &Expr, t: &str, s: &str) -> Result<Expr, TransformError> {
    // 0. If f is independent of t: L{c} = c/s (Maxima laplac.lisp:69-71)
    if !f.variables().contains(&t.to_string()) {
        if f.is_zero() {
            return Ok(Expr::int(0));
        }
        return Ok(Expr::div(f.clone(), Expr::var(s)));
    }

    // 1. Linearity: L{a + b} = L{a} + L{b}, L{a - b} = L{a} - L{b}, L{-a} = -L{a}
    match f {
        Expr::Add(a, b) => Ok(Expr::add(try_laplace(a, t, s)?, try_laplace(b, t, s)?)),
        Expr::Sub(a, b) => Ok(Expr::sub(try_laplace(a, t, s)?, try_laplace(b, t, s)?)),
        Expr::Neg(a) => Ok(Expr::neg(try_laplace(a, t, s)?)),

        // 2. Scalar multiplication: L{c * g(t)} = c * L{g(t)} (Maxima laplac.lisp:77-78)
        Expr::Mul(a, b) => {
            let a_has_t = a.variables().contains(&t.to_string());
            let b_has_t = b.variables().contains(&t.to_string());
            if !a_has_t {
                return Ok(Expr::mul(*a.clone(), try_laplace(b, t, s)?));
            }
            if !b_has_t {
                return Ok(Expr::mul(*b.clone(), try_laplace(a, t, s)?));
            }

            // Frequency shift rule: L{exp(c*t) * g(t)} = L{g(t)}|_{s -> s - c}
            if let Some(c) = extract_exp_coeff(a, t) {
                let g_s = try_laplace(b, t, s)?;
                let shifted_s = Expr::sub(Expr::var(s), c);
                return Ok(subst(&g_s, s, &shifted_s));
            }
            if let Some(c) = extract_exp_coeff(b, t) {
                let g_s = try_laplace(a, t, s)?;
                let shifted_s = Expr::sub(Expr::var(s), c);
                return Ok(subst(&g_s, s, &shifted_s));
            }

            // Multiplication by t rule: L{t * g(t)} = - d/ds (L{g(t)})
            if matches!(a.as_ref(), Expr::Var(n) if n == t) {
                let g_s = try_laplace(b, t, s)?;
                return Ok(Expr::neg(differentiate(&g_s, s)));
            }
            if matches!(b.as_ref(), Expr::Var(n) if n == t) {
                let g_s = try_laplace(a, t, s)?;
                return Ok(Expr::neg(differentiate(&g_s, s)));
            }

            Err(TransformError::UnsupportedLaplace(f.to_string()))
        }

        // 3. Division by constant
        Expr::Div(a, b) if !b.variables().contains(&t.to_string()) => {
            Ok(Expr::div(try_laplace(a, t, s)?, *b.clone()))
        }

        // 4. Time variable itself: L{t} = 1/s^2 (Maxima laplac.lisp:68)
        Expr::Var(n) if n == t => Ok(Expr::div(
            Expr::int(1),
            Expr::pow(Expr::var(s), Expr::int(2)),
        )),

        // 5. Monomial powers: L{t^n} = n! / s^(n+1) (Maxima laplac.lisp:79-80)
        Expr::Pow(base, exp) if matches!(base.as_ref(), Expr::Var(n) if n == t) => {
            if let Expr::Const(r) = exp.as_ref()
                && Rat::is_integer(r)
                && r > &BigRational::from_integer(0.into())
            {
                let n_int = r.to_integer();
                if let Ok(n_u32) = u32::try_from(&n_int) {
                    let fact = (1..=n_u32).fold(BigInt::one(), |acc, k| acc * k);
                    let n_plus_1 = Expr::int((n_u32 + 1) as i64);
                    return Ok(Expr::div(
                        Expr::constant(BigRational::from_integer(fact)),
                        Expr::pow(Expr::var(s), n_plus_1),
                    ));
                }
            }
            #[cfg(feature = "specfn")]
            {
                let a_plus_1 = Expr::add(*exp.clone(), Expr::int(1));
                Ok(Expr::div(
                    Expr::gamma(a_plus_1.clone()),
                    Expr::pow(Expr::var(s), a_plus_1),
                ))
            }
            #[cfg(not(feature = "specfn"))]
            Err(TransformError::UnsupportedLaplace(f.to_string()))
        }

        // 6. Exponential: L{exp(c*t)} = 1 / (s - c) (Maxima laplac.lisp:79-80)
        Expr::Exp(arg) => {
            if let Some(c) = extract_linear_coeff(arg, t) {
                let denom = Expr::sub(Expr::var(s), c);
                Ok(Expr::div(Expr::int(1), denom))
            } else {
                Err(TransformError::UnsupportedLaplace(f.to_string()))
            }
        }

        // 7. Sine: L{sin(w*t)} = w / (s^2 + w^2) (Maxima laplac.lisp:81-82)
        Expr::Sin(arg) => {
            let Some(w) = extract_linear_coeff(arg, t) else {
                return Err(TransformError::UnsupportedLaplace(f.to_string()));
            };
            let s_sq = Expr::pow(Expr::var(s), Expr::int(2));
            let w_sq = Expr::pow(w.clone(), Expr::int(2));
            Ok(Expr::div(w, Expr::add(s_sq, w_sq)))
        }

        // 8. Cosine: L{cos(w*t)} = s / (s^2 + w^2) (Maxima laplac.lisp:83-84)
        Expr::Cos(arg) => {
            let Some(w) = extract_linear_coeff(arg, t) else {
                return Err(TransformError::UnsupportedLaplace(f.to_string()));
            };
            let s_sq = Expr::pow(Expr::var(s), Expr::int(2));
            let w_sq = Expr::pow(w, Expr::int(2));
            Ok(Expr::div(Expr::var(s), Expr::add(s_sq, w_sq)))
        }

        // 9. Sinh: L{sinh(a*t)} = a / (s^2 - a^2) (Maxima laplac.lisp:85-86)
        Expr::Sinh(arg) => {
            let Some(a) = extract_linear_coeff(arg, t) else {
                return Err(TransformError::UnsupportedLaplace(f.to_string()));
            };
            let s_sq = Expr::pow(Expr::var(s), Expr::int(2));
            let a_sq = Expr::pow(a.clone(), Expr::int(2));
            Ok(Expr::div(a, Expr::sub(s_sq, a_sq)))
        }

        // 10. Cosh: L{cosh(a*t)} = s / (s^2 - a^2) (Maxima laplac.lisp:87-88)
        Expr::Cosh(arg) => {
            let Some(a) = extract_linear_coeff(arg, t) else {
                return Err(TransformError::UnsupportedLaplace(f.to_string()));
            };
            let s_sq = Expr::pow(Expr::var(s), Expr::int(2));
            let a_sq = Expr::pow(a, Expr::int(2));
            Ok(Expr::div(Expr::var(s), Expr::sub(s_sq, a_sq)))
        }

        // 11. Derivatives: L{y'(t)} = s*Y(s) - y(0) (Maxima laplac.lisp:91-92)
        Expr::Derivative(inner, var) if matches!(var.as_ref(), Expr::Var(n) if n == t) => {
            match inner.as_ref() {
                Expr::Var(y) => {
                    let y_s = Expr::var(&format!("{y}_{s}"));
                    let s_term = Expr::mul(Expr::var(s), y_s);
                    let init_val = Expr::var(&format!("{y}_0"));
                    Ok(Expr::sub(s_term, init_val))
                }
                Expr::Derivative(i2, v2) if matches!(v2.as_ref(), Expr::Var(n) if n == t) => {
                    if let Expr::Var(y) = i2.as_ref() {
                        let y_s = Expr::var(&format!("{y}_{s}"));
                        let s2_term = Expr::mul(Expr::pow(Expr::var(s), Expr::int(2)), y_s);
                        let s_y0 = Expr::mul(Expr::var(s), Expr::var(&format!("{y}_0")));
                        let dy0 = Expr::var(&format!("d{y}_0"));
                        Ok(Expr::sub(Expr::sub(s2_term, s_y0), dy0))
                    } else {
                        Err(TransformError::UnsupportedLaplace(f.to_string()))
                    }
                }
                _ => Err(TransformError::UnsupportedLaplace(f.to_string())),
            }
        }

        _ => Err(TransformError::UnsupportedLaplace(f.to_string())),
    }
}

/// Compute the inverse Laplace transform $\mathcal{L}^{-1}\{F(s)\} = f(t)$.
///
/// Maxima `src/laplac.lisp:746` (`defun ilt0`).
/// Returns a formal integral noun when unmatched instead of fabricating bogus expressions.
pub fn ilt(f: &Expr, s: &str, t: &str) -> Expr {
    try_ilt(f, s, t).unwrap_or_else(|_| Expr::integral(f.clone(), Expr::var(s)))
}

/// Compute the inverse Laplace transform, returning `Err(TransformError)` on unsupported inputs.
pub fn try_ilt(f: &Expr, s: &str, t: &str) -> Result<Expr, TransformError> {
    // 0. If F is independent of s: ilt{c} = c * delta(t)
    if !f.variables().contains(&s.to_string()) {
        if f.is_zero() {
            return Ok(Expr::int(0));
        }
        return Ok(Expr::mul(f.clone(), Expr::var(&format!("delta({t})"))));
    }

    // 1. Linearity: ilt{A + B} = ilt{A} + ilt{B}, ilt{A - B} = ilt{A} - ilt{B}
    match f {
        Expr::Add(a, b) => Ok(Expr::add(try_ilt(a, s, t)?, try_ilt(b, s, t)?)),
        Expr::Sub(a, b) => Ok(Expr::sub(try_ilt(a, s, t)?, try_ilt(b, s, t)?)),
        Expr::Neg(a) => Ok(Expr::neg(try_ilt(a, s, t)?)),

        // 2. Multiplication by constant independent of s
        Expr::Mul(a, b) => {
            let a_has_s = a.variables().contains(&s.to_string());
            let b_has_s = b.variables().contains(&s.to_string());
            if !a_has_s {
                return Ok(Expr::mul(*a.clone(), try_ilt(b, s, t)?));
            }
            if !b_has_s {
                return Ok(Expr::mul(*b.clone(), try_ilt(a, s, t)?));
            }
            ilt_rational(f, s, t)
        }

        // 3. Division: rational Laplace transfer functions
        Expr::Div(num, den) => {
            if !den.variables().contains(&s.to_string()) {
                return Ok(Expr::div(try_ilt(num, s, t)?, *den.clone()));
            }
            ilt_rational(f, s, t)
        }

        _ => ilt_rational(f, s, t),
    }
}

/// Extract `(A, B)` from linear expression `A*s + B` where `A, B` do not depend on `s`.
fn extract_linear_s(expr: &Expr, s: &str) -> Option<(Expr, Expr)> {
    if !expr.variables().contains(&s.to_string()) {
        return Some((Expr::int(0), expr.clone()));
    }
    match expr {
        Expr::Var(v) if v == s => Some((Expr::int(1), Expr::int(0))),
        Expr::Neg(inner) => {
            let (a, b) = extract_linear_s(inner, s)?;
            Some((Expr::neg(a), Expr::neg(b)))
        }
        Expr::Mul(x, y) => {
            if matches!(x.as_ref(), Expr::Var(v) if v == s)
                && !y.variables().contains(&s.to_string())
            {
                return Some(((**y).clone(), Expr::int(0)));
            }
            if matches!(y.as_ref(), Expr::Var(v) if v == s)
                && !x.variables().contains(&s.to_string())
            {
                return Some(((**x).clone(), Expr::int(0)));
            }
            None
        }
        Expr::Add(x, y) => {
            let (ax, bx) = extract_linear_s(x, s)?;
            let (ay, by) = extract_linear_s(y, s)?;
            Some((Expr::add(ax, ay), Expr::add(bx, by)))
        }
        Expr::Sub(x, y) => {
            let (ax, bx) = extract_linear_s(x, s)?;
            let (ay, by) = extract_linear_s(y, s)?;
            Some((Expr::sub(ax, ay), Expr::sub(bx, by)))
        }
        _ => None,
    }
}

/// Invert rational Laplace expressions $P(s)/Q(s)$.
fn ilt_rational(expr: &Expr, s: &str, t: &str) -> Result<Expr, TransformError> {
    let (num, den) = match expr {
        Expr::Div(n, d) => (n.as_ref().clone(), d.as_ref().clone()),
        _ => (expr.clone(), Expr::int(1)),
    };

    // Case 1: c / s => c (only when numerator is independent of s!)
    if matches!(&den, Expr::Var(v) if v == s) && !num.variables().contains(&s.to_string()) {
        return Ok(num);
    }

    // Case 2: c / s^n => c * t^(n-1) / (n-1)!
    if let Expr::Pow(base, exp) = &den
        && let (Expr::Var(v), Expr::Const(r)) = (base.as_ref(), exp.as_ref())
        && v == s
        && !num.variables().contains(&s.to_string())
        && Rat::is_integer(r)
        && r > &BigRational::from_integer(1.into())
    {
        let n = r.to_integer();
        if let Ok(n_u32) = u32::try_from(&n) {
            let n_minus_1 = n_u32 - 1;
            let fact = (1..=n_minus_1).fold(BigInt::one(), |acc, k| acc * k);
            let t_pow = if n_minus_1 == 1 {
                Expr::var(t)
            } else {
                Expr::pow(Expr::var(t), Expr::int(n_minus_1 as i64))
            };
            let res = if fact.is_one() {
                t_pow
            } else {
                Expr::div(t_pow, Expr::constant(BigRational::from_integer(fact)))
            };
            return Ok(if num.is_one() {
                res
            } else {
                Expr::mul(num, res)
            });
        }
    }

    // Case 3: c / (s - a) => c * exp(a*t) or c / (s + a) => c * exp(-a*t)
    if !num.variables().contains(&s.to_string()) {
        if let Expr::Sub(base_s, a) = &den {
            let is_s = matches!(base_s.as_ref(), Expr::Var(v) if v == s);
            if is_s && !a.variables().contains(&s.to_string()) {
                let exp_term = Expr::exp(Expr::mul(a.as_ref().clone(), Expr::var(t)));
                return Ok(if num.is_one() {
                    exp_term
                } else {
                    Expr::mul(num, exp_term)
                });
            }
        }
        if let Expr::Add(base_s, a) = &den {
            let is_s = matches!(base_s.as_ref(), Expr::Var(v) if v == s);
            if is_s && !a.variables().contains(&s.to_string()) {
                let neg_a = match a.as_ref() {
                    Expr::Const(r) => Expr::constant(-r),
                    _ => Expr::neg(a.as_ref().clone()),
                };
                let exp_term = Expr::exp(Expr::mul(neg_a, Expr::var(t)));
                return Ok(if num.is_one() {
                    exp_term
                } else {
                    Expr::mul(num, exp_term)
                });
            }
        }
    }

    // Case 4: Quadratic denominator s^2 + w^2
    if let Expr::Add(s_part, w_part) = &den
        && extract_s_squared(s_part, s).is_some()
        && !w_part.variables().contains(&s.to_string())
    {
        let w = match w_part.as_ref() {
            Expr::Const(r) => {
                if let Some(sqrt_r) = sqrt_exact_rational(r) {
                    Expr::constant(sqrt_r)
                } else {
                    Expr::sqrt(*w_part.clone())
                }
            }
            _ => Expr::sqrt(*w_part.clone()),
        };
        if let Some((a_coeff, b_coeff)) = extract_linear_s(&num, s) {
            if a_coeff.is_zero() {
                let sin_term = Expr::sin(Expr::mul(w.clone(), Expr::var(t)));
                return Ok(if b_coeff.is_one() {
                    Expr::div(sin_term, w)
                } else {
                    Expr::mul(Expr::div(b_coeff, w), sin_term)
                });
            }
            if b_coeff.is_zero() {
                let cos_term = Expr::cos(Expr::mul(w, Expr::var(t)));
                return Ok(if a_coeff.is_one() {
                    cos_term
                } else {
                    Expr::mul(a_coeff, cos_term)
                });
            }
            let cos_part = Expr::mul(a_coeff, Expr::cos(Expr::mul(w.clone(), Expr::var(t))));
            let sin_part = Expr::mul(
                Expr::div(b_coeff, w.clone()),
                Expr::sin(Expr::mul(w, Expr::var(t))),
            );
            return Ok(Expr::add(cos_part, sin_part));
        }
    }

    // Case 5: Hyperbolic s^2 - a^2
    if let Expr::Sub(s_part, a_part) = &den
        && extract_s_squared(s_part, s).is_some()
        && !a_part.variables().contains(&s.to_string())
        && !num.variables().contains(&s.to_string())
    {
        let a = Expr::sqrt(*a_part.clone());
        if num.is_one() {
            return Ok(Expr::div(Expr::sinh(Expr::mul(a.clone(), Expr::var(t))), a));
        }
        return Ok(Expr::mul(
            num,
            Expr::div(Expr::sinh(Expr::mul(a.clone(), Expr::var(t))), a),
        ));
    }

    // Case 6: Partial-fraction decomposition via Phase 48 `partfrac`
    if let Ok(pf) = partfrac(expr, s)
        && pf != *expr
    {
        return try_ilt(&pf, s, t);
    }

    Err(TransformError::UnsupportedIlt(expr.to_string()))
}

/// Helper: check if `expr` is of the form $s^2$
fn extract_s_squared(expr: &Expr, s: &str) -> Option<Expr> {
    match expr {
        Expr::Pow(b, e) => match (b.as_ref(), e.as_ref()) {
            (Expr::Var(v), Expr::Const(r))
                if v == s && r == &BigRational::from_integer(2.into()) =>
            {
                Some(Expr::int(1))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Helper: extract linear coefficient $c$ from $c \cdot t$ or $-c \cdot t$ or $t$.
fn extract_linear_coeff(expr: &Expr, t: &str) -> Option<Expr> {
    match expr {
        Expr::Var(n) if n == t => Some(Expr::int(1)),
        Expr::Neg(inner) => {
            let c = extract_linear_coeff(inner, t)?;
            Some(Expr::neg(c))
        }
        Expr::Mul(a, b) => {
            if matches!(b.as_ref(), Expr::Var(n) if n == t)
                && !a.variables().contains(&t.to_string())
            {
                Some(*a.clone())
            } else if matches!(a.as_ref(), Expr::Var(n) if n == t)
                && !b.variables().contains(&t.to_string())
            {
                Some(*b.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper: extract coefficient $c$ from $e^{c \cdot t}$.
fn extract_exp_coeff(expr: &Expr, t: &str) -> Option<Expr> {
    match expr {
        Expr::Exp(arg) => extract_linear_coeff(arg, t),
        Expr::Pow(base, exp) if matches!(base.as_ref(), Expr::E) => extract_linear_coeff(exp, t),
        _ => None,
    }
}

/// Substitute variable `var` with `val` in `expr`.
pub fn subst(expr: &Expr, var: &str, val: &Expr) -> Expr {
    match expr {
        Expr::Var(n) if n == var => val.clone(),
        Expr::Const(_) | Expr::Var(_) | Expr::Pi | Expr::E => expr.clone(),
        #[cfg(feature = "complex")]
        Expr::I => Expr::I,
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(_) => expr.clone(),
        Expr::Add(a, b) => Expr::add(subst(a, var, val), subst(b, var, val)),
        Expr::Sub(a, b) => Expr::sub(subst(a, var, val), subst(b, var, val)),
        Expr::Mul(a, b) => Expr::mul(subst(a, var, val), subst(b, var, val)),
        Expr::Div(a, b) => Expr::div(subst(a, var, val), subst(b, var, val)),
        Expr::Pow(a, b) => Expr::pow(subst(a, var, val), subst(b, var, val)),
        Expr::Neg(a) => Expr::neg(subst(a, var, val)),
        Expr::Sin(a) => Expr::sin(subst(a, var, val)),
        Expr::Cos(a) => Expr::cos(subst(a, var, val)),
        Expr::Tan(a) => Expr::tan(subst(a, var, val)),
        Expr::Exp(a) => Expr::exp(subst(a, var, val)),
        Expr::Ln(a) => Expr::ln(subst(a, var, val)),
        Expr::Sqrt(a) => Expr::sqrt(subst(a, var, val)),
        Expr::Sinh(a) => Expr::sinh(subst(a, var, val)),
        Expr::Cosh(a) => Expr::cosh(subst(a, var, val)),
        Expr::Tanh(a) => Expr::tanh(subst(a, var, val)),
        Expr::Asin(a) => Expr::asin(subst(a, var, val)),
        Expr::Acos(a) => Expr::acos(subst(a, var, val)),
        Expr::Atan(a) => Expr::atan(subst(a, var, val)),
        Expr::Abs(a) => Expr::abs(subst(a, var, val)),
        Expr::Atan2(y, x) => Expr::atan2(subst(y, var, val), subst(x, var, val)),
        Expr::Factorial(a) => Expr::factorial(subst(a, var, val)),
        Expr::Tensor(name, idx) => Expr::tensor(name, idx.clone()),
        Expr::Derivative(a, b) => Expr::derivative(subst(a, var, val), subst(b, var, val)),
        Expr::Integral(a, b) => Expr::integral(subst(a, var, val), subst(b, var, val)),
        Expr::Sum(a, b, c, d) => Expr::sum(
            subst(a, var, val),
            subst(b, var, val),
            subst(c, var, val),
            subst(d, var, val),
        ),
        Expr::Matrix(rows) => Expr::matrix(
            rows.iter()
                .map(|r| r.iter().map(|e| subst(e, var, val)).collect())
                .collect(),
        ),
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => {
            Expr::SpecialFn(*kind, args.iter().map(|a| subst(a, var, val)).collect())
        }
    }
}

/// Helper: exact rational square root if a perfect square
fn sqrt_exact_rational(r: &BigRational) -> Option<BigRational> {
    if r.is_negative() {
        return None;
    }
    let num_sqrt = r.numer().sqrt();
    let den_sqrt = r.denom().sqrt();
    if &num_sqrt * &num_sqrt == *r.numer() && &den_sqrt * &den_sqrt == *r.denom() {
        Some(BigRational::new(num_sqrt, den_sqrt))
    } else {
        None
    }
}
