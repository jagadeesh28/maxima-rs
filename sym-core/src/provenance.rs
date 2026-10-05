//! Metadata and registry for tracking 1:1 correspondence with Maxima Lisp sources.
//!
//! Every rewrite rule in `maxima-rs` maps to an algebraic simplification routine
//! in Maxima Common LISP (e.g. `simp.lisp`, `trigi.lisp`, `combin.lisp`, `logarc.lisp`).
//!
//! [`RuleProvenance`] provides structured traceability so that rules can be queried,
//! verified, and audited for coverage against the Maxima reference manual and source.

/// Structured provenance metadata tracing an algebraic rewrite rule to its Maxima source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleProvenance {
    /// Maxima source file (e.g., `"simp.lisp"`, `"trigi.lisp"`, `"logarc.lisp"`).
    pub source_file: &'static str,
    /// Lisp function or routine implementing the transformation (e.g., `"simpplus"`, `"simp-%sin"`).
    pub source_fn: &'static str,
    /// Mathematical description / identity of the rule (e.g., `"(+ x 0) -> x"`).
    pub identity: &'static str,
    /// Whether this rule relies on fast-math (real-number ℝ) semantics rather than strict IEEE-754.
    pub fast_math_only: bool,
}

impl RuleProvenance {
    /// Create a new provenance citation.
    pub const fn new(
        source_file: &'static str,
        source_fn: &'static str,
        identity: &'static str,
        fast_math_only: bool,
    ) -> Self {
        Self {
            source_file,
            source_fn,
            identity,
            fast_math_only,
        }
    }
}

/// A named rule bundled with its `egg::Rewrite` and its 1:1 Maxima citation.
pub struct SourcedRule<L: egg::Language, A: egg::Analysis<L>> {
    /// Unique identifier for the rule.
    pub name: &'static str,
    /// Provenance citation in Maxima Common Lisp.
    pub provenance: RuleProvenance,
    /// The executable egg rewrite rule.
    pub rewrite: egg::Rewrite<L, A>,
}

impl<L: egg::Language, A: egg::Analysis<L>> SourcedRule<L, A> {
    /// Construct a new sourced rewrite rule.
    pub fn new(
        name: &'static str,
        provenance: RuleProvenance,
        rewrite: egg::Rewrite<L, A>,
    ) -> Self {
        Self {
            name,
            provenance,
            rewrite,
        }
    }
}
