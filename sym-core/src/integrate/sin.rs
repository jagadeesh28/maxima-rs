//! Elementary-function integration rules (Moses' SIN).

use super::{Expr, is_constant_wrt, linear_in};
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
        Expr::Tan(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(
                Expr::neg(Expr::ln(Expr::cos((**u).clone()))),
                Expr::constant(a),
            ))
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
        Expr::Tanh(u) => {
            let a = linear_coeff(u, var)?;
            Some(Expr::div(
                Expr::ln(Expr::cosh((**u).clone())),
                Expr::constant(a),
            ))
        }
        Expr::Ln(u) => {
            let a = linear_coeff(u, var)?;
            let uu = (**u).clone();
            let top = Expr::sub(Expr::mul(uu.clone(), Expr::ln(uu.clone())), uu);
            Some(Expr::div(top, Expr::constant(a)))
        }
        Expr::Sqrt(u) => {
            let a = linear_coeff(u, var)?;
            let uu = (**u).clone();
            let numer = Expr::mul(Expr::int(2), Expr::pow(uu, Expr::frac(3, 2)));
            let denom = Expr::constant(a * BigRational::from_integer(3.into()));
            Some(Expr::div(numer, denom))
        }
        Expr::Asin(u) => {
            let a = linear_coeff(u, var)?;
            let uu = (**u).clone();
            let sqrt_term =
                Expr::sqrt(Expr::sub(Expr::int(1), Expr::pow(uu.clone(), Expr::int(2))));
            let top = Expr::add(Expr::mul(uu.clone(), Expr::asin(uu)), sqrt_term);
            Some(Expr::div(top, Expr::constant(a)))
        }
        Expr::Acos(u) => {
            let a = linear_coeff(u, var)?;
            let uu = (**u).clone();
            let sqrt_term =
                Expr::sqrt(Expr::sub(Expr::int(1), Expr::pow(uu.clone(), Expr::int(2))));
            let top = Expr::sub(Expr::mul(uu.clone(), Expr::acos(uu)), sqrt_term);
            Some(Expr::div(top, Expr::constant(a)))
        }
        Expr::Atan(u) => {
            let a = linear_coeff(u, var)?;
            let uu = (**u).clone();
            let ln_term = Expr::div(
                Expr::ln(Expr::add(Expr::int(1), Expr::pow(uu.clone(), Expr::int(2)))),
                Expr::int(2),
            );
            let top = Expr::sub(Expr::mul(uu.clone(), Expr::atan(uu)), ln_term);
            Some(Expr::div(top, Expr::constant(a)))
        }
        Expr::Pow(c, u) if is_constant_wrt(c, var) => {
            if c.is_zero() {
                return Some(Expr::int(0));
            }
            if c.is_one() {
                return Some(Expr::var(var));
            }
            let a = linear_coeff(u, var)?;
            let denom = Expr::mul(Expr::constant(a), Expr::ln((**c).clone()));
            Some(Expr::div(expr.clone(), denom))
        }
        _ => None,
    }
}
