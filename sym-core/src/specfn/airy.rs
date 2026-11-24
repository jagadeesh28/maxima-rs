//! Airy functions $\text{Ai}(z)$, $\text{Bi}(z)$ and their derivatives.
//!
//! Citations:
//! - Maxima 5.50: `src/airy.lisp:29, 115, 181, 268`.
//! - Solutions to the Airy differential equation $w'' - z w = 0$.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::specfn::SpecialFnKind;

/// Differentiate $\text{Ai}(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \text{Ai}(u) = \text{Ai}'(u) \frac{du}{dx}$$
///
/// Citations: Maxima 5.50 `src/airy.lisp:29-30` (`defgrad %airy_ai`).
pub fn diff_airy_ai(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let u = &args[0];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    let dai = Expr::special_fn(SpecialFnKind::AiryAiPrime, vec![u.clone()]);
    if du.is_one() { dai } else { Expr::mul(dai, du) }
}

/// Differentiate $\text{Ai}'(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \text{Ai}'(u) = u \cdot \text{Ai}(u) \frac{du}{dx}$$
///
/// Citations: Maxima 5.50 `src/airy.lisp:115-116` (`defgrad %airy_dai`).
pub fn diff_airy_ai_prime(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let u = &args[0];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    let ai = Expr::special_fn(SpecialFnKind::AiryAi, vec![u.clone()]);
    let deriv = Expr::mul(u.clone(), ai);
    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}

/// Differentiate $\text{Bi}(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \text{Bi}(u) = \text{Bi}'(u) \frac{du}{dx}$$
///
/// Citations: Maxima 5.50 `src/airy.lisp:181-182` (`defgrad %airy_bi`).
pub fn diff_airy_bi(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let u = &args[0];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    let dbi = Expr::special_fn(SpecialFnKind::AiryBiPrime, vec![u.clone()]);
    if du.is_one() { dbi } else { Expr::mul(dbi, du) }
}

/// Differentiate $\text{Bi}'(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \text{Bi}'(u) = u \cdot \text{Bi}(u) \frac{du}{dx}$$
///
/// Citations: Maxima 5.50 `src/airy.lisp:268-269` (`defgrad %airy_dbi`).
pub fn diff_airy_bi_prime(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let u = &args[0];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    let bi = Expr::special_fn(SpecialFnKind::AiryBi, vec![u.clone()]);
    let deriv = Expr::mul(u.clone(), bi);
    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}
