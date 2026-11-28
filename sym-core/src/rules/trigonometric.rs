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
        // Maxima: trigi.lisp:simp-%sin — sin(%pi) -> 0
        SourcedRule::new(
            "sin-pi",
            RuleProvenance::new("trigi.lisp", "simp-%sin", "sin(%pi) -> 0", false),
            rewrite!("sin-pi"; "(sin pi)" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(%pi) -> -1
        SourcedRule::new(
            "cos-pi",
            RuleProvenance::new("trigi.lisp", "simp-%cos", "cos(%pi) -> -1", false),
            rewrite!("cos-pi"; "(cos pi)" => "-1"),
        ),
        // Maxima: trigi.lisp:simp-%tan — tan(%pi) -> 0
        SourcedRule::new(
            "tan-pi",
            RuleProvenance::new("trigi.lisp", "simp-%tan", "tan(%pi) -> 0", false),
            rewrite!("tan-pi"; "(tan pi)" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%sin — sin(%pi/2) -> 1
        SourcedRule::new(
            "sin-pi-half",
            RuleProvenance::new("trigi.lisp", "simp-%sin", "sin(%pi/2) -> 1", false),
            rewrite!("sin-pi-half"; "(sin (/ pi 2))" => "1"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(%pi/2) -> 0
        SourcedRule::new(
            "cos-pi-half",
            RuleProvenance::new("trigi.lisp", "simp-%cos", "cos(%pi/2) -> 0", false),
            rewrite!("cos-pi-half"; "(cos (/ pi 2))" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%sin — sin(2*%pi) -> 0
        SourcedRule::new(
            "sin-two-pi",
            RuleProvenance::new("trigi.lisp", "simp-%sin", "sin(2*%pi) -> 0", false),
            rewrite!("sin-two-pi"; "(sin (* 2 pi))" => "0"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(2*%pi) -> 1
        SourcedRule::new(
            "cos-two-pi",
            RuleProvenance::new("trigi.lisp", "simp-%cos", "cos(2*%pi) -> 1", false),
            rewrite!("cos-two-pi"; "(cos (* 2 pi))" => "1"),
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
        // ── Double-angle identities (trigi.lisp) ────────────────────────
        // Maxima: trigi.lisp:simp-%sin — sin(2x) -> 2 sin(x) cos(x)
        SourcedRule::new(
            "sin-double-angle",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%sin",
                "sin(2x) -> 2*sin(x)*cos(x)",
                true,
            ),
            rewrite!("sin-double-angle"; "(sin (* 2 ?x))" => "(* 2 (* (sin ?x) (cos ?x)))"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(2x) -> cos(x)^2 - sin(x)^2
        SourcedRule::new(
            "cos-double-angle",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%cos",
                "cos(2x) -> cos(x)^2 - sin(x)^2",
                true,
            ),
            rewrite!("cos-double-angle"; "(cos (* 2 ?x))" => "(- (^ (cos ?x) 2) (^ (sin ?x) 2))"),
        ),
        // ── Angle-sum identities (trigi.lisp) ───────────────────────────
        // Maxima: trigi.lisp:simp-%sin — sin(a+b) -> sin a cos b + cos a sin b
        SourcedRule::new(
            "sin-angle-sum",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%sin",
                "sin(a+b) -> sin(a)cos(b) + cos(a)sin(b)",
                true,
            ),
            rewrite!("sin-angle-sum"; "(sin (+ ?a ?b))" => "(+ (* (sin ?a) (cos ?b)) (* (cos ?a) (sin ?b)))"),
        ),
        // Maxima: trigi.lisp:simp-%cos — cos(a+b) -> cos a cos b - sin a sin b
        SourcedRule::new(
            "cos-angle-sum",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%cos",
                "cos(a+b) -> cos(a)cos(b) - sin(a)sin(b)",
                true,
            ),
            rewrite!("cos-angle-sum"; "(cos (+ ?a ?b))" => "(- (* (cos ?a) (cos ?b)) (* (sin ?a) (sin ?b)))"),
        ),
        // Maxima: trigi.lisp:simp-%tan — tan(a+b) -> (tan a + tan b) / (1 - tan a tan b)
        SourcedRule::new(
            "tan-angle-sum",
            RuleProvenance::new(
                "trigi.lisp",
                "simp-%tan",
                "tan(a+b) -> (tan(a)+tan(b))/(1-tan(a)tan(b))",
                true,
            ),
            rewrite!("tan-angle-sum"; "(tan (+ ?a ?b))" => "(/ (+ (tan ?a) (tan ?b)) (- 1 (* (tan ?a) (tan ?b))))"),
        ),
        // ── Pythagorean-derived identities (trigi.lisp) ─────────────────
        // Maxima: trigi.lisp:trigsimp — 1 + tan(x)^2 -> 1 / cos(x)^2
        SourcedRule::new(
            "one-plus-tan-sq",
            RuleProvenance::new(
                "trigi.lisp",
                "trigsimp",
                "1 + tan(x)^2 -> 1 / cos(x)^2",
                true,
            ),
            rewrite!("one-plus-tan-sq"; "(+ 1 (^ (tan ?x) 2))" => "(/ 1 (^ (cos ?x) 2))"),
        ),
        // Maxima: trigi.lisp:trigsimp — 1 - sin(x)^2 -> cos(x)^2
        SourcedRule::new(
            "one-minus-sin-sq",
            RuleProvenance::new("trigi.lisp", "trigsimp", "1 - sin(x)^2 -> cos(x)^2", true),
            rewrite!("one-minus-sin-sq"; "(- 1 (^ (sin ?x) 2))" => "(^ (cos ?x) 2)"),
        ),
        // Maxima: trigi.lisp:trigsimp — 1 - cos(x)^2 -> sin(x)^2
        SourcedRule::new(
            "one-minus-cos-sq",
            RuleProvenance::new("trigi.lisp", "trigsimp", "1 - cos(x)^2 -> sin(x)^2", true),
            rewrite!("one-minus-cos-sq"; "(- 1 (^ (cos ?x) 2))" => "(^ (sin ?x) 2)"),
        ),
    ]
}
