# Phase 48 Sign-Off Report: Assumptions, Complex Analysis & Multivariate Polynomial/Rational Engine Parity

## 1. Summary of Deliverables & Changes
- `sym-core/src/assume/mod.rs`:
  - Replaced raw `edges: Vec<(Expr, Comparison, Expr)>` with normalized `(Expr, bool, Expr)` strict/non-strict directed inequality edges + `not_equals: HashSet<(Expr, Expr)>`.
  - Rewrote `is()`, `apply_relation()`, `apply_property()` (with full incompatibility pairs), and `dag_path()` BFS tracking path strictness to handle `<`/`<=` transitivity, mixed strict/non-strict bounds, and strict-cycle contradiction rejection.
  - Updated `csign()` for `Exp`, `Pow`, and complex `Mul` (`exp(%i*%pi) -> Neg`, `%i^2 -> Neg`, `2^%i -> Complex`, `0^(-1) -> Unknown`, `%i*%pi -> Complex`).
- `sym-core/src/expr.rs` & cross-crate AST wiring (`latex.rs`, `diff.rs`, `lang.rs`, `cost.rs`, `cse.rs`, `codegen.rs`, `bfloat.rs`, `matrix/mod.rs`, `parser/maxima.rs`, `limit/{mod,mrv}.rs`, `integrate/mod.rs`, `transform/laplace.rs`):
  - Added `Expr::Abs(Box<Expr>)` and `Expr::Atan2(Box<Expr>, Box<Expr>)` variants, constructors, and exhaustive match wiring across `sym-core`.
- `sym-core/src/complex/mod.rs`:
  - Guarded `smart_div` (`b.is_zero()`) and `smart_pow` (`0^0` / `0^-n`), and fixed `pow_pairs` general fallback to avoid infinite recursion on `rectform(&Expr::Pow)`.
  - Rewrote `carg_pair` using `Assumptions::csign`, diagonal shortcuts, and `Expr::atan2` across all 4 quadrants.
  - Fixed `cabs` via `real_abs` (preserves `Expr::abs(x)` unless proven non-negative/negative).
  - Added `logarc`-based `rectform` for `Atan`, `Asin`, `Acos` on complex arguments (`v != 0`) and handlers for `Abs`/`Atan2`.
  - Upgraded `residue` to try dynamic expansion orders `[6, 12, 24]`.
- `sym-core/src/poly/{mod,monomial,expand,gcd,rat,factor,partfrac}.rs`:
  - `monomial.rs`: Added `checked_mul` and `checked_pow`; made `mul`/`pow` saturating.
  - `mod.rs`: Fixed `Poly::scale(0)` to return `Poly::zero(self.ctx.clone())` instead of leaving `0 * mono` terms; re-exported `poly_derivative_var`.
  - `expand.rs`: Updated `structural_expand` to distribute negation (`-(a+b)`, `-(a-b)`, `--a`) and recurse into function arguments (`Sin`, `Cos`, `Tan`, `Exp`, `Ln`, `Sqrt`, `Sinh`, `Cosh`, `Tanh`, `Asin`, `Acos`, `Atan`, `Abs`, `Atan2`).
  - `gcd.rs`: Implemented multivariate primitive recursive PRS `poly_gcd` over `Z[x_0..x_{n-1}]` (`multivar_gcd_var`, `primitive_part_var`, `pseudo_rem_var`, `split_by_var`, `content_in_var`) and `poly_derivative_var(p, var_idx)`.
  - `rat.rs`: Changed `Fraction::div` to return `Result<Fraction, PolyError::DivisionByZero>`; guarded `Pow` negative exponent on zero base; upgraded `ratsimp` with `map_kernels`/`unmap_kernels` for transcendental/radical kernels.
  - `factor.rs`: Rewrote with `divisors_u128`, `yun_square_free`, `factor_poly_list`, and `factor_square_free_cz` (Cantor-Zassenhaus over `F_p[x]` + linear Hensel lifting + subset recombination).
  - `partfrac.rs`: Rewrote with `f.reduce()`, long division for improper fractions, `factor_poly_list`, `poly_ext_gcd` Bezout splitting (`split_coprime_powers`), and repeated-factor base-`f_i` expansion.
- `tests/integration/{assume_deduction,complex_analysis,poly_expand,poly_rat}.rs`:
  - Added Phase 48 integration tests covering inequality transitivity, strict-cycle rejection, `csign`, all 4 `carg` quadrants, `cabs`, `rectform(2^%i)`, `rectform(atan(1+%i))`, `Poly::scale(0)`, `expand` negation/nesting, `DivisionByZero`, multivariate `poly_gcd`, kernel `ratsimp`, `factor(x^4+4)`, and repeated/improper `partfrac`.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P48-01 | `sym-core/src/assume/mod.rs` | Logic & Sign Soundness | Unnormalized `edges`, broken mixed strictness/cycle detection, `csign` missing `Exp`/`Pow`/complex `Mul` | Normalized `(Expr, bool, Expr)` DAG + `not_equals`, strict-cycle check, `csign` `Exp`/`Pow`/`Mul`; verified in `assume_deduction.rs` |
| P48-02 | `sym-core/src/{expr,complex/*}.rs` | Complex Branch Cuts & Residues | Missing `Abs`/`Atan2` AST nodes, `carg` quadrant collapse, `cabs(x)` dropping `abs`, `Atan`/`Asin`/`Acos` complex gaps, fixed residue order | Added `Expr::Abs`/`Atan2`, 4-quadrant `carg_pair`, `real_abs`, `logarc` inverse trig `rectform`, `[6, 12, 24]` residue orders; verified in `complex_analysis.rs` |
| P48-03 | `sym-core/src/poly/{mod,expand,gcd,rat,factor,partfrac}.rs` | Polynomial & Rational Parity | `Poly::scale(0)` non-zero, `Fraction::div` returning `num/0`, univariate-only `poly_gcd`/`factor`/`partfrac`, `expand` skipping negation/func args | Fixed `Poly::scale(0)`, `DivisionByZero` error, multivariate PRS `poly_gcd`, kernel `ratsimp`, Cantor-Zassenhaus + Hensel `factor`, Bezout + repeated `partfrac`; verified in `poly_expand.rs` & `poly_rat.rs` |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical & Constants (`csign(exp(%i*%pi)) == Neg`, `csign(0^(-1)) == Unknown`, `Poly::scale(0).is_zero()`, `divisors_u128`)
- [x] Operator Precedence & Codegen (`Expr::Abs`/`Expr::Atan2` wired in `latex`, `diff`, `lang`, `cost`, `cse`, `codegen`, `parser/maxima`)
- [x] Cardinality & Boundaries (monomial `checked_mul`/`checked_pow`, dynamic residue orders `[6, 12, 24]`)
- [x] Domain Algorithms (all 4 `carg` quadrants, `cabs(x) == abs(x)`, `rectform(2^%i)`, `rectform(atan(1+%i))`, `ratsimp(1/(x-x)) -> Err(DivisionByZero)`, multivariate `poly_gcd(x^2-y^2, x-y)`, `factor(x^4+4)`, repeated/improper `partfrac`)
- [x] Parsers & I/O (`abs`/`atan2` Maxima parsing & LaTeX printing)
- [x] Concurrency, Security & Timeouts (inherited from Phase 46)
- [x] WebUI & Persistence (inherited from Phase 46)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo test -p maxima-tests --test assume_deduction --test complex_analysis --test poly_expand --test poly_rat` | `0` | `0` | `assume_deduction` (9), `complex_analysis` (11), `poly_expand` (7), `poly_rat` (5) all passed |
| `cargo test --workspace --all-features` | `0` | `0` | All workspace unit (`159` `sym-core`), integration, proptest, and doctests (`17` + `3`) passed |

## 5. Git Checkpoint Verification
- **Local Commit**: Pending creation in final step of Phase 48
- **Push Status**: Deferred pending explicit user confirmation
