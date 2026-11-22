# maxima-rs — Agent Implementation Instructions (Comprehensive CAS Expansion)

## 0. Mandatory Guardrails & Existing Architecture Alignment
1. **Read Existing Context First:** Before editing code, read `agents.md`, `GLOSSARY.md`, and all ADRs in `docs/adr/` (`0001`–`0005`). Respect the e-graph core architecture (`sym-core/src/lang.rs`, `expr.rs`, `pipeline.rs`, `rational.rs`).
2. **Hybrid Architecture Rule:**
   - Keep `sym-core/src/rules/` and `saturate.rs` for algebraic simplification, trig/hyperbolic identities, and canonical extraction.
   - Implement **procedural CAS algorithms** (`expand`, `poly`, `integrate`, `limit`, `series`, `solve`) as dedicated modules in `sym-core/src/` that operate on `Expr` and invoke `pipeline.rs` for simplification and `diff.rs` for differentiation.
   - Document this hybrid split in a new ADR: `docs/adr/0006-hybrid-egraph-and-procedural-cas.md`.
3. **Merge, Never Overwrite:** Extend `Expr`, `sym-core/src/lib.rs`, `maxima-rs/src/lib.rs`, `maxima-rs/src/prelude.rs`, and `sym-core/src/parser/maxima.rs` in place. Do not break existing `sym-macros` or `tests/integration/` tests.
4. **Exactness & Verification:** Use `sym-core/src/rational.rs` for all exact polynomial/rational arithmetic. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` to full completion after every milestone.

---

## Milestone 1: Polynomial Engine & Expression Expansion (`sym-core/src/poly/`)
**Goal:** Add `expand`, `ratsimp`, `gcd`, `factor`, and `partfrac` without causing e-graph blowup.

### Tasks
1. Create `sym-core/src/poly/mod.rs` with a sparse multivariate polynomial representation (`Poly`) over exact rationals (`rational.rs`), plus bidirectional converters `Expr <-> Poly`.
2. **Implement `expand(expr: &Expr) -> Expr`:**
   - Recursively distribute products over sums (`a*(b + c) -> a*b + a*c`) and expand positive integer powers `(A + B)^n` via binomial/multinomial expansion.
   - Do **not** run `extract.rs` with the default AST-size cost function on the output of `expand` (otherwise it will re-factor the result).
3. **Implement Rational Algebra (`sym-core/src/poly/rat.rs`):**
   - `poly_gcd`: Subresultant PRS GCD over polynomials.
   - `ratsimp`: Convert `Expr` to $P/Q$, reduce by $\gcd(P, Q)$, and convert back to `Expr`.
   - `square_free`: Yun's square-free factorization.
   - `factor`: Berlekamp / Cantor-Zassenhaus over $\mathbb{F}_p$ with Hensel lifting to $\mathbb{Z}[x]$.
   - `partfrac`: Partial fraction decomposition via extended Euclidean algorithm over polynomials.
4. **Wire Up Public API & Parser:**
   - Export `expand`, `ratsimp`, `factor`, `partfrac` in `sym-core/src/lib.rs`, `maxima-rs/src/lib.rs`, and `maxima-rs/src/prelude.rs`.
   - Add Maxima syntax support in `sym-core/src/parser/maxima.rs`.
5. **Tests:** Add `tests/integration/poly_expand.rs` testing `expand((x + y)^4)`, `ratsimp((x^2 - 1)/(x - 1))`, and `partfrac(1/(x^2 - 1), x)`.

---

## Milestone 2: Symbolic Integration (`sym-core/src/integrate/`)
**Goal:** Implement indefinite (`integrate`) and definite (`defint`) integration, verified automatically by `sym-core/src/diff.rs`.

### Tasks
1. Create `sym-core/src/integrate/mod.rs` with an unevaluated `Integrate(Box<Expr>, String)` noun fallback when no elementary antiderivative exists.
2. **Tier 1 — Moses' SIN Heuristics (`sym-core/src/integrate/sin.rs`):**
   - **Linearity & Constant Pulling:** Split sums and factor out constants independent of the integration variable.
   - **Derivative-Divides ($u$-substitution):** Use `sym-core/src/diff.rs` to detect $c \cdot f(u(x)) \cdot u'(x)$ patterns.
   - **Rational Functions (`sym-core/src/integrate/rational.rs`):** Use Milestone 1's `poly` engine to implement Hermite reduction + Rothstein-Trager algorithm.
   - **Pattern Rules & IBP:** Elementary trig/hyperbolic/exp/log integrals and LIATE-ordered Integration by Parts (with a strict recursion depth cap to prevent infinite loops).
3. **Tier 2 — Transcendental Risch (`sym-core/src/integrate/risch.rs`):**
   - Implement differential field towers for elementary `exp` and `log` extensions.
   - Return the unevaluated `Integrate` noun form when Risch proves no elementary antiderivative exists (e.g., `exp(-x^2)`).
4. **Definite Integration (`sym-core/src/integrate/defint.rs`):**
   - Compute $F(b) - F(a)$ via FTC **only after** scanning $[a, b]$ for denominator poles/discontinuities.
   - Handle improper bounds (`inf`, `minf`) once Milestone 3's `limit` is available.
5. **Tests:**
   - Add `tests/integration/integration.rs` covering polynomials, rational functions, trig/exp/log, IBP, and non-elementary integrals.
   - Add `tests/proptest/integrate_correctness.rs` mirroring `tests/proptest/diff_correctness.rs`: for every successful `F = integrate(f, x)`, assert `ratsimp(diff(F, x) - f) == 0` AND verify numerically via `tests/proptest/numerical_equivalence.rs`.

---

## Milestone 3: Power Series & Limits (`sym-core/src/series.rs`, `sym-core/src/limit.rs`)
**Goal:** Enable asymptotic analysis, Taylor expansions, and improper definite integrals.

### Tasks
1. **Series (`sym-core/src/series.rs`):** Implement truncated Taylor/Laurent series arithmetic (`+`, `-`, `*`, `/`, composition, `diff`, `integrate`) up to order $O(x^n)$.
2. **Limits (`sym-core/src/limit.rs`):** Implement **Gruntz's Algorithm** (MRV subexpression sets + asymptotic series expansion) for systematic limits of exp-log functions, including `inf`, `minf`, and one-sided limits (`plus`/`minus`).
3. **Wire `defint` to `limit`:** Enable improper definite integrals in `sym-core/src/integrate/defint.rs`.
4. **Tests:** Add `tests/integration/limits_series.rs` (e.g., `limit(sin(x)/x, x, 0) == 1`, `limit((1 + 1/x)^x, x, inf) == %e`, `taylor(sin(x), x, 0, 5)`).

---

## Milestone 4: Algebraic & Differential Solvers (`sym-core/src/solve/`)
**Goal:** Complete the core Maxima CAS feature set.

### Tasks
1. **Algebraic Solver (`sym-core/src/solve/alg.rs`):**
   - Closed-form linear, quadratic, cubic, and quartic solvers.
   - Factor-and-split using Milestone 1's `factor` for higher-degree polynomials.
   - Variable isolation for invertible elementary functions (`exp`, `log`, `sin`, `cos`, `tan`).
2. **ODE Solver (`sym-core/src/solve/ode.rs`):**
   - 1st-order ODEs: separable, linear (integrating factor), exact, Bernoulli.
   - 2nd-order constant-coefficient linear ODEs (homogeneous + undetermined coefficients).
3. **Tests:** Add `tests/integration/solvers.rs` verifying every solution by back-substitution into the original equation/ODE and simplifying to `0`.

---

## Milestone 5: Reference Parity & Coverage Expansion
1. Expand `tests/integration/maxima_coverage.rs` to pull benchmark cases from `References/maxima-master/tests/` (`rtest_integrate.mac`, `rtest_limit.mac`, `rtest_taylor.mac`, `rtest_solve.mac`).
2. Update `README.md` and `GLOSSARY.md` with the new public APIs and hybrid e-graph + procedural architecture.