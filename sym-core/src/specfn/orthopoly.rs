//! Orthogonal Polynomials (Legendre, Chebyshev $T/U$, Hermite, Laguerre).
//!
//! Citations:
//! - Maxima 5.50: `share/orthopoly/orthopoly.lisp:489, 521, 558, 801, 885`.
//! - Abramowitz & Stegun: Handbook of Mathematical Functions, Ch. 22.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::rational::{BigRational, Rat};
use crate::specfn::SpecialFnKind;

fn get_poly_degree(n: &Expr) -> Option<usize> {
    if let Expr::Const(r) = n
        && Rat::is_integer(r)
        && r >= &BigRational::from_integer(0.into())
    {
        let deg: usize = r.to_integer().try_into().ok()?;
        if deg <= 100 {
            return Some(deg);
        }
    }
    None
}

/// Generate the $n$-th Legendre polynomial $P_n(x)$ via the 3-term recurrence relation:
///
/// $$(n+1) P_{n+1}(x) = (2n+1) x P_n(x) - n P_{n-1}(x)$$
///
/// Maxima `share/orthopoly/orthopoly.lisp:558-575`.
pub fn legendre_p_poly(n: usize, x: &Expr) -> Expr {
    match n {
        0 => Expr::int(1),
        1 => x.clone(),
        _ => {
            let mut p_prev = Expr::int(1);
            let mut p_curr = x.clone();
            for k in 1..n {
                // p_next = ((2k+1)*x*p_curr - k*p_prev) / (k+1)
                let coeff1 = Expr::int((2 * k + 1) as i64);
                let term1 = Expr::mul(Expr::mul(coeff1, x.clone()), p_curr.clone());
                let term2 = if p_prev.is_one() {
                    Expr::int(k as i64)
                } else if p_prev.is_zero() {
                    Expr::int(0)
                } else {
                    Expr::mul(Expr::int(k as i64), p_prev)
                };
                let num = Expr::sub(term1, term2);
                let p_next = Expr::div(num, Expr::int((k + 1) as i64));
                p_prev = p_curr;
                p_curr = p_next;
            }
            p_curr
        }
    }
}

/// Generate the $n$-th Chebyshev polynomial of the first kind $T_n(x)$:
///
/// $$T_{n+1}(x) = 2x T_n(x) - T_{n-1}(x)$$
///
/// Maxima `share/orthopoly/orthopoly.lisp:489-507`.
pub fn chebyshev_t_poly(n: usize, x: &Expr) -> Expr {
    match n {
        0 => Expr::int(1),
        1 => x.clone(),
        _ => {
            let mut t_prev = Expr::int(1);
            let mut t_curr = x.clone();
            for _ in 1..n {
                let term1 = Expr::mul(Expr::mul(Expr::int(2), x.clone()), t_curr.clone());
                let t_next = Expr::sub(term1, t_prev);
                t_prev = t_curr;
                t_curr = t_next;
            }
            t_curr
        }
    }
}

/// Generate the $n$-th Chebyshev polynomial of the second kind $U_n(x)$:
///
/// $$U_{n+1}(x) = 2x U_n(x) - U_{n-1}(x)$$
///
/// Maxima `share/orthopoly/orthopoly.lisp:521-543`.
pub fn chebyshev_u_poly(n: usize, x: &Expr) -> Expr {
    match n {
        0 => Expr::int(1),
        1 => Expr::mul(Expr::int(2), x.clone()),
        _ => {
            let mut u_prev = Expr::int(1);
            let mut u_curr = Expr::mul(Expr::int(2), x.clone());
            for _ in 1..n {
                let term1 = Expr::mul(Expr::mul(Expr::int(2), x.clone()), u_curr.clone());
                let u_next = Expr::sub(term1, u_prev);
                u_prev = u_curr;
                u_curr = u_next;
            }
            u_curr
        }
    }
}

/// Generate the $n$-th Hermite polynomial $H_n(x)$ (physicist's convention):
///
/// $$H_{n+1}(x) = 2x H_n(x) - 2n H_{n-1}(x)$$
///
/// Maxima `share/orthopoly/orthopoly.lisp:801-825`.
pub fn hermite_h_poly(n: usize, x: &Expr) -> Expr {
    match n {
        0 => Expr::int(1),
        1 => Expr::mul(Expr::int(2), x.clone()),
        _ => {
            let mut h_prev = Expr::int(1);
            let mut h_curr = Expr::mul(Expr::int(2), x.clone());
            for k in 1..n {
                let term1 = Expr::mul(Expr::mul(Expr::int(2), x.clone()), h_curr.clone());
                let term2 = if h_prev.is_one() {
                    Expr::int((2 * k) as i64)
                } else if h_prev.is_zero() {
                    Expr::int(0)
                } else {
                    Expr::mul(Expr::int((2 * k) as i64), h_prev)
                };
                let h_next = Expr::sub(term1, term2);
                h_prev = h_curr;
                h_curr = h_next;
            }
            h_curr
        }
    }
}

/// Generate the $n$-th Laguerre polynomial $L_n(x)$:
///
/// $$(n+1) L_{n+1}(x) = (2n+1-x) L_n(x) - n L_{n-1}(x)$$
///
/// Maxima `share/orthopoly/orthopoly.lisp:885-903`.
pub fn laguerre_l_poly(n: usize, x: &Expr) -> Expr {
    match n {
        0 => Expr::int(1),
        1 => Expr::sub(Expr::int(1), x.clone()),
        _ => {
            let mut l_prev = Expr::int(1);
            let mut l_curr = Expr::sub(Expr::int(1), x.clone());
            for k in 1..n {
                let factor = Expr::sub(Expr::int((2 * k + 1) as i64), x.clone());
                let term1 = Expr::mul(factor, l_curr.clone());
                let term2 = if l_prev.is_one() {
                    Expr::int(k as i64)
                } else if l_prev.is_zero() {
                    Expr::int(0)
                } else {
                    Expr::mul(Expr::int(k as i64), l_prev)
                };
                let num = Expr::sub(term1, term2);
                let l_next = Expr::div(num, Expr::int((k + 1) as i64));
                l_prev = l_curr;
                l_curr = l_next;
            }
            l_curr
        }
    }
}

/// Differentiate $P_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} P_n(u) = \frac{n(u P_n(u) - P_{n-1}(u))}{u^2 - 1}$$
///
/// Citations: Maxima 5.50 `share/orthopoly/orthopoly.lisp:565-572`.
pub fn diff_legendre_p(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    // If n is a small constant integer, differentiate explicit polynomial
    if let Some(deg) = get_poly_degree(n) {
        let poly = legendre_p_poly(deg, u);
        return differentiate(&poly, var);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let pn = Expr::special_fn(SpecialFnKind::LegendreP, vec![n.clone(), u.clone()]);
    let pn_prev = Expr::special_fn(SpecialFnKind::LegendreP, vec![n_minus_1, u.clone()]);
    let num = Expr::mul(n.clone(), Expr::sub(Expr::mul(u.clone(), pn), pn_prev));
    let den = Expr::sub(Expr::pow(u.clone(), Expr::int(2)), Expr::int(1));
    let deriv = Expr::div(num, den);

    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}

/// Differentiate $T_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} T_n(u) = n U_{n-1}(u)$$
///
/// Citations: Maxima 5.50 `share/orthopoly/orthopoly.lisp:497-505`.
pub fn diff_chebyshev_t(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    if let Some(deg) = get_poly_degree(n) {
        let poly = chebyshev_t_poly(deg, u);
        return differentiate(&poly, var);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let un_prev = Expr::special_fn(SpecialFnKind::ChebyshevU, vec![n_minus_1, u.clone()]);
    let deriv = Expr::mul(n.clone(), un_prev);

    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}

/// Differentiate $U_n(u)$ with respect to `var`:
///
/// Citations: Maxima 5.50 `share/orthopoly/orthopoly.lisp:531-540`.
pub fn diff_chebyshev_u(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    if let Some(deg) = get_poly_degree(n) {
        let poly = chebyshev_u_poly(deg, u);
        return differentiate(&poly, var);
    }

    let n_plus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::add(r, &Rat::from_int(1))),
        _ => Expr::add(n.clone(), Expr::int(1)),
    };
    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let un = Expr::special_fn(SpecialFnKind::ChebyshevU, vec![n.clone(), u.clone()]);
    let un_prev = Expr::special_fn(SpecialFnKind::ChebyshevU, vec![n_minus_1, u.clone()]);
    let term1 = Expr::mul(n_plus_1, un_prev);
    let term2 = Expr::mul(Expr::mul(n.clone(), u.clone()), un);
    let num = Expr::sub(term1, term2);
    let den = Expr::sub(Expr::int(1), Expr::pow(u.clone(), Expr::int(2)));
    let deriv = Expr::div(num, den);

    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}

/// Differentiate $H_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} H_n(u) = 2n H_{n-1}(u)$$
///
/// Citations: Maxima 5.50 `share/orthopoly/orthopoly.lisp:819-823`.
pub fn diff_hermite_h(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    if let Some(deg) = get_poly_degree(n) {
        let poly = hermite_h_poly(deg, u);
        return differentiate(&poly, var);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let hn_prev = Expr::special_fn(SpecialFnKind::HermiteH, vec![n_minus_1, u.clone()]);
    let two_n = Expr::mul(Expr::int(2), n.clone());
    let deriv = Expr::mul(two_n, hn_prev);

    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}

/// Differentiate $L_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} L_n(u) = \frac{n(L_n(u) - L_{n-1}(u))}{u}$$
///
/// Citations: Maxima 5.50 `share/orthopoly/orthopoly.lisp:893-900`.
pub fn diff_laguerre_l(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    if let Some(deg) = get_poly_degree(n) {
        let poly = laguerre_l_poly(deg, u);
        return differentiate(&poly, var);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let ln = Expr::special_fn(SpecialFnKind::LaguerreL, vec![n.clone(), u.clone()]);
    let ln_prev = Expr::special_fn(SpecialFnKind::LaguerreL, vec![n_minus_1, u.clone()]);
    let num = Expr::mul(n.clone(), Expr::sub(ln, ln_prev));
    let deriv = Expr::div(num, u.clone());

    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}
