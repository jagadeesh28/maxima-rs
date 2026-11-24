# Phase 38C Sign-Off Report: Engine Honesty, Dynamic Verification & Defect Remediation

## 1. Summary of Deliverables & Changes
- `sym-core/src/solve/ode.rs`:
  - Remediated 1st-order linear ODE coefficient extraction bug: When `order == 1`, correctly assigned `a = coeffs[1]` (coefficient of $y'$) and `b = coeffs[0]` (coefficient of $y$). Previously both `a` and `b` were extracted from `coeffs[1]`.
  - Verified with golden unit and integration tests including $y' + 2y = 0 \implies y = C_1 e^{-2x}$ and $2y' + 5y = 0 \implies y = C_1 e^{-5/2x}$.
- `sym-core/src/integrate/mod.rs`:
  - Introduced strongly-typed tri-state outcome `IntegrationResult` enum: `Solved(Expr)`, `ProvenNonElementary`, and `Unsupported(String)`.
  - Implemented `is_proven_non_elementary(expr, var)` detecting Liouville/Risch non-elementary integrands ($e^{-x^2}$, $e^{x^2}$, $\sin(x)/x$, $\cos(x)/x$, $1/\ln(x)$, $e^x/x$).
  - Added trigonometric power reduction rules for $\int \sin(u)^2 du$ and $\int \cos(u)^2 du$ so $\int \sin(x)^2 dx = x/2 - \sin(2x)/4$ integrates cleanly without returning unintegrated forms.
  - Implemented `integrate_result(expr, var)` and public re-exports.
- `sym-core/src/integrate/defint.rs`:
  - Handled logarithmic branch cuts on strictly negative intervals: For $[a, b]$ with $a < 0, b \le 0$, automatically transformed $\ln(x) \to \ln(-x)$ via `flip_ln_if_negative`, preserving real valuation without branch cut breakdown.
  - Added divergence detection when boundary limits tend to `inf` or `minf`.
  - Added zero and one identity reductions in `fold_const` (e.g. $0 - x \to -x$, $0 + x \to x$, $1 \cdot x \to x$, $0 \cdot x \to 0$).
  - Added integration test `test_defint_reciprocal_over_negative_interval` asserting $\int_{-3}^{-1} \frac{1}{x} dx = -\ln(3)$.
- `sym-core/src/series.rs`:
  - Remediated off-by-one Taylor/Laurent series truncation: expanded to order $+ 1$ so terms of degree `order` are preserved, and remainder is $O(x^{order+1})$.
- `sym-core/src/solve/alg.rs` & `sym-core/src/solve/mod.rs`:
  - Implemented polynomial root multiplicity reduction via synthetic division loop `while let Some(q) = poly_div_exact(&cur, &lin)`.
  - Updated `solve_polynomial` and `AlgebraicSolutions` to track whether factorization was complete (`solution_set.is_complete()`).
- `maxima-web/src/api.rs`:
  - Added `Status::Partial` enum variant to represent incomplete operations (e.g. higher-degree irreducible factors remaining).
- `maxima-web/src/service.rs`:
  - Wired `integrate_result`: dispatches `IntegrationResult::Solved(antideriv)` with `verify_integral`, `IntegrationResult::ProvenNonElementary` with `Status::Unevaluated` and Liouville theorem warning, and `IntegrationResult::Unsupported(reason)`.
  - Definite integral dynamic verification: Replaced hardcoded `verified = Some(true)` with `verify_defint` using composite Simpson quadrature and transformed improper midpoint integration.
  - ODE dynamic verification: Replaced hardcoded `verified = Some(true)` with `verify_ode` which substitutes $y$, $y'$, $y''$ into the ODE expression and verifies residual $|R| < 10^{-3}$ at sample points.
  - Root solver honesty: If roots are found but irreducible higher-degree factors remain, sets `resp.status = Status::Partial` with explanatory warning.
  - Taylor remainder labeling: Remainder term formats with base `format!("({v} - {center})")` when `center != 0`.
- `maxima-web/static/js/notebook.js` & `app.css`:
  - Added `case 'partial': return 'badge-partial';` in `notebook.js` and defined `.badge-partial` styling in `app.css`. Verified with `verify_css_classes.py` (0 undefined classes).

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P38C-01 | `sym-core/src/solve/ode.rs` | Math Algorithm Bug | `ode.rs:117` extracted both `a` and `b` from `coeffs[1]`, ignoring `coeffs[0]`. Thus $y' + 2y = 0$ solved as $y = C_1 e^{-x}$ instead of $y = C_1 e^{-2x}$. | Changed `b` extraction to `coeffs[0]`. Verified in `test_first_order_linear_with_distinct_coefficients` and `test_ode2_hard_stop_verification`. |
| P38C-02 | `maxima-web/src/service.rs` | Verification Dishonesty | Definite integration and ODE solving unconditionally returned `verified = Some(true)` without computation. | Implemented `verify_defint` (composite Simpson + improper quadrature) and `verify_ode` (residual back-substitution across sample points). Never claims verified unless dynamically proven. |
| P38C-03 | `sym-core/src/integrate/mod.rs` | Mathematical Conflation | Failed integrations simply returned unsimplified noun forms, with no distinction between mathematically proven non-elementary integrals vs engine limitations. | Introduced `IntegrationResult` tri-state outcome and `is_proven_non_elementary` based on Liouville's theorem. Verified in `test_integrate_proven_non_elementary`. |
| P38C-04 | `sym-core/src/integrate/defint.rs` | Real Domain Violation | $\int_{-3}^{-1} \frac{1}{x} dx$ evaluated $\ln(x)$ at negative points, failing on complex branch cuts. | Implemented `flip_ln_if_negative` on strictly negative bounds, yielding real $-\ln(3)$. Verified in `test_defint_reciprocal_over_negative_interval`. |
| P38C-05 | `sym-core/src/series.rs` | Off-by-One Truncation | Truncated expansion with `< order` dropped terms of degree `order`. | Expanded to `order + 1`, accurately keeping terms up to degree `order` with remainder $O((x - c)^{order+1})$. Verified in `test_taylor_nonzero_center`. |
| P38C-06 | `sym-core/src/solve/alg.rs` | Multiplicity & Partial Status | Multiple roots of polynomials were not factored out, and partial roots on irreducible higher degrees were indistinguishable from complete solutions. | Added root multiplicity reduction loop and `is_complete()` flag; updated web API with `Status::Partial`. Verified in `test_solve_partial_status`. |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] First-order linear ODE with distinct coefficients ($y' + 2y = 0$, $2y' + 5y = 0$)
- [x] Liouville non-elementary detection ($e^{-x^2}$, $\sin(x)/x$, $1/\ln(x)$)
- [x] Trigonometric power reduction ($\int \sin(x)^2 dx$)
- [x] Definite integration over strictly negative intervals ($\int_{-3}^{-1} \frac{1}{x} dx = -\ln(3)$)
- [x] Definite integral divergence detection on boundary poles
- [x] Taylor series expansion with non-zero center ($O((x - c)^{n+1})$)
- [x] Dynamic ODE residual verification at sample points
- [x] Dynamic definite integral composite Simpson quadrature verification
- [x] Partial status reporting on incomplete polynomial root factorization

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo test -p maxima-tests --test solvers_ode` | `0` | `0` | 4 passed; 0 failed |
| `cargo test -p maxima-tests --test integrate_defint` | `0` | `0` | 5 passed; 0 failed |
| `cargo test -p maxima-tests --test maxima_cas_parity` | `0` | `0` | 4 passed; 0 failed |
| `cargo test -p maxima-web` | `0` | `0` | 28 passed; 0 failed |
| `E:\Anaconda\python.exe scripts/verify_css_classes.py` | `0` | `0` | Zero undefined CSS classes (93 used, 102 defined) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| **Hard Stop Check:** `diff(y, x) + 2*y = 0` | `0` | `0` | Verified: `y = _C1 * exp(-2*x)` with `verified: Some(true)` |
