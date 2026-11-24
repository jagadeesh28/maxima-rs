# Phase 40 Sign-Off Report: Symbolic Linear Algebra & Bareiss Algorithm (`matrix`)

## 1. Summary of Deliverables & Changes
- `Cargo.toml`, `sym-core/Cargo.toml`, `maxima-rs/Cargo.toml`:
  - Added feature flag `matrix = ["poly", "solve"]` to `sym-core/Cargo.toml` and wired into `full`.
  - Added feature flag `matrix = ["sym-core/matrix"]` to `maxima-rs/Cargo.toml` and wired into `full`.
  - Added integration test `matrix_bareiss` in `tests/Cargo.toml`.
- `sym-core/src/matrix/mod.rs` & `sym-core/src/matrix/bareiss.rs`:
  - Refactored `matrix.rs` into `matrix/mod.rs` preserving e-graph lossless symbol serialization (`encode_matrix_symbol`, `decode_matrix_symbol`).
  - Created `sym-core/src/matrix/bareiss.rs` implementing fraction-free matrix elimination citing Erwin Bareiss (1968) and Maxima's `newdet.lisp` (ADR-0010).
  - Implemented `Matrix` struct with rows, cols, data, indexing, and row uniformity validation (`MatrixError::RaggedMatrix`).
  - Implemented matrix operations: `add`, `sub`, `mul`, `scale`, `transpose`, and `submatrix_excluding`.
  - Implemented `determinant(&Matrix) -> Result<Expr, MatrixError>`:
    - $O(n^3)$ Bareiss fraction-free LU algorithm.
    - Exact division by previous pivot at each stage via `simplify_div(num, prev_pivot)` using polynomial GCD cancellation (`ratsimp`) and rational constant division.
    - Zero pivot detection with row exchanges and determinant sign tracking.
  - Implemented `trace(&Matrix) -> Result<Expr, MatrixError>`: Sum of diagonal elements.
  - Implemented `charpoly(&Matrix, var) -> Result<Expr, MatrixError>`: Characteristic polynomial $\det(M - \lambda I)$ matching Maxima 5.50's `charpoly` in `newdet.lisp`.
  - Implemented `adjoint(&Matrix) -> Result<Matrix, MatrixError>`: Classical adjoint (adjugate) matrix using cofactor expansion with Bareiss determinants.
  - Implemented `invert(&Matrix) -> Result<Matrix, MatrixError>`: Inverse matrix via $M^{-1} = \frac{1}{\det(M)} \text{adj}(M)$ with singular matrix detection (`MatrixError::SingularMatrix`).
  - Implemented `eigenvalues(&Matrix) -> Result<Vec<Expr>, MatrixError>`: Exact roots of $\text{charpoly}(M, \lambda) = 0$ via `sym_core::solve::solve`.
- `maxima-rs/src/prelude.rs`:
  - Re-exported `Matrix`, `MatrixError`, `determinant`, `invert`, `charpoly`, `trace`, `adjoint`, and `eigenvalues` under `#[cfg(feature = "matrix")]`.
- `maxima-web/src/service.rs`:
  - Registered 5 new operations in `OPERATIONS` catalog: `"determinant"`, `"invert"`, `"charpoly"`, `"trace"`, `"eigenvalues"`.
  - Defined `PARAM_CHARPOLY` with variable parameter defaulting to `"%lambda"`.
  - Implemented execution handlers in `run_operation` with full JSON output, LaTeX formatting, and error propagation.
- `tests/integration/matrix_bareiss.rs`:
  - Added comprehensive test suite with 10 unit and integration tests covering:
    - Matrix construction, validation, and ragged row rejection.
    - Matrix addition, subtraction, multiplication, and transposition.
    - 1x1, 2x2, and 3x3 numeric determinants (including tridiagonal Toeplitz and singular matrices).
    - Zero diagonal pivoting and row exchange verification.
    - Symbolic determinants with variables ($x^2 - 2$, $x^3 + 1$).
    - Matrix trace (numeric and symbolic).
    - Characteristic polynomial computation.
    - Classical adjoint and matrix inverse verification ($M \cdot M^{-1} = I$, singular matrix rejection).
    - Eigenvalues computation.
    - Web service compute dispatch verification for `"determinant"` and `"trace"`.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P40-01 | `sym-core/src/matrix/bareiss.rs` | Linter / Style Violation | Clippy flagged `needless_range_loop` on 2D matrix iteration and `collapsible_if`. | Refactored with idiomatic iterator `.zip()`, `.iter_mut()`, `.enumerate()`, and collapsed `if let ... && let`. |
| P40-02 | `maxima-web/src/service.rs` | Linter / Style Violation | Clippy flagged redundant closure in `evs.iter().map(\|e\| latex(e))`. | Replaced closure with direct function pointer `evs.iter().map(latex)`. |
| P40-03 | `tests/integration/matrix_bareiss.rs` | Type / Field Mismatch | `ComputeRequest` struct initialization used deprecated individual Option fields instead of `params: HashMap`. | Updated to canonical `ComputeRequest` struct layout matching `webui_e2e.rs`. |
| P40-04 | `tests/integration/matrix_bareiss.rs` | Math Sign Error in Test | Test asserted $\det(M_3) = x^3 - 1$ instead of $x^3 + 1$ for circulant matrix with anti-diagonal term. | Corrected cofactor expansion: $(-1) \cdot (-1) = +1$, so $\det = x^3 + 1$. |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Non-square matrix rejection (`MatrixError::NonSquare`)
- [x] Ragged matrix rejection (`MatrixError::RaggedMatrix`)
- [x] Matrix multiplication dimension mismatch rejection (`MatrixError::DimensionMismatch`)
- [x] Singular matrix inverse rejection (`MatrixError::SingularMatrix`)
- [x] Zero-diagonal pivot row exchange in Bareiss elimination
- [x] Exact polynomial preservation without fractional explosion
- [x] Identity matrix multiplication verification ($M \cdot M^{-1} = I$)
- [x] Zero undefined CSS classes verification (`verify_css_classes.py`)
- [x] Feature isolation: `sym-core` compiles cleanly with `--no-default-features`

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo test -p sym-core --features matrix --lib` | `0` | `0` | 149 passed; 0 failed |
| `cargo test -p maxima-tests --test matrix_bareiss` | `0` | `0` | 10 passed; 0 failed |
| `cargo test -p maxima-tests --test bfloat` | `0` | `0` | 9 passed; 0 failed |
| `cargo test -p maxima-web` | `0` | `0` | 28 passed; 0 failed |
| `E:\Anaconda\python.exe scripts/verify_css_classes.py` | `0` | `0` | 0 missing classes (93 used, 102 defined) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo check -p sym-core --no-default-features` | `0` | `0` | Clean compilation (Feature isolation intact) |
