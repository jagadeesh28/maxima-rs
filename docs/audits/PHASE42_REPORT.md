# Phase 42 Sign-Off Report: Special Functions & Table Dispatch (`specfn`)

## 1. Summary of Deliverables & Changes
- `sym-core/Cargo.toml` & `maxima-rs/Cargo.toml`: Added feature flag `specfn = ["diff", "poly"]` (bundled into `full`).
- `sym-core/src/expr.rs`: Added `Expr::SpecialFn(SpecialFnKind, Vec<Expr>)` enum variant and ergonomic constructors (`gamma`, `beta`, `erf`, `erfc`, `bessel_j`, `bessel_y`, `bessel_i`, `bessel_k`, `airy_ai`, `airy_bi`, `legendre_p`, `chebyshev_t`, `chebyshev_u`, `hermite_h`, `laguerre_l`). Handled in `collect_vars`, `eval`, `node_count`, and `Display`.
- `sym-core/src/specfn/mod.rs`: Created centralized special function kind registry (`SpecialFnKind`) and table-driven dispatcher `diff_special_fn`.
- `sym-core/src/specfn/gamma.rs`: Implemented exact evaluation for $\Gamma(z)$ (poles at non-positive integers, integer factorials, half-integers $\Gamma(1/2)=\sqrt{\pi}$), $\beta(a, b)$, $\text{erf}(0)=0$, $\text{erfc}(0)=1$, and analytical differentiation citing Maxima 5.50 `src/specfn.lisp`, `src/comm.lisp:481`, and `src/combin.lisp:41-43`.
- `sym-core/src/specfn/bessel.rs`: Implemented Bessel functions $J_\nu, Y_\nu, I_\nu, K_\nu$ with exact evaluations at $z=0$ and derivatives wrt argument citing Maxima 5.50 `src/bessel.lisp:55, 366, 693, 1021` and Abramowitz & Stegun 9.1.27, 9.6.29.
- `sym-core/src/specfn/airy.rs`: Implemented Airy functions $\text{Ai}, \text{Ai}', \text{Bi}, \text{Bi}'$ and differential equation derivatives ($w'' - z w = 0$) citing Maxima 5.50 `src/airy.lisp:29, 115, 181, 268`.
- `sym-core/src/specfn/orthopoly.rs`: Implemented 3-term recurrence polynomial generators and derivatives for Legendre $P_n$, Chebyshev $T_n/U_n$, Hermite $H_n$, and Laguerre $L_n$ citing Maxima 5.50 `share/orthopoly/orthopoly.lisp:489, 521, 558, 801, 885`.
- `sym-core/src/diff.rs`: Wired analytical derivatives via `crate::specfn::diff_special_fn(*kind, args, var)`.
- `sym-core/src/latex.rs`: Added precedence-aware formatting for all 18 special function kinds (`\Gamma`, `\mathrm{B}`, `\mathrm{erf}`, `J_\nu`, `\mathrm{Ai}`, `P_n`, `T_n`, etc.).
- `sym-core/src/cse.rs`, `codegen.rs`, `matrix/mod.rs`, `limit/mod.rs`, `bfloat.rs`: Handled `Expr::SpecialFn` across all passes and lossless matrix symbol codec.
- `sym-core/src/parser/maxima.rs`: Parsed special function calls in Maxima syntax.
- `tests/integration/specfn.rs`: Comprehensive test suite verifying exact values, table differentiation, differential equations, orthogonal polynomial generation, LaTeX formatting, and matrix symbol round-trip.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P42-01 | `sym-core/src/specfn/gamma.rs` | Type Mismatch | `Rat::div` returns `Option<BigRational>` rather than `BigRational` | Propagated via `?` operator |
| P42-02 | `sym-core/src/bfloat.rs` | Exhaustiveness | `match expr` in `bfloat_eval` did not cover `Expr::SpecialFn` | Added recursive mapping for special function arguments |
| P42-03 | `sym-core/src/specfn/orthopoly.rs` | Algebra Simplification | `2*k*h_prev` evaluated to `Expr::mul(2, 1)` when `h_prev.is_one()` | Added identity folding in 3-term recurrence generators |
| P42-04 | `sym-core/src/specfn/mod.rs` | Clippy Lint | `from_str` clashed with `std::str::FromStr::from_str` | Renamed to `from_name` and implemented `std::str::FromStr` |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical (`NaN`, `Inf`, `-0.0`, scientific notation, rational halves $\Gamma(1/2) = \sqrt{\pi}$)
- [x] Operator Precedence (`-x^2 == -9` at $x=3$, special function argument binding)
- [x] Cardinality (`0`, `1`, `2`, constant/duplicate arguments, degrees 0 to 4)
- [x] Domain Algorithms (poles at non-positive integers, Airy DE $w'' - x w = 0$, Bessel recurrence)
- [x] Parsers & I/O (recursion depth limit 256, empty input, whitespace, function call syntax)
- [x] Concurrency & Timeouts (stateless pure functions, thread-safe AST)
- [x] Zero-unwrap policy: 100% adhered to in library paths (no `.unwrap()` or `.expect()`)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo test -p sym-core --features specfn --lib` | `0` | `0` | 155 passed; 0 failed |
| `cargo test -p maxima-tests --test specfn` | `0` | `0` | 9 passed; 0 failed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo fmt --all -- --check` | `0` | `0` | Clean |
| `cargo check -p sym-core --no-default-features` | `0` | `0` | Clean |
