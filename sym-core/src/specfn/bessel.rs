//! Bessel functions of the first and second kinds ($J, Y$) and modified Bessel functions ($I, K$).
//!
//! Citations:
//! - Maxima 5.50: `src/bessel.lisp:55, 366, 693, 1021`.
//! - Abramowitz & Stegun: Handbook of Mathematical Functions, Sections 9.1.27, 9.6.29.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::rational::{BigRational, Rat};
use crate::specfn::SpecialFnKind;

/// Evaluate known exact values of $J_n(z)$.
///
/// Maxima `src/bessel.lisp`.
pub fn bessel_j_eval(n: &Expr, z: &Expr) -> Option<Expr> {
    if z.is_zero() {
        if n.is_zero() {
            Some(Expr::int(1))
        } else if let Expr::Const(r) = n {
            if Rat::is_integer(r) && r > &BigRational::from_integer(0.into()) {
                Some(Expr::int(0))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Evaluate known exact values of $I_n(z)$.
///
/// Maxima `src/bessel.lisp`.
pub fn bessel_i_eval(n: &Expr, z: &Expr) -> Option<Expr> {
    if z.is_zero() {
        if n.is_zero() {
            Some(Expr::int(1))
        } else if let Expr::Const(r) = n {
            if Rat::is_integer(r) && r > &BigRational::from_integer(0.into()) {
                Some(Expr::int(0))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Differentiate $J_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} J_n(u) = \frac{1}{2}(J_{n-1}(u) - J_{n+1}(u))$$
///
/// Citations: Maxima 5.50 `src/bessel.lisp:55-66` citing Abramowitz & Stegun 9.1.27.
pub fn diff_bessel_j(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let n_plus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::add(r, &Rat::from_int(1))),
        _ => Expr::add(n.clone(), Expr::int(1)),
    };

    let j_prev = Expr::special_fn(SpecialFnKind::BesselJ, vec![n_minus_1, u.clone()]);
    let j_next = Expr::special_fn(SpecialFnKind::BesselJ, vec![n_plus_1, u.clone()]);
    let diff_term = Expr::div(Expr::sub(j_prev, j_next), Expr::int(2));

    if du.is_one() {
        diff_term
    } else {
        Expr::mul(diff_term, du)
    }
}

/// Differentiate $Y_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} Y_n(u) = \frac{1}{2}(Y_{n-1}(u) - Y_{n+1}(u))$$
///
/// Citations: Maxima 5.50 `src/bessel.lisp:366-376` citing Abramowitz & Stegun 9.1.27.
pub fn diff_bessel_y(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let n_plus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::add(r, &Rat::from_int(1))),
        _ => Expr::add(n.clone(), Expr::int(1)),
    };

    let y_prev = Expr::special_fn(SpecialFnKind::BesselY, vec![n_minus_1, u.clone()]);
    let y_next = Expr::special_fn(SpecialFnKind::BesselY, vec![n_plus_1, u.clone()]);
    let diff_term = Expr::div(Expr::sub(y_prev, y_next), Expr::int(2));

    if du.is_one() {
        diff_term
    } else {
        Expr::mul(diff_term, du)
    }
}

/// Differentiate $I_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} I_n(u) = \frac{1}{2}(I_{n-1}(u) + I_{n+1}(u))$$
///
/// Citations: Maxima 5.50 `src/bessel.lisp:693-703` citing Abramowitz & Stegun 9.6.29.
pub fn diff_bessel_i(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let n_plus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::add(r, &Rat::from_int(1))),
        _ => Expr::add(n.clone(), Expr::int(1)),
    };

    let i_prev = Expr::special_fn(SpecialFnKind::BesselI, vec![n_minus_1, u.clone()]);
    let i_next = Expr::special_fn(SpecialFnKind::BesselI, vec![n_plus_1, u.clone()]);
    let diff_term = Expr::div(Expr::add(i_prev, i_next), Expr::int(2));

    if du.is_one() {
        diff_term
    } else {
        Expr::mul(diff_term, du)
    }
}

/// Differentiate $K_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} K_n(u) = -\frac{1}{2}(K_{n-1}(u) + K_{n+1}(u))$$
///
/// Citations: Maxima 5.50 `src/bessel.lisp:1021-1030` citing Abramowitz & Stegun 9.6.29.
pub fn diff_bessel_k(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let n = &args[0];
    let u = &args[1];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }

    let n_minus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
        _ => Expr::sub(n.clone(), Expr::int(1)),
    };
    let n_plus_1 = match n {
        Expr::Const(r) => Expr::constant(Rat::add(r, &Rat::from_int(1))),
        _ => Expr::add(n.clone(), Expr::int(1)),
    };

    let k_prev = Expr::special_fn(SpecialFnKind::BesselK, vec![n_minus_1, u.clone()]);
    let k_next = Expr::special_fn(SpecialFnKind::BesselK, vec![n_plus_1, u.clone()]);
    let diff_term = Expr::neg(Expr::div(Expr::add(k_prev, k_next), Expr::int(2)));

    if du.is_one() {
        diff_term
    } else {
        Expr::mul(diff_term, du)
    }
}
