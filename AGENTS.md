# Agent Operating Guidelines: maxima-rs

> **Target Environment:** Antigravity IDE + Gemini 4 Hydrogen (Medium)  
> **Workspace:** `maxima-rs` Symbolic Computation Engine Workspace (`sym-core`, `sym-macros`, `maxima-rs`, `maxima-web`, `maxima-tests` in `tests/`)  
> **Session Continuity:** Always consult [`SESSION_LOG.md`](SESSION_LOG.md) for cross-session context, decision history, and completion tracking before initiating work.  
> **Architecture & Call Graph Reference:** Consult [`grafify`](.agents/skills/grafify/SKILL.md) for the authoritative module call graph and cross-crate interdependency matrix.

This document serves as the canonical operating manual and technical specification for autonomous AI coding agents collaborating on the `maxima-rs` codebase. Every instruction, negative constraint, and passing gate is tuned for deterministic execution under **Gemini 4 Hydrogen (Medium)**.

---

## 1. Project Mission & North Star

`maxima-rs` (`v0.2.0` workspace) is a high-performance, compile-time symbolic mathematics, computer algebra system (CAS), and code optimization engine written in 100% Safe Rust. It bridges legacy symbolic algebraic knowledge from Maxima 5.50 with an `egg`-powered e-graph equality saturation engine, deterministic procedural CAS algorithms, compile-time procedural macros, and an air-gapped Axum WebUI.

### Core Component Breakdown

| Component | Crate / Directory | Primary Role & Capabilities |
| :--- | :--- | :--- |
| **Core Symbolic Engine** | `sym-core` | AST representation (`Expr`), exact rational arithmetic (`BigRational`), arbitrary-precision float (`dashu-float` `bfloat`), analytical calculus (`diff`, `gradient`, `jacobian`, `hessian`), 1:1 Maxima e-graph rules (`rules`), FLOP cost extraction (`cost`), CSE (`cse`), codegen (`codegen`), procedural CAS engines (`poly`, `integrate`, `series`, `limit`, `solve`, `matrix`, `complex`, `specfn`, `transform`, `assume`), `winnow` parsers (`maxima`, `latex`), and precedence-aware LaTeX formatting (`latex`). |
| **Procedural Macros** | `sym-macros` | `sym_fn!` and `sym_eval!` compile-time proc-macros transforming Rust math definitions (`expr:` and optional `gradient:`) into optimized native Rust functions via `sym-core` e-graph saturation and CSE. |
| **Public Facade** | `maxima-rs` | User-facing facade crate exposing unified feature-gated re-exports, fluent `Pipeline` builder, and `prelude`. |
| **Offline Web Service** | `maxima-web` | Air-gapped, zero-CDN interactive notebook service featuring embedded KaTeX assets (`rust-embed`), typed CAS web dispatch (`POST /api/compute`), live preview (`POST /api/parse`), worker-thread compute isolation, concurrency limits, and stateless client-carried `session_context`. |
| **Verification & Benchmarks** | `tests/` (`maxima-tests`) | Integration suites, recursive AST `proptest` invariant engines, HTTP/JSON contract tests, WebUI E2E tests, and upstream Maxima 5.50 `.mac` golden parity verification. |

---

## 2. Core Operating Principles & Execution Model

> [!IMPORTANT]
> Treat every numbered rule below as a hard invariant. Do not batch phases, suppress compiler/clippy diagnostics, or mark unverified algorithms complete.

1. **Hybrid E-Graph & Procedural Architecture (ADR-0006, ADR-0013)**:
   - Use `egg` equality saturation strictly for local algebraic simplification, trig/hyperbolic identities, and FLOP cost extraction, guarded by mandatory `egg::Condition` singularity checks (`0/0`, `0^0`, `0^(-1)`, `(-1)!`) and Maxima-aligned `SimpFlags`.
   - Execute procedural CAS algorithms (`expand`, `poly_gcd`, `ratsimp`, `factor`, `partfrac`, `integrate`, `defint`, `series`, `limit`, `solve`, `ode2`, Bareiss `matrix`, `rectform`, `laplace`/`ilt`) as deterministic AST passes outside the e-graph.
2. **64-Bit Native Architecture & Memory Scale**:
   - Target 64-bit address spaces natively. Use packed 64-bit exponent vectors (`poly::monomial`) for $\le 8$ variables (falling back dynamically for $> 8$ variables) and avoid unnecessary heap allocations in numerical/polynomial hot loops.
3. **Cross-Platform Parity (`Windows`, `Linux`, `macOS`, `wasm32`)**:
   - Maintain 100% Safe, pure Rust with zero external unmanaged C/C++ dependencies (no GMP, MPFR, or FLINT).
   - Never hardcode Windows `\` separators or platform-specific paths; use `std::path::Path`/`PathBuf` and normalize logical/URI paths with `/`.
   - Parsers and serializers must transparently handle both `\r\n` (CRLF) and `\n` (LF). Isolate any unavoidable OS-specific code behind trait facades gated with `#[cfg(target_os = ...)]`.
4. **Evidence-Based Engineering & 1:1 Maxima Citations (ADR-0005)**:
   - Every e-graph rewrite rule must carry structured `RuleProvenance` metadata citing its upstream Maxima Common LISP file and function.
   - Validate mathematical algorithms against independent reference oracles (Maxima `.mac` suites in `References/`, SymPy differential checks, and algebraic/numerical back-substitution).
5. **Extreme Performance, Resource Bounds & Cooperative Cancellation (ADR-0008, ADR-0012, ADR-0014)**:
   - Enforce a hard **Recursion Depth Guard** (limit `256`) in parsers and recursive AST traversals.
   - Bound compute inputs (`taylor` order `0..=64`, `orthopoly` degree `<= 100`, `variables <= 16`, `env <= 64`, HTTP payload `<= 64 KB`).
   - Run WebUI compute workloads (including `/api/parse`) in `spawn_blocking` threads with `>= 8 MB` stacks, owned semaphore permits (`OwnedSemaphorePermit`), and cooperative `CancelToken` (`Arc<AtomicBool>`) polling across `egg` runner hooks and CAS loops.
6. **Strict Crate Isolation & Zero Warnings**:
   - `sym-core` must NEVER depend on `sym-macros`, `maxima-web`, `tokio`, or web frameworks, and must compile cleanly with `--no-default-features`.
   - Every crate must compile with zero warnings under `cargo check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
7. **Pure, Idiomatic Rust (Zero-Unwrap Policy)**:
   - **Zero tolerance for `.unwrap()`, `.expect()`, or `panic!()` in non-test code paths** (allowed strictly inside `#[cfg(test)]` modules and `tests/`).
   - Propagate strongly typed errors via `thiserror` in libraries and `anyhow` / structured HTTP error payloads in `maxima-web`.
8. **Upfront Causal Impact Tracing & Verification Honesty (ADR-0012)**:
   - Before editing or declaring any symbol unused, trace all callers, trait bounds, and enum match arms across the workspace using `rg` (explicitly excluding `References/`—see Section 4). Verify, never assume.
   - Return structured tri-state outcomes (`Solved(Expr)`, `ProvenNonElementary`, `Unsupported(reason)`). Never mark `verified: true` unless verified dynamically at runtime, and never label partial/subset heuristics as complete.
9. **Surgical Edits & Strict Scope Lock**:
   - Make focused, surgical search-and-replace edits. Never overwrite or truncate files with placeholder comments (`// ...`).
   - Do not perform unrequested anticipatory refactoring across future phases or unrelated modules outside the active phase boundary.
10. **Patient Builds, Root-Cause Diagnostics & Zero Warning Suppression**:
    - Track build/test commands to completion and fix compiler/clippy diagnostics at the root cause.
    - Never relax test assertions or silence warnings with unprincipled `#[allow(...)]` attributes.
11. **Single-Phase Discipline & Hard Passing Gates**:
    - Execute **exactly one phase at a time**.
    - **HARD STOP:** Never begin Phase $N+1$ until every Passing Gate command in Phase $N$ exits with code `0`, `docs/audits/PHASE{N}_REPORT.md` is generated, `SESSION_LOG.md` is updated, and a local `git commit` checkpoint is recorded. Gate `git push` on explicit user confirmation.
12. **Architectural Decision Record (ADR) Compliance**:
    - Strictly preserve invariants codified in `docs/adr/`. **Do NOT read all ADRs up front;** consult only the relevant ADR on demand when working in its domain.

---

## 3. Workspace Architecture & Boundaries

### ASCII Directory Tree

```text
maxima-rs/
├── Cargo.toml                       # Workspace manifest ([workspace.package] + [workspace.dependencies])
├── AGENTS.md                        # Canonical operating manual (this file)
├── GLOSSARY.md                      # Canonical domain vocabulary
├── SESSION_LOG.md                   # Cross-session context & milestone tracker
├── README.md                        # Public overview, crate usage & deployment guide
├── Dockerfile                       # Multi-stage container build (rust >= 1.88 + Lambda Web Adapter)
├── template.yaml                    # AWS SAM deployment template
├── .github/workflows/ci.yml         # Cross-OS CI (fmt, clippy, test, feature-matrix, JS/CSS/Docker)
├── scripts/verify_css_classes.py    # Static HTML/JS vs app.css class-parity check
├── References/                      # Excluded in .dockerignore; skip during routine rg scans
│   ├── maxima-5.50.0/               # Primary Maxima 5.50.0 source & test baseline
│   ├── maxima-master/               # Auxiliary Maxima 5.50post source & rtest*.mac tree
│   └── maxima-master.zip            # Reference archive
├── docs/
│   ├── adr/                         # Architectural Decision Records (0001–0014)
│   ├── audits/                      # Phase sign-off reports (PHASE38A–PHASE45, PARITY_SCOREBOARD)
│   └── recommendations/             # Multi-model audits & expansion specs
├── sym-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   # Public API surface & Cargo feature gates
│       ├── expr.rs                  # Expr AST enum & constructors
│       ├── rational.rs              # Exact BigRational wrapper (Rat)
│       ├── bfloat.rs                # [feature="bfloat"] dashu-float arbitrary precision
│       ├── latex.rs                 # Precedence-aware LaTeX printer
│       ├── diff.rs                  # Analytical diff, gradient, jacobian, hessian
│       ├── lang.rs                  # egg::Language AST mapping (MathLang, MathAnalysis)
│       ├── provenance.rs            # RuleProvenance registry & LISP citations
│       ├── rules/                   # 1:1 Maxima rewrite rules (arithmetic, trig, hyp, log, rat, fact, exp)
│       ├── cost.rs                  # FLOP-weighted e-graph extraction cost model
│       ├── saturate.rs              # Saturation runner, limits & CancelToken hooks
│       ├── extract.rs               # Cost-optimal & multi-root extraction
│       ├── cse.rs                   # Common Subexpression Elimination DAG pass
│       ├── codegen.rs               # proc_macro2::TokenStream code generator
│       ├── tensor.rs                # [feature="tensor"] Ricci tensor index types
│       ├── pipeline.rs              # Fluent Pipeline builder API
│       ├── parser/                  # winnow parsers (mod.rs, maxima.rs, latex.rs)
│       ├── poly/                    # [feature="poly"] Sparse polynomials, heap, expand, gcd, rat, factor, partfrac
│       ├── integrate/               # [feature="integrate"] sin, hermite, lrt, ibp, risch, risch_de, defint
│       ├── series.rs                # [feature="series"] Truncated Laurent series arithmetic
│       ├── limit/                   # [feature="limit"] Gruntz MRV limits & zero-equivalence
│       ├── solve/                   # [feature="solve"] alg, isolate, ode, ode_first, ode_second
│       ├── matrix/                  # [feature="matrix"] Bareiss linear algebra & e-graph matrix codec
│       ├── complex/                 # [feature="complex"] rectform, polarform, carg, cabs, residue
│       ├── specfn/                  # [feature="specfn"] gamma, bessel, airy, orthopoly
│       ├── transform/               # [feature="transform"] laplace, ilt, bvp (ic1, ic2, bc2)
│       └── assume/                  # [feature="assume"] Scoped inequality DAG & sign deduction
├── sym-macros/
│   ├── Cargo.toml
│   └── src/                         # lib.rs, config.rs, expand.rs, parse_sym_fn.rs, parse_sym_eval.rs
├── maxima-rs/
│   ├── Cargo.toml
│   └── src/                         # lib.rs, prelude.rs (unified public facade)
├── maxima-web/
│   ├── Cargo.toml
│   ├── src/                         # lib.rs, main.rs, app.rs, api.rs, config.rs, service.rs
│   └── static/                      # index.html, js/{api,history,main,notebook,palette,preview}.js, vendor/{app.css,katex}
└── tests/                           # Package `maxima-tests`
    ├── Cargo.toml
    ├── integration/                 # Feature-isolated integration & parity suites
    └── proptest/                    # Property-based invariant & round-trip suites
```

### Directed Dependency Flow Graph

```text
┌────────────────────────────────────────────────────────┐
│                      maxima-web                        │
│     (Offline Axum Service & Embedded Static UI)        │
└───────────────────────────┬────────────────────────────┘
                            │ depends on (features = ["full"])
                            ▼
┌────────────────────────────────────────────────────────┐
│                       maxima-rs                        │
│              (Public Facade & Prelude)                 │
└───────────────┬────────────────────────┬───────────────┘
                │ optional ("macros")    │ re-exports
                ▼                        │
┌────────────────────────┐               │
│       sym-macros       │               │
│   (sym_fn!, sym_eval!) │               │
└───────────────┬────────┘               │
                │ default-features=false │
                └──────────┬─────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│                       sym-core                         │
│        (Core Symbolic Engine & CAS Algorithms)         │
└────────────────────────────────────────────────────────┘
                           ▲
                           │ tests all 4 crates
┌──────────────────────────┴─────────────────────────────┐
│                maxima-tests (`tests/`)                 │
│     (Integration, Proptest, HTTP & Golden Parity)      │
└────────────────────────────────────────────────────────┘
```

### Strict Crate & Workspace Invariants

- **`sym-core` Invariants**: Pure library. Must NEVER depend on `sym-macros`, `maxima-rs`, `maxima-web`, `tokio`, or web frameworks. Must compile cleanly under `--no-default-features` and every individual feature flag.
- **`sym-macros` Invariants**: Proc-macro library depending on `sym-core` with `default-features = false, features = ["diff", "egraph", "cse", "codegen"]`. Emits deterministic `f64`/`f32` Rust code for `expr:` and `gradient:` with zero runtime allocations.
- **`maxima-rs` Invariants**: Feature set must stay synchronized with `sym-core` (including `tensor` once added in Phase 46).
- **`maxima-web` Invariants**: Isolates all symbolic parsing and compute inside `spawn_blocking` + `catch_unwind` with `>= 8 MB` stacks, cooperative `CancelToken`, wall-clock timeouts, owned semaphore permits, and zero CDN runtime calls.
- **`Cargo` Workspace Dependency & Version Management**:
  - Centralize shared external dependencies (`serde`, `serde_json`, `tokio`, `axum`, `num-*`, `egg`, `winnow`, `thiserror`, `proptest`, `dashu-float`, etc.) under `[workspace.dependencies]` in root `Cargo.toml`, referencing `{ workspace = true }` in member crates to eliminate duplicate compilation and lockfile drift.
  - Maintain unified monorepo versioning (`version.workspace = true`, currently `0.2.0`), `edition = "2024"`, and `rust-version = "1.88"` (MSRV required by `sha2 0.11.1`) under `[workspace.package]`.
  - Keep all workspace crates sharing the root `target/` artifact cache.

---

## 4. Engineering & Toolchain Standards

1. **Rust Toolchain & Two-Speed Check Discipline**:
   - `edition = "2024"`, MSRV `>= 1.88` (verified locally on `rustc 1.100.0`).
   - **Inner-Loop Speed**: Run crate-scoped checks (`cargo check -p sym-core` or `cargo test -p maxima-web`) during iterative development.
   - **Passing Gates**: Always run workspace-wide checks (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`) before completing any phase.
2. **Ripgrep (`rg`) Scoping Standard**:
   - **CRITICAL:** `References/` is NOT listed in `.gitignore`. Running `rg -g "!References/**"` still descends into thousands of Maxima files on Windows and hangs.
   - Always pass explicit workspace directories (`rg -n "<pattern>" sym-core sym-macros maxima-rs maxima-web tests docs scripts`) or use `-g "!References"` so `rg` prunes `References/` immediately.
3. **Cross-Platform Engineering (`Windows`, `Linux`, `macOS`)**:
   - Use `Path`/`PathBuf` or `/` in asset paths; never hardcode Windows `\`.
   - Ensure parsers, `.mac` test readers, and scripts handle `\r\n` and `\n` identically.
   - Note Python command difference across hosts: CI (`ubuntu-latest`) invokes `python3 scripts/verify_css_classes.py`; this Windows workstation uses `E:\Anaconda\python.exe scripts/verify_css_classes.py`.
4. **Error Handling Architecture**:
   - Libraries (`sym-core`): Granular `thiserror` domain error enums carrying precise source/parse spans (`ParseError`, `PolyError`, `MatrixError`, `TransformError`).
   - Web Service (`maxima-web`): Explicit HTTP status mapping (`200`, `400` with populated `error` + `error_span`, `404`, `408`, `413` for `> 64 KB`, `500`, `503`) and strict security headers (`CSP`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`).
   - Configuration: Zero application environment variables (use CLI args + TOML config; `AWS_LWA_*` adapter settings in container packaging are infrastructure-only).
5. **Data Parsing & Lexing (`winnow = "0.7"`)**:
   - **Operator Precedence**: Exponentiation (`^`, `**`) binds tighter than unary minus (`-`): `-x^2 == -(x^2)`. Postfix factorial (`!`, `!!`) binds tighter than binary ops.
   - **Scientific Notation & Bounds**: Exact rational parsing for `1e-3`, `2.5e+4` with exponent cap (`<= 10_000`) to prevent OOM.
   - **Recursion Depth Limit**: Hard depth cap `256` across every parser combinator (including `power()`) and AST evaluator returning `ParseError::RecursionLimitExceeded`.
6. **Testing Rigor & Independent Oracles**:
   - Print-parse round-trips: `parse(to_string(e))` and `parse(to_latex(e))` preserve AST/numerical equivalence.
   - Recursive AST `proptest` suites with bounded cases (`256`–`500`) checking root completeness and avoiding silent `if let Some` passes.
   - Real HTTP router E2E (`oneshot`), JS syntax (`node --check maxima-web/static/js/*.js`), CSS class verification, Playwright browser E2E, and Docker `--locked` build checks.

---

## 5. Reference Map & Primary Sources

Primary Maxima 5.50.0 baseline lives in `References/maxima-5.50.0/`, with auxiliary `5.50post` sources and `rtest*.mac` golden suites in `References/maxima-master/` (both excluded from Docker builds via `.dockerignore`):

| Feature / Domain | Primary Reference Source | Key Topics Covered |
| :--- | :--- | :--- |
| **Algebraic Simplification & Flags** | `References/maxima-5.50.0/src/simp.lisp`, `globals.lisp` | `simpplus`, `simptimes`, `simpexpt`, `domain`, `radexpand`, `logexpand`, `triginverses`. |
| **Trigonometric & Hyperbolic** | `References/maxima-5.50.0/src/trigi.lisp`, `sinh.lisp` | `simp-%sin/cos/tan`, `simp-%sinh/cosh/tanh`, inverse trig branch cuts, `trigexpand`/`trigreduce`/`trigsimp`. |
| **Logarithmic & Exponential** | `References/maxima-5.50.0/src/logarc.lisp` | Logarithmic expansions, `logarc`, `demoivre`, `exponentialize`. |
| **Rational Normalization & CRE** | `References/maxima-5.50.0/src/nrat4.lisp`, `rat3a-e.lisp`, `ezgcd.lisp`, `factor.lisp` | `$ratsimp`, multivariate GCD, square-free, Berlekamp/Cantor-Zassenhaus + Hensel, `partfrac`. |
| **Factorials & Differentiation** | `References/maxima-5.50.0/src/sfacts.lisp`, `comm.lisp`, `comm2.lisp` | `factcomb`, `diff(x!) = x!*psi[0](x+1)`, FTC derivatives. |
| **Arbitrary Precision Float** | `References/maxima-5.50.0/src/float.lisp` | `bfloat`, `fpprec`, `b`/`B` notation and decimal precision contexts. |
| **Polynomial Expansion & Heap** | Monagan & Pearce (2007) | Sparse distributed multivariate polynomials, 64-bit packed `grevlex` heap arithmetic. |
| **Symbolic Integration** | Moses (1967) SIN, Hermite (1872), LRT (1991), Bronstein (2005), `sin.lisp`, `risch.lisp`, `defint.lisp` | Derivative-divides, Hermite reduction, Lazard-Rioboo-Trager real `ln`/`atan`, Risch towers, pole-checked `defint`. |
| **Series & Asymptotic Limits** | Dominik Gruntz (1996), `hayat.lisp`, `limit.lisp`, `tlimit.lisp` | Laurent/Puiseux series, MRV sets, comparability, zero-equivalence deciders, directional limits. |
| **Algebraic & ODE Solvers** | `References/maxima-5.50.0/src/solve.lisp`, `algsys.lisp`, `psolve.lisp`, `ode2.lisp` | Linear/quadratic/cubic/quartic, `linsolve`, separable/linear/exact/Bernoulli 1st-order & 2nd-order ODEs. |
| **Linear Algebra & Matrices** | Bareiss (1968), `References/maxima-5.50.0/src/newdet.lisp`, `matrix.lisp` | Fraction-free $O(n^3)$ elimination, multivariate exact division, adjoint inverse, `charpoly`, eigenvalues. |
| **Complex Analysis** | `References/maxima-5.50.0/src/rpart.lisp`, `residu.lisp` | `rectform`, `polarform`, `atan2`-based `carg`, `cabs`, `conjugate`, dynamic-order Laurent residues. |
| **Special Functions & Laplace** | `References/maxima-5.50.0/src/specfn.lisp`, `bessel.lisp`, `airy.lisp`, `laplac.lisp` | $\Gamma, \beta, \text{erf}$, Bessel $J/Y/I/K$, Airy $Ai/Bi$, orthogonal polynomials, `laplace`, `ilt`, `ic1`/`ic2`/`bc2`. |
| **Deductive Assumptions** | `References/maxima-5.50.0/src/compar.lisp`, `askp.lisp`, `db.lisp` | Inequality DAG transitivity, contradiction detection, `assume`, `forget`, `is`, `asksign`, `csign`. |
| **Golden Parity Oracles** | `References/maxima-master/tests/rtest*.mac` & `References/maxima-5.50.0/tests/rtest*.mac` | Upstream Maxima regression pairs (`rtest1`, `rtest_trig`, `rtest_integrate`, `rtest_limit`, `rtest_taylor`, `rtest_carg`, `rtest_sign`, `rtest_laplace`, `rtestode`, `rtest_algsys`). |

---

## 6. Architectural Decision Records (ADRs)

> [!NOTE]
> Architectural Decision Records live in `docs/adr/`. **Do NOT read all ADRs up front.** Consult only the specific ADR below on demand when modifying its functional domain.

| Functional Domain / Task Context | Relevant ADR | Scope & Core Invariants |
| :--- | :--- | :--- |
| **E-Graph Engine Architecture** | [`0001-egraph-engine-over-lisp-port.md`](docs/adr/0001-egraph-engine-over-lisp-port.md) | `egg` equality saturation for local algebraic rewrites and compile-time extraction. |
| **Fast-Math Semantics** | [`0002-configurable-fast-math-default.md`](docs/adr/0002-configurable-fast-math-default.md) | Real-number default equivalence; per-invocation strict IEEE-754 override (refined by ADR-0013). |
| **Legacy Rule Strategy (Superseded)** | [`0003-hand-curated-rewrite-rules.md`](docs/adr/0003-hand-curated-rewrite-rules.md) | Superseded by ADR-0005. |
| **Exact Constant Representation** | [`0004-rational-constant-representation.md`](docs/adr/0004-rational-constant-representation.md) | Arbitrary-precision `BigRational` inside e-graph; float emission at codegen only. |
| **Rewrite Rule Strategy** | [`0005-systematic-maxima-rule-coverage.md`](docs/adr/0005-systematic-maxima-rule-coverage.md) | 1:1 Maxima LISP mapping with `RuleProvenance` metadata (refined by ADR-0013). |
| **CAS Algorithm Partitioning** | [`0006-hybrid-egraph-and-procedural-cas.md`](docs/adr/0006-hybrid-egraph-and-procedural-cas.md) | Hybrid split: e-graph for local simplification; procedural AST modules for heavy CAS. |
| **Web API Dispatch & Offline UI** | [`0007-webui-cas-typed-dispatch-and-offline-architecture.md`](docs/adr/0007-webui-cas-typed-dispatch-and-offline-architecture.md) | Typed JSON dispatch (`POST /api/compute`), `rust-embed` assets, zero-CDN KaTeX. |
| **Service Concurrency & Isolation** | [`0008-compute-isolation-and-service-concurrency.md`](docs/adr/0008-compute-isolation-and-service-concurrency.md) | Bounded worker threads, wall-clock timeouts, semaphore throttling, HTTP status contract. |
| **Arbitrary Precision Float (`bfloat`)** | [`0009-maxima-5-50-core-parity-and-dashu-bfloat.md`](docs/adr/0009-maxima-5-50-core-parity-and-dashu-bfloat.md) | Core `src/` parity scope; pure-Rust `dashu-float` arbitrary precision. |
| **Complex Atoms & Bareiss Linear Algebra** | [`0010-ast-complex-atom-bfloat-and-bareiss-linear-algebra.md`](docs/adr/0010-ast-complex-atom-bfloat-and-bareiss-linear-algebra.md) | First-class `Expr::I` atom, canonical `rectform`, $O(n^3)$ fraction-free Bareiss elimination. |
| **Special Functions & Scoped Assumptions** | [`0011-special-functions-laplace-and-scoped-assumptions.md`](docs/adr/0011-special-functions-laplace-and-scoped-assumptions.md) | `Expr::SpecialFn` table dispatch, operational Laplace transforms, immutable `Assumptions` DAG. |
| **Outcomes, Honesty & Resource Safety** | [`0012-tri-state-symbolic-outcomes-and-verification-honesty.md`](docs/adr/0012-tri-state-symbolic-outcomes-and-verification-honesty.md) | Tri-state outcomes (`Solved`/`ProvenNonElementary`/`Unsupported`), dynamic verification, depth 256. |
| **E-Graph Soundness & `SimpFlags`** | [`0013-egraph-soundness-and-simp-flags.md`](docs/adr/0013-egraph-soundness-and-simp-flags.md) | Mandatory `egg::Condition` singularity guards (`0/0`, `0^0`, `0^-1`, `(-1)!`) + Maxima `SimpFlags`. |
| **Cooperative Cancellation & Web Sessions** | [`0014-cooperative-cancellation-and-stateless-web-sessions.md`](docs/adr/0014-cooperative-cancellation-and-stateless-web-sessions.md) | In-process `CancelToken` (`Arc<AtomicBool>`) + `>= 8 MB` stacks; stateless client-carried `session_context`. |

---

## 7. Mandatory Edge-Case Test Matrix

Every newly audited or implemented module MUST include unit, integration, or property tests covering all applicable categories below:

| Category | Required Test Fixtures & Edge Cases |
| :--- | :--- |
| **Numerical & Constants** | `f64::NAN`, `f64::INFINITY`, `f64::NEG_INFINITY`, `-0.0`, subnormals, `0/0`, `0^0`, `0^(-1)`, `(-1)!`, `NaN!`, `Rat::to_f64` at `10^400/(10^400+1)`, `Rat::pow_int(2, i64::MIN)`, `BfloatContext::new(0)`, `BfloatWrapper::sinh(1b-10)` cancellation, scientific notation (`1e-3`, `2.5e+4`, `1e1000000000` rejection). |
| **Operator Precedence & Codegen** | `-x^2` at $x=3 \to -9$, `-2^2 == -4`, `2^-1 == 0.5`, `5!! == 15`, `(-x)^2 == x^2`, `exp(-x^2) == Exp(Neg(Pow(x, 2)))`, `codegen` compilation & eval of `cos(-x)`, `(-x)^2`, `exp(-x)`, Rust keywords, `sym_fn!` `f32` + `#[fast_math = false]` + arity/comma errors. |
| **Cardinality & Boundaries** | Degree `0`, `1`, `2`, empty expr list, $> 8$ variables in `Poly`, `Poly::scale(0)`, `ratsimp(1/(x-x)) -> Err(DivisionByZero)`, `taylor` order `0..=64` (reject `-1` and `10000`), `orthopoly` degree `<= 100`, $3\times 3$ and $4\times 4$ multivariate matrices. |
| **Domain Algorithms** | All 4 `carg(±1 ± %i)` quadrants, `cabs(x) == abs(x)`, `asin(sin(%pi)) != %pi`, `((-1)^2)^(1/2) == 1`, `csign(exp(%i*%pi)) == Neg`, DAG cycles/contradictions (`a>b` & `b>a`), `is_zero_equivalent(sin(x)) == false`, directional limits `limit(exp(1/x), x, 0, plus/minus)`, `diff(x!, x)`, `1/(x^2+1)` & `1/(x-1)^2` integration, `exp(0*x^2)` NOT non-elementary, `defint(1/(x^2-2), 0, 2)` & `defint(1/x, 0, 1)` divergent, damped 2nd-order ODEs (`y''+3y'+2y=0`), `y''=0`, unmatched `laplace`/`ilt` returning nouns. |
| **Parsers & I/O Safety** | UTF-8 `\sum_{i=α}^{n}`, nested braces, `\pi` -> `Expr::Pi`, `\sinc` boundary, `x_{0}` subscript, `escape_latex` on `%$#&_`, `2^2^...` depth 1000 & `((((...x...))))` depth 3000 returning `RecursionLimitExceeded`, CRLF vs LF, UTF-8 BOM, trailing tokens in `parse_maxima_cas`. |
| **Concurrency, Security & Timeouts** | Real 150ms timeout releasing `OwnedSemaphorePermit` within 250ms, `/api/parse` timeout/panic isolation, TOML config vs `Cli` precedence, HTTP `413` (`> 64 KB`), Rust identifier injection rejection (`400`), `CSP`/`X-Content-Type-Options`/`X-Frame-Options` headers, XSS immunity (`/#expr=<img src=x onerror=alert(1)>`). |
| **WebUI, Contracts & Persistence** | Real `oneshot` HTTP router contract tests (`/api/operations`, `/api/rules`, `/api/functions`, `/api/parse`, `/api/compute`), all 12 starter examples without param wiping, `error_span` + Fix-It suggestions, `evaluate` `env` table, `session_context` `%`/`a:`/`assume`/`forget`/`kill(all)`, CSS parity script (`verify_css_classes.py`), `node --check maxima-web/static/js/*.js`. |

---

## 8. Historical Project Status (Phases 1–45)

Per **ADR-0012 (Truthful Capability Reporting)**, historical phases are recorded with honest scope labels reflecting the [NEON_audit_recommendations.md](docs/recommendations/NEON_audit_recommendations.md) line-by-line audit and the Phase 46–53 remediation targets:

| Phase | Description | Status | Implemented Scope, Known Audit Gaps & Remediation Target |
| :---: | :--- | :---: | :--- |
| **1–12** | Core Engine, E-Graph, Diff, CSE, Codegen & Facade | ✅ Baseline | Exact rational AST, analytical diff (`diff`/`grad`/`jac`/`hess`), initial 52 rules, FLOP extraction, CSE, codegen, `winnow` parsers, `sym_fn!` (`expr:` + `gradient:`). (*Known audit defects: `0/0` e-graph collapse, `codegen` negation precedence, `sym-macros` `f32`/`fast_math`/arity gaps → remediated in Phase 47*). |
| **13–21** | Advanced Math AST, Parser Parity & Rule Expansion | ✅ Baseline | `sinh`/`cosh`/`tanh`/`asin`/`acos`/`atan`/`%pi`/`%e`/`!`, 92 rules across 7 Lisp files, lossless matrix symbol codec, `/api/rules`. (*Known audit defects: branch-cut rule gating, `tensor` missing in `maxima-rs`, `app.js` later split in Phase 36 → remediated in Phases 46–47*). |
| **22–23** | Sparse Multivariate Polynomials & Rational Algebra | ◐ Partial | 64-bit packed exponents, Monagan-Pearce heap, `expand()`, univariate GCD, `ratsimp()`. (*Known gaps: `Poly::scale(0)`, `Fraction::div(0)->0`, univariate-only GCD, rational-root-only `factor`, distinct-linear-only `partfrac` → remediated in Phase 48*). |
| **24–25** | Symbolic Integration (Elementary & Transcendental) | ◐ Partial | Linearity, power rule, elementary linear-arg table, derivative-divides $u$-substitution, IBP. (*Known gaps: `hermite.rs`/`lrt.rs`/`risch_de.rs`/`sin.rs` stubs, `1/(x^2+1)` & `x*(x+1)` failures, missing `ln\|x\|` → remediated in Phase 49*). |
| **26–28** | Laurent Series, Gruntz Limits & Definite Integration | ◐ Partial | Truncated `Series` arithmetic, series-based limits, rational pole scanner, FTC definite integrals. (*Known gaps: Laurent truncation bugs in `mul`/`inv`/`diff`, `is_zero_equivalent` bug, dead `mrv.rs`, missing irrational/endpoint poles → remediated in Phase 49*). |
| **29–30** | Algebraic & Differential Equation Solvers | ◐ Partial | Rational-root factor-and-split, linear/quadratic real roots, basic 1st/2nd-order ODEs. (*Known gaps: `ode_second` dead for all `b != 0`, `y''=0` missing `x`, `isolate` sign/circular bugs, missing complex/parametric roots → remediated in Phase 50*). |
| **31–33** | CAS Parser Syntax, Facade & Recursive Composer | ◐ Partial | `parse_maxima_cas`, prelude re-exports, `/api/functions` composer. (*Known gaps: trailing tokens accepted, `Solve` drops extra roots, binary composer `{}`/`limit` template bugs → remediated in Phases 46–47*). |
| **34–38C** | CAS Web Dispatch, Offline WebUI, Packaging & Hardening | ◐ Partial | Typed `/api/compute`, offline KaTeX (`rust-embed`), CLI config, `-x^2 == -9`, depth guard 256, `IntegrationResult`, dynamic verification. (*Known gaps: DOM XSS sinks, preview param wipe, timeout permit leak, broken CI `app.js` & Dockerfile `rust:1.85`, `/api/parse` unisolated → remediated in Phases 46 & 52*). |
| **39** | Arbitrary-Precision Float (`bfloat`) | ◐ Partial | `dashu-float` `BfloatWrapper` & `fpprec`. (*Known gaps: silent fallback on trig errors, `pow`/prec `0` panics, `sinh`/`tanh` cancellation, unexposed in WebUI → remediated in Phases 47 & 50*). |
| **40** | Symbolic Linear Algebra & Bareiss Algorithm | ◐ Partial | Bareiss $O(n^3)$ determinant, adjoint inverse, `charpoly`, trace, eigenvalues. (*Known gaps: `poly_gcd` breaks $\ge 3\times 3$ multivariate matrices, `is_expr_zero` shallow, $O(n^5)$ adjoint → remediated in Phase 50*). |
| **41** | Complex Analysis, Rectangular Form & Residues | ◐ Partial | `Expr::I` atom, `rectform`, `polarform`, Euler/branch cuts, residues. (*Known gaps: `carg_pair` wrong quadrant via `atan`, `cabs(x)` drops `\|.\|`, hardcoded residue order 6 → remediated in Phase 48*). |
| **42** | Special Functions & Table Dispatch | ◐ Partial | 18 `SpecialFnKind` variants ($\Gamma, \beta, \text{erf}$, Bessel, Airy, orthopoly) + analytical derivatives. (*Known gaps: `MathLang` comma-join corruption, missing `diff(x!)`, unbounded orthopoly degree, no WebUI exposure → remediated in Phases 47, 49 & 50*). |
| **43** | Operational Laplace Transforms & ODE Initial Values | ◐ Partial | Forward/inverse Laplace transforms (`laplace`, `ilt`) and `ic1`/`ic2`/`bc2`. (*Known gaps: bogus formulas on unmatched inputs instead of nouns, `ilt` `s`-leak & `i64` overflow, `bvp` unfolded `is_zero()`, unexposed in WebUI → remediated in Phase 50*). |
| **44** | Scoped Deductive Inference & Assumptions Context | ◐ Partial | Thread-safe immutable `Assumptions` context, `is`/`csign`/`asksign`/`forget`. (*Known gaps: broken DAG transitivity/contradictions/negation, wrong complex `csign`, unexposed in WebUI → remediated in Phases 48 & 50*). |
| **45** | Maxima 5.50 Golden Verification Baseline | ◐ Partial | Initial `tests/integration/golden_parity.rs` (6 suites, ~25 cases). (*Known gaps: small sample vs 103 `.mac` files; full automated `.mac` parity harness & `PARITY_SCOREBOARD.md` added in Phase 53*). |

---

## 9. Phased Audit & Improvement Roadmap (Phases 46–53)

> [!CAUTION]
> **Single-Phase HARD STOP Rule:** Work strictly on one phase at a time. Never edit files for Phase $N+1$ until every Passing Gate command in Phase $N$ exits with code `0`, `docs/audits/PHASE{N}_REPORT.md` is written, `SESSION_LOG.md` is updated, and a local `git commit` is created.

---

### Phase 46: Security, Build/CI Integrity & WebUI Contract Hotfixes (Stop-Ship)
**Objective:** Eliminate DOM XSS, repair broken CI/Docker/Cargo workspace configs, stop preview parameter wiping, and align all WebUI frontend/backend JSON contracts.

- **Tasks:**
  1. **XSS & Security Headers**: Escape all dynamic text (`res.error`, `err.message`, `request.operation`, `statusLabel`) in `maxima-web/static/js/{preview,notebook,main}.js` via `textContent` or `escapeHtml`. Add `CSP`, `X-Content-Type-Options: nosniff`, and `X-Frame-Options: DENY` headers in `maxima-web/src/app.rs`.
  2. **CI, Docker & Workspace Cargo Repair**:
     - Fix `.github/workflows/ci.yml` to run `node --check` across every file in `maxima-web/static/js/*.js`, execute `scripts/verify_css_classes.py` (with expanded template-literal and `classList` inspection), and expand `feature-matrix` to test `bfloat`, `matrix`, `complex`, `specfn`, `transform`, `assume`, plus `tensor` on `maxima-rs`.
     - Upgrade `Dockerfile` to `rust:1.88-bookworm` (or current stable), remove `Cargo.lock` from `.dockerignore`, copy `Cargo.lock` into `Dockerfile`, build with `cargo build --release --locked -p maxima-web`, set `AWS_LWA_READINESS_CHECK_PATH=/api/health`, align default ports, and raise `template.yaml` `MemorySize` to `1536` MB.
     - Centralize shared dependencies (`serde`, `serde_json`, `tokio`, `axum`, `num-*`, etc.) in root `Cargo.toml` `[workspace.dependencies]`, set `rust-version = "1.88"` in `[workspace.package]`, and add `tensor = ["sym-core/tensor"]` (bundled in `full`) to `maxima-rs/Cargo.toml`.
  3. **Backend Config, Isolation & Contract Fixes**:
     - Change `Cli` fields in `maxima-web/src/config.rs` to `Option<T>` so `--config <toml>` values are preserved; add `with_graceful_shutdown` in `maxima-web/src/main.rs`.
     - Wrap `/api/parse` (`maxima-web/src/app.rs`) in `spawn_blocking` + `catch_unwind` + `timeout` + semaphore.
     - Enforce bounds on `taylor` `order` (`0..=64`), `params` string lengths, `variables <= 16`, and `env <= 64`. Validate Rust identifiers in `maxima-web/src/service.rs` before codegen emission.
     - Wire `fast_math: bool` in `ComputeRequest` (`api.rs`) → `SaturationConfig`; unify `params` vs top-level lookup (`variable`, `variables`, `float_type`, `charpoly`, `residue`); reject invalid `float_type` with HTTP `400`; gate `parse_maxima_cas` on `input_format != "latex"`; eliminate double-parse on error; update `resp.operation` after auto-dispatch; support vector `[f1, f2]` inputs in `jacobian`.
     - Sort `Expr::variables()` deterministically.
  4. **Frontend Broken-Feature Fixes**:
     - Preserve user/example params in `palette.js` when `preview.js` updates detected variables; add `AbortController`/sequence counter in `preview.js`.
     - Render `env` inputs for `evaluate`; fix Rules modal & notebook Rules tab fields (`source_fn`, `identity`, `fast_math_only`, `response.extra.rules`); render `param.accepts` enums as `<select>`; fix `d/d[x,y]` chip, binary composer `{}` templates & `limit` arity; handle non-JSON HTTP errors in `api.js`; define `opacity-75`/`border-danger` in `app.css` and remove dead `auto-render.min.js`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  E:\Anaconda\python.exe scripts/verify_css_classes.py   # or python3 on Linux/macOS
  ```
  Plus shell/container verification:
  - `for f in maxima-web/static/js/*.js; do node --check "$f" || exit 1; done` exits `0`.
  - `docker build .` succeeds with `--locked`.
  - `tests/integration/web_hardening.rs` & `webui_e2e.rs` verify HTTP router `oneshot` contracts (`400`, `413`, `/api/parse` isolation, `fast_math`, `float_type`, security headers, XSS escape).
- **HARD STOP:** Verify zero unescaped `innerHTML` sinks and zero param wipes before advancing to Phase 47.

---

### Phase 47: E-Graph Soundness, Core Numbers, Parsers, Codegen & Macros Hardening
**Objective:** Eliminate e-graph `0 = 1` collapse, enforce Maxima-aligned `SimpFlags` (ADR-0013), harden `Rat`/`bfloat`/`Expr` edge cases, and fix parser, LaTeX, CSE, codegen, and `sym-macros` defects.

- **Tasks:**
  1. **Sound Rewrite Rules & `SimpFlags` (`sym-core/src/rules/*`, `lang.rs`)**:
     - Attach `egg::Condition` guards (`is_not_zero`, `is_positive`, `is_nonneg_int`) to `div-zero`, `div-self`, `pow-zero`, `pow-zero-base`, `pow-add-exp`, and `factorial` rules.
     - Implement Maxima-aligned `SimpFlags` (`domain`, `radexpand`, `logexpand`, `triginverses`, `fast_math`) gating `(x^a)^b`, `ln(exp(x))`, `ln(a*b)`, `ln(a^b)`, and `asin(sin(x))`/`acos(cos(x))`/`atan(tan(x))`.
     - Add `(^ e ?x) <=> (exp ?x)`, `(* i i) => -1`, `(^ i 2) => -1`, and canonical `%pi`-multiple trig rules.
     - Fix `MathAnalysis::merge` (remove silent `merge_max` on conflicting constants) and represent `SpecialFn` args as child `Id`s instead of comma-joined strings.
  2. **Numbers & `bfloat` (`rational.rs`, `bfloat.rs`, `expr.rs`)**:
     - Fix `Rat::to_f64` overflow (`inf/inf = NaN`), `Rat::pow_int` (`i64::MIN`, `u32` truncation, cap `<= 10_000`, `0^0 -> None`), and exact IEEE-754 `Rat::from_f64`.
     - Return `Result<Self, String>` from `BfloatWrapper::{sin,cos,tan,atan,pow}`, validate `fpprec >= 1`, guard `sinh`/`cosh`/`tanh` cancellation/overflow, normalize `PartialEq`/`Hash`.
     - Fix `Expr::eval(NaN!) -> None`, domain-error `Pow`/`Asin`/`Acos -> None`, and `Expr::is_const()` for `Pi`/`E`.
  3. **Parsers, Printers, CSE, Codegen & Macros**:
     - `parser/latex.rs`: Fix UTF-8 byte-slice panic & nested braces in `\sum`, preserve bounds & `d<var>` in `\int`, map `\pi` to `Expr::Pi`, enforce word boundaries (`\sinc`, `\cdots`), preserve `x_{0}`, add `RecursionDepthGuard` in `power()`.
     - `parser/mod.rs` & `parser/maxima.rs`: Cap `parse_exact_rational` exponent (`<= 10_000`), add `RecursionDepthGuard` in `power()`, tokenize `!!`, reject trailing tokens in `parse_maxima_cas`, return all roots in `execute_cas(Solve)`, align `Expr::Display` with `parse_maxima` roundtrip.
     - `latex.rs`: Escape `"#$%&_"` in `escape_latex`, fix `Neg(Add/Sub)` double parens, fix `#[cfg(feature = "complex")]` formatting leak, parenthesize `Bfloat` under `Pow`/`Neg`.
     - `cse.rs`, `codegen.rs`, `pipeline.rs`: Fix CSE temp `_t0` shadowing, bound-var hoisting, `BTreeMap` determinism, dead single-use bindings; parenthesize `Expr::Neg` before method calls (`cos(-x)`); emit free-var params & tuple returns in `Pipeline::to_rust_fn`; handle `NaN`/`Inf`/Rust keywords safely.
     - `sym-macros`: Parse `#[fast_math = false]`, `node_limit`, `time_limit_ms`; infer `FloatType` (`f32`/`f64`) from signature; enforce exact arity, comma separation, duplicate-field rejection, exact decimal literal parsing, and `gradient` variable validation.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  cargo test -p maxima-tests --test print_parse_roundtrip
  cargo test -p maxima-tests --test macro_expansion
  ```
- **HARD STOP:** Verify `0/0`/`0^0` non-collapse, `cos(-x)`/`(-x)^2` codegen compilation & evaluation, `sym_fn!` `f32`/`#[fast_math = false]`/arity checks, and 1,000+ print-parse round-trips before Phase 48.

---

### Phase 48: Assumptions, Complex Analysis & Multivariate Polynomial/Rational Engine Parity
**Objective:** Make `assume`, `complex`, and `poly` mathematically sound and multivariate-capable, matching Maxima 5.50 core semantics.

- **Tasks:**
  1. **Assumptions & Sign Engine (`sym-core/src/assume/mod.rs`)**:
     - Normalize inequalities into a directed strict/non-strict graph (`<`/`<=` as reverse `>`/`>=`, `=` bidirectional, `!=` tracked); fix BFS/DFS mixed strict/non-strict transitivity, strict cycle contradiction rejection, `(Const, Var)` bounds, and negation checks (`assume(a>b) => is(a<b) == False`).
     - Fix `csign` on `Exp`/`Pow` (`exp(%i*%pi)`, `%i^2`, `2^%i`, `0^(-1)`) and `has_property` consistency checks.
  2. **Complex Analysis (`sym-core/src/complex/mod.rs`, `expr.rs`)**:
     - Add `Expr::Abs(Box<Expr>)` and `Expr::Atan2(Box<Expr>, Box<Expr>)` across parser, LaTeX printer, diff, and eval.
     - Rewrite `carg_pair` with `Assumptions::csign` + `Expr::Atan2(v, u)` (fixing all `Re(z) < 0` quadrants); fix `cabs` to preserve `abs(x)` unless proven non-negative; guard `smart_div`/`smart_pow`; fix symbolic-exponent `pow_pairs`; implement `logarc`-based `rectform` (or nouns) for inverse trig; determine pole order dynamically in `residue`.
  3. **Polynomials & Rational Functions (`sym-core/src/poly/*`)**:
     - Fix `Poly::scale(0)` normalization, return `Result<Fraction, PolyError::DivisionByZero>` from `Fraction::div`, and use checked arithmetic in `Monomial`.
     - Implement `poly_derivative(p, var_idx)`, multivariate exact division, and multivariate primitive/subresultant (or modular) GCD over $\mathbb{Z}[x_1, \dots, x_n]$.
     - Implement Yun's square-free factorization + Berlekamp/Cantor-Zassenhaus + Hensel lifting for univariate $\mathbb{Z}[x]$ `factor_poly` (fixing `i64` overflow in divisors).
     - Upgrade `partfrac` for unreduced inputs, improper fractions (long division), repeated poles, and irreducible quadratics (`1/((x-1)^2*(x^2+1))`).
     - Upgrade `ratsimp` with transcendental/radical kernel mapping (`sin(x)`, `exp(x)`, `sqrt(x)`) and `expand` with negation distribution and recursion into function arguments.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p maxima-tests --test assume_deduction --test complex_analysis --test poly_expand --test poly_rat
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- **HARD STOP:** Verify zero `Poly::scale(0)` / `DivisionByZero -> 0` bugs and all 4 `carg` quadrants match Maxima `rtest_carg.mac` before Phase 49.

---

### Phase 49: Calculus Engine Overhaul (`diff`, `series`, `limit`, `integrate`, `defint`)
**Objective:** Replace broken/stubbed calculus routines with verified algorithms aligned with Maxima 5.50 (`comm.lisp`, `hayat.lisp`, `limit.lisp`, `sin.lisp`, `risch.lisp`, `defint.lisp`).

- **Tasks:**
  1. **Differentiation (`sym-core/src/diff.rs`)**: Implement `diff(u!, x) = u! * psi(0, u + 1) * du` (or noun without `specfn`), FTC `d/dx(∫ f(x) dx) = f(x)`, differentiation inside constant-bound `Sum`, and `0^0`/`0^(-n)` guards.
  2. **Series (`sym-core/src/series.rs`)**: Fix Laurent truncation order math in `mul`, `inv`, `diff`; fix symbolic-center expansions (`sin(x)` at `%pi`) and `fold` cancellation; add `Tan`, `Sqrt`, fractional/symbolic `Pow`, hyperbolic/inverse trig, and logarithmic/Puiseux terms at `0`.
  3. **Limits (`sym-core/src/limit/*`)**: Fix `is_zero_equivalent` (`s.coeffs.is_empty()`), replace hardcoded `"t"` with gensym, return `inf`/`minf`/`und`/`ind` for `s.valuation < 0` / oscillations, add `LimitDir::{Both, Plus, Minus}`, implement real Gruntz MRV (`mrv.rs`) + L'Hôpital, and remove/tag unsound numeric fallbacks per ADR-0012.
  4. **Integration & Definite Integration (`sym-core/src/integrate/*`)**:
     - Fix `integrate_power_denominator` (`n < 2`), `is_proven_non_elementary` (`exp(0*x^2)`), `integrate_product` (`Assumptions` + expansion), and `ibp::trig_powers` polynomial-size recurrence.
     - Replace stubs in `hermite.rs` (true Hermite reduction), `lrt.rs` (Lazard-Rioboo-Trager for `1/(x^2+1)` & higher irreducible denominators), `sin.rs` (`tan`, `tanh`, `sqrt`, `asin/acos/atan`, `c^x`, `sin(a*x+b)`), and `risch.rs` (bidirectional `proportional_constant` + rational/exp/log tower).
     - Fix `defint.rs`: call `.reduce()` before pole scan; detect irrational (`1/(x^2-2)`), transcendental (`tan(x)`), and endpoint (`1/x` on `[0,1]`) poles; check `contains_integral(&f)`; apply `ln|u|` consistently and verify `minf` bounds.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p maxima-tests --test differentiation --test series --test limits --test integrate_elementary --test integrate_risch --test integrate_defint --test integrate_correctness
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- **HARD STOP:** Verify 500+ randomized `d/dx(∫ f dx) == f` differential proptests and zero false `ProvenNonElementary` / false convergent pole integrals before Phase 50.

---

### Phase 50: Solvers, Linear Algebra, Special Functions, Transforms & Web Service Wiring
**Objective:** Repair solver/matrix/Laplace defects and expose every `sym-core` mathematical capability via `maxima-web`.

- **Tasks:**
  1. **Algebraic & ODE Solvers (`sym-core/src/solve/*`)**:
     - Fix `ode_second.rs`: fold `disc` via `Poly`/`expand` before matching `Expr::Const`, handle `c == 0` (`_C1 + _C2*x`), normalize leading sign, support symbolic discriminants via `Assumptions`.
     - Fix `isolate.rs` (`a - var = 0` sign, reject circular `rhs`, handle `Add`/`Sub`).
     - Upgrade `alg.rs`: return complex roots (`x^2 + 1 -> [%i, -%i]`), parametric equations (`a*x + b -> [-b/a]`), `0 = 0` (`All`) vs `Empty`, cubic Cardano / biquadratic formulas, and `linsolve`.
     - Expand `ode.rs`/`ode_first.rs`: separable, integrating-factor linear (`y' + p(x)*y = q(x)`), exact, and non-homogeneous 2nd-order ODEs.
  2. **Linear Algebra (`sym-core/src/matrix/*`)**: Run Bareiss over `Poly` with Phase 48 multivariate exact division + kernel mapping, fix `is_expr_zero`, upgrade `adjoint` to $O(n^3)$ Gauss-Jordan, guard `den == 0` in `decode_expr`, and return complex/symbolic `eigenvalues` with multiplicities.
  3. **Special Functions & Transforms (`sym-core/src/specfn/*`, `transform/*`)**:
     - Add `Expr::Laplace`/`Expr::Ilt` nouns (or `Result<Expr, TransformError>`) so unmatched inputs never fabricate `1/s`, `1/(s^2+1)`, or `f(t)/s`.
     - Fix `ilt_rational` (`s`-independence check, Phase 48 `partfrac`, `A*s+B`, `BigInt` factorial); fix `bvp.rs` (`ic1`/`ic2`/`bc2` simplification before singularity checks).
     - Cap `orthopoly` degree (`<= 100`) and wire numerical `f64`/`bfloat` evaluation for `gamma`, `erf`, `erfc`, `bessel_j` (or document exact scope explicitly).
  4. **Full Web Service & Embedded CAS Dispatch (`maxima-web/src/service.rs`)**:
     - Expose `laplace`, `ilt`, `ic1`, `ic2`, `bc2`, `bfloat`, `assume`, `is`, `csign`, `adjoint`, `transpose`, and `orthopoly` in `OPERATIONS` and `run_operation`.
     - Wire all CAS operations into `parse_maxima_cas` auto-dispatch and repair `verify_integral`, `verify_defint`, and `verify_ode` (full AST substitution, symbolic params, `minf`, multi-point checks).

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p maxima-tests --test solvers_ode --test solvers_alg --test matrix_bareiss --test transform_laplace --test specfn --test webui_e2e
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- **HARD STOP:** Verify every `sym-core` engine is reachable via both `/api/compute` and Maxima command strings with zero bogus fallback formulas before Phase 51.

---

### Phase 51: Newbie-First UI/UX Overhaul, Stateless Session Context & Accessibility
**Objective:** Deliver an intuitive beginner WebUI with 12 starter examples, visual `error_span` Fix-It hints, WCAG AA accessibility, and stateless client-carried `session_context` (ADR-0014).

- **Tasks:**
  1. **Stateless Session Continuity (`maxima-web/src/api.rs`, `app.rs`, `service.rs`)**:
     - Implement bounded, schema-validated client-carried `session_context` JSON on `/api/compute` supporting `%`, `%oN`, variable bindings (`a: expr`), `assume`/`facts`/`forget`, and `kill(all)` (enforcing strict entry-count & 64 KB payload caps).
     - Thread session `Assumptions` into `integrate_with_assumptions`, `limit_with_assumptions`, `defint`, and `csign`.
  2. **Newbie Onboarding & Guided Input (`maxima-web/static/`)**:
     - Replace blank initial notebook with an Interactive Empty-State Onboarding panel featuring 12 categorized click-to-run starter cards + Quick Syntax Card.
     - Add mode-aware Inline Syntax Hint Bar and LaTeX Auto-Detect Banner (`\frac`, `\sin` in Maxima mode → 1-click switch).
     - Render `error_span` with escaped monospace caret (`^~~~`) and 1-click Fix-It suggestions (`2x`, `x**2`, `pi/e/i`, `=` in solve, `sin x`, unbalanced parens).
     - Move `Fast-Math` and `fpprec` into a collapsible Advanced Settings drawer with plain-English tooltips; add `CATEGORY_NAMES` for `matrix`, `complex`, `transform`, `assume`.
  3. **Notebook Workflow, Keyboard & Accessibility**:
     - Add per-cell Edit & Re-run, Delete, Export (`.md`/`.tex`), mode-aware Use as input, active-tab Copy, `Shift+Enter`/`Ctrl+Enter` inside param inputs, `Escape` modal close, focus trapping in `#rulesModal`, `<label for>`, `aria-live="polite"`, responsive mobile layout (`<= 768px`), and WCAG AA `>= 4.5:1` contrast.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p maxima-web
  cargo test -p maxima-tests --test webui_e2e
  E:\Anaconda\python.exe scripts/verify_css_classes.py   # or python3 on Linux/macOS
  ```
  Plus browser/DOM E2E checks: all 12 starter examples execute cleanly without param wiping; LaTeX auto-detect + Fix-It + `session_context` (`a: ...` → `diff` → `integrate(%)` → `assume` → `kill(all)`) + XSS & A11y checks pass 100%.
- **HARD STOP:** Verify all 12 starter examples and accessibility checks pass in headless browser tests before Phase 52.

---

### Phase 52: Maxima 5.50 Language Parity & Cooperative Compute Cancellation
**Objective:** Close core Maxima 5.50 syntax/command gaps and eliminate thread/permit leaks on timeouts via `CancelToken` (ADR-0014).

- **Tasks:**
  1. **Cooperative Cancellation (`sym-core`, `maxima-web`)**:
     - Introduce `CancelToken` (`Arc<AtomicBool>`) checked inside `egg` saturation runner hooks and `series`, `poly`, `integrate`, `bareiss`, and `solve` loops.
     - On `tokio::time::timeout` in `maxima-web/src/app.rs`, flip `cancel.store(true, Ordering::Relaxed)` so the worker aborts promptly and releases its `OwnedSemaphorePermit`. Configure `>= 8 MB` blocking worker stacks.
  2. **Maxima 5.50 Core Syntax & Function Parity**:
     - Parser (`sym-core/src/parser/maxima.rs`): Support `**`, `lhs = rhs` equations, `[a, b, c]` lists, `sec`/`csc`/`cot`/`asec`/`acsc`/`acot`/`asinh`/`acosh`/`atanh`/`atan2`/`abs`, `diff(expr, x, n)`, and noun quote `'`.
     - Core CAS + `maxima-web`: Implement `trigexpand`, `trigreduce`, `trigsimp`, `radcan`, `demoivre`, `exponentialize`, `subst`, `float`, `divide`/`quotient`/`remainder`, `num`/`denom`, finite `sum`/`product` evaluation, and directional `limit(expr, x, 0, plus/minus)`.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo test -p maxima-tests --test web_hardening --test parser_maxima --test maxima_cas_parity
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- **HARD STOP:** Verify real `150ms` timeout (`concurrency: 1`) releases its permit within `250ms` (10/10 runs) and adversarial inputs (`2^2^...` depth 1000, `1e1000000000`, `taylor` order 10000, `orthopoly` degree 1000) never panic or OOM before Phase 53.

---

### Phase 53: Automated Maxima 5.50 `.mac` Golden Harness, Fuzzing & Release Readiness
**Objective:** Establish automated 1:1 Maxima 5.50 parity measurement (`PARITY_SCOREBOARD.md`), recursive AST fuzzing, JSON schema contracts, and full ADR-0012 documentation synchronization.

- **Tasks:**
  1. **Upstream `.mac` Golden Harness (`tests/integration/golden_parity.rs`)**: Parse and execute `>= 500` real test pairs from `References/maxima-master/tests/` and `References/maxima-5.50.0/tests/` (`rtest1.mac`, `rtest_trig.mac`, `rtest_integrate.mac`, `rtest_limit.mac`, `rtest_taylor.mac`, `rtest_carg.mac`, `rtest_sign.mac`, `rtest_laplace.mac`, `rtestode.mac`, `rtest_algsys.mac`), generating `docs/audits/PARITY_SCOREBOARD.md` with exact `Passed / Failed / Unsupported` counts.
  2. **Recursive AST Proptest Fuzzing (`tests/proptest/*.rs`)**: Replace fixed templates with recursive AST strategies (`PROPTEST_CASES=500`), assert root-count completeness in `solve_correctness.rs`, and eliminate silent `if let Some` passes.
  3. **JSON Contract Tests & Full CI Gate**: Validate JSON schemas for `/api/{operations,rules,functions,parse,compute,health}` and `400` error bodies; verify end-to-end CI (`fmt`, `clippy`, `test`, full 19-feature isolation matrix, JS syntax, CSS verification, Playwright UI E2E, `docker build --locked`).
  4. **Documentation & ADR-0012 Truth Alignment**: Sync `README.md` version (`Cargo.toml` `0.2.0`), align `README.md` status badges (`✅` vs `◐`), replace all legacy `file:///` links across `README.md` and `SESSION_LOG.md` with relative repo links, update stale `agents.md` references to `AGENTS.md`, and verify zero doc/code contradictions.

- **Passing Gate (ALL must pass with exit code 0):**
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  cargo test -p sym-core --all-features --doc
  ```
  Plus full CI suite (19-feature matrix, JS syntax, CSS parity, Playwright E2E, `docker build .`), `docs/audits/PARITY_SCOREBOARD.md` generated (`>= 500` cases), and zero `file:///` links or doc/code contradictions.

---

### Master Phase Passing Gate Summary (Phases 46–53)

| Phase | Focus Area | Primary Verification Commands | Hard Exit Criteria |
| :---: | :--- | :--- | :--- |
| **46** | Security (XSS), CI/Docker, Cargo & WebUI Contracts | `cargo test --workspace --all-features`, `node --check maxima-web/static/js/*.js`, CSS check, `docker build .` | 0 XSS sinks; CI/Docker/`[workspace.dependencies]`/`tensor` fixed; 0 param wipes; Rules, `evaluate`, `float_type`, `fast_math`, TOML config, `/api/parse` verified |
| **47** | E-Graph Soundness, Numbers, Parsers, Codegen & Macros | `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0/0` & `0^0` safe; `SimpFlags` active; `Rat`/`bfloat` safe; UTF-8 `parser/latex` safe; `codegen` & `sym-macros` (`f32`, `fast_math`, arity) verified |
| **48** | Assumptions, Complex & Multivariate Poly/Rat Engine | `cargo test -p maxima-tests --test assume_deduction --test complex_analysis --test poly_expand --test poly_rat` | DAG transitivity & `csign` sound; `Atan2`/`Abs` in `carg`/`cabs`; multivariate `gcd`, full `factor`, full `partfrac`, kernel `ratsimp` pass |
| **49** | Calculus Engine (`diff`, `series`, `limit`, `integrate`, `defint`) | `cargo test -p maxima-tests --test differentiation --test series --test limits --test integrate_elementary --test integrate_risch --test integrate_defint` | `diff(x!)` exact; Laurent precision exact; `is_zero_equivalent` & Gruntz MRV sound; Hermite + LRT + Risch + `defint` pole checks pass |
| **50** | Solvers, Bareiss, SpecFn, Laplace & Service Wiring | `cargo test -p maxima-tests --test solvers_ode --test solvers_alg --test matrix_bareiss --test transform_laplace --test specfn --test webui_e2e` | `ode_second` (`b!=0`), `alg` complex/parametric, $3\times 3+$ multivariate Bareiss, `laplace`/`ilt` nouns, and full WebUI dispatch pass |
| **51** | Newbie UI/UX, Stateless `session_context` & A11y | `cargo test -p maxima-tests --test webui_e2e`, Playwright E2E + A11y suite | 12 starter examples pass in browser; `error_span` + Fix-It working; bounded client `session_context` tested; WCAG AA & ARIA pass |
| **52** | Maxima 5.50 Language Parity & Cooperative Cancellation | `cargo test -p maxima-tests --test web_hardening --test parser_maxima --test maxima_cas_parity` | Real 150ms timeout releases permit within 250ms (10/10 runs); `**`, `=`, lists, `trigexpand`/`trigreduce`/`radcan`/`subst`, directional `limit` pass |
| **53** | Upstream `.mac` Golden Harness, AST Fuzzing & Release | Full `.github/workflows/ci.yml` + `PROPTEST_CASES=500 cargo test -p maxima-tests --test '*'` | `>= 500` Maxima `.mac` cases in `PARITY_SCOREBOARD.md`; 0 `file:///` links; 0 ADR-0012 doc/code contradictions |

---

## 10. Phase Sign-Off Report Template

At the conclusion of each phase, compile `docs/audits/PHASE{N}_REPORT.md` before advancing:

```markdown
# Phase {N} Sign-Off Report: {PHASE_TITLE}

## 1. Summary of Deliverables & Changes
- `path/to/file.rs`: Concise description of deliverables, fixes, or algorithms added.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P{N}-01 | `...` | Precedence / XSS / Math / Isolation | `...` | `...` |

## 3. Mandatory Edge-Case Coverage Checklist
- [ ] Numerical & Constants (`NaN`, `Inf`, `-0.0`, `0/0`, `0^0`, scientific notation)
- [ ] Operator Precedence & Codegen (`-x^2 == -9`, `cos(-x)`)
- [ ] Cardinality & Boundaries (`0`, `1`, `2`, `>8` vars, `taylor <= 64`, `orthopoly <= 100`)
- [ ] Domain Algorithms (damped ODEs, `carg` quadrants, poles, non-elementary, nouns)
- [ ] Parsers & I/O (UTF-8, recursion depth `256`, empty input, CRLF/BOM)
- [ ] Concurrency, Security & Timeouts (`CancelToken`, owned permit recovery, XSS, `413`)
- [ ] WebUI & Persistence (12 starters, `session_context`, CSS parity, JS syntax)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo check --workspace --all-targets` | `0` | `0` | Passed |
| `cargo test --workspace --all-features` | `0` | `0` | X passed; 0 failed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |

## 5. Git Checkpoint Verification
- **Local Commit SHA / Message**: `git commit -m "feat: complete Phase {N} - <summary>"`
- **Push Status**: Deferred pending explicit user confirmation (or executed with exit code `0`)
```

---

## 11. Review & Verification Checklist for Agents

Before completing any task or requesting sign-off, verify:
- [ ] `cargo fmt --all -- --check` exits `0` with zero formatting diffs.
- [ ] `cargo check --workspace --all-targets` exits `0` with 0 errors and 0 warnings.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits `0`.
- [ ] `cargo test --workspace --all-features` passes 100% with zero ignored workarounds.
- [ ] `cargo test -p sym-core --all-features --doc` passes all doctests.
- [ ] Web assets pass `node --check` on every `maxima-web/static/js/*.js` file and `verify_css_classes.py` exits `0`.
- [ ] **Zero unwrap policy**: Zero `.unwrap()`, `.expect()`, or `panic!()` in non-test paths.
- [ ] **Zero fake verification**: Every `verified: true` flag is backed by dynamic runtime verification.
- [ ] Mandatory Edge-Case Test Matrix adhered to for all modified modules.
- [ ] `SESSION_LOG.md` updated with honest phase status and `docs/audits/PHASE{N}_REPORT.md` compiled.
- [ ] Local `git commit` checkpoint recorded for the completed phase (`git push` gated on user confirmation).
