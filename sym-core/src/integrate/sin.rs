//! Elementary-function integration rules (linear-argument subset of Moses' SIN).

use super::{Expr, linear_in};
use num_rational::BigRational;
use num_traits::Zero;

fn linear_coeff(u: &Expr, var: &str) -> Option<BigRational> {
    let (a, _) = linear_in(u, var)?;
    if a.is_zero() { None } else { Some(a) }
}

/// Integrate an elementary function with a linear argument, if recognised.
pub(crate) fn integrate_elementary(expr: &Expr, var: &str) -> Option<Expr> {
    match expr {
        Expr::Sin(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(
                Expr::neg(Expr::cos((**u).clone())),
                Expr::constant(a),
            ))
        }
        Expr::Cos(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(Expr::sin((**u).clone()), Expr::constant(a)))
        }
        Expr::Exp(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(Expr::exp((**u).clone()), Expr::constant(a)))
        }
        Expr::Sinh(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(Expr::cosh((**u).clone()), Expr::constant(a)))
        }
        Expr::Cosh(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(Expr::sinh((**u).clone()), Expr::constant(a)))
        }
        Expr::Ln(u) => {
            let a = linear_coeff(u, var)?;
            let uu = (**u).clone();
            let top = Expr::sub(Expr::mul(uu.clone(), Expr::ln(uu.clone())), uu);
            Some(Expr::div(top, Expr::constant(a)))
        }
        _ => None,
    }
}
