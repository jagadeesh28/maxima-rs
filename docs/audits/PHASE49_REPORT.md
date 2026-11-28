# Phase 49 Sign-Off Report: Calculus Engine Overhaul (`diff`, `series`, `limit`, `integrate`, `defint`)

## 1. Summary of Deliverables & Changes
- **Differentiation (`sym-core/src/diff.rs`)**:
  - Implemented `diff(u!, x) = u! * psi(0, u + 1) * du` (with `#[cfg(feature = "specfn")]`, or noun `Expr::derivative` without `specfn`).
  - Implemented Fundamental Theorem of Calculus `d/dx(∫ f(x) dx) = f(x)`.
  - Implemented differentiation inside constant-bound `Sum(f, i, lower, upper)` with summation-index shadowing protection.
  - Added `0^0` and `0^(-n)` guards in `smart_pow`.
- **Series (`sym-core/src/series.rs`)**:
  - Fixed Laurent truncation order math in `Series::mul` (`min(order_a + val_b, order_b + val_a)`), `Series::inv` (`order - 2*v`), and `Series::diff` (`order - 1`), with slack re-expansion in `expand` for `Div`/`Pow`/`Tan`/`Tanh`.
  - Upgraded `fold()` and `const_sin`/`const_cos`/`const_exp`/`const_ln`/`const_pow` for symbolic-center expansions (e.g. `sin(x)` at `%pi`) and exact cancellation.
  - Added `Tan`, `Sqrt`, fractional & symbolic `Pow`, `Sinh`, `Cosh`, `Tanh`, `Asin`, `Acos`, `Atan`, and Puiseux/logarithmic `taylor` expansions at `0`.
- **Limits (`sym-core/src/limit/{mod,mrv,zero}.rs`)**:
  - Fixed `is_zero_equivalent` to check `s.coeffs.is_empty()`.
  - Replaced hardcoded `"t"` with `fresh_var` gensym; added `LimitDir::{Both, Plus, Minus}` and `limit_dir`/`limit_dir_with_assumptions`.
  - Returned `inf`/`minf`/`und` for `s.valuation < 0` poles and `ind` for oscillatory trig at infinity.
  - Implemented Gruntz MRV comparability and asymptotic reduction (`mrv.rs`), `Ln` infinity/0+ handling, and L'Hôpital's rule (`lhopital_limit`), while removing the unsound numeric convergence fallback per ADR-0012.
- **Integration & Definite Integration (`sym-core/src/integrate/{mod,sin,ibp,hermite,lrt,risch,defint}.rs`)**:
  - `integrate/mod.rs`: Fixed `integrate_power_denominator` (`n < 2`), `is_proven_non_elementary` (`exp(0*x^2)`), and `integrate_product` (`Assumptions` propagation + `poly::expand`).
  - `ibp.rs`: Replaced exponential-size `trig_powers` recurrence with $O(n)$ polynomial-size tabular IBP.
  - `sin.rs`: Added `Tan`, `Tanh`, `Sqrt`, `Asin`, `Acos`, `Atan`, and `c^u` rules.
  - `hermite.rs` & `lrt.rs`: Replaced stubs with Yun square-free + Bezout Ostrogradsky-Hermite reduction and Lazard-Rioboo-Trager real `ln`/`atan` integration (`1/(x^2+1)`, `1/(x^2+1)^2`).
  - `risch.rs`: Added bidirectional kernel-mapped `proportional_constant` and `g' * f(g)` trig/hyperbolic/atan tower patterns.
  - `defint.rs`: Called `.reduce()` before pole scan, added irrational (`1/(x^2-2)`), transcendental (`tan(x)`), and endpoint (`1/x` on `[0,1]`) pole detection via `limit_dir`, checked `contains_integral(&f)`, and handled `minf`/negative `ln|u|` bounds.
- **Tests**:
  - Added Phase 49 integration tests in `tests/integration/{differentiation,series,limits,integrate_elementary,integrate_defint}.rs`.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P49-01 | `sym-core/src/diff.rs` | Calculus Soundness | `Factorial` treated as 0, missing FTC/Sum diff, `smart_pow(0,0)->1` | `u!*psi(0,u+1)*du`, FTC, constant-bound `Sum`, `0^0`/`0^-n` guards; verified in `differentiation.rs` |
| P49-02 | `sym-core/src/series.rs` | Truncation & Coverage | Wrong `mul`/`inv`/`diff` truncation orders, `fold` missing `%pi`/zero identities, missing `tan`/`sqrt`/hyperbolic/inv-trig | Exact Laurent orders, `pi_multiple` folding, all elementary & Puiseux/log `taylor`; verified in `series.rs` |
| P49-03 | `sym-core/src/limit/*` | Limit Soundness | `is_zero_equivalent` returned `true` for `x`, `"t"` collision, missing `inf`/`minf`/`und`/`ind` & `LimitDir`, numeric guesswork | `s.coeffs.is_empty()`, gensym, `LimitDir`, Gruntz MRV, L'Hôpital, removed numeric guess; verified in `limits.rs` |
| P49-04 | `sym-core/src/integrate/*` | Integration & Defint | `n<1` div-by-0, `exp(0*x^2)` false non-elementary, `ibp` $O(2^n)$ blowup, `hermite`/`lrt` stubs, unreduced/missing poles in `defint` | `n<2`, poly guard, $O(n)$ `ibp`, Hermite+LRT, `sin.rs`+`risch.rs`, `.reduce()`+irrational/trig/endpoint poles; verified in `integrate_*` |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical & Constants (`0^0`/`0^(-n)` in `diff`, `exp(0*x^2)` non-elementary guard, `%pi` symbolic series center)
- [x] Operator Precedence & Codegen (`Expr::Abs`/`Atan2` & `SpecialFnKind::Psi` differentiation)
- [x] Cardinality & Boundaries (Laurent `mul`/`inv`/`diff` truncation orders, $O(n)$ `ibp::trig_powers`, 1000-polynomial + proptest `integrate_correctness`)
- [x] Domain Algorithms (directional poles `1/x` -> `inf`/`minf`/`und`, `sin(x)` -> `ind`, L'Hôpital `ln(x)/x`, Hermite `1/(x^2+1)^2`, `defint` removable/irrational/transcendental/endpoint poles)
- [x] Parsers & I/O (inherited from Phase 47)
- [x] Concurrency, Security & Timeouts (inherited from Phase 46)
- [x] WebUI & Persistence (inherited from Phase 46)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo test -p maxima-tests --test differentiation --test series --test limits --test integrate_elementary --test integrate_risch --test integrate_defint --test integrate_correctness` | `0` | `0` | `differentiation` (4), `series` (10), `limits` (7), `integrate_elementary` (11), `integrate_risch` (6), `integrate_defint` (6), `integrate_correctness` (2) all passed |
| `cargo test --workspace --all-features` | `0` | `0` | All workspace unit (`159` `sym-core`), integration, proptest, and doctests (`17` + `3`) passed |

## 5. Git Checkpoint Verification
- **Local Commit**: Pending creation in final step of Phase 49
- **Push Status**: Deferred pending explicit user confirmation
