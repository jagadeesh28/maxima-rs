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

### Current Phase: **Phases 1–22 Complete ✅ — Phases 23–33 Planned (CAS Expansion) ⬜**

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
| 23. Exact Rational Algebra & Factorization | `poly` | ⬜ Planned | Subresultant PRS + modular $\mathbb{F}_p$ GCD, `ratsimp`, Yun square-free, Berlekamp/Hensel `factor`, `partfrac` |
| 24. Symbolic Integration: Rational & Elementary | `integrate` | ⬜ Planned | Moses' SIN, Hermite reduction, Lazard-Rioboo-Trager (LRT) real $\ln/\text{atan}$, IBP |
| 25. Transcendental Risch Differential Towers | `integrate` | ⬜ Planned | Bronstein differential field towers for $\exp/\ln$, non-elementary proofs (`Expr::integral` noun) |
| 26. Laurent Series & Asymptotics | `series` | ⬜ Planned | Truncated `Series { center, valuation, coeffs, order }` arithmetic and elementary expansions |
| 27. Gruntz Limits Engine | `limit` | ⬜ Planned | Dominik Gruntz MRV algorithm, $\omega \to 0$ distortion, 3-tier zero-equivalence decider |
| 28. Definite Integration & Pole Scanning | `integrate` | ⬜ Planned | Sturm pole scanning, FTC evaluation, improper limits via Gruntz |
| 29. Closed-Form Algebraic Solvers | `solve` | ⬜ Planned | Degrees 1-4 formulas, factor-and-split, transcendental variable isolation, `AlgebraicSolutions` |
| 30. ODE Solvers | `solve` | ⬜ Planned | 1st-order standard forms (separable, linear, Bernoulli, exact) + 2nd-order constant coeffs (`OdeSolution`) |
| 31. Maxima Parser CAS Syntax & Facade | `cas`/`parser` | ⬜ Planned | Maxima CAS commands parsing (`expand`, `factor`, `ratsimp`, `partfrac`, `integrate`, `limit`, `ode2`), facade & prelude |
| 32. Verification & Maxima Benchmarks | all | ⬜ Planned | Proptests for integration/ODE invariants, Maxima test suite parity (`rtest_integrate.mac`, etc.), fmt/clippy |

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


