//! 3-tier zero-equivalence decider (Phase 27).
//!
//! Tiers: (1) exact zero / polynomial zero, (2) series constant term zero,
//! (3) fall through to "not provably zero".

use crate::expr::Expr;
use crate::poly::Poly;
use crate::series::series_expand;

/// Decide whether `expr` is equivalent to zero with respect to `var`.
pub fn is_zero_equivalent(expr: &Expr, var: &str) -> bool {
    if expr.is_zero() {
        return true;
    }
    if let Ok(p) = Poly::from_expr(expr)
        && p.is_zero()
    {
        return true;
    }
    if let Some(s) = series_expand(expr, var, &Expr::int(0), 6) {
        return s.coeffs.is_empty();
    }
    false
}

/// Decide whether `a - b` is equivalent to zero.
pub fn difference_is_zero(a: &Expr, b: &Expr, var: &str) -> bool {
    is_zero_equivalent(&Expr::sub(a.clone(), b.clone()), var)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_zero_detected() {
        assert!(is_zero_equivalent(&Expr::int(0), "x"));
    }

    #[test]
    fn polynomial_cancellation_detected() {
        let x = Expr::var("x");
        let e = Expr::sub(
            Expr::add(x.clone(), Expr::int(1)),
            Expr::add(x, Expr::int(1)),
        );
        assert!(is_zero_equivalent(&e, "x"));
    }

    #[test]
    fn nonzero_is_not_zero() {
        assert!(!is_zero_equivalent(&Expr::int(1), "x"));
    }
}
