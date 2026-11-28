# Phase 50 Sign-Off Report: Solvers, Linear Algebra, Special Functions, Transforms & Web Service Wiring

## 1. Summary of Deliverables & Changes
- **Algebraic & ODE Solvers (`sym-core/src/solve/{mod,alg,isolate,ode,ode_first,ode_second}.rs`)**:
  - `ode_second.rs`: Folded `disc` via `poly::expand` before matching `Expr::Const`, handled `c == 0` (`_C1 + _C2*x` and `_C1 + _C2*exp(-(b/a)*x)`), normalized leading sign, and added symbolic discriminant support via `Assumptions::csign`.
  - `isolate.rs`: Fixed `a - var = 0` / `Add` / `Mul` / `Neg` / `Sqrt` isolation, handled `var` on either side, and rejected circular `rhs` (`exp(x) - x = 0`).
  - `alg.rs` & `solve/mod.rs`: Added complex quadratic roots (`x^2 + 1 -> [%i, -%i]`), parametric linear/quadratic equations (`a*x + b -> [-b/a]`), `SolveStatus::{Finite, All, Empty, Partial}` (`solve_classified`), cubic Cardano, biquadratic, and fraction-free `linsolve`.
  - `ode_first.rs` & `ode.rs`: Added variable-coefficient 1st-order integrating factor (`a(x)*y' + b(x)*y = q(x)` with `unwrap_ln_term`), separable and exact (`dM/dy == dN/dx`) 1st-order ODEs, and non-homogeneous 2nd-order ODEs via undetermined coefficients and trig-aware Variation of Parameters.
- **Linear Algebra (`sym-core/src/matrix/{mod,bareiss}.rs`)**:
  - Added kernel-mapped `Poly` Bareiss determinant (`bareiss_poly_det`) using Phase 48 multivariate exact division (`poly_div_exact`).
  - Enhanced `is_expr_zero` with kernel-mapped `Poly` zero checking.
  - Upgraded `adjoint` to $O(n^3)$ Bareiss-Jordan fraction-free elimination (`bareiss_jordan_adjoint_poly`) with cofactor fallback for singular matrices.
  - Guarded `den == 0` in `decode_expr` (`matrix/mod.rs`).
- **Assumptions (`sym-core/src/assume/mod.rs`)**:
  - Folded `(Expr::Const(ra), Expr::Const(rb))` in `csign` `Expr::Sub` so numeric comparisons (`is(5 > 2)`) evaluate to `True`.
- **Special Functions & Transforms (`sym-core/src/specfn/orthopoly.rs`, `sym-core/src/transform/{mod,laplace,bvp}.rs`)**:
  - Added `TransformError`, `try_laplace`, and `try_ilt` so unmatched inputs never fabricate `1/s`, `1/(s^2+1)`, or `f(t)/s` (returning formal integral nouns in `laplace`/`ilt`).
  - Fixed `ilt_rational` (`s`-independence check on `c/s`, `BigInt` factorial, general `A*s + B` linear numerator, Phase 48 `partfrac` decomposition).
  - Simplified `ic1`/`ic2`/`bc2` determinants and solutions via `ratsimp`/`expand` before singularity checks in `bvp.rs`.
  - Capped `orthopoly` degree (`<= 100`) in `get_poly_degree`.
- **Web Service & Embedded CAS Dispatch (`sym-core/src/parser/maxima.rs`, `maxima-web/src/service.rs`)**:
  - Exposed `laplace`, `ilt`, `ic1`, `ic2`, `bc2`, `bfloat`, `assume`, `is`, `csign`, `adjoint`, `transpose`, and `orthopoly` in `OPERATIONS` and `run_operation`.
  - Wired all new CAS commands into `CasCommand` (`parse_maxima_cas`, `execute_cas`) and `service.rs` auto-dispatch.
  - Repaired `verify_integral`, `verify_defint` (added `minf` quadrature), and `verify_ode` (`substitute_ode` full AST recursion).

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P50-01 | `sym-core/src/solve/*` | Solver Defects | Unfolded `disc` broke `ode_second` for `b!=0`, missing `y''=0` `x`, `isolate` circular/sign bugs, real-only `alg.rs`, missing `linsolve`/non-homog ODE | `expand(disc)`, `c==0`, `isolate` orientation/circularity, complex/parametric/`linsolve`, integrating factor/exact/separable/VoP; verified in `solvers_ode.rs` & `solvers_alg.rs` |
| P50-02 | `sym-core/src/matrix/*` | Linear Algebra | `determinant` lacked `Poly` Bareiss, $O(n^5)$ `adjoint`, `den==0` panic in `decode_expr` | Kernel-mapped `Poly` Bareiss + $O(n^3)$ Bareiss-Jordan `adjoint` + `den==0` guard; verified in `matrix_bareiss.rs` |
| P50-03 | `sym-core/src/{specfn,transform}/*` | Transforms & Specfn | Bogus `1/s`/`1/(s^2+1)`/`f/s` fallbacks, `ilt` `s`-leak & `i64` overflow, unsimplified `bvp` det, uncapped `orthopoly` | `TransformError` + `try_laplace`/`try_ilt`, `BigInt` fact + `partfrac` `ilt`, `bvp` simplification, `<= 100` cap; verified in `transform_laplace.rs` & `specfn.rs` |
| P50-04 | `maxima-web/src/service.rs`, `parser/maxima.rs` | WebUI & CAS Dispatch | 12 engines unexposed in `OPERATIONS`/auto-dispatch, shallow `verify_*` | Exposed all 12 ops + `CasCommand` auto-dispatch, repaired `verify_integral`/`verify_defint`/`verify_ode`; verified in `webui_e2e.rs` |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical & Constants (`BigInt` factorial in `ilt`, `csign(5 - 2)`, `bfloat` precision `1..=200`)
- [x] Operator Precedence & Codegen (`CasCommand` parsing for all 35+ CAS commands)
- [x] Cardinality & Boundaries (`orthopoly` degree `<= 100`, `den == 0` matrix decode guard)
- [x] Domain Algorithms (`x^2 + 1 = 0 -> [%i, -%i]`, `0 = 0` `All` vs `1 = 0` `Empty`, `y'' + 3y' + 2y = 0`, `y'' = 0`, `y' + y/x = 1`, `y'' + y = sin(2x)`, 3x3 multivariate Bareiss + adjoint, `try_laplace(sin(t^2)) -> Err`)
- [x] Parsers & I/O (`parse_maxima_cas` + `parse_relation` for `assume`/`is`)
- [x] Concurrency, Security & Timeouts (inherited from Phase 46)
- [x] WebUI & Persistence (all 12 new operations + `CasCommand` auto-dispatch verified in `webui_e2e.rs`)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo test -p maxima-tests --test solvers_ode --test solvers_alg --test matrix_bareiss --test transform_laplace --test specfn --test webui_e2e` | `0` | `0` | `solvers_ode` (5), `solvers_alg` (7), `matrix_bareiss` (11), `transform_laplace` (9), `specfn` (9), `webui_e2e` (19) all passed |
| `cargo test --workspace --all-features` | `0` | `0` | All workspace unit (`159` `sym-core`, `28` `maxima-web`), integration, proptest, and doctests (`17` + `3`) passed |

## 5. Git Checkpoint Verification
- **Local Commit**: Pending creation in final step of Phase 50
- **Push Status**: Deferred pending explicit user confirmation
