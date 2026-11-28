//! Exponential and power rewrite rules citing Maxima Common LISP routines (`logarc.lisp` / `simp.lisp`).
//!
//! Citations reference:
//! - `simp.lisp`: `simpexpt`
//! - `logarc.lisp`: `sp1log`, `log-exp`

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::rewrite;

/// Generate all exponential and power rewrite rules with 1:1 Maxima citations.
pub fn exponential_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // ── Exponential Identities (simp.lisp: simpexpt, logarc.lisp) ──
        // Maxima: simp.lisp:simpexpt — exp(0) -> 1
        SourcedRule::new(
            "exp-zero",
            RuleProvenance::new("simp.lisp", "simpexpt", "exp(0) -> 1", false),
            rewrite!("exp-zero"; "(exp 0)" => "1"),
        ),
        // Maxima: logarc.lisp:sp1log — exp(ln(x)) -> x
        SourcedRule::new(
            "exp-ln",
            RuleProvenance::new("logarc.lisp", "sp1log", "exp(ln(x)) -> x", true),
            rewrite!("exp-ln"; "(exp (ln ?x))" => "?x"),
        ),
        // Maxima: simp.lisp:simpexpt — exp(a + b) -> (* (exp a) (exp b))
        SourcedRule::new(
            "exp-add",
            RuleProvenance::new(
                "simp.lisp",
                "simpexpt",
                "exp(a + b) -> (* (exp a) (exp b))",
                true,
            ),
            rewrite!("exp-add"; "(exp (+ ?a ?b))" => "(* (exp ?a) (exp ?b))"),
        ),
        // Maxima: simp.lisp:simpexpt — (* (exp a) (exp b)) -> exp(a + b)
        SourcedRule::new(
            "exp-mul-to-add",
            RuleProvenance::new(
                "simp.lisp",
                "simpexpt",
                "(* (exp a) (exp b)) -> exp(a + b)",
                true,
            ),
            rewrite!("exp-mul-to-add"; "(* (exp ?a) (exp ?b))" => "(exp (+ ?a ?b))"),
        ),
        // Maxima: simp.lisp:simpexpt — exp(a - b) -> (/ (exp a) (exp b))
        SourcedRule::new(
            "exp-sub",
            RuleProvenance::new(
                "simp.lisp",
                "simpexpt",
                "exp(a - b) -> (/ (exp a) (exp b))",
                true,
            ),
            rewrite!("exp-sub"; "(exp (- ?a ?b))" => "(/ (exp ?a) (exp ?b))"),
        ),
        // Maxima: simp.lisp:simpexpt — (^ (sqrt x) 2) -> x
        SourcedRule::new(
            "sqrt-square",
            RuleProvenance::new("simp.lisp", "simpexpt", "(^ (sqrt x) 2) -> x", true),
            rewrite!("sqrt-square"; "(^ (sqrt ?x) 2)" => "?x"),
        ),
        // Maxima: simp.lisp:simpexpt — %e^x -> exp(x)
        SourcedRule::new(
            "e-pow-to-exp",
            RuleProvenance::new("simp.lisp", "simpexpt", "(^ e x) -> (exp x)", false),
            rewrite!("e-pow-to-exp"; "(^ e ?x)" => "(exp ?x)"),
        ),
        // Maxima: simp.lisp:simpexpt — exp(x) -> %e^x
        SourcedRule::new(
            "exp-to-e-pow",
            RuleProvenance::new("simp.lisp", "simpexpt", "(exp x) -> (^ e x)", false),
            rewrite!("exp-to-e-pow"; "(exp ?x)" => "(^ e ?x)"),
        ),
    ]
}
