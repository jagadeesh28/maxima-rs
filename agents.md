# maxima-rs — Phased Implementation Plan

> A compile-time symbolic mathematics and code optimization engine in Rust.
> Converts legacy Maxima algebraic knowledge into an `egg`-powered e-graph engine
> that generates zero-overhead optimized Rust code via procedural macros.
>
> **Session continuity**: See [`SESSION_LOG.md`](file:///e:/Rust/Max/SESSION_LOG.md) for
> cross-session context, decision history, and phase completion tracker.
> Always read `SESSION_LOG.md` first when resuming work.
>
> **Architecture & Call Graph Reference**: See [`/grafify`](file:///e:/Rust/Max/.agents/skills/grafify/SKILL.md)
> for the comprehensive interdependency matrix and function-level trace across all crates and modules.
>
> **Protocol Rule**: Always update [`README.md`](file:///e:/Rust/Max/README.md) after completing each phase
> to reflect the updated status table, deliverables, and API examples.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                   User Rust Source Code                         │
│   sym_fn! { fn f(x,y) -> ... { expr: ..., gradient: [x,y] } } │
└──────────────────────────────┬──────────────────────────────────┘
                               │ (compile-time)
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  sym-macros  (proc-macro crate) [feature = "macros"]         │
│  ┌────────┐   ┌──────────┐   ┌───────────┐                  │
│  │ parse  │──▶│ validate │──▶│ expand    │                   │
│  │ (syn)  │   │ (spans)  │   │ (codegen) │                   │
│  └────────┘   └──────────┘   └─────┬─────┘                  │
└────────────────────────────────────┼─────────────────────────┘
                                     │ calls sym-core
                                     ▼
┌──────────────────────────────────────────────────────────────┐
│  sym-core  (library crate)                                   │
│                                                              │
│  ┌────────┐   ┌──────┐   ┌─────────┐   ┌──────┐   ┌──────┐ │
│  │  Expr  │──▶│ Diff │──▶│ E-Graph │──▶│ Cost │──▶│ CSE  │ │
│  │  AST   │   │      │   │ + Rules │   │Extract│   │ Pass │ │
│  └────────┘   └──────┘   └─────────┘   └──────┘   └──────┘ │
│    [core]      [diff]     [egraph]     [egraph]    [cse]   │
│       │                      │                               │
│       │              RuleProvenance                          │
│       │         (1:1 Maxima LISP Citations)                  │
│       │                      │                               │
│       │              Rational Constants                      │
│       └──────────────────────┼───────────────────────┘       │
│                              │                               │
│                      ┌───────▼───────┐                       │
│                      │   Codegen     │ [codegen]             │
│                      │ (TokenStream) │                       │
│                      └───────────────┘                       │
│                              ▲                               │
│                      ┌───────┴───────┐                       │
│                      │  Parsers/API  │ [parser]              │
│                      │ Maxima/LaTeX  │                       │
│                      │ + Pipeline    │                       │
│                      └───────────────┘                       │
└──────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  maxima-rs  (facade crate)                                   │
│  pub use sym_core::*;  #[cfg(feature="macros")] pub use ...; │
└──────────────────────────────────────────────────────────────┘
```

### Pipeline Execution Order

```
Input Expression (Rust syntax, or string/LaTeX via parse_maxima / parse_latex)
      │
      ▼
 1. Parse (syn tokenstream or winnow string/LaTeX → Expr AST) [macros / parser]
      │
      ▼
 2. Analytical Differentiation (Expr → derivative Expr)       [diff]
      │
      ▼
 3. Inject all Exprs (value + derivatives) into EGraph        [egraph]
      │ (Rewrite rules tracked with 1:1 Maxima LISP RuleProvenance)
      ▼
 4. Equality Saturation (apply rewrite rules until limits)    [egraph]
      │
      ▼
 5. Cost-Optimal Extraction (FLOP-weighted CostFunction)      [egraph]
      │
      ▼
 6. CSE Post-Pass (identify shared subterms → let bindings)   [cse]
      │
      ▼
 7. Codegen (emit TokenStream → plain fn with bindings)       [codegen]
```

---

## Cargo Feature Architecture & Phase Grouping

The codebase is organized into modular Cargo features so users can pull only the components they need:

| Feature | Default | Corresponds to | Gated Modules & Capabilities | Additional Dependencies Activated |
|---------|---------|----------------|------------------------------|-----------------------------------|
| **`core`** (base) | Always | **Phase 1** | `expr`, `rational`: AST, exact rational arithmetic, evaluation, display | `num-rational`, `num-bigint`, `num-traits` |
| **`diff`** | Yes | **Phase 2** | `diff`: Analytical differentiation, gradient, Jacobian, Hessian | *(None — pure AST calculus)* |
| **`egraph`** | Yes | **Phases 3 & 4** | `lang`, `rules/*`, `cost`, `saturate`, `extract`, `provenance`: Saturation, FLOP extraction, 1:1 Maxima source citations | `egg = "0.9"` (optional) |
| **`cse`** | Yes | **Phase 5** | `cse`: Common subexpression elimination DAG pass | *(None — pure AST DAG)* |
| **`codegen`** | Yes | **Phase 6** | `codegen`: Conversion of expressions to `proc_macro2::TokenStream` | `proc-macro2`, `quote` (optional) |
| **`parser`** | Yes | **Phase 7** | `parser/*`: Multi-format parsers (Maxima string & LaTeX scalar math syntax) | `winnow = "0.7"` (optional) |
| **`macros`** | Yes | **Phases 8 & 9** | Procedural macros `sym_fn!` and `sym_eval!` in `sym-macros` | `syn`, `quote`, `proc-macro2` |
| **`full`** | Yes | **All Phases** | Convenience bundle activating `["diff", "egraph", "cse", "codegen", "parser", "macros"]` | All dependencies |

---

## Complete File Tree (Final State)

```
e:\Rust\Max\
├── Cargo.toml                          # Workspace root
├── GLOSSARY.md                         # Domain vocabulary
├── SESSION_LOG.md                      # Persistent cross-session continuity log
├── agents.md                           # This file — implementation plan
├── References/
│   └── maxima-master.zip               # Maxima Common Lisp reference source
├── docs/
│   └── adr/
│       ├── 0001-egraph-engine-over-lisp-port.md
│       ├── 0002-configurable-fast-math-default.md
│       ├── 0003-hand-curated-rewrite-rules.md (superseded)
│       ├── 0004-rational-constant-representation.md
│       └── 0005-systematic-maxima-rule-coverage.md (active)
│
├── sym-core/                           # Core symbolic engine library
│   ├── Cargo.toml                      # Features: diff, egraph, cse, codegen, parser, full
│   └── src/
│       ├── lib.rs                      # Public API surface, feature-gated re-exports
│       ├── expr.rs                     # [core] Symbolic Expression AST enum
│       ├── rational.rs                 # [core] Rational constant wrapper (BigRational)
│       ├── diff.rs                     # [feature="diff"] Analytical differentiation
│       ├── lang.rs                     # [feature="egraph"] egg Language definition
│       ├── provenance.rs               # [feature="egraph"] RuleProvenance metadata & registry
│       ├── rules/                      # [feature="egraph"] 1:1 Maxima LISP rules with citations
│       │   ├── mod.rs                  # Rule aggregator, fast-math gating, provenance query
│       │   ├── arithmetic.rs           # simp.lisp (simpplus, simptimes, simpexpt)
│       │   ├── trigonometric.rs        # trigi.lisp (sin, cos, tan identities)
│       │   ├── exponential.rs          # logarc.lisp / simp.lisp (exp, pow identities)
│       │   └── logarithmic.rs          # logarc.lisp (ln identities)
│       ├── cost.rs                     # [feature="egraph"] FLOP-weighted CostFunction
│       ├── saturate.rs                 # [feature="egraph"] Runner config, saturation limits
│       ├── extract.rs                  # [feature="egraph"] Extraction + multi-output
│       ├── cse.rs                      # [feature="cse"] Common subexpression elimination
│       ├── codegen.rs                  # [feature="codegen"] Expr → proc_macro2::TokenStream
│       ├── parser/                     # [feature="parser"]
│       │   ├── mod.rs                  # Parser interface & error types
│       │   ├── maxima.rs               # Maxima string syntax parser (winnow)
│       │   └── latex.rs                # LaTeX scalar math formula parser (winnow)
│       └── pipeline.rs                 # Ergonomic high-level API & Pipeline builder
│
├── sym-macros/                         # Procedural macro crate [feature="macros"]
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # #[proc_macro] entry points
│       ├── parse_sym_fn.rs             # syn parser for sym_fn! syntax
│       ├── parse_sym_eval.rs           # syn parser for sym_eval! syntax
│       ├── config.rs                   # Macro attribute parsing (fast_math, float_type, limits)
│       └── expand.rs                   # Orchestrator: parse → diff → saturate → extract → CSE → codegen
│
├── maxima-rs/                          # User-facing facade crate
│   ├── Cargo.toml                      # Features: diff, egraph, cse, codegen, parser, macros, full
│   └── src/
│       ├── lib.rs                      # Re-exports from sym-core + optional sym-macros
│       └── prelude.rs                  # Ergonomic prelude for common usage
│
└── tests/                              # Integration & property tests
    ├── integration/
    │   ├── basic_arithmetic.rs         # Golden tests: arithmetic simplification & provenance
    │   ├── trigonometric.rs            # Golden tests: trig identities & provenance
    │   ├── differentiation.rs          # Golden tests: known derivatives
    │   ├── cse.rs                      # Golden tests: CSE extraction
    │   ├── parser_maxima.rs            # Golden tests: Maxima string parser
    │   ├── parser_latex.rs             # Golden tests: LaTeX formula parser
    │   ├── pipeline_api.rs             # Golden tests: optimize() and Pipeline builder
    │   └── macro_expansion.rs          # Golden tests: sym_fn! / sym_eval! output
    └── proptest/
        ├── numerical_equivalence.rs    # Proptest: eval(original) ≈ eval(optimized)
        └── diff_correctness.rs         # Proptest: symbolic diff vs finite diff
```

---

## Phase 1 — Workspace Skeleton & Expression AST [Feature: Core (Always)]

**Goal**: Establish the Cargo workspace, define the foundational `Expr` AST enum,
and implement `Display` / parsing utilities. Always compiled with zero feature flags.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`Cargo.toml`](file:///e:/Rust/Max/Cargo.toml) | None | ✅ Completed | Workspace manifest |
| [`sym-core/Cargo.toml`](file:///e:/Rust/Max/sym-core/Cargo.toml) | None | ✅ Completed | Core library manifest with features |
| [`sym-core/src/lib.rs`](file:///e:/Rust/Max/sym-core/src/lib.rs) | None | ✅ Completed | Library root |
| [`sym-core/src/expr.rs`](file:///e:/Rust/Max/sym-core/src/expr.rs) | Core | ✅ Completed | Expression AST + unit tests |
| [`sym-core/src/rational.rs`](file:///e:/Rust/Max/sym-core/src/rational.rs) | Core | ✅ Completed | Rational constant utilities + unit tests |

---

## Phase 2 — Analytical Differentiation [Feature: diff]

**Goal**: Implement the deterministic differentiation pre-pass on `Expr`.
Gated under `feature = "diff"`. Requires no external dependencies beyond core AST.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/diff.rs`](file:///e:/Rust/Max/sym-core/src/diff.rs) | `diff` | ✅ Completed | Differentiation engine + unit & doc tests |

---

## Phase 3 — E-Graph Language, 1:1 Maxima Rules & Provenance [Feature: egraph]

**Goal**: Define the `egg` `Language` for mathematical expressions, implement
systematic 1:1 rewrite rules mapped from Maxima Common LISP simplification routines,
and wire up structured `RuleProvenance` metadata with source citations.
Gated under `feature = "egraph"`, activates optional `egg` dependency.

### Maxima LISP Source Mapping Reference

Every rule implemented must cite its corresponding function in Maxima Lisp source (`References/maxima-master.zip`):
- `simp.lisp`: `simpplus`, `simptimes`, `simpexpt` (identities for 0, 1, distribution, power rules)
- `trigi.lisp`: `simp-%sin`, `simp-%cos`, `simp-%tan` (Pythagorean identities, angle sums)
- `combin.lisp`: integer combinations, factorings
- `logarc.lisp`: `sp1log`, `log-exp` cancellations

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/lang.rs`](file:///e:/Rust/Max/sym-core/src/lang.rs) | `egraph` | ✅ Completed | `egg` Language + Analysis for `Expr` |
| [`sym-core/src/provenance.rs`](file:///e:/Rust/Max/sym-core/src/provenance.rs) | `egraph` | ✅ Completed | `RuleProvenance` struct and citation registry |
| [`sym-core/src/rules/mod.rs`](file:///e:/Rust/Max/sym-core/src/rules/mod.rs) | `egraph` | ✅ Completed | Rule aggregator + config + provenance lookup |
| [`sym-core/src/rules/arithmetic.rs`](file:///e:/Rust/Max/sym-core/src/rules/arithmetic.rs) | `egraph` | ✅ Completed | Rules citing `simp.lisp` and `combin.lisp` |
| [`sym-core/src/rules/trigonometric.rs`](file:///e:/Rust/Max/sym-core/src/rules/trigonometric.rs) | `egraph` | ✅ Completed | Rules citing `trigi.lisp` |
| [`sym-core/src/rules/exponential.rs`](file:///e:/Rust/Max/sym-core/src/rules/exponential.rs) | `egraph` | ✅ Completed | Rules citing `logarc.lisp` / `simp.lisp` |
| [`sym-core/src/rules/logarithmic.rs`](file:///e:/Rust/Max/sym-core/src/rules/logarithmic.rs) | `egraph` | ✅ Completed | Rules citing `logarc.lisp` |

---

## Phase 4 — Cost Model, Saturation Runner & Extraction [Feature: egraph]

**Goal**: Implement FLOP-weighted cost extraction, configure `egg::Runner`
with bounded saturation limits, and handle multi-output extraction.
Gated under `feature = "egraph"`.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/cost.rs`](file:///e:/Rust/Max/sym-core/src/cost.rs) | `egraph` | ✅ Completed | FLOP cost function |
| [`sym-core/src/saturate.rs`](file:///e:/Rust/Max/sym-core/src/saturate.rs) | `egraph` | ✅ Completed | Runner + saturation limits |
| [`sym-core/src/extract.rs`](file:///e:/Rust/Max/sym-core/src/extract.rs) | `egraph` | ✅ Completed | Multi-root extraction |

---

## Phase 5 — Common Subexpression Elimination (CSE) [Feature: cse]

**Goal**: Post-extraction pass scanning expressions, identifying shared subterms,
and producing a sequence of `let` bindings. Gated under `feature = "cse"`. Pure AST DAG algorithm.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/cse.rs`](file:///e:/Rust/Max/sym-core/src/cse.rs) | `cse` | ✅ Completed | CSE extraction pass |

---

## Phase 6 — Code Generation (TokenStream) [Feature: codegen]

**Goal**: Convert `CseResult` into `proc_macro2::TokenStream` that represents
a complete, compilable Rust function body.
Gated under `feature = "codegen"`, activates `proc-macro2` and `quote`.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/codegen.rs`](file:///e:/Rust/Max/sym-core/src/codegen.rs) | `codegen` | ✅ Completed | Expr → TokenStream emitter |

---

## Phase 7 — Multi-Format Parsers (Maxima & LaTeX) [Feature: parser]

**Goal**: Implement robust, ergonomic string parsers for both Maxima mathematical syntax
and LaTeX scalar formula notation using `winnow`.
Gated under `feature = "parser"`, activates `winnow`.

### Syntax Scope
- **Maxima string format**: `(sin(x)^2 + cos(x)^2) / 2`, `exp(x) * ln(y)`
- **LaTeX scalar math format**: `\frac{\sin(x)^2 + \cos(x)^2}{2}`, `\sqrt{x + 1}`, `\exp{x}` (Core scalar scope now; extended math mode in future iterations)

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/parser/mod.rs`](file:///e:/Rust/Max/sym-core/src/parser/mod.rs) | `parser` | ✅ Completed | Parser module root and parse error types |
| [`sym-core/src/parser/maxima.rs`](file:///e:/Rust/Max/sym-core/src/parser/maxima.rs) | `parser` | ✅ Completed | Maxima expression string parser (`parse_maxima`) |
| [`sym-core/src/parser/latex.rs`](file:///e:/Rust/Max/sym-core/src/parser/latex.rs) | `parser` | ✅ Completed | LaTeX formula parser (`parse_latex`) |

---

## Phase 8 — High-Level Ergonomic API & Pipeline Builder

**Goal**: Provide friendly one-call free functions (`optimize`, `optimize_gradient`)
and a fluent `Pipeline` builder connecting parsing, calculus, saturation, and CSE.

```rust
// High-level free function
let opt = optimize(&expr, &OptimizeConfig::default())?;

// Fluent Pipeline
let res = Pipeline::new()
    .from_latex(r"\frac{\sin(x)^2 + \cos(x)^2}{x}")?
    .differentiate(&["x"])
    .optimize()?
    .to_rust_fn("eval_df", FloatType::F64)?;
```

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-core/src/pipeline.rs`](file:///e:/Rust/Max/sym-core/src/pipeline.rs) | `full` / conditional | ✅ Completed | `optimize()`, `optimize_gradient()`, and `Pipeline` builder |

---

## Phase 9 — Procedural Macro Crate (`sym-macros`) [Feature: macros]

**Goal**: Compile-time procedural macros `sym_fn!` and `sym_eval!` with span-aware diagnostics.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`sym-macros/Cargo.toml`](file:///e:/Rust/Max/sym-macros/Cargo.toml) | None | ✅ Completed | Macro crate manifest |
| [`sym-macros/src/lib.rs`](file:///e:/Rust/Max/sym-macros/src/lib.rs) | None | ✅ Completed | `sym_fn!` + `sym_eval!` macros |
| [`sym-macros/src/parse_sym_fn.rs`](file:///e:/Rust/Max/sym-macros/src/parse_sym_fn.rs) | None | ✅ Completed | `sym_fn!` syn parser |
| [`sym-macros/src/parse_sym_eval.rs`](file:///e:/Rust/Max/sym-macros/src/parse_sym_eval.rs) | None | ✅ Completed | `sym_eval!` syn parser |
| [`sym-macros/src/config.rs`](file:///e:/Rust/Max/sym-macros/src/config.rs) | None | ✅ Completed | Attribute config parser |
| [`sym-macros/src/expand.rs`](file:///e:/Rust/Max/sym-macros/src/expand.rs) | None | ✅ Completed | Pipeline orchestrator |

---

## Phase 10 — Facade Crate (`maxima-rs`) & Prelude

**Goal**: User-facing facade crate with feature forwarding and ergonomic prelude (`maxima_rs::prelude::*`).

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| [`maxima-rs/Cargo.toml`](file:///e:/Rust/Max/maxima-rs/Cargo.toml) | None | ✅ Completed | Facade crate manifest |
| [`maxima-rs/src/lib.rs`](file:///e:/Rust/Max/maxima-rs/src/lib.rs) | None | ✅ Completed | Re-exports + documentation |
| [`maxima-rs/src/prelude.rs`](file:///e:/Rust/Max/maxima-rs/src/prelude.rs) | None | ✅ Completed | Convenient imports and helpers |

---

## Phase 11 — Testing Infrastructure & Maxima Coverage Verification

**Goal**: Comprehensive golden regression suite, proptest property-based tests,
and an automated check comparing our `RuleProvenance` registry against Maxima LISP sources.

### Deliverables & Status

| File | Feature Requirements | Purpose |
|------|----------------------|---------|
| [`tests/integration/basic_arithmetic.rs`](file:///e:/Rust/Max/tests/integration/basic_arithmetic.rs) | `egraph` | Arithmetic simplification golden tests (Passed) |
| [`tests/integration/trigonometric.rs`](file:///e:/Rust/Max/tests/integration/trigonometric.rs) | `egraph` | Trig golden tests (Passed) |
| [`tests/integration/differentiation.rs`](file:///e:/Rust/Max/tests/integration/differentiation.rs) | `diff`, `egraph` | Differentiation golden tests (Passed) |
| [`tests/integration/cse.rs`](file:///e:/Rust/Max/tests/integration/cse.rs) | `cse` | CSE golden tests (Passed) |
| [`tests/integration/parser_maxima.rs`](file:///e:/Rust/Max/tests/integration/parser_maxima.rs) | `parser` | Maxima string parsing golden tests (Passed) |
| [`tests/integration/parser_latex.rs`](file:///e:/Rust/Max/tests/integration/parser_latex.rs) | `parser` | LaTeX formula parsing golden tests (Passed) |
| [`tests/integration/macro_expansion.rs`](file:///e:/Rust/Max/tests/integration/macro_expansion.rs) | `macros` | Macro expansion tests (Passed) |
| [`tests/integration/maxima_coverage.rs`](file:///e:/Rust/Max/tests/integration/maxima_coverage.rs) | `egraph` | Maxima 1:1 rule citation coverage verification (52 rules mapped, Passed) |
| [`tests/proptest/numerical_equivalence.rs`](file:///e:/Rust/Max/tests/proptest/numerical_equivalence.rs) | `egraph` | Proptest: numerical equivalence (Passed) |
| [`tests/proptest/diff_correctness.rs`](file:///e:/Rust/Max/tests/proptest/diff_correctness.rs) | `diff` | Proptest: symbolic diff vs finite diff (Passed) |

---

## Phase 12 — Ergonomics, Polish & Documentation

**Goal**: Final transparency polish, span errors, benchmark comparisons, and Maxima 1:1 rule coverage reports. All workspace tests, clippy checks, and formatting verified.

- **Status**: ✅ Completed

---

## Phase 13 — Advanced Math & Calculus [Feature: Core/Parser]

**Goal**: Extend the core engine and LaTeX parser to support advanced mathematical constructs including calculus (derivatives, integrals, limits/sums), matrices, and basic tensor notation (subscripts/superscripts), maintaining the 1:1 Maxima rules engine compatibility.

### Deliverables & Status

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/tensor.rs` | Core | ✅ Completed | `Index` type, `IndexType` (Covariant/Contravariant) |
| `sym-core/src/expr.rs` | Core | ✅ Completed | Added `Tensor`, `Matrix`, `Derivative`, `Integral`, `Sum` to AST enum |
| `sym-core/src/lang.rs` | `egraph` | ✅ Completed | `egg` `MathLang` updates for new AST nodes (Matrix opaque serialization) |
| `sym-core/src/diff.rs` | `diff` | ✅ Completed | Matrix element-wise differentiation, nested unevaluated derivatives |
| `sym-core/src/parser/latex.rs` | `parser` | ✅ Completed | Extended `winnow` combinators to parse `\int`, `\sum`, `\begin{pmatrix}`, `\frac{\partial f}{\partial x}`, and Greek tensor notation |
| `tests/integration/parser_latex.rs`| `parser` | ✅ Completed | Golden tests for advanced math syntax |

---

# Roadmap — Phases 14–21: WebUI Restoration & 1:1 Maxima Parity

> **Status legend**: ⬜ Not started · 🟨 In progress · ✅ Completed
>
> **Protocol**: This plan was committed *before* implementation. Each phase is
> implemented, verified (`cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`,
> `cargo test`), documented in `README.md` + `SESSION_LOG.md`, and committed individually.

## Workstream A — 1:1 Maxima Source Parity

| Gap | Current State | Target |
|-----|---------------|--------|
| Rule coverage | 52 rules across `simp.lisp`, `trigi.lisp`, `combin.lisp`, `logarc.lisp` | Add `sinh.lisp`, `nrat4.lisp`, `sfacts.lisp`, `matrix.lisp` citations + deeper `trigi.lisp` |
| AST surface | `sin/cos/tan/exp/ln/sqrt` only | Add `sinh/cosh/tanh`, `asin/acos/atan`, `%pi`, `%e`, factorial |
| Maxima parser | Arithmetic + 6 functions | Constants, factorial, hyperbolic/inverse trig, calculus & matrix syntax |
| Matrix in e-graph | Flattened to `"matrix_unsupported_in_egraph"` (data loss) | Structured encode/decode round-trip |
| Provenance visibility | Engine-only (`all_rule_provenances`) | Exposed through `GET /api/rules` + WebUI panel |

## Workstream B — WebUI Improvements

| Gap | Current State | Target |
|-----|---------------|--------|
| Layout | Stray `<main>` after `</html>`; panes stack vertically | Correct flex-row two-pane layout |
| Script integrity | `copyText()` missing closing brace; copy buttons unwired | Valid script + working copy buttons |
| API contract | README says `optimize`, service only accepts `simplify` | Both accepted, docs aligned |
| Provenance UI | Absent | Rules/coverage panel with per-file counts |

---

## Phase 14 — WebUI Correctness & Layout Restoration

**Goal**: Make the existing `maxima-web` UI run, lay out, and copy correctly, and align the API contract.

### Requirements
- `maxima-web/static/index.html`: replace the stray `<main class="flex-1 flex overflow-hidden">`
  emitted after `</html>` with a correct wrapper enclosing the input (`.w-2/5`) and
  output (`.flex-1`) `<section>` panes.
- `maxima-web/static/app.js`: close `copyText()`; bind `copyResultBtn` and `copyCodeBtn`;
  guard clipboard errors.
- `maxima-web/src/service.rs`: accept both `"simplify"` and `"optimize"` operation ids.
- `maxima-web/src/main.rs`: keep `/api/operations` authoritative for the frontend.
- Add `#[cfg(test)]` coverage for `service::compute` (simplify, differentiate, evaluate).
- Update `README.md` API examples to use the canonical `simplify` id.

### Acceptance Criteria
- `cargo build -p maxima-web` and `cargo test -p maxima-web` succeed.
- Two-pane layout renders side-by-side; copy buttons copy raw result / Rust code.
- Both `simplify` and `optimize` requests return successful responses.

**Status**: ✅ Completed — `index.html` `<main>` wrapper restored, `app.js` passes `node --check`, copy buttons wired, 5 service tests added.

---

## Phase 15 — Advanced AST Nodes & Full-Pipeline Plumbing

**Goal**: Introduce the elementary functions and constants required for parity, threaded through every pipeline pass.

### Requirements
- `Expr` variants + constructors: `Sinh`, `Cosh`, `Tanh`, `Asin`, `Acos`, `Atan`, `Pi`, `E`, `Factorial(Box<Expr>)`.
- Update `Display`, `eval`, `variables`, `node_count` in `expr.rs`.
- Derivatives in `diff.rs` (incl. chain rule); `Pi`/`E` constants differentiate to 0.
- `lang.rs`: add `MathLang` nodes, `MathAnalysis` folding, `to_recexpr`/`from_recexpr` round-trip.
- `cost.rs`: FLOP weights (hyperbolic/inverse trig = transcendentals; factorial = iterative cost).
- `cse.rs`: traversal + substitution for new nodes.
- `codegen.rs`: emit `.sinh()`, `.cosh()`, `.tanh()`, `.asin()`, `.acos()`, `.atan()`, `PI`/`E`, factorial closure.
- `maxima-web/src/service.rs`: `latex()` rendering for the new nodes.
- Unit tests in every touched module.

### Acceptance Criteria
- `cargo test --workspace` green; round-trip tests for all new variants.
- `--no-default-features` and `--all-features` both build.

**Status**: ✅ Completed — 9 new `Expr` variants, derivatives, `MathLang` round-trip, FLOP weights, CSE traversal, codegen, macros, and web LaTeX rendering all covered; `cargo test --workspace --all-features` green (94 sym-core unit tests).

---

## Phase 16 — Maxima String Parser Parity

**Goal**: Bring `parse_maxima` to feature parity with `parse_latex`, including the new nodes.

### Requirements
- Parse `%pi`, `%e`, factorial `!` (postfix, tighter than `*`), `sinh/cosh/tanh`, `asin/acos/atan`.
- Parse Maxima calculus/matrix forms: `diff(f,x)`, `integrate(f,x)`, `sum(f,i,lo,hi)`, `matrix([a,b],[c,d])`.
- Keep `%`-prefixed constants from being treated as variables; support `log` as `ln`.
- Extend `tests/integration/parser_maxima.rs` golden tests; add round-trip tests (`parse → Display`).

### Acceptance Criteria
- New golden tests pass; symmetrical constructs parse in both parsers.

**Status**: ✅ Completed — `%pi`/`%e`/other `%`-symbols, postfix `!` (tighter than `^`), hyperbolic/inverse-trig calls, `diff`/`integrate`/`sum`, `matrix([..],[..])`; 8 integration tests + parser unit tests pass.

---

## Phase 17 — 1:1 Rule Expansion I: Hyperbolic, Inverse Trig, Deeper Trig

**Goal**: Add provenanced rewrite rules for new functions and deepen `trigi.lisp` coverage.

### Requirements
- New `sym-core/src/rules/hyperbolic.rs` citing `sinh.lisp` (`simp-%sinh`, `simp-%cosh`, `simp-%tanh`)
  and `trigi.lisp` (`simp-%asin`, `simp-%acos`, `simp-%atan`): zero/parity/inverse-cancellation rules.
- Extend `rules/trigonometric.rs`: angle-sum, double-angle, `1 + tan² = sec²`.
- Register in `rules/mod.rs`; update `tests/integration/maxima_coverage.rs` expected counts.
- Every new rule carries a non-empty `RuleProvenance`.

### Acceptance Criteria
- `maxima_coverage` passes with the increased count; provenance invariants hold.

**Status**: ✅ Completed — 27 new rules (19 hyperbolic/inverse-trig citing `sinh.lisp`, 8 deeper `trigi.lisp` identities); registry now 79 provenanced rules; `RuleConfig` gained a `hyperbolic` switch.

---

## Phase 18 — 1:1 Rule Expansion II: Rational Normalization & Factorial

**Goal**: Add `ratsimp`-style rational normalization and factorial simplification.

### Requirements
- New `sym-core/src/rules/rational.rs` citing `nrat4.lisp` (`$ratsimp`/`$nrat`):
  common-denominator merges, `(- a)/(- b) -> a/b`, `1/(1/x) -> x`, fraction addition.
- Factorial rules citing `sfacts.lisp` (`factcomb`): `0! -> 1`, `(n+1)! -> (n+1)*n!`, `n!/(n-1)! -> n`.
- Register + update coverage test.

### Acceptance Criteria
- Golden simplification tests pass; coverage count updated; provenance intact.

**Status**: ✅ Completed — `rules/rational.rs` (8 rules, `nrat4.lisp`) and `rules/factorial.rs` (5 rules, `sfacts.lisp`); registry now 92 provenanced rules; new `rational_factorial` golden suite.

---

## Phase 19 — E-Graph Fidelity for Matrices

**Goal**: Preserve matrix contents through saturation/extraction instead of collapsing to a sentinel.

### Requirements
- Add structured `encode_matrix_symbol`/`decode_matrix_symbol` (mirroring `tensor.rs`).
- `lang.rs`: `to_recexpr`/`from_recexpr` use the structured encoding; remove
  `"matrix_unsupported_in_egraph"`.
- Add `matrix.lisp`-cited element-wise `+` / scalar `*` rules where the single-symbol
  representation permits (documented limitations otherwise).
- Round-trip test: `Matrix → saturate → extract` preserves shape and entries.

### Acceptance Criteria
- Matrix round-trip test passes; no sentinel string remains in `sym-core`.

---

## Phase 20 — WebUI Provenance & Maxima Coverage Surfacing

**Goal**: Surface the engine's Maxima parity data in the UI and API.

### Requirements
- New `GET /api/rules` returning `{ total, by_file: [{file, count}], rules: [{name, source_file, source_fn, identity, fast_math_only}] }`.
- Add `provenance` operation metadata to `/api/operations`.
- Frontend: a "Maxima coverage" panel (per-file counts table + rule list), new example chips
  for `sinh(x)`, `atan(x)`, `5!`.
- Tests for the rules payload shape.

### Acceptance Criteria
- `cargo test -p maxima-web` green; panel renders counts; layout remains correct.

---

## Phase 21 — Documentation, Coverage Report & Final Verification

**Goal**: Final polish and green-workspace verification.

### Requirements
- `GLOSSARY.md`: add hyperbolic/inverse-trig/factorial/rational-normalization terms.
- `README.md`: status table → Phase 21 complete, updated rule counts/files, `/api/rules` docs.
- `SESSION_LOG.md`: Sessions for Phases 14–21.
- Final `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace`.

### Acceptance Criteria
- Zero clippy warnings, formatted, all tests pass, docs consistent with coverage numbers.

---

## Phase 14–21 Deliverables Summary

| Phase | Primary Files | Feature Gates |
|-------|---------------|---------------|
| 14 | `maxima-web/static/index.html`, `maxima-web/static/app.js`, `maxima-web/src/service.rs` | — |
| 15 | `sym-core/src/{expr,diff,lang,cost,cse,codegen}.rs` | `core`,`diff`,`egraph`,`cse`,`codegen` |
| 16 | `sym-core/src/parser/maxima.rs`, `tests/integration/parser_maxima.rs` | `parser` |
| 17 | `sym-core/src/rules/{hyperbolic.rs,trigonometric.rs,mod.rs}` | `egraph` |
| 18 | `sym-core/src/rules/{rational.rs,factorial.rs,mod.rs}` | `egraph` |
| 19 | `sym-core/src/{matrix.rs,lang.rs}`, `sym-core/src/tensor.rs` | `core`,`egraph` |
| 20 | `maxima-web/src/{api.rs,main.rs}`, `maxima-web/static/*` | — |
| 21 | `README.md`, `AGENTS.md`, `GLOSSARY.md`, `SESSION_LOG.md` | — |

