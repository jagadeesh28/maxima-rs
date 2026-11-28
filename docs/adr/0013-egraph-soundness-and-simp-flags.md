# E-Graph Soundness Guards and Maxima-Aligned Simplification Flags (`SimpFlags`)

**Status**: Accepted — refines [ADR-0002](0002-configurable-fast-math-default.md) and [ADR-0005](0005-systematic-maxima-rule-coverage.md)

**Context**:
A comprehensive repository audit (`docs/recommendations/NEON_audit_recommendations.md`) discovered that unconditional `egg` rewrite rules in `sym-core/src/rules/` cause catastrophic e-graph collapse and branch-cut unsoundness:
- `(/ 0 ?a) => 0` combined with `(/ ?a ?a) => 1` unifies `0` and `1` whenever `0 / 0` enters the e-graph, collapsing equivalence classes across unrelated constants.
- `(^ ?a 0) => 1` combined with `(^ 0 ?a) => 0` unifies `0` and `1` on `0^0`, and `(^ 0 ?a) => 0` silently evaluates `0^(-1)` to `0`.
- Unconditional inverse-trig, logarithm, and factorial rules (`asin(sin(x)) => x`, `ln(exp(x)) => x`, `ln(a^b) => b*ln(a)`, `(x^a)^b => x^(a*b)`, `(n+1)! => (n+1)*n!`) violate principal branch cuts and poles (e.g., `asin(sin(%pi)) -> %pi`, `((-1)^2)^(1/2) -> -1`, `0! -> 0*(-1)!`).

**Decision**:
1. **Mandatory Singularity & Domain Condition Guards**: Every e-graph rewrite rule with singular points or domain restrictions MUST attach an `egg::Condition` guard (`is_not_zero`, `is_positive`, `is_nonneg_int`) checking `MathAnalysis` constant data before applying. Unconditional singular rules (`0/a => 0`, `a/a => 1`, `a^0 => 1`, `0^a => 0`, `(n+1)! => (n+1)*n!`) are prohibited.
2. **Structured Maxima `SimpFlags` Configuration**: Replace monolithic `fast_math`-only rule gating with structured `SimpFlags` mirroring Maxima's `src/globals.lisp` option variables:
   - `domain`: `Real | Complex`
   - `radexpand`: `All | True | False`
   - `logexpand`: `True | All | Super | False`
   - `triginverses`: `True | All | False`
   - `fast_math: bool` (preserved as an ergonomic switch for IEEE-754 strict vs real-number algebraic rewrites per ADR-0002)
3. **Conservative Sound Defaults**: Default `SimpFlags` enforce safe principal branch-cut semantics (`triginverses: True`, conservative `radexpand`/`logexpand`) while permitting full real-algebraic FLOP optimization when explicitly opted in.

**Consequences**:
- Prevents `0 = 1` e-graph collapse and domain-unsound simplifications across all `sym-core`, `sym-macros`, and `maxima-web` entry points.
- Preserves 1:1 traceability with Maxima `src/globals.lisp` and `RuleProvenance` (ADR-0005).
- Requires `MathAnalysis::merge` to avoid unsound `merge_max` resolution on conflicting constants.
