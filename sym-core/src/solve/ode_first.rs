//! First-order ODE solver: linear integrating factor, separable, and exact (Phase 30 / Phase 50).

use super::ode::OdeSolution;
use crate::expr::Expr;
#[cfg(feature = "integrate")]
use crate::integrate::{contains_integral, integrate};
use crate::poly::rat::ratsimp;

/// Solve `a(x)·y' + b(x)·y = q(x)` for `y(x)`.
///
/// Uses closed-form `y = _C1·exp(-(b/a)·x)` when `a, b` are constant and `q == 0`,
/// and integrating factor `mu(x) = exp(∫ (b(x)/a(x)) dx)` in the general case.
pub(crate) fn solve_first_linear(a: &Expr, b: &Expr, q: &Expr, x: &str) -> Option<OdeSolution> {
    if a.is_zero() {
        return None;
    }
    let a_has_x = a.variables().iter().any(|v| v == x);
    let b_has_x = b.variables().iter().any(|v| v == x);
    if q.is_zero() && !a_has_x && !b_has_x {
        let rate = Expr::neg(Expr::div(b.clone(), a.clone()));
        let solution = Expr::mul(Expr::var("_C1"), Expr::exp(Expr::mul(rate, Expr::var(x))));
        return Some(OdeSolution {
            solution,
            constants: vec!["_C1".to_string()],
        });
    }

    #[cfg(feature = "integrate")]
    {
        let p = ratsimp(&Expr::div(b.clone(), a.clone()))
            .unwrap_or_else(|_| Expr::div(b.clone(), a.clone()));
        let int_p = if p.is_zero() {
            Expr::int(0)
        } else {
            let r = integrate(&p, x);
            if contains_integral(&r) {
                return None;
            }
            r
        };
        let mu = if int_p.is_zero() {
            Expr::int(1)
        } else {
            simplify_exp_ln(Expr::exp(int_p))
        };
        if q.is_zero() {
            let solution = Expr::div(Expr::var("_C1"), mu);
            return Some(OdeSolution {
                solution,
                constants: vec!["_C1".to_string()],
            });
        }
        let rhs_integrand = ratsimp(&Expr::div(Expr::mul(mu.clone(), q.clone()), a.clone()))
            .unwrap_or_else(|_| Expr::div(Expr::mul(mu.clone(), q.clone()), a.clone()));
        let int_rhs = integrate(&rhs_integrand, x);
        if contains_integral(&int_rhs) {
            return None;
        }
        let solution = Expr::div(Expr::add(Expr::var("_C1"), int_rhs), mu);
        Some(OdeSolution {
            solution,
            constants: vec!["_C1".to_string()],
        })
    }
    #[cfg(not(feature = "integrate"))]
    {
        let _ = x;
        None
    }
}

#[cfg(feature = "integrate")]
fn unwrap_ln_term(e: &Expr) -> Option<Expr> {
    match e {
        Expr::Ln(arg) => Some((**arg).clone()),
        Expr::Neg(inner) => unwrap_ln_term(inner).map(|a| Expr::pow(a, Expr::int(-1))),
        Expr::Mul(a, b) => {
            if a.is_one() {
                unwrap_ln_term(b)
            } else if b.is_one() {
                unwrap_ln_term(a)
            } else if let Expr::Const(r) = a.as_ref() {
                unwrap_ln_term(b).map(|arg| Expr::pow(arg, Expr::constant(r.clone())))
            } else if let Expr::Const(r) = b.as_ref() {
                unwrap_ln_term(a).map(|arg| Expr::pow(arg, Expr::constant(r.clone())))
            } else {
                None
            }
        }
        Expr::Div(a, b) => {
            if b.is_one() {
                unwrap_ln_term(a)
            } else if let Expr::Const(r) = b.as_ref() {
                let inv = num_rational::BigRational::from_integer(1.into()) / r;
                unwrap_ln_term(a).map(|arg| Expr::pow(arg, Expr::constant(inv)))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(feature = "integrate")]
fn simplify_exp_ln(e: Expr) -> Expr {
    if let Expr::Exp(inner) = &e
        && let Some(unwrapped) = unwrap_ln_term(inner)
    {
        return unwrapped;
    }
    e
}

/// Attempt separable (`y' - f(x)*g(y) = 0`) or exact (`M(x,y) + N(x,y)*y' = 0`) 1st-order ODEs.
pub(crate) fn solve_separable_or_exact(ode: &Expr, y: &str, x: &str) -> Option<OdeSolution> {
    #[cfg(feature = "integrate")]
    {
        // Expect `M(x, y) + N(x, y) * y' = 0` or `y' - rhs = 0`
        let (m_expr, n_expr) = extract_pfaffian(ode, y, x)?;
        if n_expr.is_zero() {
            return None;
        }
        // Check exactness: dM/dy == dN/dx
        let dm_dy = crate::diff::differentiate(&m_expr, y);
        let dn_dx = crate::diff::differentiate(&n_expr, x);
        let diff_exact = ratsimp(&Expr::sub(dm_dy, dn_dx)).ok()?;
        if diff_exact.is_zero() {
            let phi_x = integrate(&m_expr, x);
            if !contains_integral(&phi_x) {
                let dphi_dy = crate::diff::differentiate(&phi_x, y);
                let rem_y = ratsimp(&Expr::sub(n_expr.clone(), dphi_dy)).ok()?;
                let phi_y = if rem_y.is_zero() {
                    Expr::int(0)
                } else {
                    let r = integrate(&rem_y, y);
                    if contains_integral(&r) {
                        return None;
                    }
                    r
                };
                let potential = Expr::add(phi_x, phi_y);
                let eq = Expr::sub(potential, Expr::var("_C1"));
                let sols = super::solve(&eq, y);
                if let Some(sol) = sols.0.into_iter().next() {
                    return Some(OdeSolution {
                        solution: sol,
                        constants: vec!["_C1".to_string()],
                    });
                }
            }
        }
        // Separable: -M / N = f(x) * g(y)
        let ratio = ratsimp(&Expr::neg(Expr::div(m_expr, n_expr))).ok()?;
        if let Some((fx, gy)) = factor_separable(&ratio, y, x) {
            let lhs = integrate(&Expr::div(Expr::int(1), gy), y);
            let rhs = integrate(&fx, x);
            if !contains_integral(&lhs) && !contains_integral(&rhs) {
                let eq = Expr::sub(lhs, Expr::add(rhs, Expr::var("_C1")));
                let sols = super::solve(&eq, y);
                if let Some(sol) = sols.0.into_iter().next() {
                    return Some(OdeSolution {
                        solution: sol,
                        constants: vec!["_C1".to_string()],
                    });
                }
            }
        }
        None
    }
    #[cfg(not(feature = "integrate"))]
    {
        let _ = (ode, y, x);
        None
    }
}

#[cfg(feature = "integrate")]
fn is_yprime(e: &Expr, y: &str, x: &str) -> bool {
    matches!(e, Expr::Derivative(inner, var)
        if matches!(inner.as_ref(), Expr::Var(yn) if yn == y)
        && matches!(var.as_ref(), Expr::Var(xn) if xn == x))
}

#[cfg(feature = "integrate")]
fn extract_pfaffian(ode: &Expr, y: &str, x: &str) -> Option<(Expr, Expr)> {
    match ode {
        Expr::Add(a, b) => {
            let (ma, na) = extract_pfaffian(a, y, x)?;
            let (mb, nb) = extract_pfaffian(b, y, x)?;
            Some((Expr::add(ma, mb), Expr::add(na, nb)))
        }
        Expr::Sub(a, b) => {
            let (ma, na) = extract_pfaffian(a, y, x)?;
            let (mb, nb) = extract_pfaffian(b, y, x)?;
            Some((Expr::sub(ma, mb), Expr::sub(na, nb)))
        }
        Expr::Neg(a) => {
            let (ma, na) = extract_pfaffian(a, y, x)?;
            Some((Expr::neg(ma), Expr::neg(na)))
        }
        Expr::Mul(a, b) => {
            if is_yprime(a, y, x) && !has_derivative(b) {
                return Some((Expr::int(0), (**b).clone()));
            }
            if is_yprime(b, y, x) && !has_derivative(a) {
                return Some((Expr::int(0), (**a).clone()));
            }
            if !has_derivative(ode) {
                Some((ode.clone(), Expr::int(0)))
            } else {
                None
            }
        }
        e if is_yprime(e, y, x) => Some((Expr::int(0), Expr::int(1))),
        e if !has_derivative(e) => Some((e.clone(), Expr::int(0))),
        _ => None,
    }
}

#[cfg(feature = "integrate")]
fn has_derivative(e: &Expr) -> bool {
    match e {
        Expr::Derivative(_, _) => true,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            has_derivative(a) || has_derivative(b)
        }
        Expr::Neg(a)
        | Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::Exp(a)
        | Expr::Ln(a)
        | Expr::Sqrt(a) => has_derivative(a),
        _ => false,
    }
}

#[cfg(feature = "integrate")]
fn factor_separable(e: &Expr, y: &str, x: &str) -> Option<(Expr, Expr)> {
    let has_x = e.variables().iter().any(|v| v == x);
    let has_y = e.variables().iter().any(|v| v == y);
    if !has_x {
        return Some((Expr::int(1), e.clone()));
    }
    if !has_y {
        return Some((e.clone(), Expr::int(1)));
    }
    match e {
        Expr::Mul(a, b) => {
            let ax = a.variables().iter().any(|v| v == x);
            let ay = a.variables().iter().any(|v| v == y);
            let bx = b.variables().iter().any(|v| v == x);
            let by = b.variables().iter().any(|v| v == y);
            if !ay && !bx {
                return Some(((**a).clone(), (**b).clone()));
            }
            if !ax && !by {
                return Some(((**b).clone(), (**a).clone()));
            }
            None
        }
        Expr::Div(a, b) => {
            let ax = a.variables().iter().any(|v| v == x);
            let ay = a.variables().iter().any(|v| v == y);
            let bx = b.variables().iter().any(|v| v == x);
            let by = b.variables().iter().any(|v| v == y);
            if !ay && !bx {
                return Some(((**a).clone(), Expr::div(Expr::int(1), (**b).clone())));
            }
            if !ax && !by {
                return Some((Expr::div(Expr::int(1), (**b).clone()), (**a).clone()));
            }
            None
        }
        _ => None,
    }
}
