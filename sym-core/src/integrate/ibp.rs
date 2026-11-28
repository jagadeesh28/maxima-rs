//! Integration by parts for `x^n` multiplied by `exp`/`sin`/`cos` of a linear argument.

use super::{Expr, as_const, linear_in};
use crate::rational::Rat;
use num_traits::{ToPrimitive, Zero};

/// Attempt integration by parts on a product `x^n * f(a x + b)`.
pub(crate) fn try_ibp(expr: &Expr, var: &str) -> Option<Expr> {
    let Expr::Mul(u, v) = expr else {
        return None;
    };
    if let Some(r) = ibp_power_times(u, v, var) {
        return Some(r);
    }
    if let Some(r) = ibp_power_times(v, u, var) {
        return Some(r);
    }
    None
}

fn var_power(expr: &Expr, var: &str) -> Option<u32> {
    match expr {
        Expr::Var(n) if n == var => Some(1),
        Expr::Pow(b, e) => match (b.as_ref(), as_const(e)) {
            (Expr::Var(n), Some(r)) if n == var && Rat::is_integer(&r) => {
                let k = r.numer().to_u32()?;
                if k >= 1 { Some(k) } else { None }
            }
            _ => None,
        },
        _ => None,
    }
}

fn x_power(var: &str, k: u32) -> Expr {
    let v = Expr::var(var);
    if k == 1 {
        v
    } else {
        Expr::pow(v, Expr::int(i64::from(k)))
    }
}

fn linear_coeff_expr(w: &Expr, var: &str) -> Option<Expr> {
    let (a, _) = linear_in(w, var)?;
    if a.is_zero() {
        None
    } else {
        Some(Expr::constant(a))
    }
}

fn trig_powers(n: u32, w: &Expr, var: &str, a: &Expr) -> (Expr, Expr) {
    // O(n) polynomial-size tabular IBP for ∫ x^n sin(a*x + b) dx and ∫ x^n cos(a*x + b) dx
    let mut sin_acc = Expr::int(0);
    let mut cos_acc = Expr::int(0);
    let mut coeff: u128 = 1;
    let mut a_pow = a.clone();

    for k in 0..=n {
        let rem_deg = n - k;
        let xp = if rem_deg == 0 {
            Expr::int(1)
        } else {
            x_power(var, rem_deg)
        };
        let mag = if coeff == 1 {
            xp
        } else {
            Expr::mul(Expr::int(coeff as i64), xp)
        };

        // sin integral: k=0 -> -cos, k=1 -> +sin, k=2 -> +cos, k=3 -> -sin
        let sin_trig = if k % 2 == 0 {
            Expr::cos(w.clone())
        } else {
            Expr::sin(w.clone())
        };
        let sin_neg = k.div_ceil(2).is_multiple_of(2);
        let sin_term = Expr::div(Expr::mul(mag.clone(), sin_trig), a_pow.clone());
        sin_acc = if sin_neg {
            if sin_acc.is_zero() {
                Expr::neg(sin_term)
            } else {
                Expr::sub(sin_acc, sin_term)
            }
        } else if sin_acc.is_zero() {
            sin_term
        } else {
            Expr::add(sin_acc, sin_term)
        };

        // cos integral: k=0 -> +sin, k=1 -> +cos, k=2 -> -sin, k=3 -> -cos
        let cos_trig = if k % 2 == 0 {
            Expr::sin(w.clone())
        } else {
            Expr::cos(w.clone())
        };
        let cos_neg = !(k / 2).is_multiple_of(2);
        let cos_term = Expr::div(Expr::mul(mag, cos_trig), a_pow.clone());
        cos_acc = if cos_neg {
            if cos_acc.is_zero() {
                Expr::neg(cos_term)
            } else {
                Expr::sub(cos_acc, cos_term)
            }
        } else if cos_acc.is_zero() {
            cos_term
        } else {
            Expr::add(cos_acc, cos_term)
        };

        if k < n {
            coeff = coeff.saturating_mul(u128::from(n - k));
            a_pow = Expr::pow(a.clone(), Expr::int(i64::from(k + 2)));
        }
    }
    (sin_acc, cos_acc)
}

fn ibp_power_times(pow_expr: &Expr, other: &Expr, var: &str) -> Option<Expr> {
    let n = var_power(pow_expr, var)?;
    match other {
        Expr::Exp(w) => {
            let a = linear_coeff_expr(w, var)?;
            let w = (**w).clone();
            let e = Expr::exp(w);
            let mut acc = Expr::div(e.clone(), a.clone());
            for k in 1..=n {
                let term = Expr::div(Expr::mul(x_power(var, k), e.clone()), a.clone());
                let sub = Expr::mul(Expr::div(Expr::int(i64::from(k)), a.clone()), acc);
                acc = Expr::sub(term, sub);
            }
            Some(acc)
        }
        Expr::Sin(w) => {
            let a = linear_coeff_expr(w, var)?;
            let w = (**w).clone();
            let (s, _) = trig_powers(n, &w, var, &a);
            Some(s)
        }
        Expr::Cos(w) => {
            let a = linear_coeff_expr(w, var)?;
            let w = (**w).clone();
            let (_, c) = trig_powers(n, &w, var, &a);
            Some(c)
        }
        _ => None,
    }
}
