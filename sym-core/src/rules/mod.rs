//! Rule aggregator, fast-math configuration, and provenance queries.
//!
//! Provides collections of rewrite rules categorized by algebraic domain,
//! with fast-math gating and Maxima Common Lisp provenance traceability.

pub mod arithmetic;
pub mod exponential;
pub mod factorial;
pub mod hyperbolic;
pub mod logarithmic;
pub mod rational;
pub mod trigonometric;

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::Rewrite;

/// Configuration controlling which rule sets are enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleConfig {
    /// Enable fast-math (real-number ℝ) transformations that may alter IEEE-754 rounding.
    pub fast_math: bool,
    /// Enable trigonometric identities (`trigi.lisp`).
    pub trigonometric: bool,
    /// Enable exponential identities (`simp.lisp` / `logarc.lisp`).
    pub exponential: bool,
    /// Enable logarithmic identities (`logarc.lisp`).
    pub logarithmic: bool,
    /// Enable hyperbolic and inverse-trigonometric identities (`sinh.lisp` / `trigi.lisp`).
    pub hyperbolic: bool,
    /// Enable rational normalization (`nrat4.lisp`).
    pub rational: bool,
    /// Enable factorial simplification (`sfacts.lisp`).
    pub factorial: bool,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            fast_math: true,
            trigonometric: true,
            exponential: true,
            logarithmic: true,
            hyperbolic: true,
            rational: true,
            factorial: true,
        }
    }
}

impl RuleConfig {
    /// Strict IEEE-754 mode: disable fast-math optimizations.
    pub fn strict_ieee() -> Self {
        Self {
            fast_math: false,
            ..Default::default()
        }
    }
}

/// Collect all rewrite rules matching the given configuration.
pub fn get_rules(config: &RuleConfig) -> Vec<Rewrite<MathLang, MathAnalysis>> {
    get_sourced_rules(config)
        .into_iter()
        .map(|r| r.rewrite)
        .collect()
}

/// Collect all sourced rewrite rules with 1:1 Maxima citations.
pub fn get_sourced_rules(config: &RuleConfig) -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    let mut rules = Vec::new();

    // 1. Arithmetic rules (always included)
    for r in arithmetic::arithmetic_rules() {
        if config.fast_math || !r.provenance.fast_math_only {
            rules.push(r);
        }
    }

    // 2. Trigonometric rules
    if config.trigonometric {
        for r in trigonometric::trigonometric_rules() {
            if config.fast_math || !r.provenance.fast_math_only {
                rules.push(r);
            }
        }
    }

    // 3. Exponential rules
    if config.exponential {
        for r in exponential::exponential_rules() {
            if config.fast_math || !r.provenance.fast_math_only {
                rules.push(r);
            }
        }
    }

    // 4. Logarithmic rules
    if config.logarithmic {
        for r in logarithmic::logarithmic_rules() {
            if config.fast_math || !r.provenance.fast_math_only {
                rules.push(r);
            }
        }
    }

    // 5. Hyperbolic and inverse-trigonometric rules
    if config.hyperbolic {
        for r in hyperbolic::hyperbolic_rules() {
            if config.fast_math || !r.provenance.fast_math_only {
                rules.push(r);
            }
        }
    }

    // 6. Rational normalization rules
    if config.rational {
        for r in rational::rational_rules() {
            if config.fast_math || !r.provenance.fast_math_only {
                rules.push(r);
            }
        }
    }

    // 7. Factorial rules
    if config.factorial {
        for r in factorial::factorial_rules() {
            if config.fast_math || !r.provenance.fast_math_only {
                rules.push(r);
            }
        }
    }

    rules
}

/// Query provenance information for all rules currently registered in maxima-rs.
pub fn all_rule_provenances() -> Vec<(&'static str, RuleProvenance)> {
    let config = RuleConfig::default();
    get_sourced_rules(&config)
        .into_iter()
        .map(|r| (r.name, r.provenance))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_collection() {
        let config = RuleConfig::default();
        let rules = get_rules(&config);
        assert!(!rules.is_empty());
        assert!(rules.len() >= 92);
    }

    #[test]
    fn test_strict_ieee_excludes_fast_math() {
        let default_rules = get_rules(&RuleConfig::default());
        let strict_rules = get_rules(&RuleConfig::strict_ieee());
        assert!(strict_rules.len() < default_rules.len());
    }

    #[test]
    fn test_provenance_citations_present() {
        let provenances = all_rule_provenances();
        for (name, prov) in &provenances {
            assert!(!name.is_empty());
            assert!(!prov.source_file.is_empty());
            assert!(!prov.source_fn.is_empty());
            assert!(!prov.identity.is_empty());
        }
    }
}
