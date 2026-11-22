//! Variable isolation for invertible elementary functions.

use crate::expr::Expr;

/// Isolate `var` from equations of the form `f(g(x)) = c`.
pub(crate) fn isolate(expr: &Expr, var: &str) -> Option<Expr> {
    let Expr::Sub(lhs, rhs) = expr else {
        return None;
    };
    let (inner, target) = match lhs.as_ref() {
        Expr::Exp(u) => ((**u).clone(), Expr::ln((**rhs).clone())),
        Expr::Ln(u) => ((**u).clone(), Expr::exp((**rhs).clone())),
        Expr::Sin(u) => ((**u).clone(), Expr::asin((**rhs).clone())),
        Expr::Cos(u) => ((**u).clone(), Expr::acos((**rhs).clone())),
        Expr::Tan(u) => ((**u).clone(), Expr::atan((**rhs).clone())),
        _ => return None,
    };
    solve_linear_var(&Expr::sub(inner, target), var)
}

fn solve_linear_var(eq: &Expr, var: &str) -> Option<Expr> {
    let Expr::Sub(a, b) = eq else {
        return None;
    };
    if let Expr::Var(n) = a.as_ref()
        && n.as_str() == var
    {
        return Some((**b).clone());
    }
    if let Expr::Var(n) = b.as_ref()
        && n.as_str() == var
    {
        return Some(Expr::neg((**a).clone()));
    }
    None
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
