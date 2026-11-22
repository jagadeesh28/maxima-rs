//! Rational-function normalization rules citing Maxima `nrat4.lisp`.
//!
//! Maxima implements `ratsimp`/`nrat` in `nrat4.lisp`; these rules mirror the
//! canonical quotient merges it performs (common denominators, quotient of
//! quotients, and sign normalization).
//!
//! Citations reference:
//! - `nrat4.lisp`: `$ratsimp`, `$nrat`

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::rewrite;

/// Generate rational-normalization rewrite rules with 1:1 Maxima citations.
pub fn rational_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // Maxima: nrat4.lisp:$ratsimp — (-a)/(-b) -> a/b
        SourcedRule::new(
            "div-neg-neg",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "(-a)/(-b) -> a/b", false),
            rewrite!("div-neg-neg"; "(/ (neg ?a) (neg ?b))" => "(/ ?a ?b)"),
        ),
        // Maxima: nrat4.lisp:$ratsimp — 1/(1/x) -> x
        SourcedRule::new(
            "reciprocal-reciprocal",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "1/(1/x) -> x", true),
            rewrite!("reciprocal-reciprocal"; "(/ 1 (/ 1 ?x))" => "?x"),
        ),
        // Maxima: nrat4.lisp:$ratsimp — a/(b/c) -> (a*c)/b
        SourcedRule::new(
            "div-by-fraction",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "a/(b/c) -> (a*c)/b", true),
            rewrite!("div-by-fraction"; "(/ ?a (/ ?b ?c))" => "(/ (* ?a ?c) ?b)"),
        ),
        // Maxima: nrat4.lisp:$ratsimp — (a/b) + (c/b) -> (a+c)/b
        SourcedRule::new(
            "add-same-denom",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "a/b + c/b -> (a+c)/b", true),
            rewrite!("add-same-denom"; "(+ (/ ?a ?b) (/ ?c ?b))" => "(/ (+ ?a ?c) ?b)"),
        ),
        // Maxima: nrat4.lisp:$ratsimp — (a/b) - (c/b) -> (a-c)/b
        SourcedRule::new(
            "sub-same-denom",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "a/b - c/b -> (a-c)/b", true),
            rewrite!("sub-same-denom"; "(- (/ ?a ?b) (/ ?c ?b))" => "(/ (- ?a ?c) ?b)"),
        ),
        // Maxima: nrat4.lisp:$nrat — (a/b) + (c/d) -> (a*d + c*b)/(b*d)
        SourcedRule::new(
            "add-different-denom",
            RuleProvenance::new(
                "nrat4.lisp",
                "$nrat",
                "a/b + c/d -> (a*d + c*b)/(b*d)",
                true,
            ),
            rewrite!("add-different-denom"; "(+ (/ ?a ?b) (/ ?c ?d))" => "(/ (+ (* ?a ?d) (* ?c ?b)) (* ?b ?d))"),
        ),
        // Maxima: nrat4.lisp:$ratsimp — a * (b/c) -> (a*b)/c
        SourcedRule::new(
            "mul-by-fraction",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "a*(b/c) -> (a*b)/c", false),
            rewrite!("mul-by-fraction"; "(* ?a (/ ?b ?c))" => "(/ (* ?a ?b) ?c)"),
        ),
        // Maxima: nrat4.lisp:$ratsimp — x^a / x^b -> x^(a-b)
        SourcedRule::new(
            "div-pow-quotient",
            RuleProvenance::new("nrat4.lisp", "$ratsimp", "x^a / x^b -> x^(a-b)", true),
            rewrite!("div-pow-quotient"; "(/ (^ ?x ?a) (^ ?x ?b))" => "(^ ?x (- ?a ?b))"),
        ),
    ]
}
