# maxima-rs

A compile-time symbolic mathematics and code optimization engine in Rust.

Converts legacy Maxima algebraic knowledge into an [`egg`](https://crates.io/crates/egg)-powered e-graph equality saturation engine that generates zero-overhead optimized Rust code via procedural macros.

---

## Current Status: Phase 45 Complete ✅ (v0.5.0)

| Phase | Feature | Status | Summary |
|-------|---------|--------|---------|
| **1. Workspace & Expr AST** | `core` | ✅ **Complete** | Foundational AST, exact rational arithmetic, evaluation, display |
| **2. Analytical Differentiation** | `diff` | ✅ **Complete** | Partial derivatives, gradients, Jacobians, Hessians |
| **3. E-Graph Language, 1:1 Rules & Provenance** | `egraph` | ✅ **Complete** | `egg::Language` mapping, initial 52 1:1 Maxima LISP rules with citations & `RuleProvenance` (now 92; see coverage) |
| **4. Cost, Saturation, Extraction** | `egraph` | ✅ **Complete** | FLOP-weighted cost model, runner config, multi-root extractor |
| **5. CSE Post-Pass** | `cse` | ✅ **Complete** | DAG-based common subexpression elimination |
| **6. Codegen (TokenStream)** | `codegen` | ✅ **Complete** | `proc_macro2::TokenStream` function body generation |
| **7. Multi-Format Parsers** | `parser` | ✅ **Complete** | Maxima string & LaTeX formula parsers via `winnow = "0.7"` |
| **8. High-Level API & Pipeline Builder** | `pipeline` | ✅ **Complete** | One-call `optimize()` and fluent `Pipeline` builder |
| **9. Proc-Macro Crate** | `macros` | ✅ **Complete** | `sym_fn!` and `sym_eval!` macros with span-aware diagnostics |
| **10. Facade Crate & Prelude** | `full` | ✅ **Complete** | Re-exports, ergonomic prelude (`maxima_rs::prelude::*`) |
| **11. Testing Infrastructure & Coverage** | all | ✅ **Complete** | Golden regression suite, `proptest`, and automated 1:1 Maxima citation coverage verification |
| **12. Ergonomics & Polish** | all | ✅ **Complete** | Zero clippy warnings across workspace, formatted, zero-overhead validation |
| **13. Tensor & Matrix Calculus** | `tensor` | ✅ **Complete** | Ricci calculus foundations, Covariant/Contravariant index types, AST & E-Graph support |
| **14. WebUI Correctness** | `maxima-web` | ✅ **Complete** | Fixed pane layout, JS syntax/copy wiring, `simplify`/`optimize` API contract, service tests |
| **15. Advanced AST Nodes** | `core` | ✅ **Complete** | `sinh/cosh/tanh`, `asin/acos/atan`, `%pi`/`%e`, factorial threaded through diff, e-graph, cost, CSE, codegen, macros |
| **16. Maxima Parser Parity** | `parser` | ✅ **Complete** | `%pi`/`%e`, postfix `!`, hyperbolic/inverse trig, `diff`/`integrate`/`sum`, `matrix([...])` |
| **17. Rule Expansion I** | `egraph` | ✅ **Complete** | 79 provenanced rules; new `sinh.lisp` citations, inverse-trig cancellations, double-angle/angle-sum/`1+tan²=sec²` from `trigi.lisp` |
| **18. Rule Expansion II** | `egraph` | ✅ **Complete** | 92 provenanced rules; `nrat4.lisp` rational normalization and `sfacts.lisp` factorial recurrences |
| **19. Matrix E-Graph Fidelity** | `egraph` | ✅ **Complete** | Lossless matrix symbol codec; matrices survive saturation/extraction (no sentinel) |
| **20. WebUI Provenance** | `maxima-web` | ✅ **Complete** | `GET /api/rules`, `provenance` operation, live "Maxima coverage" panel, new example chips |
| **21. Documentation & Verification** | all | ✅ **Complete** | Coverage report, glossary/session-log updates, final `fmt`/`clippy -D warnings`/`test` |

> **Rule**: This `README.md` file is actively updated upon the completion of every phase.

### Roadmap: Phases 14–21 (planned)

The next workstream restores the WebUI and deepens **1:1 parity with Maxima LISP sources**.
The detailed requirements live in [`agents.md`](file:///e:/Rust/Max/agents.md).

| Phase | Workstream | Status | Summary |
|-------|-----------|--------|---------|
| 14 | WebUI | ✅ Complete | Fixed broken `<main>` layout, `copyText()` script bug, and `simplify`/`optimize` API contract |
| 15 | Parity | ✅ Complete | AST nodes for `sinh/cosh/tanh`, `asin/acos/atan`, `%pi`, `%e`, factorial + full plumbing |
| 16 | Parity | ✅ Complete | `parse_maxima` parity: constants, factorial, hyperbolic/inverse trig, calculus & matrix forms |
| 17 | Parity | ✅ Complete | 1:1 rules citing `sinh.lisp` + deeper `trigi.lisp` |
| 18 | Parity | ✅ Complete | 1:1 rules citing `nrat4.lisp` (ratsimp) and `sfacts.lisp` (factorial) |
| 19 | Parity | ✅ Complete | Matrix fidelity through the e-graph (structured symbol encoding) |
| 20 | WebUI | ✅ Complete | `GET /api/rules` + "Maxima coverage" panel in the UI |
| 21 | Docs | ✅ Complete | Coverage report, glossary/session-log updates, final green verification |

### Roadmap: Phases 22–32: Wolfram Mathematica-Grade CAS Expansion (Planned)

Expanding maxima-rs into a high-performance Computer Algebra System rivaling Wolfram Mathematica via hybrid procedural algorithms and equality saturation ([ADR-0006](docs/adr/0006-hybrid-egraph-and-procedural-cas.md)).

| Phase | Capability | Feature | Status | Summary |
|-------|------------|---------|--------|---------|
| 22 | Sparse Distributed Poly Engine | `poly` | ✅ Completed | 64-bit packed exponents (`grevlex`), Monagan-Pearce heap multiplication/division, `expand()` |
| 23 | Exact Rational Algebra & Factoring | `poly` | ✅ Completed | Exact univariate GCD + long division, canonical `ratsimp()`, rational-root `factor()`, distinct-linear `partfrac()` |
| 24 | Rational & Elementary Integration | `integrate` | ✅ Completed | Linearity, power rule, elementary (trig/exp/ln) rules, rational via partial fractions (ln terms), IBP for `x^n·exp/sin/cos`; noun fallback |
| 25 | Transcendental Risch Field Towers | `integrate` | ✅ Completed | Derivative-divides u-substitution for exp/power towers (`∫x·e^{x²}`, `∫e^x/(e^x+1)`, `∫2x/(x²+1)`); non-elementary proofs via `Expr::integral` noun |
| 26 | Laurent Series & Asymptotics | `series` | ✅ Completed | Truncated `Series` arithmetic (`+ - * /`, `inv`, `pow_int`, `diff`, `compose`), Laurent poles, elementary Taylor/Laurent expansions |
| 27 | Gruntz Asymptotic Limits Engine | `limit` | ✅ Completed | Series-based limits + log-transform for variable exponents + numeric convergence fallback (`sin x/x→1`, `(1+1/x)^x→e`, exp-tower difference→1) |
| 28 | Definite Integration & Pole Scanning | `integrate` | ✅ Completed | Rational pole scan → divergence detection, FTC via `F(b)-F(a)`, improper bounds via limits (`∫_0^1 x²=1/3`, `∫_0^∞ e^{-x}=1`) |
| 29 | Closed-Form Algebraic Solvers | `solve` | ✅ Completed | Rational-root factor-and-split + linear/quadratic closed forms (deg 1–4), transcendental isolation (`exp x = 2 ⇒ x = ln 2`) |
| 30 | Ordinary Differential Equation Solvers | `solve` | ✅ Completed | Linear 1st-order + 2nd-order constant-coefficient (real distinct/repeated/complex) with `_C1`/`_C2`, back-substitution verified |
| 31 | Maxima CAS Syntax & Prelude | `cas`/`parser` | ✅ Completed | `parse_maxima_cas`/`execute_cas` for expand/factor/ratsimp/partfrac/limit/integrate/defint/taylor/solve/ode2; prelude re-exports CAS ops |
| 32 | Verification & Maxima Benchmarks | all | ✅ Completed | Proptest integration/solve invariants (≥10³ cases) + Maxima-style parity cases (`rtest_integrate/limit/solve`) + clean fmt/clippy |
| 33 | WebUI Recursive Composer & E2E | `maxima-web` | ✅ Completed | Dropdown function composer (recursive wrap, undo, format-aware), `GET /api/functions`, `webui_e2e` suite |
| 34 | CAS Web API Wiring & Typed Dispatch | `maxima-web` | ✅ Completed | Declarative `OpSpec` catalog, typed parameter validation (400), uniform status model, verification checks, `POST /api/parse` |
| 35 | Precedence-Aware LaTeX Formatter | `core` | ✅ Completed | Core `to_latex(&Expr)` with operator precedence (1..5), Greek macros, subscript constants, matrices, no redundant parens |
| 36 | Offline-First WebUI Notebook & Rebuild | `maxima-web` | ✅ Completed | Vendored KaTeX + WOFF2 fonts, standalone CSS, vanilla ES modules, `In[n]`/`Out[n]` notebook, tabbed outputs |
| 37 | Service Isolation & Asset Embedding | `maxima-web` | ✅ Completed | `spawn_blocking`, timeouts, semaphore concurrency, CLI config (`clap`), static asset embedding (`rust-embed`) |
| 38 | Standalone & Lambda Packaging | `maxima-web` | ✅ Completed | Thin LTO release profile, Dockerfile with Lambda Web Adapter, SAM template |
| 38A | P0 Safety, Correctness & Hardening | all | ✅ Completed | Precedence `-x^2 == -9`, recursion limit 256, scientific notation (`1e-3`), owned semaphore in worker thread, roundtrip proptests |
| 38B | WebUI Wiring & Styling Restoration | `maxima-web` | ✅ Completed | Palette catalog parsing, TDZ elimination, embedded CAS argument extraction, CSE let-bindings visualization, 0 undefined CSS classes |
| 38C | Engine Honesty & Defect Remediation | all | ✅ Completed | 1st-order linear ODE coefficient fix, Liouville non-elementary detection, definite integral negative branch cuts, composite Simpson & ODE residual dynamic verification |
| 39 | Arbitrary-Precision Float (`bfloat`) | `bfloat` | ✅ Completed | Pure-Rust `dashu-float` arbitrary-precision decimal floating point with Maxima `$fpprec` precision control, `b`/`B` exponent indicator, exact arithmetic & transcendentals |
| 40 | Symbolic Linear Algebra & Bareiss Algorithm | `matrix` | ✅ Completed | Fraction-free Bareiss $O(n^3)$ determinant algorithm, classical adjoint inverse, characteristic polynomial (`charpoly`), trace, and eigenvalues |
| 41 | Complex Analysis, Rectangular Form & Residues | `complex` | ✅ Completed | Zero-allocation `Expr::I` atom, canonical `rectform` (`rpart.lisp`), polar form, `cabs`, `carg`, `conjugate`, Euler's formula, and Laurent series residue calculus |
| 42 | Special Functions & Table Dispatch | `specfn` | ✅ Completed | Non-elementary special functions ($\Gamma, \beta, \text{erf}, \text{erfc}$, Bessel $J, Y, I, K$, Airy $\text{Ai}, \text{Bi}$, orthogonal polynomials $P_n, T_n, U_n, H_n, L_n$) with table-driven differentiation |
| 43 | Operational Laplace Transforms & ODE Initial Values | `transform` | ✅ Completed | Operational forward Laplace (`laplace`), inverse Laplace (`ilt`), frequency shift, multiplier, derivative rules, and IVP/BVP solvers (`ic1`, `ic2`, `bc2`, `ic1_ode`, `ic2_ode`, `bc2_ode`) |
| 44 | Scoped Deductive Inference & Assumptions Context | `assume` | ✅ Completed | Thread-safe `Assumptions` context, inequality DAG inference, interval bound propagation, sign deduction (`csign`, `asksign`), predicate evaluation (`is`), domain properties (`PropertyKind`), and reversibility (`forget`) |
| 45 | Maxima 5.50 Parity Test Suite & Golden Verification | `tests` | ✅ Completed | Full upstream golden test suite ported from Maxima 5.50 (`rtest_integrate`, `rtest_limit`, `rtest_taylor`, `rtest_solve`, `rtest_laplace`, `rtest_bessel`), 100% test pass rate across workspace |


---

## Overview & Vision

`maxima-rs` enables writing symbolic mathematical functions in idiomatic Rust syntax and having them optimized and analytically differentiated at **compile time**:

```rust
use maxima_rs::sym_fn;

// Compile-time symbolic differentiation + e-graph optimization + CSE
sym_fn! {
    pub fn rosenbrock(x: f64, y: f64) -> (f64, [f64; 2]) {
        expr: (1.0 - x).powi(2) + 100.0 * (y - x.powi(2)).powi(2),
        gradient: [x, y],
    }
}

fn main() {
    let (val, grad) = rosenbrock(1.0, 1.0);
    assert_eq!(val, 0.0);
    assert_eq!(grad, [0.0, 0.0]);
}
```

The macro automatically computes the exact analytical gradient, runs equality saturation to factor and simplify common terms, performs Common Subexpression Elimination (CSE), and emits flat `let` bindings that run with zero overhead at runtime.

---

## Pipeline

```
User Rust Source Code (sym_fn! / sym_eval!)
      │
      ▼
 1. Parse (syn tokenstream → Expr AST)
      │
      ▼
 2. Analytical Differentiation (Expr → derivative Expr per requested var)
      │
      ▼
 3. Inject all Exprs (value + derivatives) into egg::EGraph
      │
      ▼
 4. Equality Saturation (apply algebraic rewrite rules until limits)
      │
      ▼
 5. Cost-Optimal Extraction (FLOP-weighted CostFunction)
      │
      ▼
 6. CSE Post-Pass (identify shared subterms across outputs → let bindings)
      │
      ▼
 7. Codegen (emit proc_macro2::TokenStream → plain fn with let bindings)
```

---

## Cargo Features

The workspace is designed with modular feature gates to keep compile times fast and dependencies minimal:

| Feature | Default | Description | Key Dependencies |
|---------|---------|-------------|------------------|
| `core` (base) | Always | Foundational `Expr` AST and exact rational arithmetic | `num-rational`, `num-bigint`, `num-traits` |
| `diff` | Yes | Analytical differentiation engine | *(None — pure AST)* |
| `egraph` | Yes | E-graph equality saturation and FLOP-cost extraction | `egg = "0.9"` (optional) |
| `cse` | Yes | Common subexpression elimination pass | *(None — pure AST)* |
| `codegen` | Yes | TokenStream code generator | `proc-macro2`, `quote` (optional) |
| `parser` | Yes | Maxima string & LaTeX math parsers | `winnow = "0.7"` (optional) |
| `tensor` | Yes | Tensor / Ricci Calculus extensions | *(None)* |
| `macros` | Yes | Procedural macros (`sym_fn!`, `sym_eval!`) | `syn = "2"` |
| `full` | Yes | Bundle enabling all features | All dependencies |

```toml
# Minimal lightweight installation (just AST + calculus, no e-graphs or macros):
[dependencies]
maxima-rs = { version = "0.2", default-features = false, features = ["diff"] }
```

---

## Maxima 1:1 Rule Coverage

Every rewrite rule carries `RuleProvenance { source_file, source_fn, identity, fast_math_only }`.
Current registry: **92 rules across 7 Maxima LISP files** (also exposed live via `GET /api/rules`).

| Maxima source | Rules | Cited routines |
|---------------|-------|----------------|
| `simp.lisp` | 34 | `simpplus`, `simptimes`, `simpexpt` |
| `trigi.lisp` | 27 | `simp-%sin/cos/tan`, `simp-%asin/acos/atan`, `trigsimp` |
| `sinh.lisp` | 10 | `simp-%sinh`, `simp-%cosh`, `simp-%tanh` |
| `nrat4.lisp` | 8 | `$ratsimp`, `$nrat` |
| `logarc.lisp` | 7 | `sp1log` |
| `sfacts.lisp` | 5 | `factcomb` |
| `combin.lisp` | 1 | `factor` |
| **Total** | **92** | |

---

## Workspace Structure

- [`sym-core`](file:///e:/Rust/Max/sym-core/) — Core engine: expression AST, rational constants, differentiation, e-graph rules, cost function, CSE, codegen.
- [`sym-macros`](file:///e:/Rust/Max/sym-macros/) — Procedural macro implementation (`sym_fn!`, `sym_eval!`).
- [`maxima-rs`](file:///e:/Rust/Max/maxima-rs/) — Public facade crate re-exporting the engine and macros.
- [`maxima-web`](file:///e:/Rust/Max/maxima-web/) — Interactive browser WebUI & JSON compute API (built with Axum).
- [`docs/adr/`](file:///e:/Rust/Max/docs/adr/) — Architectural Decision Records.
- [`GLOSSARY.md`](file:///e:/Rust/Max/GLOSSARY.md) — Canonical domain vocabulary.
- [`agents.md`](file:///e:/Rust/Max/agents.md) — Phased implementation plan.
- [`SESSION_LOG.md`](file:///e:/Rust/Max/SESSION_LOG.md) — Cross-session context log.

---

## API Usage Examples

### 1. Symbolic AST & Exact Rational Arithmetic (`core`)

```rust
use sym_core::expr::Expr;
use std::collections::HashMap;

// Build: f(x, y) = (1 - x)^2 + 100 * (y - x^2)^2
let x = Expr::var("x");
let y = Expr::var("y");

let expr = Expr::pow(Expr::int(1) - x.clone(), Expr::int(2))
    + Expr::int(100) * Expr::pow(y.clone() - Expr::pow(x.clone(), Expr::int(2)), Expr::int(2));

// Evaluate numerically
let mut env = HashMap::new();
env.insert("x".to_string(), 1.0);
env.insert("y".to_string(), 1.0);

assert_eq!(expr.eval(&env), Some(0.0));
```

### 2. Analytical Differentiation & Calculus (`feature = "diff"`)

```rust
use sym_core::expr::Expr;
use sym_core::diff::{differentiate, gradient, jacobian, hessian};
use std::collections::HashMap;

let x = Expr::var("x");
let y = Expr::var("y");

// Partial derivative: d/dx(sin(x^2)) = 2*x*cos(x^2)
let f = Expr::sin(Expr::pow(x.clone(), Expr::int(2)));
let df_dx = differentiate(&f, "x");

// Gradient of Rosenbrock: [df/dx, df/dy]
let rosenbrock = Expr::pow(Expr::int(1) - x.clone(), Expr::int(2))
    + Expr::int(100) * Expr::pow(y.clone() - Expr::pow(x.clone(), Expr::int(2)), Expr::int(2));
let grad = gradient(&rosenbrock, &["x", "y"]);

// Hessian matrix: 2x2 second partial derivatives
let h = hessian(&rosenbrock, &["x", "y"]);

// Evaluate gradient at minimum (1.0, 1.0) -> [0.0, 0.0]
let mut env = HashMap::new();
env.insert("x".to_string(), 1.0);
env.insert("y".to_string(), 1.0);
assert_eq!(grad[0].eval(&env), Some(0.0));
assert_eq!(grad[1].eval(&env), Some(0.0));
```

---

## Offline-First WebUI & CAS Service (`maxima-web`)

`maxima-web` provides a standalone, offline-first browser interface and JSON CAS API backed by the `maxima-rs` symbolic engine.

All static assets (KaTeX 0.16.11, 20 WOFF2 math fonts, responsive CSS, and vanilla ES modules) are embedded directly into the release binary via `rust-embed`. The executable runs from any folder or air-gapped system with **zero runtime CDN or external network requests**.

### Running the Standalone Web Service

Run directly with explicit CLI arguments:

```bash
# Run standalone server on default port 3000
cargo run --release -p maxima-web

# Or specify custom port, bind address, and compute timeout via CLI flags:
cargo run --release -p maxima-web -- --bind 0.0.0.0 --port 8080 --timeout-ms 5000 --concurrency 8

# Print all CLI configuration options:
cargo run -p maxima-web -- --help
```

Then open **[http://localhost:3000](http://localhost:3000)** in your browser.

### WebUI & API Capabilities
- **Interactive Notebook Model**: `In[n]` / `Out[n]` stream with "Use as input", status chips (`ok`, `unevaluated`, `divergent`, `no_solution`, `error`), and automated `verified` checkmarks.
- **Tabbed Math Outputs**: Typeset math rendered via local KaTeX, raw Maxima syntax, clean precedence-aware LaTeX source, and compilable Rust code.
- **Recursive Function Composer**: Wrap, nest, undo, and clear elementary functions and CAS transforms.
- **Procedural CAS API**: Full coverage for `expand`, `factor`, `ratsimp`, `partfrac`, `integrate`, `defint`, `limit`, `taylor`, `solve`, and `ode2`.
- **Compute Isolation & Concurrency**: Computations run in `spawn_blocking` + `catch_unwind` with strict timeouts (`408`) and semaphore concurrency throttling (`503`).
- **REST Endpoints**:
  - `POST /api/compute`: Typed execution of CAS and optimization operations.
  - `POST /api/parse`: Live typeset LaTeX preview and variable detection.
  - `GET /api/operations`: Authoritative declarative catalog of all operations and parameter schemas.
  - `GET /api/functions`: Catalog of composable unary, binary, and CAS functions.
  - `GET /api/rules`: 1:1 Maxima LISP rewrite rules and citations.
  - `GET /api/health`: Health status.

### Container & AWS Lambda Deployment

#### Docker Standalone Container
```bash
docker build -t maxima-web .
docker run -p 3000:3000 maxima-web --bind 0.0.0.0 --port 3000
```

#### AWS Lambda (Serverless Application Model)
The provided `template.yaml` packages the container using AWS Lambda Web Adapter with explicit CLI arguments:
```bash
sam build
sam deploy --guided
```

