//! Real logarithmic antiderivative terms (linear-factor subset of LRT).

use crate::expr::Expr;
use num_rational::BigRational;

/// Build `∫ num_const / (a*x + b) dx = (num_const / a) * ln(a*x + b)`.
pub(crate) fn log_over_linear(num_const: &Expr, den_linear: &Expr, a: &BigRational) -> Expr {
    Expr::div(
        Expr::mul(num_const.clone(), Expr::ln(den_linear.clone())),
        Expr::constant(a.clone()),
    )
}
