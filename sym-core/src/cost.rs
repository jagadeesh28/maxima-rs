//! FLOP-weighted cost model for extracting optimal expressions from e-graphs.
//!
//! Assigns empirical relative cycle costs to mathematical operations:
//! - Addition / Subtraction / Negation: 1 FLOP
//! - Multiplication: 2 FLOPs
//! - Division: 8 FLOPs
//! - Powers / Roots: 10 FLOPs
//! - Transcendentals (sin, cos, tan, exp, ln): 25 FLOPs
//! - Variables and Constants: 0 FLOPs

use crate::lang::MathLang;
use egg::{CostFunction, Id, Language};

/// FLOP-weighted cost function for [`egg::Extractor`].
#[derive(Debug, Clone, Copy, Default)]
pub struct FlopCost;

impl CostFunction<MathLang> for FlopCost {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &MathLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            MathLang::Var(_) | MathLang::Const(_) => 0,
            MathLang::Add(_) | MathLang::Sub(_) | MathLang::Neg(_) => 1,
            MathLang::Mul(_) => 2,
            MathLang::Div(_) => 8,
            MathLang::Pow(_) | MathLang::Sqrt(_) => 10,
            MathLang::Sin(_)
            | MathLang::Cos(_)
            | MathLang::Tan(_)
            | MathLang::Exp(_)
            | MathLang::Ln(_) => 25,
            MathLang::Tensor(_) => 0,
            MathLang::Derivative(_)
            | MathLang::Integral(_)
            | MathLang::Sum(_)
            | MathLang::Matrix(_) => 0,
        };

        enode.fold(op_cost, |sum, id| sum.saturating_add(costs(id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Expr;

    #[test]
    fn test_flop_cost_comparison() {
        let x = Expr::var("x");
        // x / 2 vs x * 0.5
        let div_expr = Expr::div(x.clone(), Expr::int(2)).to_recexpr();
        let mul_expr = Expr::mul(x, Expr::frac(1, 2)).to_recexpr();

        let mut cost_fn = FlopCost;
        let cost_div = cost_fn.cost_rec(&div_expr);
        let cost_mul = cost_fn.cost_rec(&mul_expr);

        assert!(
            cost_mul < cost_div,
            "Multiplication should be cheaper than division"
        );
    }
}
