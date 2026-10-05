//! Trigonometric rewrite rules citing Maxima Common LISP routines (`trigi.lisp`).
//!
//! Citations reference:
//! - `trigi.lisp`: `simp-%sin`, `simp-%cos`, `simp-%tan`

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::rewrite;

/// Generate all trigonometric rewrite rules with 1:1 Maxima citations.
pub fn trigonometric_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // ── Special Angle Evaluations (trigi.lisp: simp-%sin, simp-%cos, simp-%tan) ──
        // Maxima: trigi.lisp:simp-%sin — sin(0) -> 0
        SourcedRule::new(
            "sin-zero",
            RuleProvenance::new("trigi.lisp", "simp-%sin", "sin(0) -> 0", false),
            rewrite!("sin-zero"; "(sin 0)" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(0) -> 1
        SourcedRule::new(
            "cos-zero",
            RuleProvenance::new("trigi.lisp", "simp-%cos", "cos(0) -> 1", false),
            rewrite!("cos-zero"; "(cos 0)" => "1"),
        ),
        // Maxima: trigi.lisp:simp-%tan — tan(0) -> 0
        SourcedRule::new(
            "tan-zero",
            RuleProvenance::new("trigi.lisp", "simp-%tan", "tan(0) -> 0", false),
            rewrite!("tan-zero"; "(tan 0)" => "0"),
        ),
        // ── Parity / Symmetry (trigi.lisp) ──────────────────────────────
        // Maxima: trigi.lisp:simp-%sin — sin(-x) -> -sin(x)
        SourcedRule::new(
            "sin-neg",
            RuleProvenance::new("trigi.lisp", "simp-%sin", "sin(-x) -> -sin(x)", false),
            rewrite!("sin-neg"; "(sin (neg ?x))" => "(neg (sin ?x))"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(-x) -> cos(x)
        SourcedRule::new(
            "cos-neg",
            RuleProvenance::new("trigi.lisp", "simp-%cos", "cos(-x) -> cos(x)", false),
            rewrite!("cos-neg"; "(cos (neg ?x))" => "(cos ?x)"),
        ),
        // Maxima: trigi.lisp:simp-%tan — tan(-x) -> -tan(x)
        SourcedRule::new(
            "tan-neg",
            RuleProvenance::new("trigi.lisp", "simp-%tan", "tan(-x) -> -tan(x)", false),
            rewrite!("tan-neg"; "(tan (neg ?x))" => "(neg (tan ?x))"),
        ),
        // ── Fundamental Identities (trigi.lisp) ─────────────────────────
        // Maxima: trigi.lisp:pythagorean — sin(x)^2 + cos(x)^2 -> 1
        SourcedRule::new(
            "pythagorean-sin-cos",
            RuleProvenance::new("trigi.lisp", "trigsimp", "sin(x)^2 + cos(x)^2 -> 1", true),
            rewrite!("pythagorean-sin-cos"; "(+ (^ (sin ?x) 2) (^ (cos ?x) 2))" => "1"),
        ),
        // Maxima: trigi.lisp:pythagorean — cos(x)^2 + sin(x)^2 -> 1
        SourcedRule::new(
            "pythagorean-cos-sin",
            RuleProvenance::new("trigi.lisp", "trigsimp", "cos(x)^2 + sin(x)^2 -> 1", true),
            rewrite!("pythagorean-cos-sin"; "(+ (^ (cos ?x) 2) (^ (sin ?x) 2))" => "1"),
        ),
        // Maxima: trigi.lisp:simp-%tan — tan(x) -> sin(x) / cos(x)
        SourcedRule::new(
            "tan-to-sin-cos",
            RuleProvenance::new("trigi.lisp", "simp-%tan", "tan(x) -> sin(x) / cos(x)", true),
            rewrite!("tan-to-sin-cos"; "(tan ?x)" => "(/ (sin ?x) (cos ?x))"),
        ),
        // Maxima: trigi.lisp:simp-%tan — sin(x) / cos(x) -> tan(x)
        SourcedRule::new(
            "sin-cos-to-tan",
            RuleProvenance::new("trigi.lisp", "simp-%tan", "sin(x) / cos(x) -> tan(x)", true),
            rewrite!("sin-cos-to-tan"; "(/ (sin ?x) (cos ?x))" => "(tan ?x)"),
        ),
    ]
}
