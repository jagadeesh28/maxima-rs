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
use egg::{EGraph, Rewrite, Subst, Var};
use num_traits::{Signed, Zero};

/// Maxima `domain` option variable (`src/globals.lisp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DomainMode {
    #[default]
    Real,
    Complex,
}

/// Maxima `radexpand` option variable (`src/globals.lisp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RadExpandMode {
    All,
    #[default]
    True,
    False,
}

/// Maxima `logexpand` option variable (`src/globals.lisp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogExpandMode {
    #[default]
    True,
    All,
    Super,
    False,
}

/// Maxima `triginverses` option variable (`src/globals.lisp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrigInversesMode {
    #[default]
    True,
    All,
    False,
}

/// Structured Maxima-aligned simplification flags (ADR-0013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpFlags {
    pub domain: DomainMode,
    pub radexpand: RadExpandMode,
    pub logexpand: LogExpandMode,
    pub triginverses: TrigInversesMode,
    pub fast_math: bool,
}

impl Default for SimpFlags {
    fn default() -> Self {
        Self {
            domain: DomainMode::Real,
            radexpand: RadExpandMode::True,
            logexpand: LogExpandMode::True,
            triginverses: TrigInversesMode::True,
            fast_math: true,
        }
    }
}

/// Condition guard: returns `true` unless `var` is known to be constant `0`.
pub fn is_not_zero(
    var: &'static str,
) -> impl Fn(&mut EGraph<MathLang, MathAnalysis>, egg::Id, &Subst) -> bool {
    move |egraph, _, subst| {
        let Ok(v) = var.parse::<Var>() else {
            return false;
        };
        !egraph[subst[v]].data.as_ref().is_some_and(|c| c.is_zero())
    }
}

/// Condition guard: returns `true` iff `var` is known to be a strictly positive constant (`> 0`).
pub fn is_positive(
    var: &'static str,
) -> impl Fn(&mut EGraph<MathLang, MathAnalysis>, egg::Id, &Subst) -> bool {
    move |egraph, _, subst| {
        let Ok(v) = var.parse::<Var>() else {
            return false;
        };
        egraph[subst[v]]
            .data
            .as_ref()
            .is_some_and(|c| c.is_positive())
    }
}

/// Condition guard: returns `true` iff `var` is known to be a non-negative integer constant (`>= 0`).
pub fn is_nonneg_int(
    var: &'static str,
) -> impl Fn(&mut EGraph<MathLang, MathAnalysis>, egg::Id, &Subst) -> bool {
    move |egraph, _, subst| {
        let Ok(v) = var.parse::<Var>() else {
            return false;
        };
        egraph[subst[v]]
            .data
            .as_ref()
            .is_some_and(|c| c.is_integer() && !c.is_negative())
    }
}

/// Condition guard: returns `true` iff `var` is symbolic (`None`) or a positive integer constant (`>= 1`).
pub fn is_sym_or_positive_int(
    var: &'static str,
) -> impl Fn(&mut EGraph<MathLang, MathAnalysis>, egg::Id, &Subst) -> bool {
    move |egraph, _, subst| {
        let Ok(v) = var.parse::<Var>() else {
            return false;
        };
        match &egraph[subst[v]].data {
            None => true,
            Some(c) => c.is_integer() && c.is_positive(),
        }
    }
}

/// Condition guard: returns `true` iff `var` is symbolic (`None`) or a non-negative integer constant (`>= 0`).
pub fn is_sym_or_nonneg_int(
    var: &'static str,
) -> impl Fn(&mut EGraph<MathLang, MathAnalysis>, egg::Id, &Subst) -> bool {
    move |egraph, _, subst| {
        let Ok(v) = var.parse::<Var>() else {
            return false;
        };
        match &egraph[subst[v]].data {
            None => true,
            Some(c) => c.is_integer() && !c.is_negative(),
        }
    }
}

/// Condition guard: returns `true` iff `var` is known to be a positive integer constant (`>= 1`).
pub fn is_positive_int(
    var: &'static str,
) -> impl Fn(&mut EGraph<MathLang, MathAnalysis>, egg::Id, &Subst) -> bool {
    move |egraph, _, subst| {
        let Ok(v) = var.parse::<Var>() else {
            return false;
        };
        egraph[subst[v]]
            .data
            .as_ref()
            .is_some_and(|c| c.is_integer() && c.is_positive())
    }
}

/// Configuration controlling which rule sets are enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleConfig {
    /// Enable fast-math (real-number ℝ) transformations that may alter IEEE-754 rounding.
    pub fast_math: bool,
    /// Structured Maxima-aligned simplification flags (ADR-0013).
    pub simp_flags: SimpFlags,
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
            simp_flags: SimpFlags::default(),
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
            simp_flags: SimpFlags {
                fast_math: false,
                ..SimpFlags::default()
            },
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

/// Check whether a rule is enabled under both `fast_math` and `SimpFlags` (ADR-0013).
fn rule_enabled(name: &str, fast_math_only: bool, config: &RuleConfig) -> bool {
    let effective_fast = config.fast_math && config.simp_flags.fast_math;
    if fast_math_only && !effective_fast {
        return false;
    }
    let flags = &config.simp_flags;
    match name {
        "pow-mul-exp" => flags.domain == DomainMode::Real && flags.radexpand == RadExpandMode::All,
        "ln-exp" => flags.domain == DomainMode::Real && flags.logexpand != LogExpandMode::False,
        "ln-mul" | "ln-div" | "ln-pow" => {
            flags.domain == DomainMode::Real
                && matches!(flags.logexpand, LogExpandMode::All | LogExpandMode::Super)
        }
        "asin-sin" | "acos-cos" | "atan-tan" => flags.triginverses == TrigInversesMode::All,
        _ => true,
    }
}

/// Collect all sourced rewrite rules with 1:1 Maxima citations.
pub fn get_sourced_rules(config: &RuleConfig) -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    let mut rules = Vec::new();

    // 1. Arithmetic rules (always included)
    for r in arithmetic::arithmetic_rules() {
        if rule_enabled(r.name, r.provenance.fast_math_only, config) {
            rules.push(r);
        }
    }

    // 2. Trigonometric rules
    if config.trigonometric {
        for r in trigonometric::trigonometric_rules() {
            if rule_enabled(r.name, r.provenance.fast_math_only, config) {
                rules.push(r);
            }
        }
    }

    // 3. Exponential rules
    if config.exponential {
        for r in exponential::exponential_rules() {
            if rule_enabled(r.name, r.provenance.fast_math_only, config) {
                rules.push(r);
            }
        }
    }

    // 4. Logarithmic rules
    if config.logarithmic {
        for r in logarithmic::logarithmic_rules() {
            if rule_enabled(r.name, r.provenance.fast_math_only, config) {
                rules.push(r);
            }
        }
    }

    // 5. Hyperbolic and inverse-trigonometric rules
    if config.hyperbolic {
        for r in hyperbolic::hyperbolic_rules() {
            if rule_enabled(r.name, r.provenance.fast_math_only, config) {
                rules.push(r);
            }
        }
    }

    // 6. Rational normalization rules
    if config.rational {
        for r in rational::rational_rules() {
            if rule_enabled(r.name, r.provenance.fast_math_only, config) {
                rules.push(r);
            }
        }
    }

    // 7. Factorial rules
    if config.factorial {
        for r in factorial::factorial_rules() {
            if rule_enabled(r.name, r.provenance.fast_math_only, config) {
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

    #[test]
    fn test_simp_flags_gate_branch_cut_rules() {
        let default_names: Vec<&str> = get_sourced_rules(&RuleConfig::default())
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert!(!default_names.contains(&"asin-sin"));
        assert!(!default_names.contains(&"pow-mul-exp"));
        assert!(!default_names.contains(&"ln-pow"));

        let mut opt_cfg = RuleConfig::default();
        opt_cfg.simp_flags.triginverses = TrigInversesMode::All;
        opt_cfg.simp_flags.radexpand = RadExpandMode::All;
        opt_cfg.simp_flags.logexpand = LogExpandMode::All;
        let all_names: Vec<&str> = get_sourced_rules(&opt_cfg)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert!(all_names.contains(&"asin-sin"));
        assert!(all_names.contains(&"pow-mul-exp"));
        assert!(all_names.contains(&"ln-pow"));
    }
}
