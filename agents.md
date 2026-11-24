# Agent Operating Guidelines: maxima-rs

> **Target Environment:** Antigravity IDE + Gemini 3.8 Flash (Medium)  
> **Workspace:** `maxima-rs` Symbolic Computation Engine Workspace (`sym-core`, `sym-macros`, `maxima-rs`, `maxima-web`, `tests`)  
> **Session Continuity:** Always consult [`SESSION_LOG.md`](SESSION_LOG.md) for cross-session context, decision history, and completion tracking before initiating work.  
> **Architecture & Call Graph Reference:** Consult [`/grafify`](.agents/skills/grafify/SKILL.md) for the authoritative module call graph and cross-crate interdependency matrix.

This document serves as the canonical operating manual and technical specification for autonomous AI coding agents collaborating on the `maxima-rs` codebase. Every instruction, negative constraint, and passing gate is optimized for deterministic execution under **Gemini 3.8 Flash (Medium)**.

---

## 1. Project Mission & North Star

`maxima-rs` is a high-performance, compile-time symbolic mathematics and code optimization engine written in 100% Safe Rust. It bridges legacy symbolic algebraic knowledge from Maxima 5.50 with an `egg`-powered e-graph equality saturation engine, high-speed procedural CAS algorithms, and procedural macros that generate zero-overhead, vectorizable native Rust code.

### Core Component Breakdown

| Component | Crate / Directory | Primary Role & Capabilities |
| :--- | :--- | :--- |
| **Core Symbolic Engine** | `sym-core` | AST representation (`Expr`), exact rational arithmetic (`BigRational`), analytical differentiation (`diff`), 1:1 Maxima e-graph rewrite rules (`rules`), FLOP cost extraction (`cost`), common subexpression elimination (`cse`), procedural CAS engines (`poly`, `integrate`, `series`, `limit`, `solve`), multi-format parsers (`winnow`), and precedence-aware LaTeX formatting. |
| **Procedural Macros** | `sym-macros` | `sym_fn!` and `sym_eval!` compile-time proc-macros transforming high-level math definitions directly into optimized native Rust functions, gradients, Jacobians, and Hessians. |
| **Public Facade** | `maxima-rs` | Ergonomic facade crate exposing unified re-exports, fluent `Pipeline` builder, and comprehensive `prelude`. |
| **Offline Web Service** | `maxima-web` | Air-gapped, zero-CDN interactive notebook service featuring embedded KaTeX assets (`rust-embed`), typed CAS web dispatch (`/api/compute`), compute isolation, concurrency limits, and live preview (`/api/parse`). |
| **Verification & Benchmarks** | `tests` | Comprehensive integration suites, proptest invariant engines (differentiate-integral round-trips, solver back-substitution), and upstream Maxima 5.50 golden test fixtures. |

---

## 2. Core Operating Principles & Execution Model

1. **Hybrid E-Graph & Procedural Architecture (ADR-0006)**:
   - Use `egg` equality saturation strictly for algebraic simplification, trigonometric/hyperbolic identities, and FLOP cost extraction.
   - Execute procedural CAS algorithms (`expand`, `poly_gcd`, `ratsimp`, `factor`, `integrate`, `defint`, `series`, `limit`, `solve`, `ode2`) as deterministic, directed AST transformations outside the e-graph to avoid state-space combinatorial explosion.
2. **64-Bit Native Architecture & Memory Scale**:
   - Target 64-bit address spaces natively. Use packed 64-bit exponent vectors for multivariate monomials (`poly::monomial`).
   - Eliminate heap allocations in critical numerical hot loops.
3. **Cross-Compilation & Ubiquitous Portability**:
   - 100% Safe, pure Rust. Zero external C or C++ unmanaged dependencies (e.g. no GMP, MPFR, or FLINT).
   - Ensure seamless compilation to `x86_64`, `aarch64`, and `wasm32-unknown-unknown`.
4. **Evidence-Based Engineering & 1:1 Maxima Citations (ADR-0005)**:
   - Every e-graph rewrite rule must include structured `RuleProvenance` metadata citing the upstream Maxima Common LISP file, function, and equation line.
   - Validate mathematical algorithms against independent reference oracles (Maxima 5.50 test files in `References/maxima-5.50.0/tests/` and SymPy differential testing).
5. **Extreme Performance & Resource Bounds**:
   - Guard against unbounded recursion. All AST parsers and tree traversals must enforce a strict **Recursion Depth Guard** (maximum depth 256).
   - Isolate CPU-heavy CAS operations inside dedicated worker threads with wall-clock timeouts and owned concurrency semaphore permits.
6. **Strict Crate Isolation & Zero Warnings**:
   - `sym-core` must never depend on web, UI, or proc-macro crates.
   - All crates must compile with zero warnings under `cargo check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Never suppress warnings with unprincipled `#[allow(...)]`.
7. **Pure, Idiomatic Rust (Zero-Unwrap Policy)**:
   - **Zero tolerance for `.unwrap()`, `.expect()`, or `panic!()` in non-test paths.**
   - Propagate strongly typed errors using `thiserror` in libraries and `anyhow`/`miette` in application binaries.
8. **Verification Honesty & Tri-State Outcomes (ADR-0012)**:
   - Never mark an operation `verified: true` unless verified dynamically at runtime via analytical back-substitution or numerical quadrature.
   - Return structured tri-state outcomes (`Solved(T)`, `ProvenNonElementary`, `Unsupported(reason)`) rather than conflating missing features with mathematical impossibility.
9. **Verify, Never Assume (Ripgrep Discipline)**:
   - Before claiming any function, variable, or CSS class is unused or dead, run `rg` across the entire workspace to check all call sites, dynamic usages, and references.
   - Always prefer `rg` over `grep` or recursive find for millisecond multi-threaded searches and `.gitignore` compliance.
10. **Surgical Edits Only**:
    - Make focused, surgical edits using exact search-and-replace blocks. Never overwrite, truncate, or rewrite complete files from scratch.
11. **Single-Phase Discipline & Hard Passing Gates**:
    - Execute exactly **one phase at a time**.
    - **HARD STOP:** Never advance to Phase $N+1$ until every Passing Gate command in Phase $N$ exits with code `0`.
12. **Architectural Decision Record (ADR) Compliance**:
    - Strictly preserve decisions codified in `docs/adr/`. Do not read all ADRs up front; consult relevant ADRs on demand when modifying their specific domain.

---

## 3. Workspace Architecture & Boundaries

### ASCII Directory Tree

```
e:\Rust\Max\
├── Cargo.toml                       # Workspace root manifest
├── GLOSSARY.md                      # Canonical domain vocabulary
├── SESSION_LOG.md                   # Persistent cross-session continuity tracker
├── AGENTS.md                        # This operational manual
├── References/
│   ├── maxima-master.zip            # Maxima reference repository
│   ├── maxima-master/               # Uncompressed Maxima master source tree
│   └── maxima-5.50.0/               # Uncompressed upstream Maxima 5.50.0 source tree (`src/`, `tests/`)
├── docs/
│   ├── adr/                         # Architectural Decision Records (0001–0012)
│   ├── audits/                      # Phase sign-off and verification reports
│   └── recommendations/             # Upstream multi-model audits & recommendations
├── sym-core/                        # Foundational symbolic computation engine
│   ├── src/
│   │   ├── lib.rs                   # Public API surface & feature gates
│   │   ├── expr.rs                  # AST enum (Expr) & node constructors
│   │   ├── rational.rs              # Exact BigRational constant wrapper
│   │   ├── bfloat.rs                # [feature="bfloat"] Arbitrary-precision float (dashu)
│   │   ├── latex.rs                 # Precedence-aware LaTeX formatter
│   │   ├── diff.rs                  # Analytical calculus (diff, grad, jacobian, hessian)
│   │   ├── lang.rs                  # egg::Language AST mapping (MathLang)
│   │   ├── provenance.rs            # RuleProvenance registry & LISP citations
│   │   ├── rules/                   # 1:1 Maxima rewrite rules (simp, trigi, logarc, etc.)
│   │   ├── cost.rs                  # FLOP-weighted AST extraction cost model
│   │   ├── saturate.rs              # Saturation limits runner & e-graph orchestrator
│   │   ├── extract.rs               # Cost-optimal extraction & multi-output extraction
│   │   ├── cse.rs                   # Common Subexpression Elimination DAG pass
│   │   ├── codegen.rs               # proc_macro2::TokenStream code generator
│   │   ├── parser/                  # winnow parsers (Maxima strings & LaTeX)
│   │   ├── poly/                    # Multivariate polynomials, GCD, ratsimp, factor
│   │   ├── integrate/               # Symbolic integration (SIN, Hermite, LRT, Risch, defint)
│   │   ├── series.rs                # Truncated Laurent series arithmetic
│   │   ├── limit/                   # Gruntz MRV limits & zero-equivalence decider
│   │   ├── solve/                   # Algebraic & ODE solvers
│   │   ├── matrix/                  # [feature="matrix"] Bareiss fraction-free linear algebra
│   │   ├── complex/                 # [feature="complex"] rectform, polarform, residue
│   │   ├── specfn/                  # [feature="specfn"] Gamma, Bessel, Airy, Orthopolynomials
│   │   ├── transform/               # [feature="transform"] Laplace & inverse Laplace transforms
│   │   ├── assume/                  # [feature="assume"] Scoped inequality DAG & deductive inference
│   │   └── pipeline.rs              # Fluent Pipeline builder API
├── sym-macros/                      # Procedural macro crate (sym_fn!, sym_eval!)
├── maxima-rs/                       # Facade crate, prelude, high-level re-exports
├── maxima-web/                      # Offline web service, embedded assets, notebook API
│   ├── src/                         # Service isolation, config, routing, dispatch
│   └── static/                      # Embedded HTML, CSS, vanilla JS modules, vendored KaTeX
└── tests/                           # Integration suites, proptests, and parity benchmarks
```

### Directed Dependency Flow Graph

```
┌────────────────────────────────────────────────────────┐
│                      maxima-rs                         │
│                    (Facade Crate)                      │
└───────────────┬────────────────────────┬───────────────┘
                │                        │
                ▼                        ▼
┌────────────────────────┐      ┌────────────────────────┐
│       sym-macros       │      │       maxima-web       │
│   (Procedural Macros)  │      │     (Web Service)      │
└───────────────┬────────┘      └────────┬───────────────┘
                │                        │
                └───────────┬────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│                       sym-core                         │
│        (Core Symbolic Engine & CAS Algorithms)         │
└────────────────────────────────────────────────────────┘
                            ▲
                            │
┌───────────────────────────┴────────────────────────────┐
│                        tests                           │
│     (Integration, Proptest & Parity Verification)      │
└────────────────────────────────────────────────────────┘
```

### Strict Crate & Layer Invariants

- **`sym-core` Invariants**: Pure library. Must NEVER depend on `sym-macros`, `maxima-web`, `tokio`, or web frameworks. Must compile with no default features.
- **`sym-macros` Invariants**: Proc-macro library. Interacts with `sym-core` exclusively at compile time. Emits purely deterministic, vectorizable Rust code without runtime overhead.
- **`maxima-web` Invariants**: High-level web service. Isolates compute using `tokio::task::spawn_blocking` with explicit thread stack allocation, wall-clock timeouts, and owned concurrency semaphores. Serves 100% self-contained embedded assets (`rust-embed`) with zero external CDN dependencies.
- **`tests` Invariants**: Must test through public interfaces. Every integration test must be feature-gated to compile cleanly under feature isolation.

---

## 4. Engineering & Toolchain Standards

1. **Rust Toolchain**: Rust 2021/2024 edition, compatible with current stable compiler (`cargo 1.85+`).
2. **Error Handling Architecture**:
   - `sym-core` and libraries: Strongly-typed, granular domain error enums using `thiserror`. Every error must carry precise source attribution or error spans.
   - Applications (`maxima-web`): Top-level reporting using `anyhow` or structured HTTP response payloads (`Status::Error`, HTTP 400 with populated `error` text and `error_span`).
3. **Data Parsing & Lexing**:
   - Multi-format combinators built with `winnow = "0.7"`.
   - **Operator Precedence Rule**: Exponentiation (`^`) binds tighter than unary minus (`-`). `-x^2` must evaluate to `-(x^2)`.
   - **Scientific Notation Rule**: Recognize exponent notation (`1e-3`, `2.5e+4`).
   - **Recursion Depth Limit**: Enforce a hard recursion limit of 256. Abort deeply nested expressions (`((((...x...))))`) with `ParseError::RecursionLimitExceeded`.
4. **Testing Rigor & Determinism**:
   - Fast, reproducible tests. Property-based testing (`proptest`) must use bounded iterations (e.g. 256 cases) and fixed seeds.
   - Mandatory round-trip testing: For any expression $e$, assert `parse(to_latex(e)) == e` and `parse(to_string(e)) == e`.

---

## 5. Reference Map & Primary Sources

All upstream Maxima source references cite **Maxima 5.50.0** directly from the decompressed source tree at `References/maxima-5.50.0/` (with auxiliary master tree at `References/maxima-master/`).

| Feature / Domain | Primary Reference Source (Maxima 5.50.0 / Papers) | Key Topics Covered |
| :--- | :--- | :--- |
| **Algebraic Simplification** | Maxima 5.50.0 `src/simp.lisp` | `simpplus`, `simptimes`, `simpexpt`, canonical zero folding, rational exponents. |
| **Trigonometric & Hyperbolic** | Maxima 5.50.0 `src/trigi.lisp`, `src/sinh.lisp` | `simp-%sin`, `simp-%cos`, `simp-%sinh`, `simp-%cosh`, Pythagorean and angle-sum identities. |
| **Logarithmic & Exponential** | Maxima 5.50.0 `src/logarc.lisp` | Logarithmic expansions, log of powers, complex branch cuts. |
| **Rational Normalization** | Maxima 5.50.0 `src/nrat4.lisp` | `$ratsimp`, `$nrat`, quotient merging, common denominators. |
| **Factorials & Combinatorics** | Maxima 5.50.0 `src/sfacts.lisp` | `factcomb`, factorial recurrences, gamma-factorial relations. |
| **Arbitrary Precision Float** | Maxima 5.50.0 `src/float.lisp` | `bfloat`, `fpprec`, arbitrary-precision floating point conversion and arithmetic. |
| **Polynomial Expansion & Heap** | Monagan & Pearce (2007) | Sparse distributed multivariate polynomials, 64-bit packed grevlex heap arithmetic. |
| **Symbolic Integration (Heuristic)** | Joel Moses (1967) SIN | Derivative-divides ($u$-substitution), linearity, elementary integral tables. |
| **Rational Pole Reduction** | Hermite (1872), Lazard-Rioboo-Trager (1991) | Pole reduction without factoring; real $\ln/\text{atan}$ antiderivatives via Sturm sequences. |
| **Transcendental Integration** | Manuel Bronstein (2005) *Symbolic Integration I* | Differential field towers, Risch logarithmic/exponential extensions. |
| **Asymptotic Limits** | Dominik Gruntz (1996) PhD Thesis | Most Rapidly Varying (MRV) sets, asymptotic series, zero-equivalence deciders. |
| **Linear Algebra & Determinants** | Erwin Bareiss (1968), Maxima 5.50.0 `src/newdet.lisp` | Fraction-free Gaussian elimination, exact polynomial determinants, adjoint matrix inverse. |
| **Complex Decomposition** | Maxima 5.50.0 `src/rpart.lisp` | `rectform`, `polarform`, `carg`, `conjugate`, Laurent series residues. |
| **Special Functions** | Maxima 5.50.0 `src/specfn.lisp`, `src/bessel.lisp` | $\Gamma, \beta, \text{erf}$, Bessel $J/Y/I/K$, Airy $Ai/Bi$, orthogonal polynomials. |
| **Operational Transforms** | Maxima 5.50.0 `src/laplac.lisp` | Forward Laplace `laplace(f, t, s)` and inverse `ilt(F, s, t)`. |
| **Deductive Assumptions** | Maxima 5.50.0 `src/compar.lisp`, `src/askp.lisp` | Inequality DAG inference, `assume`, `forget`, `is`, `asksign`, `csign`. |
| **Verification & Golden Tests** | Maxima 5.50.0 `tests/rtest*.mac` | Upstream golden test suites for integration, limits, series, ODEs, special functions. |

---

## 6. Architectural Decision Records (ADRs)

Architectural Decision Records are located in `docs/adr/`. **Do NOT review all ADRs up front.** Only consult a specific ADR on demand when actively implementing or modifying that specific functional domain:

| Functional Domain / Task Context | Relevant ADR | Scope & Core Invariants |
| :--- | :--- | :--- |
| **E-Graph Engine Architecture** | [`0001-egraph-engine-over-lisp-port.md`](docs/adr/0001-egraph-engine-over-lisp-port.md) | Chose `egg` equality saturation over a direct LISP port for compile-time code generation. |
| **Fast-Math Semantics** | [`0002-configurable-fast-math-default.md`](docs/adr/0002-configurable-fast-math-default.md) | Default real-number equivalence; configurable IEEE-754 strict mode per invocation. |
| **Rewrite Rule Strategy** | [`0005-systematic-maxima-rule-coverage.md`](docs/adr/0005-systematic-maxima-rule-coverage.md) | Systematic 1:1 mapping with Maxima LISP citations; supersedes ADR-0003. |
| **Exact Constant Representation** | [`0004-rational-constant-representation.md`](docs/adr/0004-rational-constant-representation.md) | Arbitrary-precision `BigRational` constants throughout e-graph; float literal emission at codegen. |
| **CAS Algorithm Partitioning** | [`0006-hybrid-egraph-and-procedural-cas.md`](docs/adr/0006-hybrid-egraph-and-procedural-cas.md) | Strict split: e-graph for local simplification; procedural AST modules for heavy CAS algorithms. |
| **Web API Dispatch & Offline UI** | [`0007-webui-cas-typed-dispatch-and-offline-architecture.md`](docs/adr/0007-webui-cas-typed-dispatch-and-offline-architecture.md) | Typed JSON dispatch (`POST /api/compute`), embedded offline assets (`rust-embed`), zero-CDN KaTeX. |
| **Service Concurrency & Compute Isolation** | [`0008-compute-isolation-and-service-concurrency.md`](docs/adr/0008-compute-isolation-and-service-concurrency.md) | Bounded worker threads, wall-clock timeouts, owned semaphore permits, payload size limits. |
| **Arbitrary Precision Float (`bfloat`)** | [`0009-maxima-5-50-core-parity-and-dashu-bfloat.md`](docs/adr/0009-maxima-5-50-core-parity-and-dashu-bfloat.md) | Scope restricted to core `src/`; pure-Rust `dashu-float` for arbitrary precision floating point. |
| **Complex Atoms & Bareiss Linear Algebra** | [`0010-ast-complex-atom-bfloat-and-bareiss-linear-algebra.md`](docs/adr/0010-ast-complex-atom-bfloat-and-bareiss-linear-algebra.md) | Zero-allocation `Expr::I` atom, canonical `rectform`, and fraction-free Bareiss $O(n^3)$ elimination. |
| **Special Functions & Scoped Assumptions** | [`0011-special-functions-laplace-and-scoped-assumptions.md`](docs/adr/0011-special-functions-laplace-and-scoped-assumptions.md) | `Expr::SpecialFn` table dispatch, operational Laplace transforms, immutable `Assumptions` context. |
| **Outcomes, Honesty & Resource Safety** | [`0012-tri-state-symbolic-outcomes-and-verification-honesty.md`](docs/adr/0012-tri-state-symbolic-outcomes-and-verification-honesty.md) | Tri-state outcomes (`Solved`/`ProvenNonElementary`/`Unsupported`), zero-fake-verification, depth limit 256. |

---

## 7. Mandatory Edge-Case Test Matrix

Every newly audited or implemented module MUST include unit, integration, or property tests covering all applicable categories below:

| Category | Required Test Fixtures & Edge Cases |
| :--- | :--- |
| **Numerical & Constants** | `f64::NAN`, `f64::INFINITY`, `f64::NEG_INFINITY`, `-0.0`, subnormals, division by zero (`1/0`), exact rational zero (`0/1`), negative exponents (`2^-1`, `-x^-2`), scientific notation (`1e-3`, `2.5e+4`). |
| **Operator Precedence** | `-x^2` at $x=3$ must equal `-9` (never `9`), `-2^2 == -4`, `2^-1 == 0.5`, `-x! == -(x!)`, `(-x)^2 == x^2`, `exp(-x^2) == Exp(Neg(Pow(x, 2)))`. |
| **Cardinality & Boundaries** | Polynomial degree `0` (constant), degree `1` (linear), degree `2` (quadratic), empty expression list, single variable, $\ge 8$ variables (transition from packed 64-bit to dynamic vector), order `0` series. |
| **Domain Algorithms** | Irreducible quadratics ($x^2+1$), non-elementary integrands ($e^{-x^2}, e^{x^2}$ returning `ProvenNonElementary`), branch cuts ($\ln(-1)$), damped 2nd-order ODEs ($y''+3y'+2y=0$), singular Jacobians. |
| **Parsers & I/O Safety** | Deeply nested parentheses (`((((...x...))))` 3000 levels returning `ParseError::RecursionLimitExceeded`), UTF-8 BOM, CRLF vs LF line endings, trailing delimiters, malformed input (`x+`), empty string. |
| **Concurrency & Timeouts** | Timeout cancellation (killing thread execution or releasing owned semaphore permit), concurrent compute saturation (HTTP 503 on exhausted semaphore), body size overflow (>64 KB returning HTTP 413). |
| **WebUI & Frontend** | Complete operations catalog populated in palette, dynamic parameter form generation, HTML/JS CSS class parity (zero undefined Tailwind classes), KaTeX font caching headers. |

---

## 8. Historical Project Status (Phases 1–45)

Historical milestones completed across previous engineering sessions:

| Phase | Description | Status | Scope & Capabilities |
| :---: | :--- | :---: | :--- |
| **1–12** | Core Engine, E-Graph, Diff, CSE, Codegen & Facade | ✅ Complete | Exact rational AST, analytical diff, 52 1:1 Maxima rules, FLOP extraction, CSE pass, syn/quote codegen, winnow parsers. |
| **13–21** | Advanced Math, Parser Parity & Rule Expansion | ✅ Complete | Tensors, matrices, calculus AST, 92 1:1 rules across 7 Maxima files (`sinh.lisp`, `nrat4.lisp`, `sfacts.lisp`), web provenance API. |
| **22–23** | Sparse Multivariate Polynomials & Rational Algebra | ◐ Partial | 64-bit packed exponents, Monagan-Pearce heap, `expand()`, univariate GCD, `ratsimp()`. (*Known boundary: factor and partfrac limited to rational roots*). |
| **24–25** | Symbolic Integration (Elementary & Transcendental) | ◐ Partial | Linearity, elementary table, derivative-divides $u$-substitution (`∫x·e^{x²}`). (*Known boundary: full Risch differential towers deferred*). |
| **26–28** | Laurent Series, Gruntz Limits & Definite Integration | ◐ Partial | Laurent series arithmetic, series-based limits, rational pole scanner, FTC definite integrals. (*Known boundary: MRV tower fallback to numerical check*). |
| **29–30** | Algebraic & Differential Equation Solvers | ◐ Partial | Closed-form linear/quadratic, rational root factor-and-split, 1st/2nd-order constant coefficient ODEs. (*Known boundary: cubic/quartic radicals deferred*). |
| **31–33** | CAS Parser Syntax, Facade & Recursive Composer | ✅ Complete | `parse_maxima_cas`, prelude re-exports, interactive recursive expression composer, E2E test suite. |
| **34–38** | CAS Web Dispatch, Offline WebUI & Hardening | ✅ Complete | Typed `/api/compute`, offline KaTeX assets, CLI config, containerization. |
| **38A–38C** | P0 Hardening, WebUI Restoration & Honesty | ✅ Complete | Precedence `-x^2 == -9`, recursion limit 256, scientific notation, palette restoration, ODE/defint verification. |
| **39** | Arbitrary-Precision Float (`bfloat`) | ✅ Complete | Pure-Rust `dashu-float` with Maxima `$fpprec` precision control and scientific notation. |
| **40** | Symbolic Linear Algebra & Bareiss Algorithm | ✅ Complete | Bareiss $O(n^3)$ determinant, adjoint inverse, charpoly, trace, eigenvalues. |
| **41** | Complex Analysis, Rectangular Form & Residues | ✅ Complete | `Expr::I` atom, canonical `rectform`, polarform, branch cuts, residue calculus. |
| **42** | Special Functions & Table Dispatch | ✅ Complete | $\Gamma, \beta, \text{erf}$, Bessel $J/Y/I/K$, Airy $Ai/Bi$, orthogonal polynomials $P_n, T_n, U_n, H_n, L_n$. |
| **43** | Operational Laplace Transforms & ODE Initial Values | ✅ Complete | Forward/inverse Laplace transforms, operational theorems, and IVP/BVP solvers (`ic1`, `ic2`, `bc2`). |
| **44** | Scoped Deductive Inference & Assumptions Context | ✅ Complete | Thread-safe `Assumptions` context, inequality DAG, interval bounds, sign deduction (`csign`, `asksign`). |
| **45** | Maxima 5.50 Parity Test Suite & Golden Verification | ✅ Complete | Golden test suites ported from Maxima 5.50 (`rtest_integrate`, `rtest_limit`, `rtest_taylor`, etc.), 100% test pass rate. |

---

## 9. Phased Audit & Improvement Roadmap

> **Rule:** No phase may begin implementation until **Gate 0** is green. Every phase's passing gate must exit with code `0`.

---

### Phase 38A: P0 Safety, Correctness & Resource Hardening
**Objective:** Remediate critical parser operator precedence bugs, process aborts via stack overflow, and thread leakage.

- **Tasks:**
  1. Fix operator precedence in `sym-core/src/parser/maxima.rs` and `sym-core/src/parser/latex.rs`: bind exponentiation (`^`) strictly tighter than unary minus/plus. Ensure `-x^2` parses as `-(x^2)`.
  2. Implement `RecursionDepthGuard` (limit 256) in both parsers, returning `ParseError::RecursionLimitExceeded` to prevent thread stack overflows.
  3. Add scientific notation parsing (`1e-3`, `2.5e+4`) in `sym-core/src/parser/maxima.rs`.
  4. Fix HTTP 400 error bodies in `maxima-web/src/app.rs`: map `result.error` (not `result.result`) into JSON response and include `error_span`.
  5. Move `OwnedSemaphorePermit` into `spawn_blocking` compute closures in `maxima-web/src/app.rs` to guarantee concurrency permits are freed only when the thread finishes.
  6. Add print-parse round-trip property tests in `tests/proptest/print_parse_roundtrip.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo check --workspace --all-targets
  cargo test -p sym-core --test parser_maxima
  cargo test -p maxima-tests --test web_hardening
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- **HARD STOP:** Verify `-x^2` evaluates to `-9` at $x=3$ and 3000 parentheses return HTTP 400 before proceeding.

---

### Phase 38B: WebUI Wiring & Styling Restoration
**Objective:** Restore operation palette rendering, CAS parameter dispatch, and eliminate undefined CSS classes.

- **Tasks:**
  1. Fix palette initialization in `maxima-web/static/js/palette.js` and `main.js`: handle array responses from `/api/operations` and eliminate Temporal Dead Zone reference errors.
  2. Wire embedded CAS commands in `maxima-web/src/service.rs`: dispatch `parse_maxima_cas` parameters directly without ignoring arguments.
  3. Eradicate ~60 undefined utility classes from `maxima-web/static/index.html` and JS modules by expanding `maxima-web/static/vendor/app.css` to define all required layout, spacing, and typography classes.
  4. Display CSE let-bindings (`extra.bindings`) in `maxima-web/static/js/notebook.js`.
  5. Add HTTP caching headers (`Cache-Control: public, max-age=31536000, immutable`) for embedded KaTeX font assets.
  6. Add a CI verification script checking for zero undefined CSS classes.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p maxima-web
  cargo test -p maxima-tests --test webui_e2e
  python -c "import re, glob; css = open('maxima-web/static/vendor/app.css').read(); defined = set(re.findall(r'\.([A-Za-z_][\w\-]*)', css)); used = set(); [used.update(m.split()) for f in ['maxima-web/static/index.html'] + glob.glob('maxima-web/static/js/*.js') for m in re.findall(r'class(?:Name)?\s*=\s*[\"\'`]([^\"\'`$]+)[\"\'`]', open(f).read())]; missing = [u for u in used if u not in defined and not u.startswith('fa-')]; assert not missing, f'Missing CSS classes: {missing}'"
  ```
- **HARD STOP:** Verify all operation buttons render in the UI palette before proceeding.

---

### Phase 38C: Engine Honesty, Dynamic Verification & Defect Remediation
**Objective:** Remediate mathematical errors in ODE solvers and definite integrals, implement tri-state integration outcomes, and enforce dynamic runtime verification.

- **Tasks:**
  1. Fix first-order ODE coefficient extraction in `sym-core/src/solve/ode.rs`: properly assign `a = coeffs[1]` and `b = coeffs[0]` (resolves wrong coefficients in `diff(y, x) + 2*y`).
  2. Implement dynamic runtime verification in `maxima-web/src/service.rs`: substitute proposed ODE solutions back into original ODEs; substitute algebraic roots into equations; verify definite integrals against numerical quadrature. Hardcoded `verified: true` is strictly prohibited.
  3. Implement tri-state integration results in `sym-core/src/integrate/`: return `Solved(Expr)`, `ProvenNonElementary`, or `Unsupported(String)`.
  4. Fix absolute value in logarithmic antiderivatives: $\int \frac{1}{x} dx = \ln|x|$.
  5. Remediate Taylor series off-by-one truncation and order labeling in `sym-core/src/series.rs`.
  6. Return complete root sets in `sym-core/src/solve/alg.rs` and mark incomplete factorizations as `Status::Partial`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --test solvers_ode
  cargo test -p sym-core --test integrate_defint
  cargo test -p maxima-tests --test maxima_cas_parity
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- **HARD STOP:** Verify ODE `diff(y,x) + 2*y = 0` yields $y = C_1 e^{-2x}$ with dynamic verification passing before proceeding.

---

### Phase 39: Pure-Rust Arbitrary-Precision Float (`bfloat`) [Feature: `bfloat`]
**Objective:** Implement arbitrary-precision floating-point arithmetic with configurable `fpprec`, citing Maxima's `float.lisp` (ADR-0009).

- **Tasks:**
  1. Add `bfloat` feature flag to `sym-core/Cargo.toml` and `maxima-rs/Cargo.toml` activating `dashu-float = "0.4"`.
  2. Create `sym-core/src/bfloat.rs` wrapping `dashu::float::DBig` with precision contexts matching Maxima's `fpprec` (default 16 decimal digits).
  3. Extend `Expr` AST with `Expr::Bfloat(BfloatWrapper)`.
  4. Implement `bfloat()` conversion function parsing strings and evaluating expressions to bigfloats.
  5. Add unit and integration tests in `tests/integration/bfloat.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --features bfloat --lib
  cargo test -p maxima-tests --test bfloat
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

---

### Phase 40: Symbolic Linear Algebra & Bareiss Algorithm [Feature: `matrix`]
**Objective:** Implement fraction-free Bareiss matrix determinants, adjoint inverses, and characteristic polynomials, citing Maxima's `newdet.lisp` (ADR-0010).

- **Tasks:**
  1. Add `matrix` feature flag to `sym-core/Cargo.toml` and `maxima-rs/Cargo.toml`.
  2. Create `sym-core/src/matrix/bareiss.rs` implementing Bareiss $O(n^3)$ fraction-free LU determinant algorithm.
  3. Implement matrix adjoint inverse, characteristic polynomial (`charpoly`), trace, and eigenvalues.
  4. Wire matrix operations into `maxima-web/src/service.rs`.
  5. Add integration tests in `tests/integration/matrix_bareiss.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --features matrix --lib
  cargo test -p maxima-tests --test matrix_bareiss
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

---

### Phase 41: Complex Analysis, Rectangular Form & Residues [Feature: `complex`]
**Objective:** Add first-class imaginary unit `Expr::I` atom, canonical complex decompositions (`rectform`, `polarform`, `carg`, `conjugate`), and Laurent residues, citing Maxima's `rpart.lisp` (ADR-0010).

- **Tasks:**
  1. Add `complex` feature flag; extend `Expr` with `Expr::I` satisfying $I^2 = -1$.
  2. Implement `rectform(expr: &Expr) -> (Expr, Expr)` returning real and imaginary components.
  3. Implement `polarform`, `carg`, `conjugate`, and Laurent series residue extraction.
  4. Add integration tests in `tests/integration/complex_analysis.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --features complex --lib
  cargo test -p maxima-tests --test complex_analysis
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

---

### Phase 42: Special Functions & Table Dispatch [Feature: `specfn`]
**Objective:** Implement non-elementary special functions ($\Gamma, \beta, \text{erf}$, Bessel, Airy, orthogonal polynomials) with table-driven differentiation and limits, citing Maxima's `specfn.lisp` and `bessel.lisp` (ADR-0011).

- **Tasks:**
  1. Add `specfn` feature flag; create `Expr::SpecialFn(SpecialFnKind, Vec<Expr>)`.
  2. Create `sym-core/src/specfn/` modules for Gamma, Bessel ($J, Y, I, K$), Airy ($Ai, Bi$), and orthogonal polynomials (Legendre, Chebyshev, Hermite, Laguerre).
  3. Wire analytical derivatives into `sym-core/src/diff.rs` via table dispatch.
  4. Add integration tests in `tests/integration/specfn.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --features specfn --lib
  cargo test -p maxima-tests --test specfn
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

---

### Phase 43: Operational Laplace Transforms & ODE Initial Values [Feature: `transform`]
**Objective:** Implement forward/inverse Laplace transforms and ODE initial/boundary condition solvers, citing Maxima's `laplac.lisp` (ADR-0011).

- **Tasks:**
  1. Add `transform` feature flag; create `sym-core/src/transform/laplace.rs`.
  2. Implement `laplace(f, t, s)` using operational linearity, frequency shift, and transform tables.
  3. Implement `ilt(F, s, t)` using partial fraction pole decomposition and inverse transform tables.
  4. Implement `ic1`, `ic2`, `bc2` initial/boundary condition solvers in `sym-core/src/transform/bvp.rs`.
  5. Add integration tests in `tests/integration/transform_laplace.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --features transform --lib
  cargo test -p maxima-tests --test transform_laplace
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

---

### Phase 44: Scoped Deductive Inference & Assumptions Context [Feature: `assume`]
**Objective:** Implement thread-safe, immutable assumptions context with inequality DAG inference, citing Maxima's `compar.lisp` and `askp.lisp` (ADR-0011).

- **Tasks:**
  1. Add `assume` feature flag; create `sym-core/src/assume/mod.rs`.
  2. Implement `Assumptions` context containing an inequality DAG and sign database.
  3. Implement `assume`, `forget`, `is`, `asksign`, `csign`.
  4. Thread `Option<&Assumptions>` into integration and limit routines.
  5. Add integration tests in `tests/integration/assume_deduction.rs`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p sym-core --features assume --lib
  cargo test -p maxima-tests --test assume_deduction
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

---

### Phase 45: Maxima 5.50 Parity Test Suite & Golden Verification
**Objective:** Execute full upstream Maxima 5.50 golden verification suite against reference tests in `References/maxima-5.50.0/tests/`.

- **Tasks:**
  1. Port benchmark test cases from `rtest_integrate.mac`, `rtest_limit.mac`, `rtest_taylor.mac`, `rtest_solve.mac`, `rtest_laplace.mac`, `rtest_bessel.mac`.
  2. Implement multi-feature isolation matrix build verification in CI.
  3. Ensure 100% test pass rate across all suites.
  4. Final documentation update (`README.md`, `SESSION_LOG.md`).

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  cargo test -p sym-core --all-features --doc
  ```

---

## 10. Phase Sign-Off Report Template

At the conclusion of each phase, the agent must generate a standardized sign-off report at `docs/audits/PHASE{N}_REPORT.md` before requesting review:

```markdown
# Phase {N} Sign-Off Report: {PHASE_TITLE}

## 1. Summary of Deliverables & Changes
- `[file/path.rs]`: Concise description of deliverables, fixes, or algorithms added.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P{N}-01 | `...` | Precedence / Stack / UI | `...` | `...` |

## 3. Mandatory Edge-Case Coverage Checklist
- [ ] Numerical (`NaN`, `Inf`, `-0.0`, scientific notation)
- [ ] Operator Precedence (`-x^2 == -9` at $x=3$)
- [ ] Cardinality (`0`, `1`, `2`, constant/duplicate)
- [ ] Domain Algorithms (singular Jacobians, branch cuts, non-elementary integrands)
- [ ] Parsers & I/O (recursion depth limit 256, empty input, CRLF)
- [ ] Concurrency & Timeouts (owned semaphore permits, timeout thread termination)
- [ ] WebUI & Persistence (palette rendering, CSS class parity)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo check --workspace --all-targets` | `0` | `0` | Passed |
| `cargo test --workspace --all-targets` | `0` | `0` | X passed; 0 failed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
```

---

## 11. Review & Verification Checklist for Agents

Before completing any task, filing a PR, or requesting sign-off, verify:
- [ ] `cargo fmt --all -- --check` passes cleanly with zero formatting diffs.
- [ ] `cargo check --workspace --all-targets` passes with 0 errors and 0 warnings.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes with exit code 0.
- [ ] `cargo test --workspace --all-features` passes with 100% green tests and zero regressions.
- [ ] `cargo test -p sym-core --all-features --doc` passes all doctests.
- [ ] **Zero unwrap policy**: No `.unwrap()`, `.expect()`, or `panic!()` in non-test paths.
- [ ] **Zero fake verification**: Every `verified: true` flag is verified dynamically via back-substitution or numerical quadrature.
- [ ] Mandatory Edge-Case Test Matrix adhered to for all modified modules.
- [ ] `SESSION_LOG.md` and `README.md` updated with phase deliverables.
- [ ] Phase Sign-Off report compiled under `docs/audits/PHASE{N}_REPORT.md`.
