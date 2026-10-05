//! E-Graph runner configuration and equality saturation routines.
//!
//! Provides bounded saturation limits (node limit, iteration limit, time limit)
//! to guarantee compile-time termination (ADR-0001, ADR-0005).

use egg::{EGraph, Id, Runner};
use std::time::Duration;

use crate::expr::Expr;
use crate::lang::{MathAnalysis, MathLang};
use crate::rules::{RuleConfig, get_rules};

/// Bounded limits and rule configuration for equality saturation.
#[derive(Debug, Clone)]
pub struct SaturationConfig {
    /// Maximum number of nodes permitted in the e-graph.
    pub node_limit: usize,
    /// Maximum number of saturation iterations.
    pub iter_limit: usize,
    /// Maximum duration allowed for saturation in seconds.
    pub time_limit_secs: f64,
    /// Configuration for rule selection.
    pub rule_config: RuleConfig,
}

impl Default for SaturationConfig {
    fn default() -> Self {
        Self {
            node_limit: 50_000,
            iter_limit: 30,
            time_limit_secs: 5.0,
            rule_config: RuleConfig::default(),
        }
    }
}

/// Run equality saturation on a set of expressions.
///
/// Returns the resulting [`EGraph`] along with the root [`Id`] corresponding
/// to each input expression.
pub fn saturate(
    exprs: &[Expr],
    config: &SaturationConfig,
) -> (EGraph<MathLang, MathAnalysis>, Vec<Id>) {
    let mut runner = Runner::default()
        .with_node_limit(config.node_limit)
        .with_iter_limit(config.iter_limit)
        .with_time_limit(Duration::from_secs_f64(config.time_limit_secs));

    let mut roots = Vec::with_capacity(exprs.len());
    for expr in exprs {
        let recexpr = expr.to_recexpr();
        let root = runner.egraph.add_expr(&recexpr);
        roots.push(root);
    }

    let rules = get_rules(&config.rule_config);
    let runner = runner.run(&rules);

    (runner.egraph, roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saturate_simple_simplification() {
        // x + 0 -> x
        let x = Expr::var("x");
        let expr = Expr::add(x.clone(), Expr::int(0));
        let config = SaturationConfig::default();

        let (egraph, roots) = saturate(&[expr], &config);
        assert_eq!(roots.len(), 1);

        // Verify x is in the same e-class as x + 0
        let x_rec = x.to_recexpr();
        let mut test_graph = egraph.clone();
        let x_id = test_graph.add_expr(&x_rec);
        assert_eq!(test_graph.find(roots[0]), test_graph.find(x_id));
    }
}
