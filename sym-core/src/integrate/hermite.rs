//! Rational-function integration via partial fractions (Hermite/LRT subset).
//!
//! Rational expressions whose denominator splits into distinct linear factors are
//! decomposed by [`crate::poly::partfrac`] and integrated term by term, yielding
//! logarithmic antiderivatives. Non-splitting denominators fall back to the noun form.

use super::integrate;
use crate::expr::Expr;
use crate::poly::{partfrac, ratsimp};

/// Attempt to integrate a rational expression, returning `None` when it cannot be
/// split into integrable partial fractions.
pub(crate) fn integrate_rational(expr: &Expr, var: &str) -> Option<Expr> {
    let r = ratsimp(expr).ok()?;
    let pf = partfrac(&r, var).ok()?;
    if pf == *expr || pf == r {
        return None;
    }
    Some(integrate(&pf, var))
}
