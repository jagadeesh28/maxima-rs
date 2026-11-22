# maxima-rs

A compile-time symbolic mathematics and code optimization engine in Rust.

Converts legacy Maxima algebraic knowledge into an [`egg`](https://crates.io/crates/egg)-powered e-graph equality saturation engine that generates zero-overhead optimized Rust code via procedural macros.

---

## Current Status: Phase 13 Complete ✅

| Phase | Feature | Status | Summary |
|-------|---------|--------|---------|
| **1. Workspace & Expr AST** | `core` | ✅ **Complete** | Foundational AST, exact rational arithmetic, evaluation, display |
| **2. Analytical Differentiation** | `diff` | ✅ **Complete** | Partial derivatives, gradients, Jacobians, Hessians |
| **3. E-Graph Language, 1:1 Rules & Provenance** | `egraph` | ✅ **Complete** | `egg::Language` mapping, 52 1:1 Maxima LISP rules with citations & `RuleProvenance` |
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
| 21 | Docs | ⬜ Planned | Coverage report, glossary/session-log updates, final green verification |

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
maxima-rs = { version = "0.1", default-features = false, features = ["diff"] }
```

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

## Interactive Local Web Server (`maxima-web`)

`maxima-rs` includes an interactive local web server providing a browser-based UI and a JSON REST API for real-time symbolic optimization:

### Running the Web Server

Run the `maxima-web` crate from within its folder or with `--bin maxima-web`:

```powershell
# From the repository root:
cargo run -p maxima-web

# Or from inside the maxima-web directory:
cd maxima-web
cargo run
```

Then open **[http://localhost:3000](http://localhost:3000)** in your browser.

### Capabilities in the Web UI:
- **Real-time Simplify & Optimize**: Equality saturation using 1:1 Maxima rules.
- **Analytical Derivatives**: Compute partial derivatives, gradients, Jacobians, and Hessians.
- **CSE Breakdown**: View common subexpressions extracted as ordered intermediate let-bindings.
- **Zero-Overhead Rust Codegen**: Inspect and copy compilable Rust function bodies.
- **Math Formats**: Interactive rendering with KaTeX supporting Maxima syntax and LaTeX inputs.
- **REST API Endpoints**:
  - `POST /api/compute`: Accepts `{ "expression": "...", "input_format": "maxima"|"latex", "operation": "simplify" (alias "optimize") | "differentiate" | "gradient" | "jacobian" | "hessian" | "cse" | "rust_code" | "evaluate", "variables": [...], "env": { "x": "1" } }`
  - `GET /api/operations`: Lists all supported operations and metadata.
  - `GET /api/rules`: Returns the 1:1 Maxima rule catalog `{ total, by_file: [{ file, count }], rules: [{ name, source_file, source_fn, identity, fast_math_only }] }`.
  - `GET /api/health`: Health status and engine version.

