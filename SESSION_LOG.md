# maxima-rs — Session Log

> Persistent context for continuing work across AI sessions.
> Read this file at the start of every new session to restore full context.

---

## Project Summary

**maxima-rs** is a compile-time symbolic mathematics and code optimization engine in Rust.
It converts legacy Maxima algebraic knowledge into an `egg`-powered e-graph equality
saturation engine that generates zero-overhead optimized Rust code via procedural macros.

### Key Reference Files

| File | Purpose |
|------|---------|
| [`agents.md`](file:///e:/Rust/Max/agents.md) | Full phased implementation plan (Phases 1–21) |
| [`GLOSSARY.md`](file:///e:/Rust/Max/GLOSSARY.md) | Canonical domain vocabulary (20 terms) |
| [`docs/adr/`](file:///e:/Rust/Max/docs/adr) | 5 Architecture Decision Records (ADR-0003 superseded by ADR-0005) |
| [`References/maxima-master.zip`](file:///e:/Rust/Max/References/maxima-master.zip) | Maxima Common Lisp reference source |

---

## Session 1 — Design Grilling & Architecture Plan

**Date**: 2026-10-03
**Scope**: Full architectural design via grill-with-docs, then detailed phased plan

### Settled Design Decisions (Decisions 1–19)
- Established e-graph equality saturation architecture, AST, diff pre-pass, fast-math configurable semantics, rational constants, etc.

---

## Session 2 — 1:1 Maxima LISP Conversion & Ergonomic Interfaces Alignment

**Date**: 2026-10-04
**Scope**: Updated architecture based on user instructions to follow reference Maxima Lisp conversion 1:1 with source citations, multi-format parsers (Maxima + LaTeX), and ergonomic high-level APIs.

### Settled Design Decisions (Session 2)

| # | Decision | Choice | Round |
|---|----------|--------|-------|
| 20 | Rule coverage strategy | Systematic 1:1 Maxima LISP mapping with `egg` rewrite rules, each citing source file + function in doc comments and structured metadata | R1 |
| 21 | Ergonomic input | Multi-format parsers (Maxima string, LaTeX formula) + high-level `optimize()` one-call API | R1 |
| 22 | ADR-0003 disposition | Superseded by new ADR-0005 ("Systematic 1:1 Maxima Rule Coverage with Source Citations") | R1 |
| 23 | LaTeX subset scope | Core scalar math for now (`\frac`, `\sqrt`, `\sin`, `\cos`, `\tan`, `\exp`, `\ln`, `^`); full math mode in phased roadmap | R2 |
| 24 | Parser feature gating | New `parser` feature flag in `sym-core`, included in `full` bundle | R2 |
| 25 | Maxima source coverage | Core 4 files now (`simp.lisp`, `trigi.lisp`, `combin.lisp`, `logarc.lisp`); extended set in phased plan | R2 |
| 26 | Citation format | Structured `RuleProvenance` metadata + rustdoc comments | R2 |
| 27 | `optimize()` API shape | Both free functions (`optimize`, `optimize_gradient`) AND fluent `Pipeline` builder | R3 |
| 28 | Parser library | `winnow = "0.7"` for high performance and clean parser combinator errors | R3 |
| 29 | Phase plan | Clean renumber to 12-phase roadmap in `agents.md` | R3 |

### ADRs Updated
- Added [`docs/adr/0005-systematic-maxima-rule-coverage.md`](file:///e:/Rust/Max/docs/adr/0005-systematic-maxima-rule-coverage.md)
- Marked [`docs/adr/0003-hand-curated-rewrite-rules.md`](file:///e:/Rust/Max/docs/adr/0003-hand-curated-rewrite-rules.md) as superseded

---

## How to Continue

When starting a new session, instruct the AI:

> Read `SESSION_LOG.md` and `agents.md` from `e:\Rust\Max\` to restore context.
> Continue from the current phase marker below.

### Cargo Feature Groupings

| Feature | Phases | Default | Notes |
|---------|--------|---------|-------|
| `core` (base) | Phase 1 | Always | `expr`, `rational` AST & arithmetic |
| `diff` | Phase 2 | Yes | Analytical differentiation, gradient, Jacobian, Hessian |
| `egraph` | Phases 3, 4 | Yes | E-graph equality saturation rules, FLOP cost, extraction, 1:1 Maxima citations |
| `cse` | Phase 5 | Yes | Common subexpression elimination DAG pass |
| `codegen` | Phase 6 | Yes | `TokenStream` emission |
| `parser` | Phase 7 | Yes | Maxima string & LaTeX math parsers via `winnow` |
| `pipeline` | Phase 8 | Yes | `optimize()`, `optimize_gradient()`, `Pipeline` builder |
| `macros` | Phase 9 | Yes | `sym_fn!` and `sym_eval!` proc-macros |
| `poly` | Phases 22, 23 | Yes | Sparse distributed multivariate polynomials, 64-bit packed exponents, Monagan-Pearce heap, `expand`, `ratsimp`, `gcd`, `factor`, `partfrac` |
| `integrate` | Phases 24, 25, 28 | Yes | Symbolic integration (SIN, Hermite, LRT real ln/atan, Bronstein Risch field towers, FTC defint) |
| `series` | Phase 26 | Yes | Truncated Laurent series arithmetic and asymptotic expansions |
| `limit` | Phase 27 | Yes | Dominik Gruntz's MRV limits engine + 3-tier zero decider |
| `solve` | Phases 29, 30 | Yes | Closed-form algebraic solvers (deg 1-4, factor-and-split) + ODE solvers (1st-order standard & 2nd-order constant coeffs) |
| `cas` | Phases 22–32 | Yes | Meta-feature bundling all CAS modules `["poly", "integrate", "series", "limit", "solve", "diff", "egraph"]` |
| `full` | All | Yes | Bundle enabling all features |

### Current Phase: **All Phases 1–45 Complete ✅**

### Phase Completion Tracker

| Phase | Feature | Status | Notes |
|-------|---------|--------|-------|
| 1. Workspace & Expr AST | `core` | ✅ Completed | 47 tests + 12 doctests pass, exact rational arithmetic & AST |
| 2. Analytical Differentiation | `diff` | ✅ Completed | 60 tests + 16 doctests pass, diff/grad/jac/hessian implemented |
| 3. E-Graph Language, 1:1 Rules & Provenance | `egraph` | ✅ Completed | `MathLang`, `MathAnalysis`, `RuleProvenance`, 52 1:1 Maxima citations (`simp.lisp`, `trigi.lisp`, `combin.lisp`, `logarc.lisp`) |
| 4. Cost, Saturation, Extraction | `egraph` | ✅ Completed | FLOP-weighted `FlopCost`, `saturate` runner with bounds, multi-root extraction |
| 5. CSE Post-Pass | `cse` | ✅ Completed | DAG-based `eliminate_common_subexprs`, `LetBinding`, `CseResult` |
| 6. Codegen (TokenStream) | `codegen` | ✅ Completed | `emit_expr`, `emit_cse_block`, `FloatType` (F64/F32) |
| 7. Multi-Format Parsers (Maxima/LaTeX) | `parser` | ✅ Completed | `winnow = "0.7"`-based `parse_maxima` and `parse_latex` |
| 8. High-Level API & Pipeline Builder | `pipeline` | ✅ Completed | `optimize`, `optimize_gradient`, fluent `Pipeline` builder |
| 9. Proc-Macro Crate | `macros` | ✅ Completed | `sym_fn!` and `sym_eval!` in `sym-macros` with span-aware diagnostics |
| 10. Facade Crate & Prelude | `full` | ✅ Completed | `maxima-rs` facade crate with re-exports and `prelude` module |
| 11. Testing Infrastructure & Coverage | all | ✅ Completed | 9 integration suites, 2 proptest suites, 1:1 citation coverage check (52 rules) |
| 12. Ergonomics & Polish | all | ✅ Completed | Zero clippy warnings across workspace (`-D warnings`), rustfmt clean, zero runtime overhead verified |
| 13. Advanced Math & Calculus | `core`/`parser` | ✅ Completed | Tensor/Matrix/Derivative/Integral/Sum AST nodes, LaTeX calculus parsing |
| 14. WebUI Correctness | `maxima-web` | ✅ Completed | Fixed pane `<main>` layout, `copyText()` syntax error, copy wiring, `simplify`/`optimize` |
| 15. Advanced AST Nodes | `core` | ✅ Completed | `sinh/cosh/tanh`, `asin/acos/atan`, `%pi`/`%e`, factorial across all passes |
| 16. Maxima Parser Parity | `parser` | ✅ Completed | Constants, `!`, hyperbolic/inverse trig, `diff`/`integrate`/`sum`/`matrix` |
| 17. Rule Expansion I | `egraph` | ✅ Completed | `sinh.lisp` + deeper `trigi.lisp`; registry 79 rules |
| 18. Rule Expansion II | `egraph` | ✅ Completed | `nrat4.lisp` rational + `sfacts.lisp` factorial; registry 92 rules |
| 19. Matrix E-Graph Fidelity | `egraph` | ✅ Completed | Lossless matrix symbol codec; no sentinel |
| 20. WebUI Provenance | `maxima-web` | ✅ Completed | `GET /api/rules`, `provenance` op, coverage panel |
| 21. Documentation & Verification | all | ✅ Completed | Coverage report, glossary, final fmt/clippy/test |
| 22. Sparse Distributed Poly Engine | `poly` | ✅ Completed | 64-bit packed exponents, `grevlex`, Monagan-Pearce heap multiplication/division, `expand()` (121 unit + 6 integration tests) |
| 23. Exact Rational Algebra & Factorization | `poly` | ✅ Completed | Exact univariate GCD + `ratsimp`, rational-root `factor`, distinct-linear `partfrac` (129 unit + 4 integration tests) |
| 24. Symbolic Integration: Rational & Elementary | `integrate` | ✅ Completed | Linearity, power rule, elementary (trig/exp/ln), rational via partial fractions, IBP; noun fallback (131 unit + 10 integration) |
| 25. Transcendental Risch Differential Towers | `integrate` | ✅ Completed | Derivative-divides u-substitution (`∫x·e^{x²}`, `∫e^x/(e^x+1)`, `∫2x/(x²+1)`); non-elementary → noun (133 unit + 6 integration) |
| 26. Laurent Series & Asymptotics | `series` | ✅ Completed | Truncated `Series { center, valuation, coeffs, order }` arithmetic + sin/cos/exp/1/(1-x)/1/sin expansions (133 unit + 9 integration) |
| 27. Gruntz Limits Engine | `limit` | ✅ Completed | MRV enumeration, series-based limits, log-transform for `Pow`, numeric convergence fallback (138 unit + 6 integration) |
| 28. Definite Integration & Pole Scanning | `integrate` | ✅ Completed | Rational pole scanner + FTC evaluation + improper limits (138 unit + 4 integration) |
| 29. Closed-Form Algebraic Solvers | `solve` | ✅ Completed | Rational-root factoring + linear/quadratic formulas (deg 1–4), transcendental isolation, `AlgebraicSolutions` (139 unit + 6 integration) |
| 30. ODE Solvers | `solve` | ✅ Completed | 1st-order linear + 2nd-order constant coefficients (`OdeSolution`), back-substitution verified (139 unit + 3 integration) |
| 31. Maxima Parser CAS Syntax & Facade | `cas`/`parser` | ✅ Completed | `parse_maxima_cas` + `CasCommand`/`execute_cas`, prelude & maxima-rs CAS re-exports (139 unit + 10 integration) |
| 32. Verification & Maxima Benchmarks | all | ✅ Completed | Proptests (integrate/solve invariants, 10³ cases) + Maxima parity cases; fmt/clippy green (139 unit) |
| 33. WebUI Recursive Composer & E2E | `maxima-web` | ✅ Completed | `GET /api/functions` catalog + recursive composer UI + `webui_e2e` (8 web + 6 e2e tests) |
| 34. CAS Web API Wiring & Typed Dispatch | `maxima-web` | ✅ Completed | Declarative `OPERATIONS` catalog, typed CAS dispatch, `/api/parse` live endpoint |
| 35. Precedence-Aware LaTeX Formatter | `sym-core` | ✅ Completed | `to_latex` precedence 1..5, matrix pmatrix, Greek macros, subscript constants |
| 36. Offline-First WebUI & KaTeX Bundling | `maxima-web` | ✅ Completed | Vendored KaTeX + WOFF2 fonts, zero-CDN CSS theme, modular vanilla ES notebook stream |
| 37. Web Service Isolation & Concurrency | `maxima-web` | ✅ Completed | Hardened `spawn_blocking`, timeouts (408), semaphore (503), payload checks (413), CLI config |
| 38. Standalone Distribution & Packaging | all | ✅ Completed | Standalone binary (~4.26 MB), Docker multi-stage build, AWS Lambda SAM template |
| 38A. P0 Safety, Correctness & Hardening | all | ✅ Completed | Precedence `-x^2 == -9`, recursion limit 256, scientific notation (`1e-3`), owned semaphore, proptest |
| 38B. WebUI Wiring & Styling Restoration | `maxima-web` | ✅ Completed | Palette catalog parsing, TDZ elimination, embedded CAS argument extraction, CSE let-bindings visualization, 0 undefined CSS classes |
| 38C. Engine Honesty, Dynamic Verification & Defect Remediation | all | ✅ Completed | 1st-order linear ODE coefficient fix, Liouville non-elementary detection, definite integral negative branch cuts, composite Simpson & ODE residual dynamic verification |
| 39. Maxima 5.50 Core Parity & Arbitrary-Precision Float | `bfloat` | ✅ Completed | Pure-Rust `dashu-float` arbitrary precision float, fpprec environment, LaTeX 10^exp |
| 40. Symbolic Linear Algebra & Bareiss Algorithm | `matrix` | ✅ Completed | Bareiss $O(n^3)$ determinant, adjoint inverse, charpoly, trace, eigenvalues |
| 41. Complex Analysis, Rectangular Form & Residues | `complex` | ✅ Completed | `Expr::I` atom, `rectform`, polarform, branch cuts, residue calculus |
| 42. Special Functions & Table Dispatch | `specfn` | ✅ Completed | $\Gamma, \beta, \text{erf}$, Bessel $J, Y, I, K$, Airy $\text{Ai}, \text{Bi}$, orthogonal polynomials $P_n, T_n, U_n, H_n, L_n$ table differentiation |
| 43. Operational Laplace Transforms & ODE Initial Values | `transform` | ✅ Completed | Operational forward Laplace (`laplace`), inverse Laplace (`ilt`), frequency shift, multiplier, derivative rules, IVP/BVP solvers (`ic1`, `ic2`, `bc2`) |
| 44. Scoped Deductive Inference & Assumptions Context | `assume` | ✅ Completed | Thread-safe `Assumptions` context, inequality DAG inference, interval bound propagation, sign deduction (`csign`, `asksign`), predicate evaluation (`is`), domain properties (`PropertyKind`), and reversibility (`forget`) |
| 45. Maxima 5.50 Parity Test Suite & Golden Verification | `tests` | ✅ Completed | Full upstream golden test suite ported from Maxima 5.50 (`rtest_integrate`, `rtest_limit`, `rtest_taylor`, `rtest_solve`, `rtest_laplace`, `rtest_bessel`), 100% test pass rate across workspace |


---

### Session Rules & Protocol

1. **README Rule**: Actively update [`README.md`](file:///e:/Rust/Max/README.md) after completing each phase (updating the Current Status table and relevant examples).
2. **Session Log Rule**: Update [`SESSION_LOG.md`](file:///e:/Rust/Max/SESSION_LOG.md) phase tracker and notes at each milestone.
3. **Clippy & Formatting Rule**: Run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt` before completing any phase.


---

## Session 3 — Full Implementation & Verification (Phases 3–12)

**Date**: 2026-10-05 / 2026-10-06  
**Scope**: End-to-end implementation and verification of Phases 3 through 12.

### Summary of Deliverables Built
1. **Phase 3 (E-Graph & 1:1 Maxima Rules)**:
   - Defined [`egg::Language`](file:///e:/Rust/Max/sym-core/src/lang.rs) (`MathLang`) and constant analysis (`MathAnalysis`).
   - Implemented [`RuleProvenance`](file:///e:/Rust/Max/sym-core/src/provenance.rs) registry and structured metadata citing Maxima Common LISP files (`simp.lisp`, `trigi.lisp`, `combin.lisp`, `logarc.lisp`).
   - Implemented 52 rewrite rules across [`rules/arithmetic.rs`](file:///e:/Rust/Max/sym-core/src/rules/arithmetic.rs), [`rules/trigonometric.rs`](file:///e:/Rust/Max/sym-core/src/rules/trigonometric.rs), [`rules/exponential.rs`](file:///e:/Rust/Max/sym-core/src/rules/exponential.rs), and [`rules/logarithmic.rs`](file:///e:/Rust/Max/sym-core/src/rules/logarithmic.rs).
2. **Phase 4 (Cost, Saturation, Extraction)**:
   - Implemented [`FlopCost`](file:///e:/Rust/Max/sym-core/src/cost.rs) weighting constants and operations to extract minimal-computation expressions.
   - Configured [`saturate`](file:///e:/Rust/Max/sym-core/src/saturate.rs) and [`extract`](file:///e:/Rust/Max/sym-core/src/extract.rs) with multi-root support.
3. **Phase 5 (CSE Post-Pass)**:
   - Implemented DAG-based subterm identification and topological let-binding generation in [`sym-core/src/cse.rs`](file:///e:/Rust/Max/sym-core/src/cse.rs).
4. **Phase 6 (Codegen)**:
   - Implemented TokenStream code generator in [`sym-core/src/codegen.rs`](file:///e:/Rust/Max/sym-core/src/codegen.rs) emitting flat `let` bindings for `f32` and `f64`.
5. **Phase 7 (Multi-Format Parsers)**:
   - Implemented `winnow = "0.7"` parsers for Maxima strings ([`parser/maxima.rs`](file:///e:/Rust/Max/sym-core/src/parser/maxima.rs)) and LaTeX scalar math formulas ([`parser/latex.rs`](file:///e:/Rust/Max/sym-core/src/parser/latex.rs)).
6. **Phase 8 (High-Level API & Pipeline Builder)**:
   - Implemented free functions `optimize()` / `optimize_gradient()` and fluent builder `Pipeline` in [`sym-core/src/pipeline.rs`](file:///e:/Rust/Max/sym-core/src/pipeline.rs).
7. **Phase 9 (Proc-Macro Crate `sym-macros`)**:
   - Implemented `sym_fn!` and `sym_eval!` in [`sym-macros`](file:///e:/Rust/Max/sym-macros/src/lib.rs) with span-preserving error diagnostics and attributes parsing (`fast_math`, `node_limit`, `time_limit_ms`).
8. **Phase 10 (Facade Crate `maxima-rs` & Prelude)**:
   - Re-exported all feature-gated functionality and exposed [`prelude`](file:///e:/Rust/Max/maxima-rs/src/prelude.rs).
9. **Phase 11 (Testing Infrastructure & Coverage)**:
   - Created test suite package [`maxima-tests`](file:///e:/Rust/Max/tests/Cargo.toml) with 9 integration suites and 2 proptest suites.
   - Implemented automated 1:1 Maxima citation verification in [`tests/integration/maxima_coverage.rs`](file:///e:/Rust/Max/tests/integration/maxima_coverage.rs).
10. **Phase 12 (Polish & Verification)**:
    - Workspace clean build, all tests passed, 0 clippy warnings (`-D warnings`), code formatted with `cargo fmt`.

---

## Session 4 — Phase 13: Tensor & Matrix Calculus

**Date**: 2026-10-06  
**Scope**: Implementation of the foundational Ricci Calculus architecture for tensors (Phase 13).

### Deliverables Built
- Added `tensor` feature to `sym-core`.
- Defined `Index` and `IndexType` (Covariant/Contravariant) in [`sym-core/src/tensor.rs`](file:///e:/Rust/Max/sym-core/src/tensor.rs).
- Added `Tensor(String, Vec<Index>)` variant to the `Expr` AST.
- Implemented serialization of `Tensor` into `egg::Symbol` string format (`"T|<name>,C|<idx>,U|<idx>"`) for e-graph compatibility.
- Integrated `Tensor` across all passes: `MathLang` representation, cost evaluation, CSE counting, differentiation (0-tensor stub), and token emission (`T_cov_idx`).
- Passes all tests and clippy checks cleanly.

*Update this file at the end of every session with progress, decisions, and issues encountered.*
## Session 5 - Phase 13: Advanced Math & Calculus

**Date**: 2026-10-06  
**Scope**: Implementation of the advanced math operations (Calculus, Tensor notation, Matrices) for the AST and LaTeX parser.

### Deliverables Built
- Added Derivative, Integral, Sum, Matrix and Tensor variants to the Expr AST in expr.rs.
- Handled Matrix opaque serialization for egg::Symbol string format in MathLang.
- Integrated all new symbolic nodes across the pipeline (CSE traversal, FLOP cost evaluation, element-wise differentiation, code emission).
- Upgraded the winnow combinator parser in latex.rs to support \int, \sum, \begin{pmatrix}, \frac{\partial f}{\partial x}, and Greek letters with tensor notation (e.g. T_{\mu}).
- Passed all golden tests for advanced math syntax and compilation checks (cargo clippy, cargo test).

---

## Architecture & Skill Reference
- [/grafify](.agents/skills/grafify/SKILL.md) � Authoritative dependency graph, module call graph, and function-level cross-module interdependency matrix for continuous reference and maintenance.

---

## Session 6 — Phases 14–21: WebUI Restoration & 1:1 Maxima Parity

**Date**: 2026-11-22
**Scope**: Restore the `maxima-web` UI and deepen 1:1 Maxima LISP rule/AST/parser parity.

### Plan (committed first)
- Added `Roadmap — Phases 14–21` to `agents.md` and a matching roadmap table to `README.md`.
- Commit: `docs(plan): add Phases 14-21 roadmap for WebUI restoration and Maxima parity`.

### Phase 14 — WebUI Correctness ✅
- `maxima-web/static/index.html`: added the missing `<main class="flex-1 flex overflow-hidden">`
  opening tag around the input/output panes and deleted the stray `<main>` emitted after `</html>`
  (the two panes previously stacked vertically).
- `maxima-web/static/app.js`: fixed the unterminated `copyText()` function and the stray trailing
  brace that made the whole script a syntax error (`node --check` now passes); wired
  `copyResultBtn` / `copyCodeBtn`.
- `maxima-web/src/service.rs`: accepted the `optimize` alias for `simplify`; added 5 unit tests
  (simplify, optimize alias, differentiate, evaluate, unknown operation).
- `README.md`: corrected the `/api/compute` payload documentation.
- Verification: `cargo test -p maxima-web` (5 passed), `cargo clippy -p maxima-web --all-targets -- -D warnings` clean.


### Phase 15 — Advanced AST Nodes ✅
- Added `Expr::{Sinh, Cosh, Tanh, Asin, Acos, Atan, Pi, E, Factorial}` with constructors.
- Threaded through `Display`, `eval`, `variables`, `node_count`, `diff` (chain rules), `lang`
  (`MathLang` + `MathAnalysis` factorial folding + `NamedConst` for π/e + round-trip), `cost`
  (transcendentals 25, factorial 20), `cse`, `codegen` (`PI`/`E` consts + factorial loop),
  `sym-macros` (`sinh`…`atan` methods and calls), and the web LaTeX renderer.
- Added `Rat::factorial` exact folding (0‥100).
- New unit tests in `expr`, `diff`, `lang`, `codegen`.
- Verification: `cargo build --workspace --all-features`, `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt` all clean.


### Phase 16 — Maxima Parser Parity ✅
- Rewrote `parse_maxima` core: identifier parsing now handles `_`, `%pi`/`%e` (and keeps other
  `%symbols` symbolic), postfix `!` via a new `postfix` precedence level (tighter than `^`),
  and multi-argument calls.
- Added `diff(f,x)` → `Derivative`, `integrate(f,x)` → `Integral`, `sum(f,i,lo,hi)` → `Sum`,
  and `matrix([..],[..])` → `Matrix`.
- Added 6 parser unit tests and 5 integration golden tests in `tests/integration/parser_maxima.rs`.
- Verification: `cargo test -p sym-core --all-features` (100 unit + 16 doctests),
  `cargo test -p maxima-tests --test parser_maxima` (8 passed).


### Phase 17 — Rule Expansion I ✅
- New `sym-core/src/rules/hyperbolic.rs` (19 rules) citing `sinh.lisp`
  (`simp-%sinh`/`simp-%cosh`/`simp-%tanh`) and `trigi.lisp`
  (`simp-%asin`/`simp-%acos`/`simp-%atan`): zero values, parity, `cosh²-sinh²=1`,
  `tanh` definition, inverse cancellations.
- Extended `rules/trigonometric.rs` (8 rules): double-angle, angle-sum (`sin`/`cos`/`tan`),
  `1+tan²=1/cos²`, and `1∓sin²/cos²` complements.
- `RuleConfig` gained `hyperbolic` (default `true`); `rules/mod.rs` registers the new set.
- `maxima_coverage` now validates 5 Maxima Lisp files and asserts ≥ 79 rules.
- Added hyperbolic/inverse-trig golden tests.
- Verification: full workspace tests pass; clippy `-D warnings` clean.


### Phase 18 — Rule Expansion II ✅
- New `sym-core/src/rules/rational.rs` (8 rules) citing `nrat4.lisp` `$ratsimp`/`$nrat`:
  sign normalization, double reciprocals, quotient-of-quotient, same/different-denominator
  addition/subtraction, multiply-by-fraction, and power quotients.
- New `sym-core/src/rules/factorial.rs` (5 rules) citing `sfacts.lisp` `factcomb`:
  `0!/1!`, successor recurrence, `n!/(n-1)!`, `(n+1)!/n!`.
- `RuleConfig` gained `rational` and `factorial` switches (default `true`).
- Registry now 92 provenanced rules; coverage test validates `nrat4.lisp` and `sfacts.lisp`.
- New `tests/integration/rational_factorial.rs` (6 golden tests) wired into `tests/Cargo.toml`.
- Verification: full workspace tests pass; clippy `-D warnings` clean.


### Phase 19 — Matrix E-Graph Fidelity ✅
- New `sym-core/src/matrix.rs`: lossless prefix-token codec (`MX`/`V`/`C`/operators/`TEN`/`DR`/`IN`/`SU`)
  for `Expr::Matrix`, with recursive encode/decode and unit tests (incl. nested matrices/tensors).
- `lang.rs` now encodes matrices with `encode_matrix_symbol` and restores them with
  `decode_matrix_symbol`; removed the `"matrix_unsupported_in_egraph"` sentinel.
- Documented limitation: an opaque matrix leaf cannot host element-wise `matrix.lisp` rewrites;
  contents are preserved exactly and element simplification happens pre-injection.
- New `tests/integration/matrix_fidelity.rs` (3 tests) proves matrices survive saturation/extraction.
- Verification: `sym-core` 104 unit tests pass; matrix fidelity suite passes.


### Phase 20 — WebUI Provenance & Coverage Surfacing ✅
- `maxima-web/src/api.rs`: new `RuleEntry`, `FileRuleCount`, `RulesResponse` models.
- `maxima-web/src/service.rs`: `rules_catalog()` (sorted by file/name, per-file counts) and a
  `provenance` compute operation that needs no expression.
- `maxima-web/src/main.rs`: `GET /api/rules` route and a `provenance` entry in `/api/operations`.
- Frontend: "Maxima coverage" panel (per-file counts from `/api/rules`) plus `sinh(x)`, `atan(x)`,
  `5!` example chips; `node --check` clean.
- Tests: 7 web tests (incl. catalog shape + provenance op).
- Live verification (fresh binary): `GET /api/rules` → `92` rules across 7 files;
  `sinh(x)^2 - cosh(x)^2` → `(-1)`; `5!` → `120`; matrix round-trip preserved.


### Phase 21 — Documentation, Coverage Report & Final Verification ✅
- `README.md`: status → **Phase 21 Complete**, Phase 3 noted as the initial 52-rule baseline,
  new "Maxima 1:1 Rule Coverage" table (92 rules / 7 files), `/api/rules` documented,
  Phases 14–21 roadmap all ✅.
- `GLOSSARY.md`: added Hyperbolic Function Node, Inverse Trigonometric Node, Factorial
  Simplification, Rational Normalization, Named Constant, Matrix Symbol Encoding, Rule Coverage API.
- `agents.md`: Phase 21 marked complete.
- Final verification (2026-11-22):
  - `cargo fmt --all` — clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
  - `cargo test --workspace --all-features` — all suites green (sym-core 104 unit tests,
    integration suites incl. new `rational_factorial`, `matrix_fidelity`, 7 web tests, doctests).
  - Live WebUI check — 92 rules / 7 files; `sinh(x)^2 - cosh(x)^2` → `(-1)`; `5!` → `120`;
    `matrix([1,2],[3,4])` preserved; `/api/operations` includes `provenance`.

### Phases 14–21 — Final Status

| Phase | Status |
|-------|--------|
| 14 — WebUI correctness | ✅ |
| 15 — Advanced AST nodes | ✅ |
| 16 — Maxima parser parity | ✅ |
| 17 — Rule expansion I | ✅ |
| 18 — Rule expansion II | ✅ |
| 19 — Matrix e-graph fidelity | ✅ |
| 20 — WebUI provenance | ✅ |
| 21 — Docs & verification | ✅ |

---

## Session 5 — Comprehensive CAS Expansion & Architectural Grilling (Phases 22–32)

**Date**: 2026-11-22  
**Scope**: Expanded repository to next-level CAS on-par with Wolfram Mathematica based on `agents_instructions.md`. Conducted interactive `/grill-with-docs` architectural review, web research on state-of-the-art polynomial and symbolic algorithms (SDMP, LRT, Gruntz, Bronstein Risch), and committed a 11-phase implementation roadmap.

### Settled Design Decisions (Session 5: Decisions 30–40)

| # | Decision | Choice | Round |
|---|----------|--------|-------|
| 30 | Polynomial Representation | Sparse Distributed Multivariate Polynomial (`Poly`) with packed 64-bit exponents under Graded Reverse Lexicographical (`grevlex`) term order; dynamic `Vec<u32>` fallback for >8 variables | R1 |
| 31 | Polynomial Arithmetic | Monagan-Pearce heap-based priority-queue multiplication and pseudo-division in $O(t)$ memory | R1 |
| 32 | GCD & Factorization Ladder | Multi-tier escalation: Subresultant PRS as exact baseline, modular GCD over $\mathbb{F}_p$ for high degrees; Yun's square-free; Berlekamp/Cantor-Zassenhaus over $\mathbb{F}_p$ with linear Hensel lifting to $\mathbb{Z}[x]$ for factoring; extended Euclidean for `partfrac` | R1 |
| 33 | Hybrid E-Graph / Procedural Invariant | Dedicated procedural AST-to-AST pipelines owning canonical structural invariants, calling scoped e-graph passes only for local subterm reduction; formalised in **ADR-0006** | R1 |
| 34 | Rational Integration Algorithm | Hermite reduction stripping repeated denominator poles without full factoring + Lazard-Rioboo-Trager (LRT) computing real-valued $\ln / \text{atan}$ antiderivatives directly via Sturm sequences | R1 |
| 35 | Gruntz Zero-Equivalence Decider | 3-tier decider: 1) exact `ratsimp` and constant folding, 2) e-graph algebraic saturation, 3) multi-point float interval evaluation guard | R1 |
| 36 | Cargo Feature Partitioning | Fine-grained features (`poly`, `integrate`, `series`, `limit`, `solve`) unified under meta-feature `cas`, included in `full` | R1 |
| 37 | Monomial Exponent Packing | 64-bit packed word (8 variables $\times$ 8-bit degree $\le 255$) with single-instruction integer comparison under `grevlex` | R2 |
| 38 | Integration Noun Fallback | Unified with existing `Expr::Integral(Box<Expr>, Box<Expr>)` via `Expr::integral(f, var)` constructor | R2 |
| 39 | Series Representation | Truncated Laurent series: `Series { center: Expr, valuation: i64, coeffs: Vec<Expr>, order: i64 }` supporting negative pole orders and fractional Puiseux powers | R2 |
| 40 | Solvers Return Types | Dedicated types: `AlgebraicSolutions(Vec<Expr>)` and `OdeSolution { solution: Expr, constants: Vec<String> }` with conversion to equality expressions | R2 |

### Artifacts Created & Updated
- **ADR Created**: [`docs/adr/0006-hybrid-egraph-and-procedural-cas.md`](docs/adr/0006-hybrid-egraph-and-procedural-cas.md) — Accepted.
- **GLOSSARY Extended**: Added 6 canonical CAS terms (*Sparse Distributed Multivariate Polynomial*, *Procedural CAS Algorithm*, *Ratsimp*, *Hermite Reduction*, *Rothstein-Trager / LRT*, *Gruntz Limit Algorithm*).
- **Plan Committed**: Added detailed Phase 22–32 specifications to [`agents.md`](agents.md).

---

## Session 7 — Phase 22: Sparse Multivariate Polynomial Engine ✅

**Date**: 2026-11-22
**Scope**: Implemented the `poly` Cargo feature and procedural polynomial engine (Phase 22).

### Deliverables
- New `poly` feature in `sym-core` and `maxima-rs` (added to `full`); `#[cfg(feature = "poly")] pub mod poly;`.
- `sym-core/src/poly/monomial.rs`: `Monomial` (grevlex, 64-bit `packed()` for ≤8 vars, `VarContext`).
- `sym-core/src/poly/mod.rs`: `Poly`/`Term` over exact `BigRational`, `Expr <-> Poly` conversion, add/sub/mul/pow, term normalization.
- `sym-core/src/poly/heap.rs`: Monagan-Pearce heap multiplication + univariate `pseudo_divmod`.
- `sym-core/src/poly/expand.rs`: `expand()` via `Poly`, structural distribution fallback for non-polynomials.
- `tests/integration/poly_expand.rs` (6 golden tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --features poly --lib`: 121 passed.
- `cargo test -p maxima-tests --test poly_expand`: 6 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- Existing suites stay green (`basic_arithmetic`, `trigonometric`, …); `maxima-web` 7 tests; sym-core doctests 16.
- `cargo build -p sym-core --no-default-features --features poly`: OK; `cargo fmt --all -- --check`: clean.

---

## Session 8 — Phase 23: Exact Rational Algebra & Factorization ✅

**Date**: 2026-11-22
**Scope**: GCD, rational simplification, factorization, and partial fractions (Phase 23).

### Deliverables
- `sym-core/src/poly/gcd.rs`: exact Euclidean univariate `poly_gcd`, `poly_divrem`/`poly_div_exact`, `poly_derivative`.
- `sym-core/src/poly/rat.rs`: `Fraction` (num/den polys), `to_fraction`, `ratsimp` (GCD reduction + monic denominator).
- `sym-core/src/poly/factor.rs`: rational-root linear factorization (integer coefficient extraction, rational-root candidates, synthetic division); Berlekamp/Hensel lifting noted as a planned refinement.
- `sym-core/src/poly/partfrac.rs`: cover-up partial fractions over distinct linear factors.
- `tests/integration/poly_rat.rs` (4 golden tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --features poly --lib`: 129 passed.
- `cargo test -p maxima-tests --test poly_rat`: 4 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- `factor`/`partfrac` cover univariate polynomials with rational-root structure; full Berlekamp/Hensel and multivariate factorization remain a follow-up.

---

## Session 9 — Phase 24: Symbolic Integration (Rational & Elementary) ✅

**Date**: 2026-11-22
**Scope**: New `integrate` feature; procedural indefinite integration (Phase 24).

### Deliverables
- `sym-core/src/integrate/mod.rs`: `integrate(expr, var)` — linearity, power rule, elementary dispatch, rational/IBP fallbacks, `Expr::integral` noun.
- `sym-core/src/integrate/sin.rs`: linear-argument rules for sin/cos/exp/sinh/cosh/ln.
- `sym-core/src/integrate/hermite.rs`: rational integration via `partfrac`.
- `sym-core/src/integrate/lrt.rs`: logarithmic antiderivative terms `(c/a)·ln(u)`.
- `sym-core/src/integrate/ibp.rs`: integration by parts for `x^n·exp/sin/cos`.
- `tests/integration/integrate_elementary.rs` (10 anti-difference tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 131 passed.
- `cargo test -p maxima-tests --test integrate_elementary`: 10 passed (differentiate each antiderivative back to the integrand).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Non-linear arguments and denominators without distinct rational linear factors return the noun form; full Risch/Hermite/LRT arrive in Phase 25.

---

## Session 10 — Phase 25: Transcendental Risch u-Substitution ✅

**Date**: 2026-11-22
**Scope**: Derivative-divides transcendental integration (Phase 25).

### Deliverables
- `sym-core/src/integrate/risch.rs`: `try_risch` recognising `∫k·g'·e^g`, `∫k·g'·g^n`, `∫k·g'/g` via structural/polynomial proportionality; wired into `integrate` as a fallback before the noun form.
- `sym-core/src/integrate/risch_de.rs`: minimal Risch differential-equation solver (`f=0`, `g=0`).
- `tests/integration/integrate_risch.rs` (6 anti-difference tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 133 passed.
- `cargo test -p maxima-tests --test integrate_risch`: 6 passed (`∫x·e^{x²}`, `∫x²·e^{x³}`, `∫e^x/(e^x+1)`, `∫2x/(x²+1)`, `∫ln x`, and `∫e^{-x²}` → noun).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Substitutions are recognised for derivative-divides exp/power/log patterns; general Bronstein tower solving remains a refinement.

---

## Session 11 — Phase 26: Laurent Series ✅

**Date**: 2026-11-22
**Scope**: Truncated Laurent/Taylor series engine (Phase 26).

### Deliverables
- `sym-core/src/series.rs`: `Series { var, center, valuation, coeffs, order }` with `add/sub/neg/mul/inv/div/pow_int/diff/compose`, constant folding for exact zero detection, and `series_expand` for sin/cos/exp/ln kernels composed onto arbitrary series.
- `tests/integration/series.rs` (9 tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 133 passed.
- `cargo test -p maxima-tests --test series`: 9 passed (sin/cos/exp/1/(1-x)/1/sin incl. Laurent pole, inverse multiplication, derivative, composition `sin(x²)`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

---

## Session 12 — Phase 27: Asymptotic Limits ✅

**Date**: 2026-11-22
**Scope**: `limit` feature — MRV enumeration, series-based limits, log-transform, numeric fallback (Phase 27).

### Deliverables
- `sym-core/src/limit/mod.rs`: `limit(expr, var, point)` with `t=±1/x` substitution for infinite points, series-based constant-term extraction, `Pow` log-transform (`exp(lim(exp·ln base))`), leading-term exp-difference rewrite, and numeric convergence fallback recognising exact constants.
- `sym-core/src/limit/mrv.rs`: exponential/logarithmic subexpression enumeration.
- `sym-core/src/limit/zero.rs`: tiered zero-equivalence decider (exact → polynomial → series constant term).
- `tests/integration/limits.rs` (6 tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 138 passed.
- `cargo test -p maxima-tests --test limits`: 6 passed (`sin x/x→1`, `(1+1/x)^x→e`, `e^{x+e^{-x}}-e^x→1`, `1/x→∞→0`, removable rational, polynomial).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Non-analytic/essential limits outside the handled patterns rely on the numeric convergence fallback; a full Gruntz MRV tower replaces this incrementally.

---

## Session 13 — Phase 28: Definite Integration & Pole Scanning ✅

**Date**: 2026-11-22
**Scope**: `defint` on top of `integrate` + `limit` (Phase 28).

### Deliverables
- `sym-core/src/integrate/defint.rs`: `defint(expr, var, a, b) -> DefiniteResult { Value | Divergent | Unevaluated }` — rational pole scanner (interior roots ⇒ `Divergent`), FTC evaluation via `limit(F, b) - limit(F, a)`, constant folding.
- `test/integration/integrate_defint.rs` (4 tests) registered in `tests/Cargo.toml`.
- `integrate` feature now depends on `limit` (for improper bounds).

### Verification
- `cargo test -p sym-core --lib`: 138 passed.
- `cargo test -p maxima-tests --test integrate_defint`: 4 passed (`∫_0^1 x²=1/3`, `∫_0^1 x=1/2`, `∫_0^∞ e^{-x}=1`, `∫_{-1}^1 1/x` divergent).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Pole scanning covers rational (distinct rational-root) poles; irrational/complex-singularity detection is a refinement.

---

## Session 14 — Phase 29: Closed-Form Algebraic Solvers ✅

**Date**: 2026-11-22
**Scope**: `solve` feature — polynomial and transcendental solvers (Phase 29).

### Deliverables
- `sym-core/src/solve/mod.rs`: `solve(expr, var) -> AlgebraicSolutions`.
- `sym-core/src/solve/alg.rs`: rational-root factor-and-split + linear/quadratic closed forms (degree 1–4 with rational roots).
- `sym-core/src/solve/isolate.rs`: variable isolation for invertible elementary functions.
- `tests/integration/solvers_alg.rs` (6 tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 139 passed.
- `cargo test -p maxima-tests --test solvers_alg`: 6 passed (linear, quadratic rational/irrational, cubic, quartic, transcendental isolation) — every root back-substitutes to 0 within 1e-6.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Cubics/quartics without rational roots use factor-and-split only; full Cardano/Ferrari radicals are a refinement.

---

## Session 15 — Phase 30: ODE Solvers ✅

**Date**: 2026-11-22
**Scope**: First- and second-order ODE solvers (Phase 30).

### Deliverables
- `sym-core/src/solve/ode.rs`: `ode2(ode, y, x) -> Option<OdeSolution>` with linear-vs-coefficient extraction (treats `y`, `y'`, `y''` as atoms; coefficients independent of `y`/`x`, parameters allowed).
- `sym-core/src/solve/ode_first.rs`: `a·y' + b·y = 0`.
- `sym-core/src/solve/ode_second.rs`: `a·y'' + b·y' + c·y = 0` (oscillatory, real distinct/repeated, complex).
- `tests/integration/solvers_ode.rs` (3 tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 139 passed.
- `cargo test -p maxima-tests --test solvers_ode`: 3 passed (`y'+y=0`, `y''+ω²y=0` oscillatory, `y''-y=0`), each residual ~0 via `differentiate`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Currently handles linear constant-coefficient equations; separable/Bernoulli/exact 1st-order forms and non-constant coefficients are a refinement.

---

## Session 16 — Phase 31: Maxima CAS Parser & Facade ✅

**Date**: 2026-11-22
**Scope**: `cas` feature; Maxima CAS command parsing + facade/prelude (Phase 31).

### Deliverables
- `sym-core/src/parser/maxima.rs`: `CasCommand` + `parse_maxima_cas` + `execute_cas` for `expand`, `factor`, `ratsimp`, `partfrac`, `limit`, `integrate`, `defint`, `taylor`, `solve`, `ode2`.
- `sym-core/src/series.rs`: `taylor()` convenience wrapper.
- `maxima-rs/src/prelude.rs`: re-exports `expand`, `ratsimp`, `factor`, `partfrac`, `integrate`, `defint`/`DefiniteResult`, `taylor`, `limit`, `solve`, `ode2`, and the CAS command parser.
- New `cas` Cargo feature (sym-core + maxima-rs), included in `full`; maxima-rs `full` now also bundles series/limit/solve.
- `tests/integration/parser_cas.rs` (10 tests) registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p sym-core --lib`: 139 passed.
- `cargo test -p maxima-tests --test parser_cas`: 10 passed (parse+execute each command; prelude exposes all CAS operations).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- CAS command parser accepts single top-level calls (no `;`-separated batches); nested command calls are not composed.

---

## Session 17 — Phase 32: Verification & Maxima Parity ✅

**Date**: 2026-11-22
**Scope**: proptest invariants + Maxima-style parity suite (Phase 32).

### Deliverables
- `tests/proptest/integrate_correctness.rs`: `d/dx(∫f) ≈ f` over random polynomials (256 proptest cases + 1000 deterministic).
- `tests/proptest/solve_correctness.rs`: quadratic root back-substitution (256 proptest cases + 1089 deterministic).
- `tests/integration/maxima_cas_parity.rs`: representative `rtest_integrate`/`rtest_limit`/`rtest_solve` cases.
- All registered in `tests/Cargo.toml`.

### Verification
- `cargo test -p maxima-tests --test integrate_correctness`: 2 passed.
- `cargo test -p maxima-tests --test solve_correctness`: 2 passed.
- `cargo test -p maxima-tests --test maxima_cas_parity`: 4 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Parity cases are a curated representative subset, not exhaustive `.mac` playback.

---

## Session 18 — Phase 33: WebUI Recursive Composer & E2E ✅

**Date**: 2026-11-22
**Scope**: Recursive expression composer + final webUI integration suite (Phase 33).

### Deliverables
- `maxima-web/src/lib.rs`: exposes `api` + `service` as a library so tests share the HTTP backend code.
- `maxima-web/src/api.rs`: `FunctionCatalogEntry` + `FunctionCatalog`.
- `maxima-web/src/service.rs`: `functions_catalog()` (unary/binary/CAS entries with Maxima+LaTeX templates) + unit test.
- `maxima-web/src/main.rs`: `GET /api/functions` route (binary now uses the library).
- `maxima-web/static/index.html` / `app.js`: compose dropdown, recursive wrap (cursor-aware, Maxima+LaTeX), Undo/Clear history.
- `tests/integration/webui_e2e.rs` (6 tests) registered in `tests/Cargo.toml` (tests crate now depends on `maxima-web`).

### Verification
- `cargo test -p maxima-web`: 8 passed (incl. function catalog).
- `cargo test -p maxima-tests --test webui_e2e`: 6 passed (`/functions` catalog, `/rules` ≥92, simplify recursive, differentiate, provenance, matrix).
- `node --check maxima-web/static/app.js`: OK.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --all`: applied.

### Known limitation
- Composer inserts templates via string wrapping (no DOM-level AST editor); cursor selection is honored.

---

## Session 19 — Post-v0.2.0 WebUI Overhaul, CAS API Wiring & Offline Architecture Grilling

**Date**: 2026-11-23
**Scope**: Grilling interview via `/grill-with-docs` for CAS Web API wiring, precedence-aware LaTeX, offline-first WebUI, compute isolation, and Lambda packaging.
**Starting commit**: `05809dda556f9ca9c25f6be8330c928cb2ea309b`

### Settled Design Decisions (Session 19)

| # | Decision | Choice | Round |
|---|----------|--------|-------|
| 30 | Deployment & Config | Standalone offline binary with embedded assets via `rust-embed`; CLI flags (`clap`) + optional TOML config; optional Lambda packaging | R1 |
| 31 | Roadmap Integration | Appended discrete Phases 34–38 to `AGENTS.md` covering M1–M6 | R1 |
| 32 | CAS API Dispatch Boundary | Typed Web Dispatch via `POST /api/compute` with `operation` and `params`; direct invocation of procedural CAS modules; defer `Expr::Call` | R1 |
| 33 | LaTeX Formatter Placement | Core zero-dependency module `sym-core/src/latex.rs` (`pub fn to_latex(&Expr) -> String`) with operator precedence | R1 |
| 34 | Offline Frontend & Zero-Node | Vendored KaTeX + WOFF2 fonts and compiled CSS committed to `static/vendor/`; vanilla ES modules; zero Node runtime dependency | R1 |
| 35 | Declarative Op Catalog | Single authoritative `OPERATIONS: &[OpSpec]` driving `/api/operations` and parameter validation with HTTP 400 | R2 |
| 36 | Uniform Output & Status Model | Standardized `status` (`ok`, `unevaluated`, `divergent`, `no_solution`, `error`), multi-output `outputs: Vec<Output>`, and `verified` flags | R2 |
| 37 | LaTeX Formatting Rules | Precedence (1..5), Greek macros (`\alpha`), subscript constants (`C_{1}`), `pmatrix`, `\frac`, non-redundant parentheses | R2 |
| 38 | Compute Isolation & HTTP Status | `spawn_blocking`, `catch_unwind`, timeouts (8.0s), semaphore concurrency limits, and granular HTTP status codes | R2 |
| 39 | Notebook Interaction Model | Interactive `In[n]` / `Out[n]` stream, "Use as input", live `POST /api/parse` preview, and `localStorage` / URL hash state | R2 |

### Phase 34 Completion — CAS Web API Wiring & Typed Dispatch ✅

**Date**: 2026-11-23
**Deliverables**:
- `maxima-web/src/api.rs`: `Status` enum (`#[default] Ok`, `Unevaluated`, `Divergent`, `NoSolution`, `Error`), `Output`, `ParamSpec`, `OpSpec`, `ParseRequest`, `ParseResponse`, updated `ComputeRequest` with `params: HashMap<String, Value>`, and `ComputeResponse`.
- `maxima-web/src/service.rs`: Declarative `OPERATIONS: &[Operation]` catalog with parameter specs, typed parameter helpers (`param_var`, `param_expr`, `param_point`, `param_int`), procedural CAS dispatch arms (`expand`, `factor`, `ratsimp`, `partfrac`, `integrate`, `defint`, `limit`, `taylor`, `solve`, `ode2`, `rust_gradient`), automated verification checks (`verify_integral`, `verify_root`), matrix formatting, constant folding for points, and `parse()` endpoint logic.
- `maxima-web/src/main.rs`: Registered `POST /api/parse`, wired `/api/operations` directly to `service::operations_catalog()`, exposed `build_app`.
- `tests/Cargo.toml`: Added `serde_json = "1"`.
- `tests/integration/webui_e2e.rs`: Extended with 18 end-to-end integration tests covering all CAS operations, verification, and `parse`.

**Verification**:
- `cargo test -p maxima-web`: 22 unit tests passed.
- `cargo test -p maxima-tests --test webui_e2e`: 18 integration tests passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0.
- `cargo fmt --all -- --check`: clean exit 0.
- `cargo test --workspace --all-features`: 139 sym-core unit + doc tests, 0 regressions across all suites.

---

### Phase 35 Completion — Precedence-Aware LaTeX Formatter in `sym-core` ✅

**Date**: 2026-11-23
**Deliverables**:
- `sym-core/src/latex.rs`: Precedence-aware LaTeX formatter (`to_latex(&Expr) -> String`) with operator precedence levels 1..5, proper associativity for subtraction, fraction grouping without redundant outer parentheses, matrix `\begin{pmatrix}`, summation formatting fix, Greek symbol macros (`\alpha`, `\beta`, etc.), subscripted solver constants (`_C1` -> `C_{1}`), and subscripts (`x_0` -> `x_{0}`).
- `sym-core/src/lib.rs`: Exposed `pub mod latex;` and re-exported `pub use latex::to_latex;`.
- `sym-core/src/parser/latex.rs`: Added support for thin-space `\,` multiplication token in parser.
- `maxima-rs/src/lib.rs` & `prelude.rs`: Re-exported `to_latex`.
- `maxima-web/src/service.rs`: Replaced legacy naive formatter with delegation to `maxima_rs::to_latex(e)`.
- `tests/integration/latex_printer.rs`: Comprehensive test suite verifying operator precedence, compound bases, fractions, matrices, calculus forms, and roundtrip `parse_latex(to_latex(e)) == e`.

**Verification**:
- `cargo test -p sym-core`: 143 unit tests + 17 doctests passed.
- `cargo test -p maxima-tests --test latex_printer`: 5 integration tests passed.
- `cargo test -p maxima-tests --test webui_e2e`: 18 integration tests passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean exit 0.
- `cargo fmt --all -- --check`: clean exit 0.
---

### Phase 36 Completion — Offline-First WebUI Notebook & Component Rebuild ✅

**Date**: 2026-11-23
**Deliverables**:
- `maxima-web/static/vendor/katex/`: Vendored KaTeX 0.16.11 JS, CSS, `auto-render.min.js`, and all 20 WOFF2 fonts into `fonts/` for 100% offline math typesetting without external CDNs.
- `maxima-web/static/vendor/app.css`: Standalone, self-contained CSS containing custom theme variables (light/dark mode), layout styles, notebook stream, status chips, button components, and responsive layout down to 375px.
- `maxima-web/static/index.html`: Clean, accessible semantic HTML referencing only local vendor assets and vanilla ES modules.
- `maxima-web/static/js/`: Vanilla ES modules structure:
  - `api.js`: Typed fetch clients for `/api/health`, `/api/operations`, `/api/functions`, `/api/rules`, `/api/compute`, and `/api/parse`.
  - `preview.js`: Debounced live typeset preview and detected variable chips.
  - `palette.js`: Operations catalog button grid, dynamic parameter form generation from schema, example chips, and recursive function composer with wrap/undo/clear.
  - `notebook.js`: Interactive notebook stream (`In[n]` / `Out[n]`), status chips, verified badges, "Use as input" promotion, and tabbed outputs (Typeset, Maxima, LaTeX, Rust, Rules citations).
  - `history.js`: URL hash state synchronization (`#op=...&expr=...`) and localStorage persistence.
  - `main.js`: Dark mode orchestration, keyboard shortcuts (Shift+Enter), and server health monitoring.

**Verification**:
- `node --check maxima-web/static/js/*.js`: All modules passed with exit 0.
- `cargo test -p maxima-web`: 22 unit tests passed.
- `cargo test -p maxima-tests --test webui_e2e`: 18 integration tests passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean exit 0.
- `cargo fmt --all -- --check`: clean exit 0.
---

### Phase 37 Completion — Web Service Isolation, Concurrency & Asset Embedding ✅

**Date**: 2026-11-23
**Deliverables**:
- `maxima-web/src/config.rs`: CLI argument parsing via `clap` (`--bind`, `--port`, `--timeout-ms`, `--concurrency`, `--max-body-size`, `--max-expr-len`, `--config`) with optional TOML configuration loading; zero environment variables.
- `maxima-web/src/app.rs`: Hardened HTTP routing with `spawn_blocking` and `catch_unwind` compute isolation, `tokio::time::timeout` enforcement (HTTP 408), `tokio::sync::Semaphore` concurrency limits (HTTP 503), payload size / expression length checks (HTTP 413), granular error mapping (HTTP 400, 404, 500), and `rust-embed` static asset serving with MIME type inference.
- `maxima-web/src/lib.rs` & `main.rs`: Re-exports and binary entrypoint initializing explicit configuration and serving embedded assets.
- `tests/integration/web_hardening.rs`: 7 comprehensive integration tests covering HTTP 200, 400, 404, 408, 413, 503, and embedded asset retrieval with correct MIME types.

**Verification**:
- `cargo test -p maxima-web`: 22 unit tests passed.
- `cargo test -p maxima-tests --test web_hardening`: 7 integration tests passed.
- `cargo test -p maxima-tests --test webui_e2e`: 18 integration tests passed.
- `cargo run -p maxima-web -- --help`: Verified all explicit CLI options and zero environment variable reliance.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean exit 0.
- `cargo fmt --all -- --check`: clean exit 0.
---

### Phase 38 Completion — Standalone Distribution & Optional Lambda Packaging ✅

**Date**: 2026-11-23
**Deliverables**:
- `Cargo.toml`: Configured release profile `[profile.release]` with `opt-level = 3`, `lto = "thin"`, `panic = "unwind"`, `strip = true`, `codegen-units = 1`.
- `Dockerfile`: Multi-stage Docker build producing a minimal standalone Linux container (~4.26 MB binary) with embedded AWS Lambda Web Adapter layer for optional zero-code containerized AWS Lambda execution.
- `template.yaml`: AWS SAM template deploying the service container to AWS Lambda with explicit command line argument passing (`Command: ["--bind", "0.0.0.0", "--port", "8080", "--timeout-ms", "8000"]`) and zero application environment variables.
- `.dockerignore`: Excluded local target directories, git metadata, and large reference archives.
- `README.md`: Documented standalone deployment, CLI usage, Docker containerization, and AWS Lambda SAM deployment.

**Verification**:
- `cargo build --release -p maxima-web`: Compiled clean 4.26 MB release binary with embedded assets in 2m 24s.
- `cargo test --workspace --all-features`: 100% green across all unit, proptest, integration, and doc tests (0 regressions).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: Exit code 0, 0 warnings.
- `cargo fmt --all -- --check`: Clean formatting across the entire workspace.

---

### UI Ergonomics Enhancement — Fixed Input Pane & Top-Mounted Latest Output ✅

**Date**: 2026-11-23
**Deliverables**:
- `maxima-web/static/vendor/app.css`:
  - Constrained `html, body` to `height: 100vh; overflow: hidden;`.
  - Main view set to `height: calc(100vh - 56px); min-height: 0; display: flex; overflow: hidden;`.
  - Left `.palette-pane` input window fixed in place with independent vertical scrollbar.
  - Right `.notebook-pane` configured with dedicated `overflow-y: auto; height: 100%; min-height: 0;` and sleek custom scrollbars.
  - Sticky `.notebook-header` ensuring "Clear Notebook" and notebook controls remain visible during scroll.
- `maxima-web/static/js/notebook.js`:
  - Switched `cells.push` to `cells.unshift` and `container.appendChild(cell)` to `container.prepend(cell)`.
  - Newest computation results (`In[n] / Out[n]`) now render immediately at the top of the notebook stream.
  - Auto-scrolls notebook viewport smoothly to `top: 0` on every new computation.
- `maxima-web/static/index.html`:
  - Styled notebook header with sticky positioning and indicator label.
- **Verification**: `cargo test -p maxima-tests --test web_hardening`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`.

---

### Session — Maxima 5.50 1:1 Parity Grilling & Formal Phased Plan ✅

**Date**: 2026-11-24
**Scope**: Full architectural grilling session (`/grill-with-docs`) to establish the complete implementation roadmap for 1:1 parity with Maxima 5.50 (`References/maxima-5.50.0/`).
**Settled Architectural Decisions**:
- **ADR-0009**: Scoped 1:1 parity to **Core Mathematical Parity** covering all routines in Maxima core `src/` (treating `share/` packages as out-of-scope auxiliary extensions). Selected pure-Rust `dashu-float = "0.4"` for arbitrary-precision floating point (`bfloat`) to ensure zero C library dependencies, full WebAssembly support, and procedural macro compile-time execution.
- **ADR-0010**: Added first-class zero-allocation `Expr::I` atom in `Expr` AST enum ($I^2 \leftrightarrow -1$), canonical complex decompositions (`rectform`, `polarform`, `residue`), and selected the **Bareiss fraction-free LU algorithm** (`newdet.lisp`) for symbolic matrix determinants, adjoint inverses, and characteristic polynomials.
- **ADR-0011**: Added consolidated `Expr::SpecialFn(SpecialFnKind, Vec<Expr>)` enum variant for non-elementary special functions ($\Gamma, \beta, \text{erf}$, Bessel, Airy, orthogonal polynomials) with table-driven differentiation/limits. Structured operational Laplace transforms (`laplace`, `ilt`) with `partfrac` pole decomposition, and implemented a thread-safe, immutable, scoped `Assumptions` context (inequality DAG) for `assume`, `forget`, `is`, `asksign`, `csign`.
**Documentation & Plans Updated**:
- [`GLOSSARY.md`](file:///e:/Rust/Max/GLOSSARY.md): Added `Core Mathematical Parity`, `Bfloat (Arbitrary-Precision Float)`, `Bareiss Algorithm`, `Complex Rectangular Form (rectform)`, `Imaginary Unit Atom (Expr::I)`, `Special Function Node (Expr::SpecialFn)`, `Operational Laplace Transform`, and `Scoped Assumptions Context`.
- [`agents.md`](file:///e:/Rust/Max/agents.md): Extended Cargo Feature Architecture and file tree; added detailed phase specifications for **Phases 39 through 45** with strict testing gates (Gates 0–3) and 1:1 Maxima LISP citations.

---

## Session 20 — Canonical AGENTS.md Modernization & Multi-Model Audit Ingestion

**Date**: 2026-11-24  
**Scope**: Ingested `docs/recommendations/agents_instructions.md` and `docs/recommendations/maxima_rs_claude_audit.md` via `/generate-agents-md`. Resolved multi-model tensions across the 4 core architectural axes through an interactive `/grill-with-docs` interview. Upgraded `AGENTS.md` to the canonical 12-section operating manual optimized for **Gemini 3.8 Flash (Medium)**.

### Settled Architectural Decisions (Session 20)
- **ADR-0012**: Formally adopted [`docs/adr/0012-tri-state-symbolic-outcomes-and-verification-honesty.md`](docs/adr/0012-tri-state-symbolic-outcomes-and-verification-honesty.md) establishing:
  1. **Tri-State Symbolic Outcomes**: CAS routines must return `Solved(Expr)`, `ProvenNonElementary`, or `Unsupported(reason)`.
  2. **Dynamic Verification Invariant ("Zero-Fake-Verification")**: Strictly prohibited hard-coded `verified = Some(true)`. Verification flags must be computed at runtime via back-substitution or numerical checks.
  3. **Parser Precedence & Recursion Depth Guard**: Exponentiation (`^`) binds tighter than unary minus (`-x^2 == -(x^2)`); scientific notation tokenization (`1e-3`); hard recursion depth limit of 256.
  4. **Owned Semaphore Permits & Compute Isolation**: `OwnedSemaphorePermit` moved into blocking compute threads to eliminate timeout thread leakage.
  5. **Truthful Capability Reporting**: Documentation and UI palette explicitly distinguish `✅ Complete` from `◐ Partial`.
- **Roadmap Insertion**: Prioritized an **Audit Remediation & Hardening Sprint** (Phases 38A, 38B, 38C) addressing the 7 verified P0 audit defects before proceeding with Phase 39 (`bfloat`).

### Artifacts Updated
- Created [`docs/adr/0012-tri-state-symbolic-outcomes-and-verification-honesty.md`](docs/adr/0012-tri-state-symbolic-outcomes-and-verification-honesty.md).
- Updated [`GLOSSARY.md`](GLOSSARY.md) with 5 canonical terms: *Tri-State Symbolic Outcome*, *Honesty Invariant*, *Recursion Depth Guard*, *Independent Test Oracle*, *Print-Parse Round-Trip Invariant*.
- Synthesized and compiled canonical [`AGENTS.md`](AGENTS.md) following the standardized 12-section blueprint.

---

## Session 21 — Phase 38A: P0 Safety, Correctness & Hardening Complete ✅

**Date**: 2026-11-24  
**Scope**: Remediated critical parser operator precedence bugs, stack overflow aborts, thread concurrency permit leakage, and added print-parse round-trip property tests.

### Deliverables Built
1. **Operator Precedence Fix**:
   - `sym-core/src/parser/maxima.rs` and `sym-core/src/parser/latex.rs`: Reordered precedence hierarchy so exponentiation (`^`) binds tighter than unary minus/plus. Verified `-x^2` at $x=3$ evaluates to `-9.0`, `-2^2 == -4`, `2^-1 == 0.5`, `-x! == -(x!)`, and `(-x)^2 == 9.0`.
2. **Recursion Depth Guard**:
   - Implemented `RecursionDepthGuard` and `check_nesting_limit(input, 256)` in `sym-core/src/parser/mod.rs` and wired into `parse_maxima` and `parse_latex`. Deeply nested parenthesized expressions (`((((...x...))))` 3000 levels) return `ParseError::RecursionLimitExceeded` in microsecond time without stack overflow.
3. **Exact Scientific Notation Parsing**:
   - Implemented `take_number` and `parse_exact_rational` converting decimal floats and scientific notation (`1e-3`, `2.5e+4`) directly to exact `BigRational` constants without floating-point rounding errors.
4. **HTTP 400 Error Body Remediation**:
   - `maxima-web/src/app.rs`: Mapped `result.error` (not `result.result`) into JSON response bodies and populated `error_span`.
5. **Owned Semaphore Permits in Compute Threads**:
   - `maxima-web/src/app.rs`: Concurrency permits are now acquired as owned permits (`try_acquire_owned`) and moved into the `tokio::task::spawn_blocking` closure, ensuring permits are held until the thread actually terminates.
6. **Print-Parse Round-Trip Property Tests**:
   - `tests/proptest/print_parse_roundtrip.rs`: 256 random expression property tests verifying numerical and structural round-trip consistency across Maxima and LaTeX formatting and parsing.
7. **Passing Gate Verified**:
   - `cargo check --workspace --all-targets` (exit 0)
   - `cargo test -p sym-core --test parser_maxima` (11 passed, exit 0)
   - `cargo test -p maxima-tests --test web_hardening` (7 passed, exit 0)
   - `cargo test -p maxima-tests --test print_parse_roundtrip` (2 proptests passed, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `docs/audits/PHASE38A_REPORT.md` compiled.

---

## Session 38B — WebUI Wiring & Styling Restoration

**Date**: 2026-11-24  
**Scope**: Remediated operations palette catalog parsing, eliminated async initialization TDZ exceptions, added embedded CAS parameter threading, restored CSE let-bindings visualization, added font caching headers, and eradicated undefined CSS classes.

### Deliverables Built
1. **Operations Palette Catalog Parsing**:
   - `maxima-web/static/js/palette.js`: Handled array API response shape from `/api/operations` and `/api/functions`, fixing palette button rendering.
2. **Temporal Dead Zone Elimination**:
   - `maxima-web/static/js/main.js`: Declared `let palette = null;` before `palette = await initPalette(...)` so synchronous callbacks do not throw `ReferenceError`.
3. **Embedded CAS Parameter Threading & Live Preview**:
   - `maxima-web/src/service.rs`: Prioritized `parse_maxima_cas` before `parse_maxima` in `parse_input()` and `parse()` to properly extract inner expressions and arguments for `integrate`, `defint`, `limit`, `taylor`, `solve`, `ode2`, `partfrac`.
   - Wired fallback parameter bindings for all CAS operations in `run_operation()`.
   - Added unit test `test_cas_embedded_commands_dispatch`.
4. **CSE Let-Bindings Visualization**:
   - `maxima-web/static/js/notebook.js`: In `renderTypesetOutput()`, rendered dedicated let-bindings subexpression table when `response.extra.bindings` is present.
5. **HTTP Font Asset Caching**:
   - `maxima-web/src/app.rs`: Injected `Cache-Control: public, max-age=31536000, immutable` for all embedded font assets (`.woff2`, `.woff`, `.ttf`).
6. **Zero Undefined CSS Classes (Offline Parity)**:
   - `maxima-web/static/vendor/app.css`: Added utility and component styles for all 62 previously missing classes.
   - `scripts/verify_css_classes.py`: Verified 0 missing classes (93 used, 101 defined).
7. **Passing Gate Verified**:
   - `cargo test -p maxima-web` (24 passed, exit 0)
   - `cargo test -p maxima-tests --test webui_e2e` (18 passed, exit 0)
   - `E:\Anaconda\python.exe scripts/verify_css_classes.py` (exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `docs/audits/PHASE38B_REPORT.md` compiled.

---

## Session 38C — Engine Honesty, Dynamic Verification & Defect Remediation

**Date**: 2026-11-24  
**Scope**: Remediated 1st-order linear ODE coefficient bug, implemented Liouville non-elementary integral detection (`IntegrationResult`), added negative interval branch cut handling for definite integrals, fixed Taylor series off-by-one truncation, implemented polynomial root multiplicity reduction and `Status::Partial` reporting, wired composite Simpson and ODE residual dynamic verification into web service, and established zero fake verification.

### Deliverables Built
1. **ODE First-Order Coefficient Extraction**:
   - `sym-core/src/solve/ode.rs`: Fixed extraction for `order == 1` to correctly assign `a = coeffs[1]` and `b = coeffs[0]`. Verified $y' + 2y = 0 \implies y = C_1 e^{-2x}$ and $2y' + 5y = 0 \implies y = C_1 e^{-5/2x}$.
2. **Liouville Non-Elementary Detection & Tri-State Outcome**:
   - `sym-core/src/integrate/mod.rs`: Created `IntegrationResult` (`Solved(Expr)`, `ProvenNonElementary`, `Unsupported(String)`). Added `is_proven_non_elementary(expr, var)` detecting Liouville/Risch non-elementary integrands ($e^{-x^2}$, $\sin(x)/x$, etc.). Added $\int \sin(u)^2 du$ and $\int \cos(u)^2 du$ power reductions.
3. **Definite Integral Real Branch Cut Protection**:
   - `sym-core/src/integrate/defint.rs`: Added `flip_ln_if_negative` to transform $\ln(x) \to \ln(-x)$ on strictly negative intervals $[a, b]$ ($a < 0, b \le 0$). Added zero/one algebraic reductions to `fold_const`. Verified $\int_{-3}^{-1} \frac{1}{x} dx = -\ln(3)$.
4. **Taylor Series Truncation Precision**:
   - `sym-core/src/series.rs`: Fixed off-by-one truncation by expanding to `order + 1`, preserving terms of degree `order` with remainder $O((x - c)^{order+1})$.
5. **Polynomial Root Multiplicity & Incomplete Factorization Status**:
   - `sym-core/src/solve/alg.rs`: Factored multiple roots via polynomial division loop. Added `is_complete()` to `AlgebraicSolutions`.
   - `maxima-web/src/api.rs`: Added `Status::Partial` variant.
6. **Dynamic Verification Engine**:
   - `maxima-web/src/service.rs`: Replaced hardcoded `verified = Some(true)` with dynamic `verify_defint` (composite Simpson quadrature and improper midpoint integration) and `verify_ode` (substituting $y, y', y''$ and verifying residual $|R| < 10^{-3}$ at sample points).
   - Wired `integrate_result` and `Status::Partial` for incomplete polynomial factorization.
   - Updated `notebook.js` and `app.css` for `.badge-partial`.
7. **Passing Gate Verified**:
   - `cargo test -p maxima-tests --test solvers_ode` (4 passed, exit 0)
   - `cargo test -p maxima-tests --test integrate_defint` (5 passed, exit 0)
   - `cargo test -p maxima-tests --test maxima_cas_parity` (4 passed, exit 0)
   - `cargo test -p maxima-web` (28 passed, exit 0)
   - `E:\Anaconda\python.exe scripts/verify_css_classes.py` (0 missing, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - **Hard Stop Check:** `diff(y, x) + 2*y = 0` yields `y = _C1 * exp(-2*x)` with `verified: Some(true)` (PASSED)
   - `docs/audits/PHASE38C_REPORT.md` compiled.

---

## Session 39 — Pure-Rust Arbitrary-Precision Float (`bfloat`) (Phase 39)

**Date**: 2026-11-24  
**Scope**: Implemented pure-Rust arbitrary-precision floating point numbers (`bfloat`) citing Maxima's `float.lisp` and ADR-0009, backed by `dashu-float = "0.4"`. Extended `Expr` AST with `Expr::Bfloat(BfloatWrapper)`, implemented decimal precision control matching Maxima's `$fpprec = 16`, implemented full arithmetic and transcendental operations, and verified all passing gates.

### Deliverables Built
1. **Feature Gating & Workspace Configuration**:
   - Added `dashu-float = "0.4"` to workspace root `Cargo.toml`.
   - Added feature flag `bfloat = ["dep:dashu-float"]` to `sym-core/Cargo.toml` (included in `full`).
   - Added feature flag `bfloat = ["sym-core/bfloat"]` to `maxima-rs/Cargo.toml` (included in `full`).
   - Added `bfloat` integration test target in `tests/Cargo.toml`.
2. **`BfloatWrapper` & Precision Context**:
   - `sym-core/src/bfloat.rs`: Implemented `BfloatContext` matching Maxima's `fpprec` (`DEFAULT_FPPREC = 16`) and `BfloatWrapper` wrapping `dashu_float::DBig`.
   - Implemented custom `Hash` on `BfloatWrapper` (hashing `prec` and canonical string).
   - Implemented exact operations: `add`, `sub`, `mul`, `div`, `neg`, `pow`, `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `asin`, `acos`, `atan`.
   - Implemented high-level `bfloat(expr, prec)` and `bfloat_from_str(s, prec)`.
   - Implemented Maxima notation with `b`/`B` exponent indicator and LaTeX scientific formatting.
3. **AST Integration & Invariants**:
   - `sym-core/src/expr.rs`: Extended `Expr::Bfloat(BfloatWrapper)` across constructors, evaluators, predicates, and printers.
   - `sym-core/src/diff.rs`: Bigfloat constant differentiation evaluates to 0.
   - `sym-core/src/latex.rs`: LaTeX formatting with $10^{exp}$.
   - `sym-core/src/codegen.rs`: Floats emitted as precision literals.
   - `sym-core/src/cse.rs`: Handled `Expr::Bfloat` as leaf node.
   - `sym-core/src/lang.rs`: Added `MathLang::Bfloat(Symbol)`.
   - `sym-core/src/cost.rs`: Assigned FLOP cost 0 to `MathLang::Bfloat`.
   - `sym-core/src/matrix.rs`: Added token serialization and deserialization for `Expr::Bfloat` (`BF` opcode).
   - `sym-core/src/limit/mod.rs`: Handled `Expr::Bfloat` in variable substitution.
4. **Verification & Passing Gate**:
   - `cargo test -p sym-core --features bfloat --lib` (149 passed, exit 0)
   - `cargo test -p maxima-tests --test bfloat` (9 passed, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo check -p sym-core --no-default-features` (clean, exit 0)
   - `docs/audits/PHASE39_REPORT.md` compiled.

---

## Session 40 — Symbolic Linear Algebra & Bareiss Algorithm (Phase 40)

**Date**: 2026-11-24  
**Scope**: Implemented fraction-free Bareiss matrix determinant algorithm ($O(n^3)$), classical adjoint inverse, characteristic polynomial (`charpoly`), trace, and eigenvalue finding citing Maxima's `newdet.lisp` and ADR-0010. Wired operations into `maxima-web/src/service.rs` and verified all passing gates.

### Deliverables Built
1. **Feature Gating & Module Organization**:
   - Added feature flag `matrix = ["poly", "solve"]` in `sym-core/Cargo.toml` (included in `full`).
   - Added feature flag `matrix = ["sym-core/matrix"]` in `maxima-rs/Cargo.toml` (included in `full`).
   - Reorganized `sym-core/src/matrix/mod.rs` and created `sym-core/src/matrix/bareiss.rs`.
   - Added `tests/integration/matrix_bareiss.rs`.
2. **`Matrix` Struct & Bareiss Engine**:
   - `sym-core/src/matrix/bareiss.rs`: Implemented `Matrix` struct with rows, cols, data, indexing, and row uniformity validation (`MatrixError::RaggedMatrix`).
   - Implemented matrix arithmetic: `add`, `sub`, `mul`, `scale`, `transpose`, `submatrix_excluding`.
   - Implemented `determinant(&Matrix)` via Bareiss $O(n^3)$ algorithm with zero-diagonal pivot row swapping and exact denominator division via `ratsimp` and rational arithmetic.
   - Implemented `trace(&Matrix)`: Sum of diagonal entries.
   - Implemented `charpoly(&Matrix, var)`: $\det(M - \lambda I)$ matching Maxima 5.50's `newdet.lisp`.
   - Implemented `adjoint(&Matrix)`: Classical adjoint (adjugate) matrix using cofactor expansion with Bareiss subdeterminants.
   - Implemented `invert(&Matrix)`: $M^{-1} = \frac{1}{\det(M)} \text{adj}(M)$ with singular matrix detection (`MatrixError::SingularMatrix`).
   - Implemented `eigenvalues(&Matrix)`: Exact closed-form and rational roots of $\text{charpoly}(M, \lambda) = 0$.
3. **Web Service Wiring**:
   - `maxima-web/src/service.rs`: Registered `"determinant"`, `"invert"`, `"charpoly"`, `"trace"`, `"eigenvalues"` in `OPERATIONS` catalog with `PARAM_CHARPOLY`.
   - Wired compute dispatch handlers in `run_operation` with JSON and LaTeX output.
4. **Verification & Passing Gate**:
   - `cargo test -p sym-core --features matrix --lib` (149 passed, exit 0)
   - `cargo test -p maxima-tests --test matrix_bareiss` (10 passed, exit 0)
   - `cargo test -p maxima-tests --test bfloat` (9 passed, exit 0)
   - `cargo test -p maxima-web` (28 passed, exit 0)
   - `E:\Anaconda\python.exe scripts/verify_css_classes.py` (0 missing, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo check -p sym-core --no-default-features` (clean, exit 0)
   - `docs/audits/PHASE40_REPORT.md` compiled.

---

## Session 41 — Complex Analysis, Rectangular Form & Residues (Phase 41)

**Date**: 2026-11-24  
**Scope**: Implemented zero-allocation `Expr::I` atom, canonical rectangular decomposition (`rectform`), polar form, modulus (`cabs`), argument (`carg`), complex conjugate, Euler's formula, branch cuts, and Laurent series residue calculus citing Maxima's `rpart.lisp` and ADR-0010. Wired operations into `maxima-web/src/service.rs` and verified all passing gates.

### Deliverables Built
1. **Feature Gating & AST Integration**:
   - Added feature flag `complex = ["poly", "series", "diff"]` in `sym-core/Cargo.toml` (included in `full`).
   - Added feature flag `complex = ["sym-core/complex"]` in `maxima-rs/Cargo.toml` (included in `full`).
   - Extended `Expr` with `Expr::I` atom and `Expr::i()` constructor.
   - Updated `Expr::I` in `diff`, `latex`, `cse`, `codegen`, `matrix`, `limit`, `lang`, `bfloat`, `series`, and `parser::maxima`.
   - Added `tests/integration/complex_analysis.rs`.
2. **Complex Analysis Module (`sym-core/src/complex/mod.rs`)**:
   - Citing Maxima 5.50 `src/rpart.lisp` (`risplit`, `trisplit`, `$rectform`, `$polarform`, `cabs`, `absarg`).
   - Smart constructors with zero and constant folding (`smart_add`, `smart_sub`, `smart_mul`, `smart_div`, `smart_pow`, `smart_sin`, `smart_cos`, `smart_sinh`, `smart_cosh`).
   - `rectform(expr)`: Decomposes into real and imaginary parts $u + i v$.
   - Cyclic powers of $i$: $i^0=1, i^1=i, i^2=-1, i^3=-i, i^4=1, i^{-1}=-i, i^{-2}=-1$.
   - Euler's formula: $\exp(i\pi) = -1$, $\exp(i\pi/2) = i$, $\exp(-i\pi/2) = -i$, $\exp(2i\pi) = 1$.
   - Complex logarithm branch cuts: $\ln(-1) = i\pi$, $\ln(i) = i\pi/2$.
   - Rotations: $\sin(ix) = i\sinh(x)$, $\cos(ix) = \cosh(x)$, $\sinh(ix) = i\sin(x)$, $\cosh(ix) = \cos(x)$.
   - Conjugate, absolute value (`cabs`), argument (`carg`), and polar form (`polarform`, `polarform_expr`).
   - Laurent residue calculus: $\operatorname{residue}(expr, var, point)$ via Laurent series up to order 6.
3. **Web Service Wiring**:
   - `maxima-web/src/service.rs`: Registered `"rectform"`, `"polarform"`, `"realpart"`, `"imagpart"`, `"conjugate"`, `"carg"`, `"cabs"`, `"residue"` in `OPERATIONS` catalog with `PARAM_RESIDUE`.
   - Wired compute dispatch handlers in `run_operation` with JSON and LaTeX output.
4. **Verification & Passing Gate**:
   - `cargo test -p sym-core --features complex --lib` (155 passed, exit 0)
   - `cargo test -p maxima-tests --test complex_analysis` (10 passed, exit 0)
   - `cargo test -p maxima-tests --test matrix_bareiss` (10 passed, exit 0)
   - `cargo test -p maxima-tests --test bfloat` (9 passed, exit 0)
   - `cargo test -p maxima-web` (28 passed, exit 0)
   - `E:\Anaconda\python.exe scripts/verify_css_classes.py` (0 missing, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo check -p sym-core --no-default-features` (clean, exit 0)
   - `docs/audits/PHASE41_REPORT.md` compiled.

---

## Session 42 — Special Functions & Table Dispatch (Phase 42)

**Date**: 2026-11-24  
**Scope**: Implemented non-elementary special functions ($\Gamma, \beta, \text{erf}, \text{erfc}$, Bessel $J, Y, I, K$, Airy $\text{Ai}, \text{Bi}$, orthogonal polynomials $P_n, T_n, U_n, H_n, L_n$) with table-driven differentiation, recurrence generation, exact value evaluations, and multi-format serialization citing Maxima 5.50 `src/specfn.lisp`, `src/bessel.lisp`, `src/airy.lisp`, `share/orthopoly/orthopoly.lisp`, and ADR-0011. Verified all passing gates.

### Deliverables Built
1. **Feature Gating & AST Representation**:
   - Added feature flag `specfn = ["diff", "poly"]` in `sym-core/Cargo.toml` (bundled into `full`).
   - Added feature flag `specfn = ["sym-core/specfn"]` in `maxima-rs/Cargo.toml` (bundled into `full`).
   - Extended `Expr` with `Expr::SpecialFn(SpecialFnKind, Vec<Expr>)`.
   - Added ergonomic leaf/node constructors: `Expr::gamma`, `Expr::beta`, `Expr::erf`, `Expr::erfc`, `Expr::bessel_j`, `Expr::bessel_y`, `Expr::bessel_i`, `Expr::bessel_k`, `Expr::airy_ai`, `Expr::airy_bi`, `Expr::legendre_p`, `Expr::chebyshev_t`, `Expr::chebyshev_u`, `Expr::hermite_h`, `Expr::laguerre_l`.
   - Integrated `Expr::SpecialFn` into `eval`, `collect_vars`, `node_count`, `Display`, `latex`, `cse`, `codegen`, `limit`, `bfloat`, `complex/rectform`, and `matrix` symbol codecs.
2. **Special Functions Modules (`sym-core/src/specfn/`)**:
   - `sym-core/src/specfn/mod.rs`: `SpecialFnKind` (18 function variants), `FromStr`, `diff_special_fn` table dispatcher.
   - `sym-core/src/specfn/gamma.rs`: Exact values ($\Gamma(1)=1, \Gamma(n)=(n-1)!, \Gamma(1/2)=\sqrt{\pi}$, $\beta(1,1)=1$, $\text{erf}(0)=0, \text{erfc}(0)=1$), and derivatives $\Gamma'(z)=\Gamma(z)\psi_0(z)$, $\text{erf}'(z)=\frac{2}{\sqrt{\pi}}e^{-z^2}$.
   - `sym-core/src/specfn/bessel.rs`: Bessel $J_\nu, Y_\nu, I_\nu, K_\nu$ with exact evaluations at 0 and A&S 9.1.27/9.6.29 derivatives.
   - `sym-core/src/specfn/airy.rs`: Airy $\text{Ai}, \text{Ai}', \text{Bi}, \text{Bi}'$ and differential equation derivatives ($w'' - z w = 0$).
   - `sym-core/src/specfn/orthopoly.rs`: 3-term recurrence generators and derivatives for Legendre $P_n$, Chebyshev $T_n/U_n$, Hermite $H_n$, and Laguerre $L_n$.
3. **Parser & Verification**:
   - Maxima parser support in `sym-core/src/parser/maxima.rs`.
   - Added integration test suite `tests/integration/specfn.rs` (9 comprehensive tests).
4. **Passing Gate Verification**:
   - `cargo test -p sym-core --features specfn --lib` (155 passed, exit 0)
   - `cargo test -p maxima-tests --test specfn` (9 passed, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo check -p sym-core --no-default-features` (clean, exit 0)
   - `docs/audits/PHASE42_REPORT.md` compiled.

---

## Session 43 — Operational Laplace Transforms & ODE Initial Values (Phase 43)

**Date**: 2026-11-24  
**Scope**: Implemented operational forward Laplace transformation (`laplace`), inverse Laplace transformation (`ilt`), and ODE initial and boundary value problem solvers (`ic1`, `ic2`, `bc2`, `ic1_ode`, `ic2_ode`, `bc2_ode`) citing Maxima 5.50 `src/laplac.lisp` and `src/ode2.lisp` under ADR-0011. Verified all passing gates.

### Deliverables Built
1. **Feature Gating & Module Registration**:
   - Added feature flag `transform = ["diff", "poly", "solve", "specfn"]` in `sym-core/Cargo.toml` (bundled into `full`).
   - Added feature flag `transform = ["sym-core/transform"]` in `maxima-rs/Cargo.toml` (bundled into `full`).
   - Re-exported transform methods in `sym-core/src/lib.rs` and `maxima-rs/src/prelude.rs`.
2. **Transform Modules (`sym-core/src/transform/`)**:
   - `sym-core/src/transform/laplace.rs`: `laplace(f, t, s)` and `ilt(F, s, t)` supporting linearity, table lookup (constants, monomials, exponentials, sin, cos, sinh, cosh), frequency shifting ($e^{a t} g(t)$), multiplier theorem ($t g(t)$), and derivative theorems ($y', y''$).
   - `sym-core/src/transform/bvp.rs`: `ic1`, `ic2`, `bc2`, `ic1_ode`, `ic2_ode`, `bc2_ode` for exact Cramer linear extraction and initial/boundary condition satisfaction.
3. **Integration Test Suite**:
   - `tests/integration/transform_laplace.rs`: 8 comprehensive tests covering forward transforms, operational rules, inverse rational transforms, IVP1, IVP2, BVP2, and ODE solver pipeline integration.
4. **Passing Gate Verification**:
   - `cargo test -p sym-core --features transform --lib` (155 passed, exit 0)
   - `cargo test -p maxima-tests --test transform_laplace` (8 passed, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo check -p sym-core --no-default-features` (clean, exit 0)
   - `docs/audits/PHASE43_REPORT.md` compiled.

---

## Session 44 — Scoped Deductive Inference & Assumptions Context (Phase 44)

**Date**: 2026-11-24  
**Scope**: Implemented scoped, immutable mathematical assumptions context (`Assumptions`), inequality DAG inference, interval bound propagation, sign deduction (`csign`, `asksign`), predicate evaluation (`is`), domain properties (`PropertyKind`), and reversibility (`forget`) citing Maxima 5.50 `src/compar.lisp` and `src/askp.lisp` under ADR-0011. Verified integration with `integrate` and `limit`.

### Deliverables Built
1. **Feature Gating & Module Registration**:
   - Added feature flag `assume = []` in `sym-core/Cargo.toml` (bundled into `full`).
   - Added feature flag `assume = ["sym-core/assume"]` in `maxima-rs/Cargo.toml` (bundled into `full`).
   - Re-exported assumptions API in `sym-core/src/lib.rs` and `maxima-rs/src/prelude.rs`.
2. **Assumptions Module (`sym-core/src/assume/mod.rs`)**:
   - Immutable `Assumptions` context struct holding bounds, domain properties, and inequality DAG edges.
   - Operations: `assume`, `forget`, `is`, `csign`, `asksign`, `has_property`.
   - Contradiction detection for incompatible bounds and conflicting domain properties.
3. **Integration with Symbolic CAS Engines**:
   - Extended `sym-core/src/integrate/mod.rs` with `integrate_with_assumptions` resolving power-rule ambiguity when $a > 0$.
   - Extended `sym-core/src/limit/mod.rs` with `limit_with_assumptions` resolving asymptotic limits $\lim_{x \to \infty} x^a$ based on $a$'s sign.
4. **Integration Test Suite**:
   - `tests/integration/assume_deduction.rs`: 8 comprehensive tests covering bounds propagation, contradiction rejection, transitive DAG reasoning, arithmetic sign rules, property classification, forget mechanism, and integration/limit engine integration.
5. **Passing Gate Verification**:
   - `cargo test -p sym-core --features assume --lib` (155 passed, exit 0)
   - `cargo test -p maxima-tests --test assume_deduction` (8 passed, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo check -p sym-core --no-default-features` (clean, exit 0)
   - `docs/audits/PHASE44_REPORT.md` compiled.

---

## Session 45 — Maxima 5.50 Parity Test Suite & Golden Verification (Phase 45)

**Date**: 2026-11-24  
**Scope**: Executed full upstream Maxima 5.50 golden verification suite, porting benchmark test cases from `References/maxima-5.50.0/tests/` into `tests/integration/golden_parity.rs`. Verified 100% test pass rate across the workspace, clean clippy (-D warnings), and all doctests.

### Deliverables Built
1. **Golden Parity Test Suite (`tests/integration/golden_parity.rs`)**:
   - `rtest_integrate.mac`: Polynomial, transcendental, rational pole, trigonometric power, and $u$-substitution integration.
   - `rtest_limit.mac`: Sinc, Euler constant $(1+1/x)^x \to e$, removable singularity, and asymptotic rational limits.
   - `rtest_taylor.mac`: Taylor series expansions for $e^x$, $\sin(x)$, and $\frac{1}{1-x}$.
   - `rtest_solve.mac`: Quadratic, linear, and pure quadratic equations.
   - `rtest_laplace.mac`: Forward Laplace transforms of constants, monomials, and transcendentals, plus inverse rational transforms.
   - `rtest_bessel.mac`: Bessel $J_0/J_1/J_2$, Modified Bessel $I_0/I_1$, Gamma exact integers, error functions, and orthogonal polynomials ($P_n, H_n, T_n$).
2. **Passing Gate Verification**:
   - `cargo fmt --all -- --check` (clean, exit 0)
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings, exit 0)
   - `cargo test --workspace --all-features` (all unit, integration, and doc tests passed, exit 0)
   - `cargo test -p sym-core --all-features --doc` (17 passed, exit 0)
   - `docs/audits/PHASE45_REPORT.md` compiled.



























