//! Logarithmic rewrite rules citing Maxima Common LISP routines (`logarc.lisp`).
//!
//! Citations reference:
//! - `logarc.lisp`: `sp1log`, `log-exp`, `log-quotient`

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::rewrite;

/// Generate all logarithmic rewrite rules with 1:1 Maxima citations.
pub fn logarithmic_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // ── Logarithmic Identities (logarc.lisp: sp1log) ─────────────────
        // Maxima: logarc.lisp:sp1log — ln(1) -> 0
        SourcedRule::new(
            "ln-one",
            RuleProvenance::new("logarc.lisp", "sp1log", "ln(1) -> 0", false),
            rewrite!("ln-one"; "(ln 1)" => "0"),
        ),
        // Maxima: logarc.lisp:sp1log — ln(exp(x)) -> x
        SourcedRule::new(
            "ln-exp",
            RuleProvenance::new("logarc.lisp", "sp1log", "ln(exp(x)) -> x", true),
            rewrite!("ln-exp"; "(ln (exp ?x))" => "?x"),
        ),
        // Maxima: logarc.lisp:sp1log — ln(a * b) -> (+ (ln a) (ln b))
        SourcedRule::new(
            "ln-mul",
            RuleProvenance::new(
                "logarc.lisp",
                "sp1log",
                "ln(a * b) -> (+ (ln a) (ln b))",
                true,
            ),
            rewrite!("ln-mul"; "(ln (* ?a ?b))" => "(+ (ln ?a) (ln ?b))"),
        ),
        // Maxima: logarc.lisp:sp1log — (+ (ln a) (ln b)) -> ln(a * b)
        SourcedRule::new(
            "ln-add-to-mul",
            RuleProvenance::new(
                "logarc.lisp",
                "sp1log",
                "(+ (ln a) (ln b)) -> ln(a * b)",
                true,
            ),
            rewrite!("ln-add-to-mul"; "(+ (ln ?a) (ln ?b))" => "(ln (* ?a ?b))"),
        ),
        // Maxima: logarc.lisp:sp1log — ln(a / b) -> (- (ln a) (ln b))
        SourcedRule::new(
            "ln-div",
            RuleProvenance::new(
                "logarc.lisp",
                "sp1log",
                "ln(a / b) -> (- (ln a) (ln b))",
                true,
            ),
            rewrite!("ln-div"; "(ln (/ ?a ?b))" => "(- (ln ?a) (ln ?b))"),
        ),
        // Maxima: logarc.lisp:sp1log — ln(^ a b) -> (* b (ln a))
        SourcedRule::new(
            "ln-pow",
            RuleProvenance::new("logarc.lisp", "sp1log", "ln(^ a b) -> (* b (ln a))", true),
            rewrite!("ln-pow"; "(ln (^ ?a ?b))" => "(* ?b (ln ?a))"),
        ),
    ]
}
