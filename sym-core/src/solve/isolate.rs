//! Variable isolation for invertible elementary functions and linear forms.

use crate::expr::Expr;

/// Isolate `var` from equations of the form `lhs - rhs = 0` or `lhs + rhs = 0`.
pub(crate) fn isolate(expr: &Expr, var: &str) -> Option<Expr> {
    let (lhs, rhs) = match expr {
        Expr::Sub(a, b) => ((**a).clone(), (**b).clone()),
        Expr::Add(a, b) => ((**a).clone(), Expr::neg((**b).clone())),
        _ => return None,
    };

    // Orient so the side containing `var` is on the left, and reject circular occurrences.
    let lhs_has = lhs.variables().iter().any(|v| v == var);
    let rhs_has = rhs.variables().iter().any(|v| v == var);
    let (func_side, const_side) = match (lhs_has, rhs_has) {
        (true, false) => (lhs, rhs),
        (false, true) => (rhs, lhs),
        _ => return None, // Rejects missing var or circular `exp(x) - x = 0`
    };

    if let Some(root) = solve_linear_pair(&func_side, &const_side, var) {
        return Some(root);
    }

    let (inner, target) = match func_side {
        Expr::Exp(u) => (*u, Expr::ln(const_side)),
        Expr::Ln(u) => (*u, Expr::exp(const_side)),
        Expr::Sin(u) => (*u, Expr::asin(const_side)),
        Expr::Cos(u) => (*u, Expr::acos(const_side)),
        Expr::Tan(u) => (*u, Expr::atan(const_side)),
        Expr::Sqrt(u) => (*u, Expr::pow(const_side, Expr::int(2))),
        _ => return None,
    };
    if target.variables().iter().any(|v| v == var) {
        return None;
    }
    solve_linear_pair(&inner, &target, var)
}

/// Solve `lhs = rhs` where `lhs` is linear in `var` (`var`, `a*var + b`, `b - var`, etc.).
fn solve_linear_pair(lhs: &Expr, rhs: &Expr, var: &str) -> Option<Expr> {
    match lhs {
        Expr::Var(n) if n == var => {
            if rhs.variables().iter().any(|v| v == var) {
                None
            } else {
                Some(rhs.clone())
            }
        }
        Expr::Neg(inner) => solve_linear_pair(inner, &Expr::neg(rhs.clone()), var),
        Expr::Add(a, b) => {
            if !a.variables().iter().any(|v| v == var) {
                solve_linear_pair(b, &Expr::sub(rhs.clone(), (**a).clone()), var)
            } else if !b.variables().iter().any(|v| v == var) {
                solve_linear_pair(a, &Expr::sub(rhs.clone(), (**b).clone()), var)
            } else {
                None
            }
        }
        Expr::Sub(a, b) => {
            if !b.variables().iter().any(|v| v == var) {
                solve_linear_pair(a, &Expr::add(rhs.clone(), (**b).clone()), var)
            } else if !a.variables().iter().any(|v| v == var) {
                solve_linear_pair(b, &Expr::sub((**a).clone(), rhs.clone()), var)
            } else {
                None
            }
        }
        Expr::Mul(a, b) => {
            if !a.variables().iter().any(|v| v == var) && !a.is_zero() {
                solve_linear_pair(b, &Expr::div(rhs.clone(), (**a).clone()), var)
            } else if !b.variables().iter().any(|v| v == var) && !b.is_zero() {
                solve_linear_pair(a, &Expr::div(rhs.clone(), (**b).clone()), var)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::solve;
    use crate::expr::Expr;

    #[test]
    fn isolates_exponential() {
        let x = Expr::var("x");
        let e = Expr::sub(Expr::exp(x), Expr::int(2));
        let sols = solve(&e, "x");
        assert_eq!(sols.0, vec![Expr::ln(Expr::int(2))]);
    }
}
