# Phase 41 Sign-Off Report: Complex Analysis, Rectangular Form & Residues (`complex`)

## 1. Summary of Deliverables & Changes
- `sym-core/Cargo.toml`, `maxima-rs/Cargo.toml`, `tests/Cargo.toml`:
  - Added feature flag `complex = ["poly", "series", "diff"]` to `sym-core/Cargo.toml` and wired into `full`.
  - Added feature flag `complex = ["sym-core/complex"]` to `maxima-rs/Cargo.toml` and wired into `full`.
  - Registered integration test `complex_analysis` in `tests/Cargo.toml`.
- `sym-core/src/expr.rs`:
  - Added zero-allocation unit atom `Expr::I` gated by `#[cfg(feature = "complex")]` (ADR-0010).
  - Added constructor `Expr::i()`.
  - Integrated `Expr::I` into core AST inspection methods: `is_const`, `collect_vars`, `eval` (returns `None`), `node_count` (1), and `Display` (`"%i"`).
- `sym-core/src/latex.rs`:
  - Formatted `Expr::I` to render as LaTeX `"i"`.
- `sym-core/src/diff.rs`:
  - Implemented analytical calculus for `Expr::I`: $\frac{d}{dx}(i) = 0$.
- `sym-core/src/cse.rs`:
  - Extended CSE subexpression counting and variable substitution for `Expr::I`.
- `sym-core/src/codegen.rs`:
  - Emitted native complex representation `(0.0, 1.0)` for `Expr::I`.
- `sym-core/src/matrix/mod.rs`:
  - Integrated `Expr::I` into lossless matrix symbol serialization: `"II"`.
- `sym-core/src/limit/mod.rs`:
  - Handled `Expr::I` in limit variable substitution `subst_var`.
- `sym-core/src/lang.rs`:
  - Mapped `Expr::I` cleanly to existing `MathLang::NamedConst(Symbol::from("i"))` without breaking e-graph language variants.
- `sym-core/src/bfloat.rs`:
  - Preserved `Expr::I` in arbitrary-precision evaluation `bfloat_eval`.
- `sym-core/src/parser/maxima.rs`:
  - Parsed `"%i"` as `Expr::i()` when `feature = "complex"` is enabled.
- `sym-core/src/series.rs`:
  - Allowed series expansion of expressions independent of the expansion variable (such as constants `i`, `\pi`, `e`, symbolic constants) as constant series terms.
- `sym-core/src/complex/mod.rs`:
  - Implemented comprehensive complex analysis module citing Maxima 5.50 `src/rpart.lisp` and ADR-0010:
    - Smart constructors: `smart_add`, `smart_sub`, `smart_neg`, `smart_mul`, `smart_div`, `smart_pow`, `smart_sqrt`, `smart_sin`, `smart_cos`, `smart_sinh`, `smart_cosh`.
    - Exact $\pi$-multiple angle detector and trigonometric evaluator for integer and fractional multiples of $\pi$ ($\pi/2, \pi, 3\pi/2, 2\pi$).
    - `rectform(expr) -> (Expr, Expr)`: Canonical decomposition into real and imaginary parts $u + i v$ (variables assumed real).
    - Cyclic powers of $i$: $i^0=1, i^1=i, i^2=-1, i^3=-i, i^4=1, i^{-1}=-i, i^{-2}=-1$.
    - Complex arithmetic: addition, subtraction, multiplication, and division.
    - Euler's identity: $\exp(u + iv) = e^u(\cos v + i\sin v)$, producing exact values for $\exp(i\pi) = -1$, $\exp(i\pi/2) = i$, $\exp(-i\pi/2) = -i$, $\exp(2i\pi) = 1$.
    - Complex logarithm: $\ln(u + iv) = \frac{1}{2}\ln(u^2+v^2) + i\operatorname{atan2}(v, u)$, producing exact values for $\ln(-1) = i\pi$, $\ln(i) = i\pi/2$.
    - Complex trigonometric and hyperbolic functions: $\sin(iv) = i\sinh(v)$, $\cos(iv) = \cosh(v)$, $\sinh(iv) = i\sin(v)$, $\cosh(iv) = \cos(v)$, $\tan$, $\tanh$.
    - Complex conjugate: $\operatorname{conjugate}(u + iv) = u - iv$.
    - Modulus and argument: $\operatorname{cabs}(expr) = \sqrt{u^2+v^2}$, $\operatorname{carg}(expr) \in (-\pi, \pi]$.
    - Polar form: $\operatorname{polarform}(expr) = (r, \theta)$, $\operatorname{polarform\_expr}(expr) = r \cdot e^{i\theta}$.
    - Laurent series residue extraction: $\operatorname{residue}(expr, var, point) -> Result<Expr, String>$ using series expansion up to order 6 to extract coefficient of $(z - z_0)^{-1}$.
- `maxima-rs/src/prelude.rs` & `sym-core/src/lib.rs`:
  - Re-exported `cabs`, `carg`, `conjugate`, `imagpart`, `polarform`, `polarform_expr`, `realpart`, `rectform`, `rectform_expr`, `residue` under `#[cfg(feature = "complex")]`.
- `maxima-web/src/service.rs`:
  - Registered 8 operations in `OPERATIONS` catalog: `"rectform"`, `"polarform"`, `"realpart"`, `"imagpart"`, `"conjugate"`, `"carg"`, `"cabs"`, `"residue"`.
  - Added parameter spec `PARAM_RESIDUE` with `variable` defaulting to `"z"` and `point` defaulting to `"0"`.
  - Wired execution handlers in `match op_name` with JSON outputs and LaTeX formatting.
- `tests/integration/complex_analysis.rs`:
  - Added comprehensive integration test suite with 10 unit and integration tests covering all complex operations and web service compute dispatch.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P41-01 | `sym-core/src/bfloat.rs` | Compiler Error (E0004) | Non-exhaustive match in `bfloat_eval` missing `Expr::I`. | Added `#[cfg(feature = "complex")] Expr::I => expr.clone()`. |
| P41-02 | `sym-core/src/series.rs` | Incomplete Match | Subexpressions independent of series expansion variable fell through to `None`. | Handled subexpressions with `!expr.variables().contains(var)` as constant series terms. |
| P41-03 | `sym-core/src/complex/mod.rs` | Incomplete Folding | Unfolded `sin(0)` in $\sin(ix)$ produced $\sin(0)\cosh(x)$ rather than $0$. | Implemented `smart_sin`, `smart_cos`, `smart_sinh`, `smart_cosh` folding zero arguments and multiples of $\pi$. |
| P41-04 | `sym-core/src/complex/mod.rs` | Linter / Style Violation | Clippy flagged 21 `collapsible_if` and `manual_range_contains` warnings. | Fixed with idiomatic collapsed `if let ... && let` patterns and `(0..=64).contains(&n)`. |
| P41-05 | `tests/integration/complex_analysis.rs` | Linter / Style Violation | Unused imports `imagpart`, `polarform`, `realpart`. | Added explicit assertions testing `imagpart`, `polarform`, and `realpart`. |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Cyclic imaginary unit powers $i^k$ ($k \in \{-4, -3, -2, -1, 0, 1, 2, 3, 4\}$)
- [x] Euler's identity: $\exp(i\pi) = -1$, $\exp(i\pi/2) = i$, $\exp(-i\pi/2) = -i$, $\exp(2i\pi) = 1$
- [x] Complex branch cuts: $\ln(-1) = i\pi$, $\ln(i) = i\pi/2$
- [x] Trigonometric-hyperbolic imaginary rotations: $\sin(ix) = i\sinh(x)$, $\cos(ix) = \cosh(x)$, $\sinh(ix) = i\sin(x)$, $\cosh(ix) = \cos(x)$
- [x] Modulus and argument boundary values: $|-5| = 5$, $|5i| = 5$, $|3+4i| = 5$, $\operatorname{carg}(-5) = \pi$, $\operatorname{carg}(5i) = \pi/2$, $\operatorname{carg}(-5i) = -\pi/2$
- [x] Laurent series poles: simple pole $1/z$, double pole $1/z^2$, order-3 pole $e^z/z^3$, shifted pole $1/(z-3)$
- [x] Web compute API dispatch for all 8 complex operations
- [x] Zero undefined CSS classes verification (`verify_css_classes.py`)
- [x] Feature isolation: `sym-core` compiles cleanly with `--no-default-features`

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo test -p sym-core --features complex --lib` | `0` | `0` | 155 passed; 0 failed |
| `cargo test -p maxima-tests --test complex_analysis` | `0` | `0` | 10 passed; 0 failed |
| `cargo test -p maxima-tests --test matrix_bareiss` | `0` | `0` | 10 passed; 0 failed |
| `cargo test -p maxima-tests --test bfloat` | `0` | `0` | 9 passed; 0 failed |
| `cargo test -p maxima-web` | `0` | `0` | 28 passed; 0 failed |
| `E:\Anaconda\python.exe scripts/verify_css_classes.py` | `0` | `0` | 0 missing classes (93 used, 102 defined) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo check -p sym-core --no-default-features` | `0` | `0` | Clean compilation (Feature isolation intact) |
