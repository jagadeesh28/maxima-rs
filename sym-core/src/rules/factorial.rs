//! Factorial simplification rules citing Maxima `sfacts.lisp`.
//!
//! Maxima implements factorial combination in `sfacts.lisp` (`factcomb`).
//! These rules mirror the base cases and the successor/quotient recurrences.
//!
//! Citations reference:
//! - `sfacts.lisp`: `factcomb`

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use crate::rules::{is_nonneg_int, is_sym_or_nonneg_int, is_sym_or_positive_int};
use egg::rewrite;

/// Generate factorial rewrite rules with 1:1 Maxima citations.
pub fn factorial_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // Maxima: sfacts.lisp:factcomb — 0! -> 1
        SourcedRule::new(
            "factorial-zero",
            RuleProvenance::new("sfacts.lisp", "factcomb", "0! -> 1", false),
            rewrite!("factorial-zero"; "(fac 0)" => "1"),
        ),
        // Maxima: sfacts.lisp:factcomb — 1! -> 1
        SourcedRule::new(
            "factorial-one",
            RuleProvenance::new("sfacts.lisp", "factcomb", "1! -> 1", false),
            rewrite!("factorial-one"; "(fac 1)" => "1"),
        ),
        // Maxima: sfacts.lisp:factcomb — (n+1)! -> (n+1) * n! (when n is non-negative integer)
        SourcedRule::new(
            "factorial-successor",
            RuleProvenance::new("sfacts.lisp", "factcomb", "(n+1)! -> (n+1) * n!", true),
            rewrite!("factorial-successor"; "(fac (+ ?n 1))" => "(* (+ ?n 1) (fac ?n))" if is_nonneg_int("?n")),
        ),
        // Maxima: sfacts.lisp:factcomb — n! / (n-1)! -> n (when n is symbolic or positive integer)
        SourcedRule::new(
            "factorial-quotient",
            RuleProvenance::new("sfacts.lisp", "factcomb", "n! / (n-1)! -> n", true),
            rewrite!("factorial-quotient"; "(/ (fac ?n) (fac (- ?n 1)))" => "?n" if is_sym_or_positive_int("?n")),
        ),
        // Maxima: sfacts.lisp:factcomb — (n+1)! / n! -> n+1 (when n is symbolic or non-negative integer)
        SourcedRule::new(
            "factorial-successor-quotient",
            RuleProvenance::new("sfacts.lisp", "factcomb", "(n+1)! / n! -> n+1", true),
            rewrite!("factorial-successor-quotient"; "(/ (fac (+ ?n 1)) (fac ?n))" => "(+ ?n 1)" if is_sym_or_nonneg_int("?n")),
        ),
    ]
}
