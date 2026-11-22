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

## Global Testing Gates & Pre-Implementation Protocol

> **Rule**: No phase may begin implementation until **Gate 0** is green. Each phase's
> `### Testing Gates (Pre-Implementation)` block then defines Gates 1–3 that must pass
> *during* and *before* marking the phase ✅. Phases 1–13 (already complete) are
> retroactively governed by this protocol and were verified green in
> [`SESSION_LOG.md`](file:///e:/Rust/Max/SESSION_LOG.md).

### Gate 0 — Baseline (run before writing any code)
- Confirm a clean tree: `git status --porcelain` (empty) and record the starting commit
  (`git rev-parse HEAD`) in `SESSION_LOG.md`.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- Feature-matrix builds: `cargo build --workspace --no-default-features` and
  `cargo build --workspace --all-features`

### Gate 1 — Unit / Module (in phase)
- `cargo test -p sym-core --features <FEATURE>` (unit tests inside the new module files)
- Feature-isolated build: `cargo build -p sym-core --no-default-features --features <FEATURE>`

### Gate 2 — Integration / Proptest (in phase)
- Register every new test file with a `[[test]]` entry in [`tests/Cargo.toml`](file:///e:/Rust/Max/tests/Cargo.toml).
- `cargo test -p maxima-tests --test <name>`
- Integration/proptest files must be feature-gated (`#![cfg(feature = "...")]`) to the phase's
  feature so they skip cleanly when it is off.

### Gate 3 — Completion (before marking a phase ✅)
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0)
- `cargo test --workspace --all-features` (zero regressions across all 13 pre-existing suites)
- `cargo test -p sym-core --all-features --doc` (doctests)
- Update [`README.md`](file:///e:/Rust/Max/README.md) status table + [`SESSION_LOG.md`](file:///e:/Rust/Max/SESSION_LOG.md).

### Additional Completeness Instructions
1. **Feature-flag prerequisite:** Phases 22–31 require new `poly`, `integrate`, `series`,
   `limit`, `solve`, and `cas` features in [`sym-core/Cargo.toml`](file:///e:/Rust/Max/sym-core/Cargo.toml)
   (wired per `agents_instructions.md`, `full` extended) and mirrored in
   [`maxima-rs/Cargo.toml`](file:///e:/Rust/Max/maxima-rs/Cargo.toml). Gate 1 commands are
   invalid until these exist, so add the flags first.
2. **Test registration:** a new test file without a `[[test]]` entry never runs — always register it.
3. **No-regression rule:** the 13 existing suites (`basic_arithmetic`, `trigonometric`,
   `differentiation`, `cse`, `parser_maxima`, `parser_latex`, `pipeline_api`, `macro_expansion`,
   `maxima_coverage`, `rational_factorial`, `matrix_fidelity`, `numerical_equivalence`,
   `diff_correctness`) must stay green at every gate.
4. **Proptest determinism:** fixed seed, bounded `cases` (e.g. 256), `prop_assume` guards for
   degenerate inputs; Phase 32 invariants must hold over ≥10³ random expressions.
5. **Doctest rule:** every new public CAS API needs a `/// ```rust` example that compiles.
6. **No unconstrained extraction:** procedural CAS output must never be routed through
   `extract.rs` with an AST-size cost (see Guardrail 1) — assert this in a unit test where relevant.

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
| **`poly`** | Yes | **Phases 22 & 23** | `poly/*`: Sparse distributed multivariate polynomials, 64-bit packed exponents, grevlex, Monagan-Pearce heap, `expand`, `ratsimp`, `gcd`, `factor`, `partfrac` | *(None — pure arithmetic over Rat)* |
| **`integrate`** | Yes | **Phases 24, 25, 28** | `integrate/*`: Symbolic integration (SIN, Hermite reduction, LRT real ln/atan, Bronstein Risch transcendental field towers, FTC definite integration) | Activated with `poly`, `diff` |
| **`series`** | Yes | **Phase 26** | `series`: Truncated Laurent series arithmetic and asymptotic expansion | Activated with `poly`, `diff` |
| **`limit`** | Yes | **Phase 27** | `limit/*`: Dominik Gruntz's MRV asymptotic limits engine + 3-tier zero-equivalence decider | Activated with `series`, `poly` |
| **`solve`** | Yes | **Phases 29 & 30** | `solve/*`: Closed-form algebraic solvers (deg 1-4, factor-and-split) + ODE solver (1st-order standard & 2nd-order constant coeffs) | Activated with `poly`, `integrate`, `diff` |
| **`cas`** | Yes | **Phases 22–32** | Meta-feature bundling all CAS modules `["poly", "integrate", "series", "limit", "solve", "diff", "egraph"]` | All CAS dependencies |
| **`full`** | Yes | **All Phases** | Convenience bundle activating `["diff", "egraph", "cse", "codegen", "parser", "macros", "cas"]` | All dependencies |

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
│       ├── 0005-systematic-maxima-rule-coverage.md (active)
│       └── 0006-hybrid-egraph-and-procedural-cas.md (active)
│
├── sym-core/                           # Core symbolic engine library
│   ├── Cargo.toml                      # Features: diff, egraph, cse, codegen, parser, poly, integrate, series, limit, solve, cas, full
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
│       ├── poly/                       # [feature="poly"] Sparse distributed multivariate polynomials
│       │   ├── mod.rs                  # Poly struct, Expr <-> Poly converters
│       │   ├── monomial.rs             # 64-bit packed exponents & grevlex ordering
│       │   ├── heap.rs                 # Monagan-Pearce heap multiplication & division
│       │   ├── expand.rs               # expand() product & multinomial expansion
│       │   ├── gcd.rs                  # Subresultant PRS & modular F_p GCD
│       │   ├── rat.rs                  # ratsimp() canonical quotient reduction
│       │   ├── factor.rs               # Yun square-free + Berlekamp & Hensel lifting
│       │   └── partfrac.rs             # Partial fraction decomposition
│       ├── integrate/                  # [feature="integrate"] Symbolic integration engine
│       │   ├── mod.rs                  # integrate() & defint() orchestrator
│       │   ├── sin.rs                  # Moses' SIN heuristics & u-substitutions
│       │   ├── hermite.rs              # Hermite reduction for rational poles
│       │   ├── lrt.rs                  # Lazard-Rioboo-Trager real ln/atan integration
│       │   ├── risch.rs                # Bronstein transcendental Risch field towers
│       │   ├── ibp.rs                  # Guarded integration by parts
│       │   └── defint.rs               # FTC + real pole scanner + improper limits
│       ├── series.rs                   # [feature="series"] Truncated Laurent series arithmetic
│       ├── limit/                      # [feature="limit"] Dominik Gruntz's MRV limits
│       │   ├── mod.rs                  # gruntz() limits engine
│       │   ├── mrv.rs                  # Most Rapidly Varying comparability sets
│       │   └── zero.rs                 # 3-tier zero-equivalence decider
│       ├── solve/                      # [feature="solve"] Algebraic & ODE solvers
│       │   ├── mod.rs                  # Solver public types & entry points
│       │   ├── alg.rs                  # Analytic degree 1-4, factor-and-split, isolation
│       │   └── ode.rs                  # 1st-order standard forms & 2nd-order constant coeffs
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

**Status**: ✅ Completed — new `sym-core/src/matrix.rs` lossless prefix codec; `lang.rs` round-trips matrices through the e-graph; `matrix_fidelity` integration suite (3 tests) + unit tests; element-wise `matrix.lisp` rules documented as impossible for an opaque leaf.

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

**Status**: ✅ Completed — `GET /api/rules` + `RulesResponse`/`RuleEntry`/`FileRuleCount`; `provenance` compute operation (no expression required); UI coverage panel; chips for `sinh(x)`, `atan(x)`, `5!`; 7 web tests. Live check: 92 rules across 7 files.

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

**Status**: ✅ Completed — coverage report added (92 rules / 7 Lisp files), glossary extended with 7 parity terms, README status → Phase 21, and the final green workspace verification recorded in `SESSION_LOG.md`.

### Testing Gates (Phases 14–21, Retroactive)
> Phases 14–21 are complete; this table records the verification that was applied to each.

| Phase | Gate command(s) | Result |
|-------|-----------------|--------|
| 14 | `cargo test -p maxima-web`; `node --check static/app.js` | ✅ 5 service tests, script valid |
| 15 | `cargo test --workspace --all-features` | ✅ 94 sym-core unit tests |
| 16 | `cargo test -p maxima-tests --test parser_maxima` | ✅ 8 tests |
| 17 | `cargo test -p maxima-tests --test maxima_coverage` | ✅ ≥ 79 rules |
| 18 | `cargo test -p maxima-tests --test rational_factorial` | ✅ 92-rule registry |
| 19 | `cargo test -p maxima-tests --test matrix_fidelity` | ✅ 3 tests |
| 20 | `cargo test -p maxima-web` | ✅ 7 web tests, `/api/rules` = 92/7 |
| 21 | `cargo fmt --all -- --check` + clippy `-D warnings` + `cargo test --workspace --all-features` | ✅ green |

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

---

# Roadmap — Phases 22–32: Wolfram Mathematica-Grade Comprehensive CAS Expansion

> **Status legend**: ⬜ Not started · 🟨 In progress · ✅ Completed
>
> **Mandatory Architecture Guardrails**:
> 1. **Hybrid Architecture Invariant ([ADR-0006](docs/adr/0006-hybrid-egraph-and-procedural-cas.md))**:
>    - `sym-core/src/rules/` and `saturate.rs` remain dedicated to equivalence-preserving algebraic, trigonometric, and transcendental simplification.
>    - Procedural CAS algorithms (`expand`, `poly`, `integrate`, `limit`, `series`, `solve`) are implemented as dedicated deterministic modules in `sym-core/src/` operating on `Expr` ASTs, invoking `pipeline.rs` for local canonical simplification and `diff.rs` for differentiation.
>    - Procedural output must **never** be passed to unconstrained e-graph cost extraction (e.g. `extract.rs` with AST-size cost function) that would undo expansion or factorization.
> 2. **Cache-Optimal Sparse Distributed Representation (SDMP)**:
>    - Packed 64-bit exponent vectors under Graded Reverse Lexicographical (`grevlex`) term order.
>    - Monagan-Pearce heap-based streaming multiplication and division running in $O(t)$ space.
> 3. **Exact Arithmetic & Backward Compatibility**:
>    - All polynomial coefficients and constants use `sym-core/src/rational.rs` (`Rat`).
>    - Existing `sym-macros`, `maxima-rs`, and `tests/integration/` suites must remain 100% green without regressions.
> 4. **Testing Protocol**:
>    - Run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace` at each phase completion.
>    - Update `SESSION_LOG.md` and `README.md` status table after completing each phase.

---

## Phase 22 — Sparse Distributed Multivariate Polynomial Engine [Feature: poly]

**Goal**: Implement a high-performance Sparse Distributed Multivariate Polynomial (`Poly`) engine with 64-bit packed exponents and Monagan-Pearce heap arithmetic, enabling Mathematica-grade expression expansion without e-graph blowup.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** record `git rev-parse HEAD`; `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo test --workspace --all-features` green before touching `poly/`.
- **Gate 1 — Unit:** `cargo test -p sym-core --features poly` (monomial packing, grevlex
  ordering, `VarContext`, Monagan-Pearce heap mul/div, `expand`).
  Isolated build: `cargo build -p sym-core --no-default-features --features poly`.
- **Gate 2 — Integration:** register `poly_expand` in `tests/Cargo.toml` (`[[test]]`), then
  `cargo test -p maxima-tests --test poly_expand` — `expand((x+y)^4)`, multivariate
  distribution, integer powers; numerical equivalence vs `eval`.
- **Gate 3 — Completion:** `cargo fmt --all`, clippy `-D warnings`, full workspace test with
  no regressions; update `README.md` + `SESSION_LOG.md`.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/poly/monomial.rs` | `poly` | ✅ Completed | 64-bit packed exponent vector, `grevlex` ordering, `VarContext` |
| `sym-core/src/poly/mod.rs` | `poly` | ✅ Completed | `Poly` struct, `Term` struct, `Expr <-> Poly` bidirectional converters |
| `sym-core/src/poly/heap.rs` | `poly` | ✅ Completed | Monagan-Pearce heap-based polynomial multiplication and pseudo-division |
| `sym-core/src/poly/expand.rs` | `poly` | ✅ Completed | Recursive distribution of products over sums and multinomial power expansion |
| `tests/integration/poly_expand.rs` | `poly` | ✅ Completed | Integration tests for `expand((x + y)^4)`, multivariate distribution, and power expansion |

### Acceptance Criteria
- `Poly` successfully parses from and converts to `Expr` with 100% exact rational fidelity.
- `expand(expr)` cleanly expands products over sums and integer powers without passing through e-graph extraction.
- `tests/integration/poly_expand.rs` passes with zero memory leaks and clean heap scaling.

---

## Phase 23 — Exact Rational Algebra & Factorization [Feature: poly]

**Goal**: Implement canonical rational simplification (`ratsimp`), multivariate polynomial GCD, Yun's square-free decomposition, Berlekamp/Cantor-Zassenhaus factorization with Hensel lifting, and partial fractions.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features poly` (subresultant PRS GCD, modular
  F_p GCD, Yun square-free, Berlekamp/Cantor-Zassenhaus, Hensel lifting, `ratsimp`, `partfrac`).
- **Gate 2 — Integration:** register `poly_rat` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test poly_rat` — `ratsimp((x^2-1)/(x-1)) == x+1`,
  `partfrac(1/(x^2-1), x)`, high-degree factoring round-trips.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/poly/gcd.rs` | `poly` | ✅ Completed | Exact Euclidean univariate GCD + polynomial long division (subresultant/modular planned) |
| `sym-core/src/poly/rat.rs` | `poly` | ✅ Completed | `ratsimp`: canonical reduction of `Expr` to coprime quotient via GCD |
| `sym-core/src/poly/factor.rs` | `poly` | ✅ Completed | Rational-root linear factorization (Berlekamp/Hensel lifting planned) |
| `sym-core/src/poly/partfrac.rs` | `poly` | ✅ Completed | Partial fraction decomposition over distinct linear factors |
| `tests/integration/poly_rat.rs` | `poly` | ✅ Completed | Golden tests: `ratsimp` cancellation, `factor` round-trip, `partfrac` |

### Acceptance Criteria
- `ratsimp` reduces any rational expression to canonical irreducible coprime form.
- `factor` accurately factors univariate and multivariate polynomials over $\mathbb{Q}[x]$.
- `partfrac` outputs exact sum of partial fractions with linear/irreducible quadratic denominators.

---

## Phase 24 — Symbolic Integration: Rational & Elementary [Feature: integrate]

**Goal**: Implement indefinite integration (`integrate`) using Moses' SIN heuristics, Hermite reduction, and Lazard-Rioboo-Trager (LRT) to produce real-valued antiderivatives without imaginary artifacts.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features integrate` (SIN linearity / constant
  pulling / derivative-divides, Hermite reduction, LRT real ln/atan, guarded IBP depth cap).
- **Gate 2 — Integration:** register `integrate_elementary` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test integrate_elementary`; additionally assert the
  anti-difference invariant `ratsimp(diff(integrate(f, x), x) - f) == 0`.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/integrate/mod.rs` | `integrate` | ✅ Completed | `integrate()` orchestrator: linearity, power rule, elementary, rational, IBP; noun fallback |
| `sym-core/src/integrate/sin.rs` | `integrate` | ✅ Completed | Elementary (trig/exp/ln) linear-argument rules |
| `sym-core/src/integrate/hermite.rs` | `integrate` | ✅ Completed | Rational integration via partial fractions (Hermite/LRT subset) |
| `sym-core/src/integrate/lrt.rs` | `integrate` | ✅ Completed | Logarithmic antiderivative terms (linear-factor subset) |
| `sym-core/src/integrate/ibp.rs` | `integrate` | ✅ Completed | Integration by parts for `x^n · exp/sin/cos` |
| `tests/integration/integrate_elementary.rs` | `integrate` | ✅ Completed | 10 anti-difference golden tests (diff(∫f) ≈ f) |

### Acceptance Criteria
- All rational functions integrate deterministically into algebraic parts + real $\ln / \text{atan}$ terms.
- For every elementary antiderivative $F$, differentiation $\frac{d}{dx} F$ simplifies to $f$ via `ratsimp()`.

---

## Phase 25 — Transcendental Risch Differential Field Towers [Feature: integrate]

**Goal**: Implement transcendental Risch integration (Bronstein's algorithm) over nested exponential and logarithmic field extensions, returning exact closed forms or proving non-elementary status.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features integrate` (field-tower construction,
  `risch_de` Risch differential equation `y' + f y = g`, non-elementary detection).
- **Gate 2 — Integration:** register `integrate_risch` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test integrate_risch` — `∫ x e^{x^2}`, `∫ ln(x)`,
  `∫ e^x/(e^x+1)`, and `∫ e^{-x^2}` returned as `Expr::integral` noun.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/integrate/risch.rs` | `integrate` | ⬜ Not started | Differential field tower $k(t_1)\dots(t_n)$ for $\exp$ and $\ln$ extensions |
| `sym-core/src/integrate/risch_de.rs` | `integrate` | ⬜ Not started | Risch differential equation solver: $y' + f y = g$ in differential fields |
| `tests/integration/integrate_risch.rs` | `integrate` | ⬜ Not started | Tests proving non-elementary integrals (e.g. $\int e^{-x^2} dx \to \text{noun}$) and solving transcendental towers |

### Acceptance Criteria
- Successfully integrates nested exp-log expressions (e.g. $\int x e^{x^2} dx$, $\int \ln(x) dx$, $\int \frac{e^x}{e^x + 1} dx$).
- Proves non-elementary integrals and returns unevaluated `Expr::integral(f, var)` noun form.

---

## Phase 26 — Laurent Series & Asymptotic Expansions [Feature: series]

**Goal**: Implement truncated Laurent series arithmetic and generalized power series expansions supporting poles and fractional Puiseux powers.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features series` (Laurent `Series` `+ - * /`,
  inversion, composition, `diff`, negative valuations).
- **Gate 2 — Integration:** register `series` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test series` — Taylor/Laurent for sin/cos/exp/1/(1-x)/1/sin(x)
  truncated to O(x^n), checked against known coefficients.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/series.rs` | `series` | ⬜ Not started | `Series { center: Expr, valuation: i64, coeffs: Vec<Expr>, order: i64 }` with `+`, `-`, `*`, `/`, composition, `diff` |
| `tests/integration/series.rs` | `series` | ⬜ Not started | Taylor/Laurent expansions for $\sin(x)$, $\cos(x)$, $\exp(x)$, $\frac{1}{1-x}$, $\frac{1}{\sin(x)}$ at $x=0$ up to $O(x^n)$ |

### Acceptance Criteria
- Correct arithmetic on series with negative valuations (Laurent poles).
- Inversion and composition of series accurate up to requested truncation order $O(x^n)$.

---

## Phase 27 — Gruntz Limits Engine [Feature: limit]

**Goal**: Implement Dominik Gruntz's MRV algorithm for computing asymptotic limits of exp-log functions, equipped with a 3-tier zero-equivalence decider.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features limit` (MRV set extraction, comparability,
  ω→0 distortion, 3-tier zero decider: ratsimp → saturation → interval guard).
- **Gate 2 — Integration:** register `limits` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test limits` — `lim_{x→0} sin(x)/x = 1`,
  `lim_{x→∞} (1+1/x)^x = e`, `lim_{x→∞} (e^{x+e^{-x}} - e^x) = 1`; two-sided/one-sided/infinite.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/limit/mrv.rs` | `limit` | ⬜ Not started | Most Rapidly Varying (MRV) subexpression set extraction and comparability relation |
| `sym-core/src/limit/mod.rs` | `limit` | ⬜ Not started | Gruntz algorithm orchestrator: $\omega \to 0$ distortion variable substitution and series expansion |
| `sym-core/src/limit/zero.rs` | `limit` | ⬜ Not started | 3-tier zero-equivalence decider (`ratsimp` -> e-graph saturation -> interval evaluation guard) |
| `tests/integration/limits.rs` | `limit` | ⬜ Not started | Golden tests: $\lim_{x\to 0} \frac{\sin x}{x} = 1$, $\lim_{x\to\infty} \left(1 + \frac{1}{x}\right)^x = e$, $\lim_{x\to\infty} (e^{x + e^{-x}} - e^x) = 1$ |

### Acceptance Criteria
- Solves challenging exp-log limits where naive L'Hôpital / Taylor expansions fail or loop.
- Supports two-sided, one-sided (`plus`, `minus`), and infinite limits ($\pm\infty$).

---

## Phase 28 — Definite Integration & Pole Scanning [Feature: integrate]

**Goal**: Implement definite integration (`defint`) using the Fundamental Theorem of Calculus (FTC), guarded by real pole detection and improper limit routing.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features integrate` (Sturm root pole scanner,
  FTC split at poles, improper-limit routing to `limit`).
- **Gate 2 — Integration:** register `integrate_defint` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test integrate_defint` — `∫_0^1 x^2 dx = 1/3`,
  `∫_0^∞ e^{-x} dx = 1`, and `∫_{-1}^1 1/x dx` marked divergent.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/integrate/defint.rs` | `integrate` | ⬜ Not started | `defint(expr, var, a, b)` with Sturm root pole scanning and FTC evaluation |
| `tests/integration/integrate_defint.rs` | `integrate` | ⬜ Not started | Proper and improper definite integrals ($\int_0^1 x^2 dx = 1/3$, $\int_0^\infty e^{-x} dx = 1$) |

### Acceptance Criteria
- Rejects application of naive FTC across real denominator poles (e.g. $\int_{-1}^1 \frac{1}{x} dx$ marked divergent).
- Improper integrals with infinite bounds ($\pm\infty$) evaluate cleanly via Gruntz limits.

---

## Phase 29 — Closed-Form Algebraic Solvers [Feature: solve]

**Goal**: Implement exact algebraic equation solvers for polynomial degrees $1 \le d \le 4$, factor-and-split for higher degrees, and transcendental variable isolation.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features solve` (linear/quadratic/Cardano/Ferrari,
  factor-and-split, transcendental isolation).
- **Gate 2 — Integration:** register `solvers_alg` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test solvers_alg` — exact roots for deg 1–4, with every root
  back-substituted and simplified to `0`.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/solve/mod.rs` | `solve` | ⬜ Not started | `solve(expr, var)` returning `AlgebraicSolutions(Vec<Expr>)` |
| `sym-core/src/solve/alg.rs` | `solve` | ⬜ Not started | Linear, quadratic, Cardano's cubic, Ferrari's quartic, and factor-and-split via Phase 23 `factor` |
| `sym-core/src/solve/isolate.rs` | `solve` | ⬜ Not started | Variable isolation for invertible elementary functions ($\exp$, $\ln$, $\sin$, $\cos$, $\tan$) |
| `tests/integration/solvers_alg.rs` | `solve` | ⬜ Not started | Algebraic equation solving and exact root verification by back-substitution |

### Acceptance Criteria
- Exact closed-form solutions for degrees 1, 2, 3, 4.
- All returned solutions simplify to 0 when substituted into the original equation.

---

## Phase 30 — Ordinary Differential Equation (ODE) Solvers [Feature: solve]

**Goal**: Implement 1st-order standard ODE solvers (separable, linear integrating factor, Bernoulli, exact) and 2nd-order linear constant-coefficient ODE solvers.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features solve` (separable, linear integrating
  factor, Bernoulli, exact 1st-order; 2nd-order real-distinct/repeated/complex roots).
- **Gate 2 — Integration:** register `solvers_ode` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test solvers_ode` — `y' + y = 0`, `y'' + ω^2 y = 0`, verified by
  substituting back with `diff` + `ratsimp` (expect `0`), yielding `_C1`/`_C2`.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/solve/ode.rs` | `solve` | ⬜ Not started | `ode2(ode, y, x)` returning `OdeSolution { solution: Expr, constants: Vec<String> }` |
| `sym-core/src/solve/ode_first.rs` | `solve` | ⬜ Not started | 1st-order ODE classification: separable, linear $y' + P(x)y = Q(x)$, Bernoulli, exact |
| `sym-core/src/solve/ode_second.rs`| `solve` | ⬜ Not started | 2nd-order constant-coefficient linear ODEs: characteristic roots (real distinct, repeated, complex) |
| `tests/integration/solvers_ode.rs` | `solve` | ⬜ Not started | Solving $y' + y = 0$, $y'' + \omega^2 y = 0$, verifying ODE solutions via differentiation |

### Acceptance Criteria
- Solves classical 1st-order and 2nd-order linear ODEs producing general solutions with symbolic constants `_C1`, `_C2`.
- Back-substitution with `diff.rs` and `ratsimp()` confirms the solution identically satisfies the ODE.

---

## Phase 31 — Maxima Parser CAS Syntax & Facade Integration [Features: cas, parser, full]

**Goal**: Extend the Maxima string parser with CAS top-level commands and expose all new procedural CAS operations across `sym-core`, `maxima-rs`, and `maxima-rs::prelude`.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --features "cas parser"` (Maxima CAS-command
  combinators: `expand`, `factor`, `ratsimp`, `partfrac`, `limit`, `ode2`).
- **Gate 2 — Integration:** register `parser_cas` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test parser_cas` — parse each CAS command and execute it
  end-to-end; assert `maxima_rs::prelude::*` exposes `expand`, `ratsimp`, `factor`, `partfrac`,
  `integrate`, `defint`, `limit`, `taylor`, `solve`, `ode2`.
- **Gate 3 — Completion:** fmt, clippy `-D warnings`, full workspace test, no regressions; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `sym-core/src/parser/maxima.rs` | `parser` | ⬜ Not started | Syntax combinators for `expand(...)`, `factor(...)`, `ratsimp(...)`, `partfrac(..., x)`, `limit(..., x, a)`, `ode2(..., y, x)` |
| `sym-core/src/lib.rs` | `cas` | ⬜ Not started | Module exports for `poly`, `integrate`, `series`, `limit`, `solve` |
| `maxima-rs/src/lib.rs` | `cas` | ⬜ Not started | Re-exports of all CAS operations and solver structures |
| `maxima-rs/src/prelude.rs` | `cas` | ⬜ Not started | Convenient prelude exports for one-line CAS usage |
| `tests/integration/parser_cas.rs` | `parser`, `cas`| ⬜ Not started | Golden tests parsing Maxima CAS commands and executing them end-to-end |

### Acceptance Criteria
- Maxima command syntax parses cleanly and executes corresponding CAS routines.
- `maxima_rs::prelude::*` exposes `expand`, `ratsimp`, `factor`, `partfrac`, `integrate`, `defint`, `limit`, `taylor`, `solve`, `ode2`.

---

## Phase 32 — Verification Suite, Proptest Invariants & Maxima Parity Benchmarks

**Goal**: Comprehensive verification with automated proptest invariant testing, Maxima benchmark tests from `References/maxima-master/tests/`, and zero-warning linting across the workspace.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** full fmt/clippy/test baseline green (per Global Testing Gates).
- **Gate 1 — Unit:** `cargo test -p sym-core --all-features` (all CAS modules).
- **Gate 2 — Integration/Proptest:** register `integrate_correctness`, `solve_correctness`, and
  `maxima_cas_parity` in `tests/Cargo.toml`, then
  `cargo test -p maxima-tests --test integrate_correctness --test solve_correctness --test maxima_cas_parity`
  — invariants over ≥10³ random expressions; parity against `rtest_integrate.mac`,
  `rtest_limit.mac`, `rtest_solve.mac`.
- **Gate 3 — Completion:** `cargo fmt --all -- --check` clean,
  `cargo clippy --all-targets --all-features -- -D warnings` = 0,
  `cargo test --workspace --all-features` 100% green; docs updated.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `tests/proptest/integrate_correctness.rs` | `integrate`, `poly` | ⬜ Not started | Proptest: $\forall f, \text{ratsimp}\left(\frac{d}{dx}\int f dx - f\right) == 0$ + numerical equivalence check |
| `tests/proptest/solve_correctness.rs` | `solve`, `poly` | ⬜ Not started | Proptest: root back-substitution $f(\text{root}) == 0$ |
| `tests/integration/maxima_cas_parity.rs` | `cas` | ⬜ Not started | Parity benchmark suite comparing against test cases in `rtest_integrate.mac`, `rtest_limit.mac`, `rtest_solve.mac` |
| `SESSION_LOG.md` | — | ⬜ Not started | Final session log update and phase completion tracker |
| `README.md` | — | ⬜ Not started | Updated feature matrix, CAS capabilities, and usage examples |

### Acceptance Criteria
- All proptest invariants hold over $10^3$ random algebraic and trigonometric expressions.
- `cargo fmt --all -- --check` passes cleanly.
- `cargo clippy --all-targets --all-features -- -D warnings` exits with code 0.
- `cargo test --workspace --all-features` passes with 100% green tests.

---

# Final Phase — WebUI & End-to-End Verification

## Phase 33 — WebUI Recursive Expression Composer & Final Integration Test Suite [Feature: maxima-web]

**Goal**: Add dropdown-driven **recursive function composition** to the webUI expression
input so a user can nest elementary functions and CAS transforms onto the current
expression, and add a final end-to-end integration test suite covering the whole web API.

### Testing Gates (Pre-Implementation)
- **Gate 0 — Baseline:** `git status` clean; `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo test --workspace --all-features`; `node --check maxima-web/static/app.js` — all green
  before touching `maxima-web/`.
- **Gate 1 — Unit:** `cargo test -p maxima-web` — `service::functions_catalog` (arity,
  templates), `service::compute` (composed expressions, every operation), API model
  (de)serialization.
- **Gate 2 — Frontend/Integration:** `node --check maxima-web/static/app.js`; register
  `webui_e2e` in `tests/Cargo.toml` (`[[test]]`); `cargo test -p maxima-tests --test webui_e2e`.
- **Gate 3 — Completion:** `cargo fmt --all`; clippy `-D warnings`; `cargo test --workspace --all-features`
  (zero regressions); live check of recursive composition; update `README.md` + `SESSION_LOG.md`.

### Deliverables & Files

| File | Feature Gate | Status | Purpose |
|------|--------------|--------|---------|
| `maxima-web/static/index.html` | `maxima-web` | ⬜ Not started | "Compose / function builder" dropdown beside `#exprInput`; undo/clear controls |
| `maxima-web/static/app.js` | `maxima-web` | ⬜ Not started | Recursive `compose()`/`nest()` logic, cursor-aware insertion, input-format-aware templates (Maxima vs LaTeX), undo history |
| `maxima-web/src/api.rs` | `maxima-web` | ⬜ Not started | `FunctionCatalogEntry` + `FunctionCatalog` response models |
| `maxima-web/src/service.rs` | `maxima-web` | ⬜ Not started | `functions_catalog()` (name, arity, kind, maxima/latex templates) + unit tests |
| `maxima-web/src/main.rs` | `maxima-web` | ⬜ Not started | Register `GET /api/functions`; keep `/api/operations` authoritative |
| `tests/integration/webui_e2e.rs` | `maxima-web` | ⬜ Not started | Final integration test suite: API contract + recursive composition + CAS round-trips |

### Requirements
- **Composable catalog** (served by `GET /api/functions`):
  - Unary functions: `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `asin`, `acos`, `atan`,
    `exp`, `ln`, `sqrt`, factorial `!`.
  - Binary/infix operators: `+`, `-`, `*`, `/`, `^`.
  - CAS transforms (available once Phase 31 lands): `expand`, `factor`, `ratsimp`, `integrate`,
    `limit`, `solve`, `ode2`.
- **Recursive nesting semantics:**
  - Selecting a unary function wraps the whole expression (or the current cursor selection):
    `x` → `sin(x)` → `sin(cos(x))`; repeatedly applicable (true recursion).
  - Selecting a binary operator composes two operands around the current expression.
  - Insertion keeps the active `input_format`: `sin(x)` in Maxima mode vs `\sin{x}` in LaTeX mode.
  - Preserve an undo/clear history; the composed text remains freely editable in the textarea
    and is sent unchanged by **Compute**.
- **Backward compatibility:** existing operations, example chips, coverage panel, and
  `/api/rules` behavior are unchanged.

### Acceptance Criteria
- Any unary, binary, or CAS function picked from the dropdown nests onto the current expression
  text recursively, in both Maxima and LaTeX input modes.
- `GET /api/functions` returns the full composable catalog with name, arity, kind, and templates.
- `tests/integration/webui_e2e.rs` passes: `/health`, `/operations`, `/functions`, `/rules`
  (≥ 92 rules), and `/compute` for every operation plus recursive cases
  (`sin(cos(x))`, `expand(factor((x+1)^2))`), matrix round-trip, and `provenance`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  and `cargo test --workspace --all-features` all exit green with no regressions.

**Status**: ⬜ Not started

