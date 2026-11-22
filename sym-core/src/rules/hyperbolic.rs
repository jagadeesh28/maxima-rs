//! Hyperbolic and inverse-trigonometric rewrite rules citing Maxima Common LISP routines.
//!
//! Citations reference:
//! - `sinh.lisp`: `simp-%sinh`, `simp-%cosh`, `simp-%tanh`
//! - `trigi.lisp`: `simp-%asin`, `simp-%acos`, `simp-%atan`

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::rewrite;

/// Generate hyperbolic and inverse-trigonometric rewrite rules with 1:1 Maxima citations.
pub fn hyperbolic_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // ── Hyperbolic zero values (sinh.lisp) ──────────────────────────
        // Maxima: sinh.lisp:simp-%sinh — sinh(0) -> 0
        SourcedRule::new(
            "sinh-zero",
            RuleProvenance::new("sinh.lisp", "simp-%sinh", "sinh(0) -> 0", false),
            rewrite!("sinh-zero"; "(sinh 0)" => "0"),
        ),
        // Maxima: sinh.lisp:simp-%cosh — cosh(0) -> 1
        SourcedRule::new(
            "cosh-zero",
            RuleProvenance::new("sinh.lisp", "simp-%cosh", "cosh(0) -> 1", false),
            rewrite!("cosh-zero"; "(cosh 0)" => "1"),
        ),
        // Maxima: sinh.lisp:simp-%tanh — tanh(0) -> 0
        SourcedRule::new(
            "tanh-zero",
            RuleProvenance::new("sinh.lisp", "simp-%tanh", "tanh(0) -> 0", false),
            rewrite!("tanh-zero"; "(tanh 0)" => "0"),
        ),
        // ── Hyperbolic parity (sinh.lisp) ───────────────────────────────
        // Maxima: sinh.lisp:simp-%sinh — sinh(-x) -> -sinh(x)
        SourcedRule::new(
            "sinh-neg",
            RuleProvenance::new("sinh.lisp", "simp-%sinh", "sinh(-x) -> -sinh(x)", false),
            rewrite!("sinh-neg"; "(sinh (neg ?x))" => "(neg (sinh ?x))"),
        ),
        // Maxima: sinh.lisp:simp-%cosh — cosh(-x) -> cosh(x)
        SourcedRule::new(
            "cosh-neg",
            RuleProvenance::new("sinh.lisp", "simp-%cosh", "cosh(-x) -> cosh(x)", false),
            rewrite!("cosh-neg"; "(cosh (neg ?x))" => "(cosh ?x)"),
        ),
        // Maxima: sinh.lisp:simp-%tanh — tanh(-x) -> -tanh(x)
        SourcedRule::new(
            "tanh-neg",
            RuleProvenance::new("sinh.lisp", "simp-%tanh", "tanh(-x) -> -tanh(x)", false),
            rewrite!("tanh-neg"; "(tanh (neg ?x))" => "(neg (tanh ?x))"),
        ),
        // ── Hyperbolic fundamental identity (sinh.lisp) ─────────────────
        // Maxima: sinh.lisp:simp-%cosh — cosh(x)^2 - sinh(x)^2 -> 1
        SourcedRule::new(
            "cosh-sq-minus-sinh-sq",
            RuleProvenance::new(
                "sinh.lisp",
                "simp-%cosh",
                "cosh(x)^2 - sinh(x)^2 -> 1",
                true,
            ),
            rewrite!("cosh-sq-minus-sinh-sq"; "(- (^ (cosh ?x) 2) (^ (sinh ?x) 2))" => "1"),
        ),
        // Maxima: sinh.lisp:simp-%sinh — sinh(x)^2 - cosh(x)^2 -> -1
        SourcedRule::new(
            "sinh-sq-minus-cosh-sq",
            RuleProvenance::new(
                "sinh.lisp",
                "simp-%sinh",
                "sinh(x)^2 - cosh(x)^2 -> -1",
                true,
            ),
            rewrite!("sinh-sq-minus-cosh-sq"; "(- (^ (sinh ?x) 2) (^ (cosh ?x) 2))" => "-1"),
        ),
        // ── Hyperbolic definitions (sinh.lisp: simp-%tanh) ──────────────
        // Maxima: sinh.lisp:simp-%tanh — tanh(x) -> sinh(x) / cosh(x)
        SourcedRule::new(
            "tanh-to-sinh-cosh",
            RuleProvenance::new(
                "sinh.lisp",
                "simp-%tanh",
                "tanh(x) -> sinh(x) / cosh(x)",
                true,
            ),
            rewrite!("tanh-to-sinh-cosh"; "(tanh ?x)" => "(/ (sinh ?x) (cosh ?x))"),
        ),
        // Maxima: sinh.lisp:simp-%tanh — sinh(x) / cosh(x) -> tanh(x)
        SourcedRule::new(
            "sinh-cosh-to-tanh",
            RuleProvenance::new(
                "sinh.lisp",
                "simp-%tanh",
                "sinh(x) / cosh(x) -> tanh(x)",
                true,
            ),
            rewrite!("sinh-cosh-to-tanh"; "(/ (sinh ?x) (cosh ?x))" => "(tanh ?x)"),
        ),
        // ── Inverse-trig special values (trigi.lisp) ────────────────────
        // Maxima: trigi.lisp:simp-%asin — asin(0) -> 0
        SourcedRule::new(
            "asin-zero",
            RuleProvenance::new("trigi.lisp", "simp-%asin", "asin(0) -> 0", false),
            rewrite!("asin-zero"; "(asin 0)" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%acos — acos(1) -> 0
        SourcedRule::new(
            "acos-one",
            RuleProvenance::new("trigi.lisp", "simp-%acos", "acos(1) -> 0", false),
            rewrite!("acos-one"; "(acos 1)" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%atan — atan(0) -> 0
        SourcedRule::new(
            "atan-zero",
            RuleProvenance::new("trigi.lisp", "simp-%atan", "atan(0) -> 0", false),
            rewrite!("atan-zero"; "(atan 0)" => "0"),
        ),
        // ── Inverse-trig cancellations (trigi.lisp) ─────────────────────
        // Maxima: trigi.lisp:simp-%asin — asin(sin(x)) -> x (principal branch)
        SourcedRule::new(
            "asin-sin",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%asin",
                "asin(sin(x)) -> x (principal branch)",
                true,
            ),
            rewrite!("asin-sin"; "(asin (sin ?x))" => "?x"),
        ),
        // Maxima: trigi.lisp:simp-%asin — sin(asin(x)) -> x
        SourcedRule::new(
            "sin-asin",
            RuleProvenance::new("trigi.lisp", "simp-%asin", "sin(asin(x)) -> x", true),
            rewrite!("sin-asin"; "(sin (asin ?x))" => "?x"),
        ),
        // Maxima: trigi.lisp:simp-%acos — acos(cos(x)) -> x (principal branch)
        SourcedRule::new(
            "acos-cos",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%acos",
                "acos(cos(x)) -> x (principal branch)",
                true,
            ),
            rewrite!("acos-cos"; "(acos (cos ?x))" => "?x"),
        ),
        // Maxima: trigi.lisp:simp-%acos — cos(acos(x)) -> x
        SourcedRule::new(
            "cos-acos",
            RuleProvenance::new("trigi.lisp", "simp-%acos", "cos(acos(x)) -> x", true),
            rewrite!("cos-acos"; "(cos (acos ?x))" => "?x"),
        ),
        // Maxima: trigi.lisp:simp-%atan — atan(tan(x)) -> x (principal branch)
        SourcedRule::new(
            "atan-tan",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%atan",
                "atan(tan(x)) -> x (principal branch)",
                true,
            ),
            rewrite!("atan-tan"; "(atan (tan ?x))" => "?x"),
        ),
        // Maxima: trigi.lisp:simp-%atan — tan(atan(x)) -> x
        SourcedRule::new(
            "tan-atan",
            RuleProvenance::new("trigi.lisp", "simp-%atan", "tan(atan(x)) -> x", true),
            rewrite!("tan-atan"; "(tan (atan ?x))" => "?x"),
        ),
    ]
}
