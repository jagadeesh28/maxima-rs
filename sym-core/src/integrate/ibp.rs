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
    let mut s = Expr::div(Expr::neg(Expr::cos(w.clone())), a.clone());
    let mut c = Expr::div(Expr::sin(w.clone()), a.clone());
    for k in 1..=n {
        let xp = x_power(var, k);
        let ns = Expr::add(
            Expr::div(
                Expr::neg(Expr::mul(xp.clone(), Expr::cos(w.clone()))),
                a.clone(),
            ),
            Expr::mul(Expr::div(Expr::int(i64::from(k)), a.clone()), c.clone()),
        );
        let nc = Expr::sub(
            Expr::div(Expr::mul(xp, Expr::sin(w.clone())), a.clone()),
            Expr::mul(Expr::div(Expr::int(i64::from(k)), a.clone()), s.clone()),
        );
        s = ns;
        c = nc;
    }
    (s, c)
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
