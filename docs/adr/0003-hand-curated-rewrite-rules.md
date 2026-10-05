# Hand-Curated Rewrite Rules Over Automatic Maxima Extraction

**Status**: Superseded by [ADR-0005](0005-systematic-maxima-rule-coverage.md)

Context: The project aims to bring Maxima's algebraic optimization capability into Rust, so a natural approach would be to auto-parse Maxima's Common Lisp simplification source (e.g., `simp.lisp`, `combin.lisp`, `trigi.lisp`) and generate `egg` rewrite rules programmatically. We chose hand-curation instead because e-graph equality saturation is extremely sensitive to rule count and rule shape — a single pathological bidirectional rule can cause exponential e-class explosion and blow up compile times. Hand-curation allows precise control over rule priorities, directionality, and iteration budgets. Maxima's source is used as a reference manual of algebraic identities to cherry-pick, not as a translation input.
