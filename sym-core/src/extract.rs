//! Multi-root expression extraction from saturated e-graphs.
//!
//! Extracts cost-minimal expressions for one or more roots in an e-graph
//! using the FLOP-weighted cost model [`FlopCost`].

use egg::{EGraph, Extractor, Id};

use crate::cost::FlopCost;
use crate::expr::Expr;
use crate::lang::{MathAnalysis, MathLang};

/// Extract the cost-optimal [`Expr`] for a single e-class root.
pub fn extract_single(egraph: &EGraph<MathLang, MathAnalysis>, root: Id) -> Expr {
    let extractor = Extractor::new(egraph, FlopCost);
    let (_best_cost, best_expr) = extractor.find_best(root);
    Expr::from_recexpr(&best_expr)
}

/// Extract cost-optimal [`Expr`]s for multiple e-class roots simultaneously.
pub fn extract_all(egraph: &EGraph<MathLang, MathAnalysis>, roots: &[Id]) -> Vec<Expr> {
    let extractor = Extractor::new(egraph, FlopCost);
    roots
        .iter()
        .map(|&root| {
            let (_best_cost, best_expr) = extractor.find_best(root);
            Expr::from_recexpr(&best_expr)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saturate::{SaturationConfig, saturate};

    #[test]
    fn test_extract_pythagorean() {
        // sin(x)^2 + cos(x)^2 -> 1
        let x = Expr::var("x");
        let expr = Expr::add(
            Expr::pow(Expr::sin(x.clone()), Expr::int(2)),
            Expr::pow(Expr::cos(x), Expr::int(2)),
        );

        let (egraph, roots) = saturate(&[expr], &SaturationConfig::default());
        let extracted = extract_single(&egraph, roots[0]);
        assert_eq!(extracted, Expr::int(1));
    }

    #[test]
    fn test_extract_multi_root() {
        // [x + 0, y * 1] -> [x, y]
        let x = Expr::var("x");
        let y = Expr::var("y");
        let e1 = Expr::add(x.clone(), Expr::int(0));
        let e2 = Expr::mul(y.clone(), Expr::int(1));

        let (egraph, roots) = saturate(&[e1, e2], &SaturationConfig::default());
        let results = extract_all(&egraph, &roots);
        assert_eq!(results, vec![x, y]);
    }
}
