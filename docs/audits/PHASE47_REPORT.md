# Phase 47 Sign-Off Report: E-Graph Soundness, Core Numbers, Parsers, Codegen & Macros Hardening

## 1. Summary of Deliverables & Changes
- `sym-core/src/lang.rs`, `sym-core/src/cost.rs`, `sym-core/src/rules/{mod,arithmetic,exponential,factorial,trigonometric}.rs`:
  - Attached `egg::Condition` guards (`is_not_zero`, `is_positive`, `is_nonneg_int`, `is_sym_or_positive_int`, `is_sym_or_nonneg_int`) to `div-zero`, `div-self`, `pow-zero`, `pow-zero-base`, `pow-add-exp`, and `factorial` rules, eliminating `0/0` and `0^0` e-graph collapse.
  - Added Maxima-aligned `SimpFlags` (`DomainMode`, `RadExpandMode`, `LogExpandMode`, `TrigInversesMode`, `fast_math`) gating `(x^a)^b`, `ln(exp(x))`, `ln(a*b)`, `ln(a/b)`, `ln(a^b)`, and `asin(sin(x))`/`acos(cos(x))`/`atan(tan(x))`.
  - Added `(^ e ?x) <=> (exp ?x)`, `(* i i) => -1`, `(^ i 2) => -1`, and canonical `%pi`-multiple trig rules (`sin/cos/tan` at `pi`, `pi/2`, `2*pi`), with first-class `MathLang::{Pi, E, I}` variants.
  - Replaced `egg::merge_max` in `MathAnalysis::merge` with conflict-clearing merge (`*to = None`) and represented `MathLang::SpecialFn` using child `Id`s (`Box<[Id]>`) instead of comma-joined strings.
- `sym-core/src/{rational,bfloat,expr}.rs`:
  - Fixed `Rat::to_f64` large-integer overflow via bit-scaling, hardened `Rat::pow_int` (`0^0 -> None`, `i64::MIN` safe, exponent cap `<= 10_000`), used exact IEEE-754 `BigRational::from_float` in `Rat::from_f64`, and centralized `Rat::parse_exact_rational` with exponent cap `<= 10_000`.
  - Updated `BfloatWrapper::{sin,cos,tan,atan,pow}` to return `Result<Self, String>`, clamped `fpprec >= 1`, and computed `sinh`/`cosh`/`tanh` at extended working precision to eliminate cancellation (`sinh(1b-10)`).
  - Fixed `Expr::eval(NaN!) -> None`, domain-error `Pow`/`Asin`/`Acos -> None`, and `Expr::is_const()` for `Expr::Pi` / `Expr::E`; aligned `Expr::Display` (`diff`, `integrate`, `sum`, `matrix`) with `parse_maxima` round-trip syntax.
- `sym-core/src/parser/{mod,latex,maxima}.rs`:
  - `parser/latex.rs`: fixed UTF-8 byte-slice panic & nested braces in `\sum`, preserved `d<var>` in `\int`, mapped `\pi` to `Expr::Pi`, enforced word boundaries (`\sinc`, `\cdots`), preserved `x_{0}`, and added `RecursionDepthGuard` in `power()`.
  - `parser/maxima.rs`: added `RecursionDepthGuard` in `power()`, tokenized `!!` double factorial (`5!! == 15`), rejected trailing tokens in `parse_maxima_cas`, and returned all roots in `execute_cas(Solve)`.
- `sym-core/src/{latex,cse,codegen,pipeline}.rs` & `sym-macros/src/{config,expand,parse_sym_fn}.rs`:
  - `latex.rs`: escaped `#$%&_` in `escape_latex`, fixed `Neg(Add/Sub)` double parens, fixed `#[cfg(feature = "complex")]` `Factorial` arm leak, and parenthesized `Bfloat` under `Pow`/`Neg`.
  - `cse.rs`: avoided `_tN` shadowing, skipped bound-var hoisting in `Sum`/`Derivative`/`Integral`, used `BTreeMap` for deterministic ordering, and pruned dead 0-use bindings.
  - `codegen.rs` & `pipeline.rs`: parenthesized `Expr::Neg` (`(-#ea)`), handled `NaN`/`Inf`/Rust keywords safely, and emitted free-var parameters + tuple return types in `Pipeline::to_rust_fn`.
  - `sym-macros`: parsed `#[fast_math = false]`, `#[node_limit = ...]`, `#[time_limit_ms = ...]`; inferred `FloatType` (`f32`/`f64`) from signature; enforced exact arity, comma separation, duplicate-field rejection, exact decimal literal parsing, and `gradient` variable validation.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P47-01 | `sym-core/src/rules/*`, `lang.rs` | E-Graph Soundness | Unconditional `0/a`, `a/a`, `a^0`, `0^a`, `fac`, `merge_max`, comma `SpecialFn` | Added `Condition` guards, `SimpFlags`, conflict-clearing merge, `Box<[Id]>` `SpecialFn`; verified in `basic_arithmetic.rs` & `rules::tests` |
| P47-02 | `sym-core/src/{rational,bfloat,expr}.rs` | Numeric & Domain Safety | `to_f64` `inf/inf`, `i64::MIN` panic, `bfloat` silent fallback & cancellation, `NaN!` | Bit-scaled `to_f64`, capped `pow_int`, `Result` trig/pow, extended-prec `sinh`/`tanh`, `eval` domain checks |
| P47-03 | `sym-core/src/parser/{latex,maxima}.rs`, `latex.rs` | Parser & Printer | UTF-8 `\sum` panic, `\sinc` boundary, missing `power()` depth guard, `!!`, `Display` | Char-aware `\sum`, word boundaries, `RecursionDepthGuard` in `power()`, `!!`, `Display` round-trip |
| P47-04 | `sym-core/src/{cse,codegen,pipeline}.rs`, `sym-macros` | Codegen & Macros | `(-x)^2` precedence, `_t0` shadow, dead `#[fast_math]`/`f32`/arity in `sym_fn!` | `(-#ea)` parens, `BTreeMap` + bound-var CSE, `MacroConfig::from_attrs`, `f32` inference, arity/comma checks |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical & Constants (`10^400/(10^400+1)`, `pow_int(2, i64::MIN)`, `0^0 -> None`, `BfloatContext::new(0)`, `sinh(1b-10)`, `NaN!`, `1e1000000000`)
- [x] Operator Precedence & Codegen (`(-x).powi(2) == 9`, `(-x).cos()`, `5!! == 15`, `sym_fn!` `f32` + `#[fast_math = false]`)
- [x] Cardinality & Boundaries (`RecursionDepthGuard` at `power()` depth 1000, `SimpFlags` gating)
- [x] Domain Algorithms (`asin(sin(%pi))` & `((-1)^2)^(1/2)` gated by `SimpFlags`)
- [x] Parsers & I/O (UTF-8 `\sum_{i=\alpha}^{n}`, `\pi`, `\sinc` boundary, `x_{0}`, trailing token rejection)
- [x] Concurrency, Security & Timeouts (inherited from Phase 46)
- [x] WebUI & Persistence (inherited from Phase 46)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo test --workspace --all-features` | `0` | `0` | All suites green (157 `sym-core` unit, all integration + 500-case `print_parse_roundtrip`, `macro_expansion`, 17+3 doctests) |

## 5. Git Checkpoint Verification
- **Local Commit**: Pending creation in final step of Phase 47
- **Push Status**: Deferred pending explicit user confirmation
