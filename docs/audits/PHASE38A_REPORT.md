# Phase 38A Sign-Off Report: P0 Safety, Correctness & Resource Hardening

## 1. Summary of Deliverables & Changes
- `sym-core/src/parser/mod.rs`: Implemented `RecursionDepthGuard`, `check_nesting_limit(input, 256)`, `parse_exact_rational(s)`, and `ParseError::RecursionLimitExceeded`.
- `sym-core/src/parser/maxima.rs`:
  - Fixed operator precedence to bind exponentiation (`^`) tighter than unary minus/plus (`multiplicative` -> `unary` -> `power`).
  - Added `take_number` supporting scientific notation (`1e-3`, `2.5e+4`) and decimals without floating point loss via `parse_exact_rational`.
  - Added recursion depth limit enforcement (limit 256) preventing thread stack overflows.
  - Added unit tests for precedence, scientific notation, and recursion depth limit.
- `sym-core/src/parser/latex.rs`:
  - Fixed operator precedence to bind exponentiation (`^`) tighter than unary minus/plus.
  - Added scientific notation and decimal number parsing.
  - Added `RecursionDepthGuard` enforcement and `check_nesting_limit`.
  - Extended `TRIG_FUNCS` with hyperbolic (`\sinh`, `\cosh`, `\tanh`) and inverse trigonometric functions.
- `maxima-web/src/app.rs`:
  - Fixed HTTP 400 error bodies: mapped `result.error` (not `result.result`) into JSON response and included `error_span`.
  - Moved `OwnedSemaphorePermit` into `tokio::task::spawn_blocking` closure so concurrency permits remain held until the blocking compute thread terminates.
- `maxima-web/src/service.rs`: Handled `ParseError::RecursionLimitExceeded` in `extract_error_span`.
- `tests/proptest/print_parse_roundtrip.rs`: Added 256-case property tests verifying print-parse round-trip invariant across Maxima and LaTeX syntax.
- `tests/integration/parser_maxima.rs`: Added integration tests verifying `-x^2 == -9` at $x=3$, `-2^2 == -4`, `2^-1 == 0.5`, scientific notation, and 3000 parentheses.
- `tests/integration/web_hardening.rs`: Added integration verification that 3000 parentheses return HTTP 400 with structured `error` and `error_span`.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P38A-01 | `sym-core/src/parser/maxima.rs`, `latex.rs` | Operator Precedence | `power` called `unary` as its base, causing `-x^2` to parse as `(-x)^2 = 9` at $x=3$ | Reordered precedence hierarchy so `unary` calls `power`, parsing `-x^2` as `-(x^2) = -9`. Verified in unit & integration tests. |
| P38A-02 | `sym-core/src/parser/maxima.rs`, `latex.rs` | Stack Overflow Vulnerability | Deeply nested expressions recursed without a depth limit, blowing the thread stack | Added `RecursionDepthGuard` and `check_nesting_limit(limit = 256)`. Verified 3000 parentheses return `ParseError::RecursionLimitExceeded`. |
| P38A-03 | `sym-core/src/parser/maxima.rs` | Parsing Incompleteness | Identifiers absorbed numbers or failed on `e-3` / decimal points | Added `take_number` and exact `BigRational` parsing for decimals and scientific notation (`1e-3`, `2.5e+4`). |
| P38A-04 | `maxima-web/src/app.rs` | Web API Error Formatting | HTTP 400 mapped `result.result` rather than `result.error` and omitted `error_span` | Mapped `result.error.unwrap_or(result.result)` and populated `error_span`. Verified in `web_hardening`. |
| P38A-05 | `maxima-web/src/app.rs` | Concurrency Permit Leakage | Concurrency permit dropped when async timeout elapsed while worker thread kept running | Moved `OwnedSemaphorePermit` into `spawn_blocking` closure. Permit is held until worker thread terminates. |
| P38A-06 | `tests/proptest/print_parse_roundtrip.rs` | Testing Invariant | Missing round-trip property test suite | Added proptest suite testing print-parse equivalence across Maxima and LaTeX forms. |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical (`NaN`, `Inf`, `-0.0`, scientific notation `1e-3`, `2.5e+4`)
- [x] Operator Precedence (`-x^2 == -9` at $x=3$, `-2^2 == -4`, `2^-1 == 0.5`, `-x! == -(x!)`, `(-x)^2 == 9`)
- [x] Cardinality (`0`, `1`, `2`, constant/duplicate)
- [x] Parsers & I/O (recursion depth limit 256, 3000 parentheses returning `RecursionLimitExceeded`, empty input)
- [x] Concurrency & Timeouts (owned semaphore permits held in worker thread)
- [x] WebUI & API (HTTP 400 error body mapping with `error` and `error_span`)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo check --workspace --all-targets` | `0` | `0` | Passed cleanly |
| `cargo test -p sym-core --test parser_maxima` | `0` | `0` | 11 passed; 0 failed |
| `cargo test -p maxima-tests --test web_hardening` | `0` | `0` | 7 passed; 0 failed |
| `cargo test -p maxima-tests --test print_parse_roundtrip` | `0` | `0` | 2 proptests passed (256 cases each) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
