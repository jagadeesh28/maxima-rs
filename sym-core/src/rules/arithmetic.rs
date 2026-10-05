//! Arithmetic rewrite rules citing Maxima Common LISP routines.
//!
//! Citations reference:
//! - `simp.lisp`: `simpplus` (addition), `simptimes` (multiplication), `simpexpt` (exponentiation)
//! - `combin.lisp`: integer combinations and factorings

use crate::lang::{MathAnalysis, MathLang};
use crate::provenance::{RuleProvenance, SourcedRule};
use egg::rewrite;

/// Generate all arithmetic rewrite rules with 1:1 Maxima citations.
pub fn arithmetic_rules() -> Vec<SourcedRule<MathLang, MathAnalysis>> {
    vec![
        // ── Addition Identities (simp.lisp: simpplus) ───────────────────
        // Maxima: simp.lisp:simpplus — (+ x 0) -> x
        SourcedRule::new(
            "add-zero-right",
            RuleProvenance::new("simp.lisp", "simpplus", "(+ x 0) -> x", false),
            rewrite!("add-zero-right"; "(+ ?a 0)" => "?a"),
        ),
        // Maxima: simp.lisp:simpplus — (+ 0 x) -> x
        SourcedRule::new(
            "add-zero-left",
            RuleProvenance::new("simp.lisp", "simpplus", "(+ 0 x) -> x", false),
            rewrite!("add-zero-left"; "(+ 0 ?a)" => "?a"),
        ),
        // Maxima: simp.lisp:simpplus — (+ x x) -> (* 2 x)
        SourcedRule::new(
            "add-self",
            RuleProvenance::new("simp.lisp", "simpplus", "(+ x x) -> (* 2 x)", false),
            rewrite!("add-self"; "(+ ?a ?a)" => "(* 2 ?a)"),
        ),
        // Maxima: simp.lisp:simpplus — Commutativity (+ a b) = (+ b a)
        SourcedRule::new(
            "add-comm",
            RuleProvenance::new("simp.lisp", "simpplus", "(+ a b) -> (+ b a)", true),
            rewrite!("add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        ),
        // Maxima: simp.lisp:simpplus — Associativity (+ (+ a b) c) = (+ a (+ b c))
        SourcedRule::new(
            "add-assoc",
            RuleProvenance::new(
                "simp.lisp",
                "simpplus",
                "(+ (+ a b) c) -> (+ a (+ b c))",
                true,
            ),
            rewrite!("add-assoc"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        ),
        // ── Subtraction Identities (simp.lisp: simpplus / plusin) ───────
        // Maxima: simp.lisp:simpplus — (- x 0) -> x
        SourcedRule::new(
            "sub-zero",
            RuleProvenance::new("simp.lisp", "simpplus", "(- x 0) -> x", false),
            rewrite!("sub-zero"; "(- ?a 0)" => "?a"),
        ),
        // Maxima: simp.lisp:simpplus — (- x x) -> 0
        SourcedRule::new(
            "sub-self",
            RuleProvenance::new("simp.lisp", "simpplus", "(- x x) -> 0", false),
            rewrite!("sub-self"; "(- ?a ?a)" => "0"),
        ),
        // Maxima: simp.lisp:simpplus — (- 0 x) -> (neg x)
        SourcedRule::new(
            "sub-zero-left",
            RuleProvenance::new("simp.lisp", "simpplus", "(- 0 x) -> (neg x)", false),
            rewrite!("sub-zero-left"; "(- 0 ?a)" => "(neg ?a)"),
        ),
        // Maxima: simp.lisp:simpplus — (- a b) = (+ a (neg b))
        SourcedRule::new(
            "sub-to-add-neg",
            RuleProvenance::new("simp.lisp", "simpplus", "(- a b) -> (+ a (neg b))", false),
            rewrite!("sub-to-add-neg"; "(- ?a ?b)" => "(+ ?a (neg ?b))"),
        ),
        // ── Negation Identities (simp.lisp: simpplus) ───────────────────
        // Maxima: simp.lisp:simpplus — (neg (neg x)) -> x
        SourcedRule::new(
            "neg-neg",
            RuleProvenance::new("simp.lisp", "simpplus", "(neg (neg x)) -> x", false),
            rewrite!("neg-neg"; "(neg (neg ?a))" => "?a"),
        ),
        // Maxima: simp.lisp:simpplus — (neg 0) -> 0
        SourcedRule::new(
            "neg-zero",
            RuleProvenance::new("simp.lisp", "simpplus", "(neg 0) -> 0", false),
            rewrite!("neg-zero"; "(neg 0)" => "0"),
        ),
        // ── Multiplication Identities (simp.lisp: simptimes) ────────────
        // Maxima: simp.lisp:simptimes — (* x 1) -> x
        SourcedRule::new(
            "mul-one-right",
            RuleProvenance::new("simp.lisp", "simptimes", "(* x 1) -> x", false),
            rewrite!("mul-one-right"; "(* ?a 1)" => "?a"),
        ),
        // Maxima: simp.lisp:simptimes — (* 1 x) -> x
        SourcedRule::new(
            "mul-one-left",
            RuleProvenance::new("simp.lisp", "simptimes", "(* 1 x) -> x", false),
            rewrite!("mul-one-left"; "(* 1 ?a)" => "?a"),
        ),
        // Maxima: simp.lisp:simptimes — (* x 0) -> 0
        SourcedRule::new(
            "mul-zero-right",
            RuleProvenance::new("simp.lisp", "simptimes", "(* x 0) -> 0", false),
            rewrite!("mul-zero-right"; "(* ?a 0)" => "0"),
        ),
        // Maxima: simp.lisp:simptimes — (* 0 x) -> 0
        SourcedRule::new(
            "mul-zero-left",
            RuleProvenance::new("simp.lisp", "simptimes", "(* 0 x) -> 0", false),
            rewrite!("mul-zero-left"; "(* 0 ?a)" => "0"),
        ),
        // Maxima: simp.lisp:simptimes — (* x -1) -> (neg x)
        SourcedRule::new(
            "mul-neg-one",
            RuleProvenance::new("simp.lisp", "simptimes", "(* x -1) -> (neg x)", false),
            rewrite!("mul-neg-one"; "(* ?a -1)" => "(neg ?a)"),
        ),
        // Maxima: simp.lisp:simptimes — Commutativity (* a b) = (* b a)
        SourcedRule::new(
            "mul-comm",
            RuleProvenance::new("simp.lisp", "simptimes", "(* a b) -> (* b a)", true),
            rewrite!("mul-comm"; "(* ?a ?b)" => "(* ?b ?a)"),
        ),
        // Maxima: simp.lisp:simptimes — Associativity (* (* a b) c) = (* a (* b c))
        SourcedRule::new(
            "mul-assoc",
            RuleProvenance::new(
                "simp.lisp",
                "simptimes",
                "(* (* a b) c) -> (* a (* b c))",
                true,
            ),
            rewrite!("mul-assoc"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
        ),
        // Maxima: simp.lisp:simptimes — Distribution (* a (+ b c)) = (+ (* a b) (* a c))
        SourcedRule::new(
            "distribute",
            RuleProvenance::new(
                "simp.lisp",
                "simptimes",
                "(* a (+ b c)) -> (+ (* a b) (* a c))",
                true,
            ),
            rewrite!("distribute"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        ),
        // Maxima: combin.lisp:factoring — Factoring (+ (* a b) (* a c)) -> (* a (+ b c))
        SourcedRule::new(
            "factor",
            RuleProvenance::new(
                "combin.lisp",
                "factor",
                "(+ (* a b) (* a c)) -> (* a (+ b c))",
                true,
            ),
            rewrite!("factor"; "(+ (* ?a ?b) (* ?a ?c))" => "(* ?a (+ ?b ?c))"),
        ),
        // ── Division Identities (simp.lisp: simptimes) ──────────────────
        // Maxima: simp.lisp:simptimes — (/ x 1) -> x
        SourcedRule::new(
            "div-one",
            RuleProvenance::new("simp.lisp", "simptimes", "(/ x 1) -> x", false),
            rewrite!("div-one"; "(/ ?a 1)" => "?a"),
        ),
        // Maxima: simp.lisp:simptimes — (/ 0 x) -> 0
        SourcedRule::new(
            "div-zero",
            RuleProvenance::new("simp.lisp", "simptimes", "(/ 0 x) -> 0", true),
            rewrite!("div-zero"; "(/ 0 ?a)" => "0"),
        ),
        // Maxima: simp.lisp:simptimes — (/ x x) -> 1
        SourcedRule::new(
            "div-self",
            RuleProvenance::new("simp.lisp", "simptimes", "(/ x x) -> 1", true),
            rewrite!("div-self"; "(/ ?a ?a)" => "1"),
        ),
        // ── Exponentiation Identities (simp.lisp: simpexpt) ─────────────
        // Maxima: simp.lisp:simpexpt — (^ x 1) -> x
        SourcedRule::new(
            "pow-one",
            RuleProvenance::new("simp.lisp", "simpexpt", "(^ x 1) -> x", false),
            rewrite!("pow-one"; "(^ ?a 1)" => "?a"),
        ),
        // Maxima: simp.lisp:simpexpt — (^ x 0) -> 1
        SourcedRule::new(
            "pow-zero",
            RuleProvenance::new("simp.lisp", "simpexpt", "(^ x 0) -> 1", true),
            rewrite!("pow-zero"; "(^ ?a 0)" => "1"),
        ),
        // Maxima: simp.lisp:simpexpt — (^ 1 x) -> 1
        SourcedRule::new(
            "pow-one-base",
            RuleProvenance::new("simp.lisp", "simpexpt", "(^ 1 x) -> 1", false),
            rewrite!("pow-one-base"; "(^ 1 ?a)" => "1"),
        ),
        // Maxima: simp.lisp:simpexpt — (^ 0 x) -> 0
        SourcedRule::new(
            "pow-zero-base",
            RuleProvenance::new("simp.lisp", "simpexpt", "(^ 0 x) -> 0", true),
            rewrite!("pow-zero-base"; "(^ 0 ?a)" => "0"),
        ),
        // Maxima: simp.lisp:simpexpt — (* x x) -> (^ x 2)
        SourcedRule::new(
            "mul-self-pow",
            RuleProvenance::new("simp.lisp", "simpexpt", "(* x x) -> (^ x 2)", false),
            rewrite!("mul-self-pow"; "(* ?a ?a)" => "(^ ?a 2)"),
        ),
        // Maxima: simp.lisp:simpexpt — (* (^ x a) (^ x b)) -> (^ x (+ a b))
        SourcedRule::new(
            "pow-add-exp",
            RuleProvenance::new(
                "simp.lisp",
                "simpexpt",
                "(* (^ x a) (^ x b)) -> (^ x (+ a b))",
                true,
            ),
            rewrite!("pow-add-exp"; "(* (^ ?x ?a) (^ ?x ?b))" => "(^ ?x (+ ?a ?b))"),
        ),
        // Maxima: simp.lisp:simpexpt — (^ (^ x a) b) -> (^ x (* a b))
        SourcedRule::new(
            "pow-mul-exp",
            RuleProvenance::new(
                "simp.lisp",
                "simpexpt",
                "(^ (^ x a) b) -> (^ x (* a b))",
                true,
            ),
            rewrite!("pow-mul-exp"; "(^ (^ ?x ?a) ?b)" => "(^ ?x (* ?a ?b))"),
        ),
    ]
}
