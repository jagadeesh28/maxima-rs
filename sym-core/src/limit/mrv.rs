//! Most Rapidly Varying (MRV) subexpression enumeration (Phase 27).
//!
//! Collects the exponential and logarithmic subexpressions of an expression with
//! respect to a variable — the candidates whose growth rates determine a limit.

use crate::expr::Expr;

/// Exponential/logarithmic subexpressions of `expr` that depend on `var`, ordered
/// by decreasing size (most rapidly varying first).
pub fn mrv_set(expr: &Expr, var: &str) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    collect(expr, var, &mut out);
    out.sort_by_key(|e| std::cmp::Reverse(e.node_count()));
    out.dedup();
    out
}

fn collect(e: &Expr, var: &str, out: &mut Vec<Expr>) {
    if !e.variables().iter().any(|v| v == var) {
        return;
    }
    if matches!(e, Expr::Exp(_) | Expr::Ln(_)) && !out.contains(e) {
        out.push(e.clone());
    }
    match e {
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            collect(a, var, out);
            collect(b, var, out);
        }
        Expr::Neg(a)
        | Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::Exp(a)
        | Expr::Ln(a)
        | Expr::Sqrt(a)
        | Expr::Sinh(a)
        | Expr::Cosh(a)
        | Expr::Tanh(a)
        | Expr::Asin(a)
        | Expr::Acos(a)
        | Expr::Atan(a)
        | Expr::Factorial(a) => collect(a, var, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_exp_and_ln() {
        let x = Expr::var("x");
        let e = Expr::add(Expr::exp(x.clone()), Expr::ln(x));
        let set = mrv_set(&e, "x");
        assert!(set.iter().any(|s| matches!(s, Expr::Exp(_))));
        assert!(set.iter().any(|s| matches!(s, Expr::Ln(_))));
    }

    #[test]
    fn empty_when_independent() {
        let e = Expr::add(Expr::var("y"), Expr::exp(Expr::var("y")));
        assert!(mrv_set(&e, "x").is_empty());
    }
}
