# `max-rs` Comprehensive Repository Audit & Phased Implementation Roadmap

**Audit Target:** `max-rs` workspace (`sym-core`, `sym-macros`, `maxima-rs`, `maxima-web`, `tests`, CI/Docker/docs — ~30,559 LOC across ~130 non-vendor files)  
**Parity Baseline:** Bundled `References/maxima-master/` (**Maxima 5.50post** — 199 `src/*.lisp` files, 103 `tests/rtest*.mac` suites, 2,749 entries in `share/builtins-list.txt`) and official [Maxima 5.50.0 Release](https://sourceforge.net/projects/maxima/files/Maxima-Windows/5.50.0-Windows/) / [Maxima Manual](https://maxima.sourceforge.io/docs/manual/maxima_singlepage.html)  
**Audit Scope:** 1:1 Maxima 5.50 core parity, logical & mathematical correctness, concurrency/security hardening, WebUI/UX & newbie onboarding, and test suite rigor.

---

## 1. Executive Summary & Audit Scorecard

`max-rs` has a clean architectural foundation (hybrid `egg` e-graph + procedural CAS modules per ADR-0006, offline-first `rust-embed` Axum WebUI per ADR-0007, and pure-Rust `dashu-float` arbitrary-precision `bfloat` per ADR-0009). However, a full line-by-line audit of the entire repository reveals **severe silent mathematical wrong-answer bugs**, **critical WebUI security (DOM XSS) and usability regressions**, **a broken CI pipeline and unbuildable Dockerfile**, and **wide 1:1 parity gaps against Maxima 5.50** (~8% of core `src/*.lisp` files partially covered, with several advertised modules remaining stubs).

### Severity Breakdown Across Subsystems

| Subsystem | Critical | High | Medium | Low | Readiness Assessment |
| :--- | :---: | :---: | :---: | :---: | :--- |
| DevOps, CI, Docker, Facade & Macros | 1 | 6 | 5 | 2 | **Broken:** CI fails unconditionally (`app.js` deleted); Docker MSRV fails (`rust:1.85` vs `sha2 0.11.1`) |
| Core AST, Rules, Numbers, Parser & Codegen | 13 | 11 | 7 | 2 | **Unsound:** E-graph collapses on `0/0` & `0^0`; complex `carg` wrong quadrant; `codegen` negation precedence broken |
| Symbolic Math (`poly`, `diff`, `integrate`, `limit`, `series`, `solve`, `matrix`, `specfn`, `transform`) | 18 | 23 | 15 | 4 | **Silent Wrong Answers:** `ode_second` dead for all `b != 0`; `limit/zero.rs` marks `sin(x)` as `0`; `laplace`/`ilt` return bogus formulas on unmatched inputs |
| Web Backend (`maxima-web/src`) | 3 | 7 | 6 | 2 | **Fragile:** Timeout leaks semaphore permits permanently; `/api/parse` unisolated; TOML config silently overwritten; zero session state |
| Web Frontend UI/UX (`maxima-web/static`) | 2 | 7 | 6 | 3 | **Broken & Unfriendly:** Reflected DOM XSS via `#expr=`; 180ms preview debounce wipes user/example params; blank initial notebook; 8 broken UI features |
| Test Suite (`tests/`) | 1 | 5 | 4 | 2 | **Shallow:** Zero JS/UI tests; `golden_parity.rs` has 6 tests (~25 trivial cases); timeout test only tests `0ms` |
| **Total** | **38** | **59** | **43** | **15** | **Requires structured remediation across 8 gated phases (Phases 46–53)** |

### Top 10 Immediate "Stop-Ship" Defects

1. **Reflected DOM XSS in WebUI (`maxima-web/static/js/preview.js:33,53`, `notebook.js:61-62`, `main.js:196`)**: Unescaped `res.error`, `err.message`, and `request.operation` injected into `innerHTML`; navigating to `/#expr=<img src=x onerror=alert(1)>` executes arbitrary JS on load.
2. **CI Broken Unconditionally (`.github/workflows/ci.yml:108`) & Docker Build Broken (`Dockerfile:12`, `.dockerignore:11`)**: CI runs `node --check maxima-web/static/app.js` (file deleted in Phase 36); Dockerfile pins `rust:1.85` while `Cargo.lock` requires Rust `>= 1.88` (`sha2 0.11.1`) and `.dockerignore` excludes `Cargo.lock`.
3. **E-Graph Collapse (`0 = 1`) (`sym-core/src/rules/arithmetic.rs:167-201`)**: Unconditional `(/ 0 ?a) => 0` + `(/ ?a ?a) => 1` and `(^ ?a 0) => 1` + `(^ 0 ?a) => 0` union `0` and `1` on `0/0` or `0^0`; `(^ 0 ?a) => 0` turns `0^(-1)` into `0`.
4. **Second-Order ODE Solver 100% Dead for All `b != 0` (`sym-core/src/solve/ode_second.rs:58-64`)**: Matches `Expr::Const(dr)` on an unfolded `Expr::sub(...)`, so every damped/general 2nd-order ODE (`y'' + 2y' + y = 0`) returns `None`; meanwhile `y'' = 0` returns `_C1 + _C2` instead of `_C1 + _C2*x` (lines 31–56).
5. **Limit Zero-Equivalence Declares `x`, `sin(x)`, `exp(x)-1` Identically Zero (`sym-core/src/limit/zero.rs:11-26`)**: Checks `s.constant_term().is_zero()` instead of `s.coeffs.is_empty()`, and `numeric_limit` (`limit/mod.rs:324-329`) returns one-sided limits as two-sided (`limit(exp(1/x), x, 0) -> 0` instead of `und`).
6. **Laplace & Inverse Laplace Silently Return Bogus Formulas (`sym-core/src/transform/laplace.rs:104-172, 221-347`)**: Unmatched inputs silently return `f(t)/s`, `1/s`, or `1/(s^2+1)`; `ilt_rational` leaks `s` into the time domain for `s`-dependent numerators; `bvp.rs:25,99` checks unfolded `is_zero()` so singular checks never fire.
7. **Complex `carg` Wrong Quadrant & `cabs` Drops Absolute Value (`sym-core/src/complex/mod.rs:588-606, 674-693`)**: `carg_pair` uses `atan(v/u)` instead of `atan2(v, u)` (wrong for all `Re(z) < 0`); `cabs(x)` returns `x` instead of `|x|`; `Assumptions::csign` (`assume/mod.rs:566-588`) returns `Pos` for `exp(%i*%pi)` and `NonNeg` for `%i^2`.
8. **Codegen Precedence & Uncompilable Rust Output (`sym-core/src/codegen.rs:86-142`, `pipeline.rs:119-131`)**: `Expr::Cos(Expr::Neg(x))` emits `-x.cos()` (`-(cos(x))`); `Pipeline::to_rust_fn` emits zero parameters and hardcoded scalar return type; `maxima-web/src/service.rs:1241-1249` interpolates unvalidated variable names into Rust signatures.
9. **Timeout Leaks Permits Permanently & `/api/parse` Unisolated (`maxima-web/src/app.rs:95-111, 155-194`)**: `tokio::time::timeout` drops the `JoinHandle` while the blocking thread keeps `OwnedSemaphorePermit` forever, allowing repeated slow queries to permanently lock the server in HTTP 503; `/api/parse` runs synchronously on Tokio async workers.
10. **Live Preview Wipes User & Example Parameters Every 180ms (`maxima-web/static/js/preview.js:49-51`, `palette.js:24-27,148-236`)**: Clicking `defint(1/x², -1, 1)` or editing an expression resets `lower`/`upper`/`point`/`order` back to defaults 180ms later.

---

## 2. Detailed Audit Findings by Subsystem

### 2.1 DevOps, CI, Docker, Facade (`maxima-rs`) & Macros (`sym-macros`)

| Sev | File & Lines | Finding & Root Cause |
| :--- | :--- | :--- |
| **Critical** | `.github/workflows/ci.yml:108` | `node --check maxima-web/static/app.js` fails because `app.js` was split into `static/js/*.js` in Phase 36. |
| **High** | `Dockerfile:12,17,33`, `.dockerignore:11` | `rust:1.85-bookworm` cannot compile `sha2 0.11.1` (needs Rust `>= 1.88`); `.dockerignore` excludes `Cargo.lock` and build omits `--locked`. |
| **High** | `.github/workflows/ci.yml:77-90` | `feature-matrix` omits all 6 Phase 39–44 features (`bfloat`, `matrix`, `complex`, `specfn`, `transform`, `assume`) and never runs `scripts/verify_css_classes.py`. |
| **High** | `sym-macros/src/config.rs:7-25`, `expand.rs:30-35,115-120` | `MacroConfig::default()` is hardcoded; `#[fast_math = false]` (ADR-0002), `node_limit`, `time_limit_ms`, and `f32` return/param types are ignored (passing `f32` emits `f64` code that fails to compile). |
| **High** | `sym-macros/src/parse_sym_fn.rs:32-40,57-80,141-268` | Extra arguments in unary/binary calls (`x.sin(1,2)`, `pow(x,2,99)`) are silently dropped; missing commas between params/fields are silently accepted; duplicate `expr:`/`gradient:` silently overwrite; float literals go through lossy `f64` instead of exact `BigRational`. |
| **High** | `sym-macros/src/expand.rs:63-82` | `gradient: [z]` is never validated against function params (silently emits `0.0`); generated `_t0` CSE bindings shadow user parameters named `_t0`. |
| **High** | `maxima-rs/Cargo.toml:10-31`, `prelude.rs:12-84` | `tensor` feature is missing from `maxima-rs/Cargo.toml` (unlike `sym-core`); `prelude.rs` omits `SaturationConfig`, `RuleConfig`, `saturate`, `eliminate_common_subexprs`, `integrate_with_assumptions`, `limit_with_assumptions`. |
| **Medium** | `scripts/verify_css_classes.py:14,21` | Regex skips template literals containing `$` (e.g. `class="badge ${cls}"`) and ignores `classList.add/toggle/remove(...)`. |
| **Medium** | `Dockerfile:44,63`, `template.yaml:10,18-24` | Container `CMD` defaults to port `3000` while Lambda Web Adapter expects `8080` (no `AWS_LWA_READINESS_CHECK_PATH=/api/health`); Lambda `MemorySize: 512` (~0.3 vCPU) risks 8s timeouts on saturation. |
| **Medium** | `README.md:9,37-67,149-164,189-196`, `GLOSSARY.md`, `agents.md`, `SESSION_LOG.md`, `PHASE45_REPORT.md` | ADR-0012 truth violations & drift: `README.md` claims `v0.5.0` while `Cargo.toml:6` is `0.2.0`; marks Phases 22–30 `✅ Completed` instead of `◐ Partial`; embeds broken Windows `file:///e:/Rust/Max/...` links; `GLOSSARY.md` / `agents.md:21` falsely claim Jacobian/Hessian macro codegen; `PHASE45_REPORT.md:12` claims "full upstream Maxima 5.50 golden verification suite" for 6 tests. |

### 2.2 Core AST, Rewrite Rules, Numbers, Complex, Assumptions, Parsers & Codegen

| Sev | File & Lines | Finding & Root Cause |
| :--- | :--- | :--- |
| **Critical** | `sym-core/src/rules/arithmetic.rs:167-229` | Unconditional `0/a => 0`, `a/a => 1`, `a^0 => 1`, `0^a => 0` collapse e-graph on `0/0`, `0^0`, and mis-evaluate `0^(-1) -> 0`; `(x^a)^b => x^(a*b)` is unsound for negative bases (`((-1)^2)^(1/2) = 1 != -1`). |
| **Critical** | `sym-core/src/rules/logarithmic.rs:21-64`, `hyperbolic.rs:119-162`, `factorial.rs:29-45` | Unconditional `ln(exp(x)) => x`, `ln(a*b) <=> ln(a)+ln(b)`, `ln(a^b) => b*ln(a)`, `asin(sin(x)) => x`, `acos(cos(x)) => x`, `atan(tan(x)) => x`, and `(n+1)! => (n+1)*n!` violate principal branch cuts and poles (e.g. `asin(sin(%pi)) -> %pi`, `0! -> 0*(-1)!`). Maxima gates these behind `domain`, `radexpand`, `logexpand`, and `triginverses` (`References/maxima-master/src/globals.lisp:1242,1258,1262,1518`). |
| **Critical** | `sym-core/src/complex/mod.rs:104-145, 340-369, 455-464, 536-606, 674-693` | `carg_pair` uses `atan(v/u)` instead of `atan2(v, u)` (wrong quadrant for `Re(z) < 0`); `cabs(x)` returns `x` instead of `|x|`; `smart_div(0,0)->0`; `smart_pow(0,-1)->0`; `pow_pairs` sets `Im((-2)^x) = 0`; `rectform` sets `Im(asin(%i)) = 0`. |
| **Critical** | `sym-core/src/assume/mod.rs:287-391, 566-588, 684-776` | `csign(exp(%i*%pi)) -> Pos`, `csign(%i^2) -> NonNeg`, `csign(2^%i) -> Pos`, `csign(0^(-1)) -> Zero`. DAG transitivity ignores `<`/`<=`/`=` edges, requires all edges strict (`a>b, b>=c` fails `a>c`), ignores `(Const, Var)` (`5 < x`), never checks negation (`is(a<b)` returns `Unknown` after `assume(a>b)`), allows cyclic contradictions (`a>b` & `b>a`), and ignores `NotEqual`. |
| **Critical** | `sym-core/src/rational.rs:121-123, 179-195` | `Rat::to_f64` computes `numer.to_f64() / denom.to_f64()` -> `inf/inf = NaN` for rationals with `numer, denom > 1.8e308`; `Rat::pow_int` panics on `i64::MIN`, truncates `|n| > u32::MAX`, and has no exponent cap (OOM on `2^(10^9)`). |
| **Critical** | `sym-core/src/bfloat.rs:192-197, 232-269, 332-343` | `sin`/`cos`/`tan`/`atan` silently return input `x` on error via `.unwrap_or_else(|| self.value.clone())`; `pow` panics on negative base or `0^y (y<=0)`; `BfloatContext::new(0)` panics. |
| **Critical** | `sym-core/src/parser/latex.rs:258-261, 289-316`, `parser/mod.rs:100-137`, `parser/maxima.rs:124-135` | `\sum` slices UTF-8 by 1 byte (panics on non-ASCII) and breaks on nested `{}`; `\int` discards bounds and hardcodes variable `"x"`; `parse_exact_rational("1e1000000000")` OOMs; `power()` skips `RecursionDepthGuard` (stack overflow on `2^2^2^...`). |
| **Critical** | `sym-core/src/codegen.rs:44-65, 86-169`, `pipeline.rs:119-131` | Unary `-` emitted unparenthesized before `.powf`/`.cos`/`.exp` (`cos(-x)` emits `-x.cos()`); `to_rust_fn` emits zero params and wrong return type for tuples; `NaN`/`Inf` silently emit `0.0`; `Expr::I` emits `(0.0, 1.0)` tuple; `format_ident!` panics on Rust keywords or `%` vars. |
| **High** | `sym-core/src/bfloat.rs:56-67, 271-308`, `rational.rs:230-271` | `sinh`/`tanh` suffer catastrophic cancellation near `0` and overflow at large `|x|`; `PartialEq`/`Hash` depend on `prec`; `continued_fraction_approx` saturates floats `>= 2^63` and zeroes floats `< 1e-15`. |
| **High** | `sym-core/src/parser/latex.rs:87-98, 378-544`, `latex.rs:46-47, 168-172, 226-236, 390-395` | `\pi` parses as `Var("pi")` not `Expr::Pi`; `\sinc` misparses as `\sin` + `c`; `x_{0}` drops `_0`; `escape_latex` fails to escape `%$#&_` (`%k1` comments out rest of LaTeX); `Neg(Add)` double-parenthesizes; `#[cfg(feature="complex")]` accidentally gates `Var`/`Pi`/`E` factorial formatting. |
| **High** | `sym-core/src/parser/maxima.rs:138-150, 752-840`, `expr.rs:768-789` | `5!!` parses as `(5!)!` instead of double factorial `15`; `Display` output (`d(f)/d(x)`, `int(f dx)`, `[[a]]`) cannot roundtrip through `parse_maxima`; `parse_maxima_cas` silently drops trailing tokens (`expand(x)+1`); `execute_cas(Solve)` drops all roots except the first. |
| **High** | `sym-core/src/cse.rs:32-77, 110-197`, `lang.rs:69-71, 262-282, 341-391` | CSE shadows existing `_t0` vars, hoists bound variables out of `Sum`/`Integral`, has non-deterministic `HashMap` order, and emits dead single-use bindings. `MathLang` uses `merge_max` on conflicting constants, encodes `SpecialFn` args via raw `","` join (corrupting nested calls), and turns decode failures into `Expr::var`. |
| **Medium** | `sym-core/src/expr.rs:485-494, 610, 627-646`, `rules/exponential.rs`, `complex/mod.rs:721-729` | `Expr::eval(NaN!)` returns `Some(1.0)`; `is_const()` is `false` for `Pi`/`E` but `true` for `I`; no e-graph bridge between `%e^x` and `exp(x)` or `%i^2` and `-1`; `residue` hardcodes Laurent order 6. |

### 2.3 Symbolic Math Engines (`poly`, `diff`, `integrate`, `limit`, `series`, `solve`, `matrix`, `tensor`, `specfn`, `transform`)

| Sev | File & Lines | Finding & Root Cause |
| :--- | :--- | :--- |
| **Critical** | `sym-core/src/poly/rat.rs:47-52, 92-99`, `poly/mod.rs:162-174` | `Fraction::div` by zero produces `den = 0` which `to_expr` silently converts to `0` (`ratsimp(1/(x-x)) -> 0`); `Poly::scale(0)` retains zero terms, breaking `is_zero()` and `leading_term()`. |
| **Critical** | `sym-core/src/limit/zero.rs:11-26`, `limit/mod.rs:205-234, 292-330` | `is_zero_equivalent` checks `constant_term().is_zero()` (declaring `x`, `sin(x)`, `exp(x)-1` identically zero); `numeric_limit` returns one-sided limits as two-sided and false-converges on `sin(2*pi*x)` at `inf`; `symbolic_limit` never returns `inf`/`minf` and collides with user variable `"t"`. |
| **Critical** | `sym-core/src/series.rs:219-292, 329-353` | `Series::mul`, `Series::inv`, and `Series::diff` compute wrong Laurent truncation orders (`order.min`, unchanged `order`), fabricating garbage high-order terms or dropping leading terms. |
| **Critical** | `sym-core/src/integrate/defint.rs:37-60, 162-193`, `integrate/mod.rs:99-156, 308-333` | `defint` checks poles without `.reduce()` (flags removable poles as divergent), misses irrational/transcendental/endpoint poles (`1/(x^2-2)`, `tan(x)`), and only flips `ln` on strictly negative intervals; `integrate_power_denominator` allows `n == 1` causing division by `0`; `is_proven_non_elementary` flags `exp(0*x^2)` as non-elementary. |
| **Critical** | `sym-core/src/diff.rs:210-211` | `diff(x!, x)` returns `0` instead of `x! * psi[0](x + 1)` (`References/maxima-master/src/comm.lisp:448`). |
| **Critical** | `sym-core/src/solve/ode_second.rs:31-100` | `disc` is unfolded `Expr::Sub`, so `let Expr::Const(dr) = &disc` fails for **every** `b != 0`; `y'' = 0` returns `_C1 + _C2` instead of `_C1 + _C2*x`; `looks_positive` treats `0^2` as positive. |
| **Critical** | `sym-core/src/matrix/bareiss.rs:277-309, 317-383` | `simplify_div` calls `ratsimp` whose `poly_gcd` returns `1` for `>1` variable, breaking Bareiss exact pivot division on all $\ge 3\times 3$ multivariate matrices; `is_expr_zero` returns `false` for `sin(x) - sin(x)`, risking division by zero. |
| **Critical** | `sym-core/src/transform/laplace.rs:64, 104-172, 221-347`, `transform/bvp.rs:20-25, 95-100` | `laplace` silently returns `1/s` for `exp(t^2)`, `1/(s^2+1)` for `sin(t^2)`, and `f(t)/s` for every unsupported function; `ilt_rational` leaks `s` into results on `s`-dependent numerators and overflows `i64` factorial at `n >= 22`; `bvp` (`ic1`/`ic2`) checks unfolded `a.is_zero()` / `det.is_zero()`, so singularity checks never fire. |
| **High** | `sym-core/src/poly/gcd.rs:105-144`, `factor.rs:23-104`, `partfrac.rs:16-68` | `poly_gcd` degrades to `1` for `>1` variable; `poly_derivative` hardcodes variable index `0`; `factor_poly` only finds `i64` rational roots (overflows on `i64::MIN` / `d*d`), fails on rationals/quadratics/multivariate; `partfrac` skips improper fractions, repeated poles, irreducible quadratics, and unreduced fractions. |
| **High** | `sym-core/src/integrate/{hermite,lrt,risch_de,sin,risch,ibp}.rs`, `series.rs:29-72, 496-588`, `limit/mrv.rs` | `hermite.rs` (20L), `lrt.rs` (12L), `risch_de.rs` (38L), `sin.rs` (46L), `mrv.rs` (66L, dead code) are stubs; `integrate_product` drops `Assumptions` and fails on `x*(x+1)`; `ibp::trig_powers` has $O(2^n)$ AST blowup; `series.rs` fails on symbolic centers (`taylor(sin(x), x, pi, 5)`). |
| **High** | `sym-core/src/solve/{alg,isolate,ode,ode_first}.rs` | `isolate` has sign bug (`a - x = 0 -> -a`) and circular non-solution bug (`exp(x) - x = 0 -> x = ln(x)` marked complete); `alg.rs` drops complex roots (`x^2+1=0 -> []`) and rejects parametric equations (`a*x+b=0 -> []`); `ode_first.rs` is a 1-line constant-coeff homogeneous formula only. |
| **High** | `sym-core/src/matrix/bareiss.rs:420-476`, `specfn/*` | `eigenvalues` drops complex/symbolic eigenvalues; `adjoint` is $O(n^5)$; `specfn` has no numerical (`f64`/`bfloat`) evaluation, no recurrence simplification, and `orthopoly` has no degree cap (OOM risk). |

### 2.4 Web Backend (`maxima-web/src/`)

| Sev | File & Lines | Finding & Root Cause |
| :--- | :--- | :--- |
| **Critical** | `maxima-web/src/app.rs:155-194` | `tokio::time::timeout` drops `JoinHandle` without cancelling the blocking worker; `OwnedSemaphorePermit` stays held until the runaway computation finishes, permanently exhausting concurrency. |
| **Critical** | `maxima-web/src/app.rs:95-111` | `/api/parse` runs synchronously on Tokio async workers with no `spawn_blocking`, `catch_unwind`, timeout, or semaphore. |
| **Critical** | `maxima-web/src/service.rs:915-920`, `app.rs:99,119` | `taylor` `order` has no min/max cap (negative panics, `order: 100000` OOMs); `params`, `variables`, and `env` lengths are unbounded. |
| **High** | `maxima-web/src/config.rs:17-45, 77-95` | `Cli` fields use `default_value`/`default_value_t`, unconditionally overwriting 5 of 6 settings loaded from `--config <toml>`. |
| **High** | `maxima-web/src/api.rs:63-88`, `service.rs:560, 1128-1131, 1193, 1212, 1327, 1459, 1882-1897` | `fast_math` silently dropped; `params.variables` and `params.float_type` ignored by handlers; `charpoly`/`residue` ignore top-level `variable`; `jacobian` only accepts scalar expressions; `parse_float_type` silently coerces invalid enums to `f64`. |
| **High** | `maxima-web/src/service.rs:1241-1249, 1926-1932` | Raw variable strings interpolated into generated Rust function signatures without identifier validation (code injection). |
| **High** | `maxima-web/src/app.rs`, `service.rs` | Zero session state (`%iN`/`%oN`/`%`, variable bindings `a: expr`, function defs `f(x):=...`, session-scoped `assume`/`facts`/`forget`). Major `sym-core` capabilities (`laplace`, `ilt`, `ic1`/`ic2`/`bc2`, `bfloat`, `assume`/`is`, `adjoint`, `orthopoly`) are completely unexposed in `service.rs`, and 17+ operations lack embedded `CasCommand` auto-dispatch. |
| **Medium** | `maxima-web/src/app.rs:43-63, 197-208, 223-252`, `main.rs:26`, `service.rs:536, 1486, 1743-1750, 1935-2180, 2246-2323` | Missing CSP/`X-Content-Type-Options`/`X-Frame-Options` headers and graceful shutdown; unknown paths return `200 + index.html`; `parse_maxima_cas` runs on LaTeX input; double-parse on error; `resp.operation` stale after auto-dispatch; `verify_integral`/`verify_defint`/`verify_ode` fail on symbolic params and `minf`; `functions_catalog()` has invalid `\sqrt({})` and broken `limit({}, x)` arity. |

### 2.5 Web Frontend UI/UX (`maxima-web/static/`)

| Sev | File & Lines | Finding & Root Cause |
| :--- | :--- | :--- |
| **Critical** | `maxima-web/static/js/preview.js:33,53`, `notebook.js:61-62`, `main.js:196` | Reflected/stored DOM XSS via unescaped `innerHTML` interpolation of `res.error`, `err.message`, and URL-hash-derived `request.operation`. |
| **Critical** | `maxima-web/static/js/preview.js:49-51`, `palette.js:24-27, 148-236` | 180ms live preview calls `setDetectedVariables` -> `renderParams(..., null)`, clearing `#paramsContainer` and wiping user-edited and example-chip parameters. |
| **High** | `maxima-web/static/js/palette.js:154-158, 196-221, 299-305`, `main.js:114-134, 262-271`, `notebook.js:79,86,232-240`, `index.html:62,116` | Eight broken UI features: (1) `evaluate` renders no `env` inputs and sends no `env`; (2) Rules modal & notebook Rules tab read wrong JSON fields (`source_function`/`rule`/`description`/`response.provenance`), showing blank entries; (3) `fastMathToggle` is a placebo; (4) `float_type` renders as text input and is sent in wrong JSON slot; (5) `d/d[x,y]` chip passes `"x, y"` as one literal variable -> `0`; (6) composer binary templates leave literal `{}` and `limit` omits point; (7) initial notebook pane is completely blank; (8) `error_span` is returned by backend but never rendered. |
| **Medium** | `maxima-web/static/js/api.js:35,52`, `preview.js:11-55`, `notebook.js:118-131`, `main.js:117,158-207,284`, `palette.js:89-110`, `index.html:10,47-53,99` | `api.js` crashes on non-JSON 413/502 errors; `preview.js` has out-of-order race condition; `Use as input` ignores LaTeX mode; `Copy` ignores active tab; undefined CSS classes (`opacity-75`, `border-danger`); dead `auto-render.min.js`; `matrix`/`complex` missing from `CATEGORY_NAMES`; missing `<label>` associations, `aria-live`, `Escape` modal close, and `Shift+Enter` inside param inputs. |

### 2.6 Test Suite & Verification Honesty Audit (`tests/`)

| Sev | File & Lines | Finding & Root Cause |
| :--- | :--- | :--- |
| **Critical** | Entire `maxima-web/static/js/` | **Zero automated JS/UI tests** (no Playwright, Vitest, or DOM smoke tests), allowing DOM XSS, param wiping, broken Rules modal, broken `evaluate`, and broken `d/d[x,y]` chip to ship undetected. |
| **High** | `tests/integration/web_hardening.rs` | Timeout test uses `timeout_ms: 0` (immediate shortcut), never verifying real worker interruption or permit recovery; misses HTTP 413 body-limit enforcement and panic-recovery tests. |
| **High** | `tests/integration/golden_parity.rs` (358 lines, 6 tests) | Samples ~25 hand-picked trivial cases across 6 of 100 upstream `rtest*.mac` files while `PHASE45_REPORT.md` claims "full upstream Maxima 5.50 golden verification suite". |
| **High** | `tests/proptest/*.rs` (5 files, 10 tests total) | Proptests only sample small numeric coefficients on fixed templates rather than generating arbitrary recursive ASTs; `solve_correctness.rs` never checks root count/completeness; several tests use `if let Some(...)` that silently pass when the solver returns `None`. |
| **Medium** | `tests/integration/webui_e2e.rs` | Calls `service::compute` directly in-process instead of going through Axum HTTP router (skipping JSON deserialization, `app.rs` status mapping, and headers), and omits 15+ operations. |

---

## 3. Maxima 5.50 1:1 Parity Gap Matrix

Benchmarked against bundled `References/maxima-master/` (Maxima `5.50post`: 199 `src/*.lisp`, 103 `tests/rtest*.mac`, 2,749 `builtins-list.txt`) and the [Maxima Manual](https://maxima.sourceforge.io/docs/manual/maxima_singlepage.html). Per ADR-0009, core `src/` mathematical parity is in scope; `share/` packages (except `orthopoly`) and plotting (`gnuplot`/`vtk`) remain out of scope unless explicitly adopted.

| Maxima 5.50 Core Subsystem | Reference Lisp Sources (`src/`) | `max-rs` Status | Exact 1:1 Parity Gaps |
| :--- | :--- | :---: | :--- |
| **Parser & Core Syntax** | `nparse.lisp`, `grind.lisp` | **Partial** | Missing `**` power, `=` equations, lists `[a, b]`, sets `{a, b}`, double factorial `!!`, `sec/csc/cot/asec/acsc/acot/asinh/acosh/atanh/atan2/abs`, `diff(f,x,n)`, noun quote `'`, assignments `:`, function defs `:=`. |
| **Simplifier & Option Flags** | `simp.lisp`, `globals.lisp`, `trigi.lisp`, `logarc.lisp` | **Partial** | 92 e-graph rules vs thousands of core simplifications; missing `domain` (`real`/`complex`), `radexpand`, `logexpand`, `triginverses`, `trigexpand`, `trigreduce`, `trigsimp`, `radcan`, `demoivre`, `exponentialize`, `%e^x <-> exp(x)`, `%i^2 -> -1`, `%pi`-multiple trig exact values. |
| **Numbers & `bfloat`** | `float.lisp`, `numth.lisp`, `ifactor.lisp` | **Partial** | `BigRational` + `dashu-float` `bfloat` exist, but `bfloat` unexposed in WebUI, `sin/cos/tan/atan` error silently, `sinh`/`tanh` cancel near 0; missing `float(...)`, `num`/`denom`, `gcd`/`lcm` on integers, `primep`/`factor(n)` integers, `mod`. |
| **Database & Assumptions** | `compar.lisp`, `db.lisp`, `askp.lisp` | **Partial** | Broken DAG transitivity/contradictions/reverse edges and wrong `csign` on complex `exp`/`pow`; not wired into WebUI, `defint`, `solve`, `complex`, or simplifier; missing `facts()`, `contexts`, `declare`/`featurep` parity ([Maxima Database](https://maxima.sourceforge.io/docs/manual/Maxima_0027s-Database.html)). |
| **Polynomials & CRE (`rat`)** | `rat3a–e.lisp`, `ezgcd.lisp`, `factor.lisp`, `ratout.lisp` | **Partial** | Univariate $\mathbb{Q}[x]$ only; multivariate `gcd` returns `1`; `factor` is rational-root-only; `partfrac` skips improper/repeated/irreducible; `ratsimp` rejects transcendental kernels; missing `divide`, `quotient`, `remainder`, `resultant`, `sqfr`. |
| **Differentiation** | `comm.lisp`, `comm2.lisp` | **Mostly Done** | Wrong `diff(x!)` (`0` instead of `x!*psi[0](x+1)`); missing FTC `diff(integrate(...))`, `diff(sum(...))`, `diff(expr, x, n)` parser syntax, `at(...)`. |
| **Indefinite & Definite Integration** | `sin.lisp`, `risch.lisp`, `sinint.lisp`, `defint.lisp`, `residu.lisp` | **Partial / Stubbed** | `hermite.rs`, `lrt.rs`, `risch_de.rs`, `sin.rs` are stubs; fails on `1/(x^2+1)`, repeated poles, `x*(x+1)`, symbolic params in `sin(a*x)`; `defint` misses irrational/trig/endpoint poles and `minf` verification; missing `logabs` flag. |
| **Limits** | `limit.lisp`, `tlimit.lisp` | **Partial / Broken** | `is_zero_equivalent` broken; `mrv.rs` dead code; no directional `plus`/`minus` API; no `inf`/`minf`/`und`/`ind` symbolic outputs; unsound numeric fallback. |
| **Series (`taylor`)** | `hayat.lisp`, `series.lisp`, `pade.lisp` | **Partial** | Wrong Laurent truncation in `mul`/`inv`/`diff`; fails on symbolic centers; missing `tan`, `sqrt`, fractional/symbolic powers, Puiseux/log-series, `powerseries`, `pade`. |
| **Algebraic & ODE Solvers** | `solve.lisp`, `algsys.lisp`, `psolve.lisp`, `ode2.lisp`, `desoln.lisp` | **Partial / Broken** | `ode_second` dead for `b != 0`; `ode_first` 1-liner; `alg.rs` drops complex roots and parametric equations; missing cubic/quartic radicals, `linsolve`, `algsys`, non-homogeneous/separable/exact/Bernoulli ODEs, `desolve`. |
| **Linear Algebra & Matrices** | `matrix.lisp`, `mat.lisp`, `newdet.lisp`, `invert.lisp` | **Partial** | Bareiss breaks on $\ge 3\times 3$ multivariate matrices; `eigenvalues` drops complex/symbolic roots; `adjoint` is $O(n^5)$; missing `echelon`, `rank`, `nullspace`, `eigenvectors`, `triangularize`, matrix arithmetic in WebUI. |
| **Complex Analysis** | `rpart.lisp`, `conjugate.lisp`, `residu.lisp` | **Partial / Broken** | `carg_pair` uses `atan` instead of `atan2`; `cabs` drops `|.|`; `rectform` ignores inverse trig branch cuts; `residue` hardcoded to order 6. |
| **Special Functions** | `gamma.lisp`, `bessel.lisp`, `airy.lisp`, `specfn.lisp`, `orthopoly.lisp` | **Partial** | 18 variants with exact points & derivatives; missing numerical evaluation (`f64`/`bfloat`), `zeta`, `expintegral_*`, `beta_incomplete`, `lambert_w`, `elliptic_*`, and WebUI exposure. |
| **Transforms & BVP** | `laplac.lisp`, `ode2.lisp` (`$ic1`/`$ic2`/`$bc2`) | **Partial / Broken** | `laplace`/`ilt` silently fabricate wrong formulas on unmatched inputs instead of returning nouns; `bvp` singularity checks never fire; unexposed in WebUI. |
| **Session / Evaluation Model** | `macsys.lisp`, `mlisp.lisp`, `suprv1.lisp` | **Missing** | No `%iN`/`%oN`/`%`, no variable/function bindings (`:`, `:=`), no `subst`/`ev`/`kill`, no session-scoped `assume`/`facts`/`forget` ([Maxima Function Definition](https://maxima.sourceforge.io/docs/manual/Function-Definition.html)). |

---

## 4. Newbie-Friendly UI/UX Blueprint & Starter Input Examples

### 4.1 High-Impact UI/UX Improvements for First-Time Users

1. **Interactive Empty-State Onboarding (Right Pane on First Load)**: Replace the blank notebook pane (`index.html:139-141`) with a welcoming 3-part starter card: (a) 1-line plain-English description ("Type a math expression on the left or click any example below to see step-by-step symbolic results"), (b) categorized **1-click "Try It" cards** that populate *and immediately execute* the computation, and (c) a compact **Quick Syntax Card** (`*` for multiply: `2*x`, `^` for power: `x^2`, constants: `%pi`, `%e`, `%i`, derivatives: `diff(x^3, x)`).
2. **Mode-Aware Inline Syntax Helper & Auto-Detection Banner**:
   - Dynamically update `#exprInput` placeholder and display a 1-line syntax bar beneath the input matching the selected mode (`Maxima` vs `LaTeX`).
   - If the user types a backslash command (e.g. `\frac{1}{x}`, `\sin(x)`) while in `Maxima` mode, show a non-blocking banner: *"That looks like LaTeX. [Switch to LaTeX mode & re-parse]"*.
3. **Visual Error Highlighting (`error_span`) + One-Click "Fix It" Suggestions**:
   - Render the backend's `error_span` (`start..end`) directly in `#previewBox` using an escaped monospace snippet with a red caret underline (`^~~~`).
   - Map the 6 most common newbie syntax mistakes to actionable hints with a one-click **Apply Fix** button:
     - `2x` / `3sin(x)` -> *"Missing `*` for multiplication. Try `2*x`."*
     - `x**2` -> *"Use `^` for powers in Maxima (or enable `**` support). Try `x^2`."*
     - `pi`, `e`, `i` in trig/complex ops -> *"Tip: use `%pi`, `%e`, `%i` for exact symbolic constants."*
     - `x^2 = 4` in `solve` -> *"Equations are solved as `lhs - rhs = 0` (or `lhs = rhs` once supported). Try `x^2 - 4`."*
     - `sin x` -> *"Function arguments need parentheses: `sin(x)`."*
     - Unbalanced `(` / `)` -> *"Missing closing `)` near character N."*
4. **Plain-Language Labels & Progressive Disclosure**:
   - Rename jargon in the primary UI: `"e-graph Saturation · Procedural CAS"` -> `"Symbolic Math Engine (Maxima-compatible)"`; move `"Fast-Math IEEE relaxing"` into a collapsible **"Advanced Settings"** drawer with a plain-English tooltip (*"Applies algebraic identities like `sqrt(x^2) = x` assuming real positive domains"*).
   - Provide human-readable `CATEGORY_NAMES` for `matrix` ("Linear Algebra & Matrices") and `complex` ("Complex Numbers") and render `param.accepts` enums (`f64`/`f32`, `plus`/`minus`/`both`) as `<select>` dropdowns.
5. **Parameter Persistence & Notebook Ergonomics**:
   - Never wipe user-edited or example-chip params when `/api/parse` detects variables.
   - Add per-cell **Edit & Re-run**, **Delete**, and **Export (.md / .tex)** actions, fix **Use as input** to respect the current input mode (Maxima vs LaTeX), fix **Copy** to copy the currently active tab (`Result`, `LaTeX`, or `Rust`), bind `Shift+Enter` and `Ctrl/Cmd+Enter` inside parameter fields, and bind `Escape` to close modals.
   - Fix Accessibility: add `<label for="...">` on every control, `aria-live="polite"` on `#previewBox` and `#notebook`, focus trapping inside `#rulesModal`, and WCAG AA `>= 4.5:1` contrast in both light and dark themes.

### 4.2 Curated Newbie Starter Example Pack (12 Click-to-Run Examples)

Ship these 12 categorized examples as clickable starter cards (each sets `input_format`, `expression`, `operation`, and `params`, and optionally auto-runs):

| # | Category | Button Label | Mode | Input Expression | Operation & Params | Expected Result | What It Teaches a Newbie |
| :-: | :--- | :--- | :---: | :--- | :--- | :--- | :--- |
| 1 | Basics | Simplify trig identity | Maxima | `sin(x)^2 + cos(x)^2` | `simplify` | `1` | Basic syntax, `^` powers, instant symbolic simplification |
| 2 | Algebra | Expand a binomial | Maxima | `(x + 2)^3` | `expand` | `x^3 + 6*x^2 + 12*x + 8` | Explicit `*` vs `+`, polynomial expansion |
| 3 | Algebra | Factor a quadratic | Maxima | `x^2 - 5*x + 6` | `factor` | `(x - 2)*(x - 3)` | Polynomial factoring with `*` multiplication |
| 4 | Equations | Solve a quadratic | Maxima | `x^2 - 4` | `solve` (`variable: "x"`) | `[-2, 2]` | Solving `expr = 0` for a target variable |
| 5 | Calculus | Derivative of a product | Maxima | `x^3 * sin(x)` | `differentiate` (`variable: "x"`) | `3*x^2*sin(x) + x^3*cos(x)` | Product rule & explicit `*` between `x^3` and `sin(x)` |
| 6 | Calculus | Indefinite integral | Maxima | `3*x^2 + cos(x)` | `integrate` (`variable: "x"`) | `x^3 + sin(x)` | Antiderivatives & symbolic integration |
| 7 | Calculus | Definite integral | Maxima | `sin(x)` | `defint` (`variable: "x"`, `lower: "0"`, `upper: "%pi"`) | `2` | Using exact constant `%pi` in parameter bounds |
| 8 | Calculus | Classic limit | Maxima | `sin(x) / x` | `limit` (`variable: "x"`, `point: "0"`) | `1` | Removable singularities (`0/0`) via symbolic limits |
| 9 | Calculus | Taylor series | Maxima | `exp(x)` | `taylor` (`variable: "x"`, `center: "0"`, `order: "5"`) | `1 + x + x^2/2 + x^3/6 + x^4/24 + x^5/120` | Power series expansion around a center |
| 10 | Linear Algebra | 2x2 determinant | Maxima | `matrix([a, b], [c, d])` | `determinant` | `a*d - b*c` | Symbolic `matrix([...])` syntax |
| 11 | Complex | Complex polar/rect form | Maxima | `(1 + %i)^2` | `rectform` | `2*%i` | Exact imaginary unit `%i` |
| 12 | LaTeX Mode | Integrate from LaTeX | LaTeX | `\frac{1}{x} + x^2` | `integrate` (`variable: "x"`) | `\ln(x) + \frac{x^3}{3}` | Pasting textbook LaTeX directly into LaTeX mode |

---

## 5. Phased Implementation Plan, Tests & Phase Passing Gates (Phases 46–53)

Every phase below is self-contained, strictly ordered by dependency and risk, and governed by a **hard passing gate**. Do not advance to Phase $N+1$ until every test and command in Phase $N$'s gate exits `0`.

---

### Phase 46 — Security, Build/CI Integrity & WebUI Contract Hotfixes (Stop-Ship)

**Goal:** Eliminate XSS, fix broken CI/Docker builds, stop parameter wiping, and align all frontend/backend JSON contracts.

**Implementation Tasks:**
1. **XSS Remediation**: Escape all dynamic text (`res.error`, `err.message`, `request.operation`, `statusLabel`) in `maxima-web/static/js/{preview,notebook,main}.js` via `textContent` or `escapeHtml`. Add HTTP security headers (`CSP`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`) in `maxima-web/src/app.rs`.
2. **CI & Docker Repair**:
   - Update `.github/workflows/ci.yml:108` to check every JS file (`for f in maxima-web/static/js/*.js; do node --check "$f"; done`).
   - Add `scripts/verify_css_classes.py` to CI and expand its parser to inspect interpolated template classes and `classList.add/toggle/remove`.
   - Expand `.github/workflows/ci.yml:77-90` `feature-matrix` to include `bfloat`, `matrix`, `complex`, `specfn`, `transform`, `assume`, plus `tensor` on `maxima-rs`.
   - Upgrade `Dockerfile:12` to `rust:1.88-bookworm` (or latest MSRV), remove `Cargo.lock` from `.dockerignore:11`, copy `Cargo.lock` in `Dockerfile`, build with `cargo build --release --locked -p maxima-web`, set `AWS_LWA_READINESS_CHECK_PATH=/api/health`, align default port, and bump `template.yaml` `MemorySize` to `1536` MB.
   - Add `tensor = ["sym-core/tensor"]` to `maxima-rs/Cargo.toml` and include it in `full`.
3. **Backend Config, Isolation & Contract Fixes**:
   - Change `Cli` fields in `maxima-web/src/config.rs` to `Option<T>` so `--config <toml>` values are not overwritten by clap defaults; add `with_graceful_shutdown` in `maxima-web/src/main.rs`.
   - Wrap `/api/parse` (`maxima-web/src/app.rs:95-111`) in `spawn_blocking` + `catch_unwind` + `timeout` + semaphore.
   - Add hard bounds on `taylor` `order` (`0..=64`), `params` string lengths, `variables` count (`<= 16`), and `env` count (`<= 64`).
   - Validate Rust identifiers in `service.rs:1241-1249, 1926-1932` before emitting function signatures.
   - Add `fast_math: bool` to `ComputeRequest` (`api.rs`) and wire it into `SaturationConfig`; unify `params` vs top-level lookup for `variable`, `variables`, `float_type`, `charpoly`, and `residue`; reject invalid `float_type` with HTTP 400; gate `parse_maxima_cas` on `input_format != "latex"`; eliminate double-parse on error; update `resp.operation` after auto-dispatch; support vector `[f1, f2]` inputs in `jacobian`.
4. **Frontend Broken-Feature Fixes**:
   - Preserve user/example params in `palette.js` when `preview.js` updates detected variables; add `AbortController`/sequence counter in `preview.js`.
   - Render dynamic `env` key-value inputs when `op.needs_env` is true (`evaluate`).
   - Fix Rules modal (`main.js:262-271`) and notebook Rules tab (`notebook.js:79,86,232-240`) to read `source_fn`, `identity`, `fast_math_only`, and `response.extra.rules`.
   - Render `param.accepts` (`float_type`) as `<select>`; fix `d/d[x,y]` chip to split comma-separated variables or target `gradient`; fix binary composer templates & `limit` arity in `service.rs:2246-2323` and `palette.js:299-305`; handle non-JSON HTTP errors in `api.js`; add missing CSS classes (`opacity-75`, `border-danger`) and remove dead `auto-render.min.js`.

**Required Tests:**
- `tests/integration/web_hardening.rs`: Add HTTP router tests for TOML config precedence, `/api/parse` timeout/panic isolation, `taylor` `order = -1` and `order = 10000` rejection (400), Rust codegen identifier injection rejection (400), `fast_math: false` vs `true` behavioral difference, `float_type: "f32"` and invalid `"f128"` (400), `CSP` / `X-Content-Type-Options` headers, and HTTP 413 body limit.
- `tests/integration/webui_e2e.rs`: Route tests through `build_app(...).oneshot(...)` (real HTTP/JSON), verifying `/api/rules` schema matches frontend expectations, `evaluate` with `env`, vector `jacobian`, and `functions_catalog()` arity/templates.
- JS syntax & CSS verification: `node --check maxima-web/static/js/*.js` and `python3 scripts/verify_css_classes.py`.

**Phase 46 Passing Gate:**
- `cargo test --workspace --all-features` passes with 0 failures.
- `for f in maxima-web/static/js/*.js; do node --check "$f" || exit 1; done` exits `0`.
- `python3 scripts/verify_css_classes.py` exits `0` with 0 missing classes.
- `docker build .` succeeds cleanly with `--locked` and no network fetches outside Cargo cache.
- Zero unescaped `innerHTML` sinks remain in `maxima-web/static/js/` (verified via grep audit).

---

### Phase 47 — E-Graph Soundness, Core Numbers, Parser, Codegen & Macros Hardening

**Goal:** Eliminate mathematical unsoundness in rewrite rules, number conversions, parsers, LaTeX printer, CSE, codegen, and `sym-macros`.

**Implementation Tasks:**
1. **Sound Rewrite Rules & Maxima Simplification Flags (`sym-core/src/rules/*`, `lang.rs`)**:
   - Add `egg::Condition` guards (`is_not_zero`, `is_positive`, `is_nonneg_int`) to `div-zero`, `div-self`, `pow-zero`, `pow-zero-base`, `pow-add-exp`, and `factorial` rules so `0/0`, `0^0`, `0^(-1)`, and `(-1)!` never collapse the e-graph.
   - Replace monolithic `fast_math` with structured Maxima-aligned `SimpFlags` (`domain: Real | Complex`, `radexpand: All | True | False`, `logexpand: True | All | Super | False`, `triginverses: True | All | False`, `fast_math: bool`). Gate `pow-mul-exp`, `ln-exp`/`ln-mul`/`ln-div`/`ln-pow`, and `asin-sin`/`acos-cos`/`atan-tan` appropriately.
   - Add missing core bridges: `(^ e ?x) <=> (exp ?x)`, `(* i i) => -1`, `(^ i 2) => -1`, and canonical `%pi`-multiple trig rules.
   - Fix `MathAnalysis::merge` to never silently pick `merge_max` on conflicting constants; represent `SpecialFn` arguments as child `Id`s instead of comma-joined strings.
2. **Numbers & `bfloat` (`rational.rs`, `bfloat.rs`, `expr.rs`)**:
   - Fix `Rat::to_f64` (avoid `inf/inf = NaN` via `ToPrimitive`/bit-shift scaling); fix `Rat::pow_int` (`i64::MIN`, `u32` truncation, exponent cap `<= 10_000`, `0^0 -> None`); fix `Rat::from_f64` (exact IEEE-754 mantissa/exponent conversion instead of lossy `i64` continued fraction).
   - Change `BfloatWrapper::{sin,cos,tan,atan,pow}` to return `Result<Self, String>` (never returning input `x` on error or panicking); validate `fpprec >= 1`; add guard digits / `expm1` / asymptotic clamping for `sinh`/`cosh`/`tanh`; normalize `PartialEq`/`Hash`.
   - Fix `Expr::eval(NaN!) -> None`, domain-error `Pow`/`Asin`/`Acos -> None`, and `Expr::is_const()` for `Pi`/`E`.
3. **Parsers, Printers, CSE, Codegen & Macros**:
   - `parser/latex.rs`: Fix UTF-8 byte-slice panic & nested braces in `\sum`; parse `d<var>` and preserve bounds in `\int`; map `\pi` to `Expr::Pi`; enforce word boundaries (`\sinc`, `\cdots`); preserve numeric subscripts (`x_{0}`); add `RecursionDepthGuard` in `power()`.
   - `parser/mod.rs` & `parser/maxima.rs`: Cap exponent in `parse_exact_rational` (`<= 10_000`); add `RecursionDepthGuard` in `power()`; tokenize `!!` distinctly; reject trailing tokens in `parse_maxima_cas`; return all roots in `execute_cas(Solve)`; align `Expr::Display` with `parse_maxima` roundtrip.
   - `latex.rs`: Escape `"#$%&_"` in `escape_latex`; fix double parens in `Neg(Add/Sub)`; fix `#[cfg(feature = "complex")]` arm leak; parenthesize `Bfloat` under `Pow`/`Neg`.
   - `cse.rs`, `codegen.rs`, `pipeline.rs`: Fix CSE temp shadowing, bound-variable hoisting, `BTreeMap` determinism, and single-use binding elimination; parenthesize `Expr::Neg` and negative literals before method calls in `codegen.rs`; emit free-variable parameters and tuple return types in `Pipeline::to_rust_fn`; handle `NaN`/`Inf`/keywords safely.
   - `sym-macros`: Parse `#[fast_math = false]`, `node_limit`, `time_limit_ms`, and infer `FloatType` from function signature (`f32`/`f64`); enforce exact arity, comma separation, duplicate-field rejection, exact decimal literal parsing, and `gradient` variable validation.

**Required Tests:**
- Unit/integration regression tests for every bug above: `0/0` & `0^0` non-collapse, `asin(sin(%pi)) != %pi` under default `triginverses`, `((-1)^2)^(1/2) == 1`, `Rat::to_f64` on `10^400/(10^400+1)`, `Rat::pow_int(2, i64::MIN)`, `BfloatWrapper::sinh(1b-10)` accuracy, `parser/latex` UTF-8 `\sum_{i=α}^{n}`, `2^2^...` depth guard (no stack overflow), `codegen` compiling and numerically verifying `cos(-x)`, `(-x)^2`, `exp(-x)`, and `sym_fn!` `f32` + `#[fast_math = false]` compile-pass / arity-error compile-fail tests (`trybuild` or macro unit tests).

**Phase 47 Passing Gate:**
- `cargo test --workspace --all-features` passes 100%.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes with 0 warnings.
- `tests/proptest/print_parse_roundtrip.rs` passes 1,000+ cases including `%pi`, `%e`, `%i`, `x_0`, `Matrix`, `Bfloat`, and `SpecialFn`.

---

### Phase 48 — Assumptions, Complex Analysis & Polynomial/Rational Engine Parity

**Goal:** Make `Assumptions`, `complex`, and `poly` mathematically sound and multivariate-capable, matching Maxima 5.50 core semantics.

**Implementation Tasks:**
1. **Assumptions & Sign Engine (`sym-core/src/assume/mod.rs`)**:
   - Normalize all relations into a directed strict/non-strict inequality graph (`<`/`<=` stored as reverse `>`/`>=`, `=` as bidirectional `>=`, `!=` tracked explicitly).
   - Fix BFS/DFS transitivity so `a > b` and `b >= c` proves `a > c`, detect strict cycles as contradictions, check `(Const, Var)` bounds, and check negated relations (`assume(a > b) => is(a < b) == False`).
   - Fix `csign` for `Exp` and `Pow` (require real exponent; handle `0^0`, `0^(-n)`, `Complex^even`, `2^%i`).
   - Fix `has_property` (`Complex` includes reals; `Pi`/`E`/`I` properties; contradiction checks between `var_bounds` and `var_props`).
2. **Complex Analysis (`sym-core/src/complex/mod.rs`, `expr.rs`)**:
   - Add `Expr::Abs(Box<Expr>)` and `Expr::Atan2(Box<Expr>, Box<Expr>)` to `sym-core` (with parser, LaTeX printer, diff, and eval support).
   - Rewrite `carg_pair` to use `Assumptions::csign` + `Expr::Atan2(v, u)` (fixing all `Re(z) < 0` quadrants).
   - Fix `cabs` to return `Expr::Abs(u)` unless proven `Pos`/`NonNeg`.
   - Guard `smart_div`/`smart_pow` on zero/non-positive exponents; fix symbolic-exponent `pow_pairs` via `exp(e * (ln(|r|) + %i*%pi))`; implement `logarc`-based `rectform` for `Asin`/`Acos`/`Atan` (or unevaluated `realpart`/`imagpart` nouns); make `residue` dynamically determine pole order instead of hardcoding 6.
3. **Polynomials & Rational Functions (`sym-core/src/poly/*`)**:
   - Fix `Poly::scale(0)` normalization; return `Result<Fraction, PolyError::DivisionByZero>` from `Fraction::div` and remove `den == 0 => 0` in `to_expr`; use `checked_add`/`checked_mul` in `Monomial`.
   - Implement `poly_derivative(p, var_idx)` and multivariate exact division + primitive/subresultant (or modular) multivariate GCD over $\mathbb{Z}[x_1,\dots,x_n]$.
   - Implement Yun's square-free factorization + Berlekamp/Cantor-Zassenhaus + Hensel lifting for univariate $\mathbb{Z}[x]$ factorization (`factor_poly`), clearing denominators, grouping repeated powers `(x-1)^2`, and normalizing integer content (fixing `i64` overflow in `divisors`).
   - Upgrade `partfrac` to reduce first, perform polynomial long division for improper fractions, and use extended Euclidean decomposition over repeated poles and irreducible quadratics (`1/(x^2+1)`, `1/(x-1)^2`).
   - Upgrade `ratsimp` to map transcendental/radical kernels (`sin(x)`, `exp(x)`, `sqrt(x)`) to temporary variables, and `expand` to distribute unary negation and recurse into function arguments.

**Required Tests:**
- `tests/integration/assume_deduction.rs`: Add tests for `a<b & b<c => a<c`, `a>b & b>=c => a>c`, `assume(a>b) => is(a<b) == False`, `assume(x>5) => is(5<x) == True`, contradiction rejection (`a>b` & `b>a`; `x>0` & `Neg`), `csign(exp(%i*%pi)) == Neg`, `csign(%i^2) == Neg`, `csign(2^%i) == Complex`.
- `tests/integration/complex_analysis.rs`: Test all 4 quadrants of `carg(±1 ± %i)`, `cabs(x) == abs(x)`, `rectform((-2)^x)`, `rectform(asin(%i))`, `residue(exp(z)/z^8, z, 0) == 1/5040`.
- `tests/integration/poly_rat.rs` & `poly_expand.rs`: Test `ratsimp(1/(x-x))` returns `Err(DivisionByZero)`, multivariate `gcd(x^2-y^2, x-y) == x-y`, `factor(x^4 + 3*x^2 + 2) == (x^2+1)*(x^2+2)`, `factor(2*x^2 - 3*x + 1) == (2*x-1)*(x-1)`, `partfrac((x^3+1)/(x^2-1))`, `partfrac(1/((x-1)^2*(x^2+1)))`, `ratsimp((sin(x)^2 - 1)/(sin(x) - 1)) == sin(x) + 1`.

**Phase 48 Passing Gate:**
- All `assume_deduction`, `complex_analysis`, `poly_expand`, and `poly_rat` tests pass with zero `#[ignore]`.
- Zero `Poly::scale(0)` invariant violations and zero `DivisionByZero -> 0` silent fallbacks.
- All 4 quadrants of `carg` match Maxima 5.50 `References/maxima-master/tests/rtest_carg.mac` core cases.

---

### Phase 49 — Calculus Engine Overhaul (`diff`, `series`, `limit`, `integrate`, `defint`)

**Goal:** Replace broken/stubbed calculus routines with mathematically verified algorithms aligned with Maxima 5.50 (`comm.lisp`, `hayat.lisp`, `limit.lisp`, `sin.lisp`, `risch.lisp`, `defint.lisp`).

**Implementation Tasks:**
1. **Differentiation (`sym-core/src/diff.rs`)**:
   - Differentiate `Expr::Factorial(u)` as `u! * psi(0, u + 1) * du` (or unevaluated `Derivative` without `specfn`).
   - Apply FTC `d/dx(∫ f(x) dx) = f(x)` and differentiate inside constant-bound `Sum`; guard `smart_pow(0, -n)` and `0^0`.
2. **Series (`sym-core/src/series.rs`)**:
   - Fix Laurent truncation orders in `Series::mul`, `Series::inv`, and `Series::diff`.
   - Fix `sin`/`cos`/`exp`/`ln` series around symbolic centers (zero coefficient 0 explicitly + upgrade `fold` to cancel `c - c`, `0*x`, `0+x`).
   - Add `Tan`, `Sqrt`, fractional/symbolic `Pow` (`(1+x)^a`), `Sinh`/`Cosh`/`Tanh`, `Asin`/`Acos`/`Atan`, and logarithmic/Puiseux terms at `0`.
3. **Limits (`sym-core/src/limit/*`)**:
   - Fix `is_zero_equivalent` (`s.coeffs.is_empty()` and never fall through after `Poly::from_expr` proves non-zero).
   - Use fresh gensym instead of hardcoded `"t"`; handle `s.valuation < 0` to return `inf`, `minf`, or `und`; add directional `LimitDir::{Both, Plus, Minus}` API.
   - Implement real Gruntz MRV algorithm in `mrv.rs` (proper comparability `f ≻ g`, rewriting in terms of most rapidly varying subexpression) + L'Hôpital fallback; remove unsound `numeric_limit` fallback (or require strict symbolic proof and tag any numeric estimate explicitly as `Status::Partial` per ADR-0012).
4. **Integration & Definite Integration (`sym-core/src/integrate/*`)**:
   - Fix `integrate_power_denominator` (`n < 2`), `is_proven_non_elementary` (reject zero constant factors), `integrate_product` (expand polynomial products and propagate `Assumptions`), and `ibp::trig_powers` (polynomial-size recurrence instead of $O(2^n)$ cloning).
   - Replace stubs in `hermite.rs` (true Hermite reduction for repeated poles) and `lrt.rs` (Lazard-Rioboo-Trager for irreducible quadratic/higher denominators, handling `1/(x^2+1) -> atan(x)`).
   - Expand `sin.rs` (table of `tan`, `tanh`, `sqrt`, `asin/acos/atan`, `c^x`, symbolic linear params `sin(a*x+b)`) and `risch.rs` (bidirectional `proportional_constant`, rational-function + elementary logarithmic/exponential Risch tower).
   - Fix `defint.rs`: call `.reduce()` before pole detection; detect irrational poles (`1/(x^2-2)`), transcendental poles (`tan(x)`), and endpoint poles (`1/x` on `[0,1]`); use `contains_integral(&f)` for nested unevaluated integrals; apply `ln|u|` consistently and handle `minf` bounds cleanly.

**Required Tests:**
- `tests/integration/differentiation.rs`: `diff(x!, x)`, `diff(integral(sin(x), x), x) == sin(x)`.
- `tests/integration/series.rs`: Laurent `mul`/`inv`/`diff` precision checks; `taylor(sin(x), x, %pi, 5)`, `taylor(tan(x), x, 0, 7)`, `taylor(sqrt(1+x), x, 0, 4)`.
- `tests/integration/limits.rs`: `is_zero_equivalent(sin(x)) == false`, `limit(exp(1/x), x, 0) == und`, `limit(exp(1/x), x, 0, plus) == inf`, `limit(exp(1/x), x, 0, minus) == 0`, `limit(sin(2*%pi*x), x, inf) == ind`, `limit(1/x^2, x, 0) == inf`, `limit(t*x/(x+1), x, inf) == t` (no `"t"` collision).
- `tests/integration/integrate_elementary.rs`, `integrate_risch.rs`, `integrate_defint.rs`: `integrate(x*(x+1), x)`, `integrate(1/(x^2+1), x) == atan(x)`, `integrate(1/(x-1)^2, x) == -1/(x-1)`, `integrate(sin(a*x), x) == -cos(a*x)/a`, `integrate(exp(0*x^2), x) == x`, `defint((x^2-1)/(x-1), x, 0, 2) == 4`, `defint(1/(x^2-2), x, 0, 2) == Divergent`, `defint(tan(x), x, 0, %pi) == Divergent`, `defint(1/(x-2), x, 0, 1) == -ln(2)`.

**Phase 49 Passing Gate:**
- All calculus integration & proptest suites (`differentiation`, `series`, `limits`, `integrate_*`) pass 100%.
- Differential-verification proptest (`d/dx(∫ f dx) == f`) passes across 500+ randomized rational, trig, exp, and product integrands.
- Zero false `ProvenNonElementary` and zero false `Convergent` results on pole-containing definite integrals.

---

### Phase 50 — Solvers, Linear Algebra, Special Functions, Transforms & Web Service Wiring

**Goal:** Repair all solver/matrix/Laplace bugs and expose every implemented `sym-core` mathematical engine in `maxima-web/src/service.rs`.

**Implementation Tasks:**
1. **Algebraic & ODE Solvers (`sym-core/src/solve/*`)**:
   - Fix `ode_second.rs`: fold `disc` via `Poly`/`expand` before matching `Expr::Const`; handle `c == 0` (`_C1 + _C2*x`); normalize leading sign `a`; support symbolic discriminants via `Assumptions`.
   - Fix `isolate.rs`: fix `a - var = 0` sign error (`Some(a)`), reject circular `rhs` containing `var`, support `Add(f(x), c)` and `Sub(c, f(x))`.
   - Upgrade `alg.rs`: return complex roots (`solve(x^2+1, x) -> [%i, -%i]`), support parametric/multivariate linear & quadratic equations (`solve(a*x+b, x) -> [-b/a]`), distinguish `0 = 0` (`All`) from no solution (`Empty`), and add cubic (Cardano), biquadratic, and `linsolve` (Gaussian/Bareiss elimination).
   - Expand `ode.rs`/`ode_first.rs`: normalize `a.is_zero()` via `Poly`, support `y'/c`, separable, linear integrating-factor (`y' + p(x)*y = q(x)`), exact, and non-homogeneous constant-coefficient 2nd-order ODEs (undetermined coefficients / Laplace).
2. **Linear Algebra (`sym-core/src/matrix/bareiss.rs`, `mod.rs`)**:
   - Run Bareiss directly over `Poly` using Phase 48's multivariate exact division (and kernel mapping for transcendental entries); fix `is_expr_zero` to use full algebraic simplification; upgrade `adjoint` to $O(n^3)$ fraction-free Gauss-Jordan; guard `decode_expr` against `den == 0`; upgrade `eigenvalues` to return complex & symbolic roots with multiplicities.
3. **Special Functions & Transforms (`sym-core/src/specfn/*`, `transform/*`)**:
   - Add `Expr::Laplace` / `Expr::Ilt` unevaluated noun forms (or `Result<Expr, TransformError>`) in `transform/laplace.rs` so unmatched inputs **never** silently return `1/s`, `1/(s^2+1)`, or `f(t)/s`.
   - Fix `ilt_rational` to verify `s`-independence of numerator/constants, use Phase 48 `partfrac` for general rational $P(s)/Q(s)$, handle `A*s + B` with `A != 1`, and use `BigInt` factorial (preventing `i64` overflow).
   - Fix `bvp.rs` (`ic1`/`ic2`/`bc2`) to simplify `a` and `det` before checking `is_zero()`, and validate linearity in integration constants (or delegate to `solve`).
   - Add degree cap in `orthopoly.rs` (`n <= 100`) and basic `f64`/`bfloat` evaluation for `gamma`, `erf`, `erfc`, `bessel_j` (or document exact-only scope cleanly).
4. **Full Web Service & Embedded CAS Dispatch (`maxima-web/src/service.rs`)**:
   - Expose `laplace`, `ilt`, `ic1`, `ic2`, `bc2`, `bfloat` (with `fpprec`), `assume`, `is`, `csign`, `adjoint`, `transpose`, and `orthopoly` in `OPERATIONS` and `run_operation`.
   - Wire all operations into `parse_maxima_cas` embedded auto-dispatch (`diff`, `determinant`, `invert`, `charpoly`, `trace`, `eigenvalues`, `rectform`, `polarform`, `realpart`, `imagpart`, `conjugate`, `carg`, `cabs`, `residue`, `laplace`, `ilt`, `bfloat`, `assume`, `is`).
   - Fix `verify_integral`, `verify_defint`, and `verify_ode` to use full AST substitution, handle symbolic parameters & `minf`, and sample multiple non-trivial points.

**Required Tests:**
- `tests/integration/solvers_ode.rs`: Test `y'' + 3*y' + 2*y = 0`, `y'' + 2*y' + y = 0` (repeated root), `y'' + 2*y' + 5*y = 0` (damped oscillatory), `y'' = 0` (`_C1 + _C2*x`), `-y'' - y = 0`, and non-homogeneous `y' + y = x`.
- `tests/integration/solvers_alg.rs`: Test `solve(x^2 + 1, x) == [%i, -%i]`, `solve(a*x + b, x) == [-b/a]`, `solve(5 - x, x) == [5]`, `solve(exp(x) - x, x)` does NOT return circular `[ln(x)]`, `linsolve([x+y-3, x-y-1], [x, y])`.
- `tests/integration/matrix_bareiss.rs`: Test $3\times 3$ and $4\times 4$ multivariate symbolic determinants/inverses, `det([[sin(x)-sin(x), 1], [1, 1]]) == -1`, `eigenvalues([[0, -1], [1, 0]]) == [%i, -%i]`.
- `tests/integration/transform_laplace.rs`: Test `laplace(exp(t^2), t, s)` returns unevaluated noun (NOT `1/s`), `laplace(sin(t^2), t, s)` returns noun (NOT `1/(s^2+1)`), `ilt((2*s + 3)/(s^2 + 4), s, t) == 2*cos(2*t) + (3/2)*sin(2*t)`, `ilt(1/s^25, s, t)` (no overflow), and singular `ic2` returns `Err`.

**Phase 50 Passing Gate:**
- `cargo test --workspace --all-features` passes 100% including all new solver, matrix, Laplace, and HTTP API tests.
- Every operation implemented in `sym-core` is reachable via both typed `/api/compute` and Maxima command syntax.
- Zero silent bogus-formula fallbacks in `laplace`, `ilt`, `ode_second`, or `isolate`.

---

### Phase 51 — Newbie-First UI/UX Overhaul, Session State & Accessibility

**Goal:** Make the WebUI effortless for first-time math users while supporting stateful Maxima notebook workflows.

**Implementation Tasks:**
1. **Session State (`maxima-web/src/app.rs`, `service.rs`, `api.rs`)**:
   - Add optional `session_id` (in-memory LRU/TTL session store bounded by max sessions & idle timeout, or client-carried `session_context`) supporting `%` (last output), `%oN` references, variable assignments (`a: x^2 + 1`), `assume(x > 0)` / `facts()` / `forget(...)`, and `kill(all)`.
   - Thread session `Assumptions` into `integrate_with_assumptions`, `limit_with_assumptions`, `defint`, and `csign`.
2. **Newbie Onboarding & Guided Input (`maxima-web/static/`)**:
   - Implement the **Interactive Empty-State Onboarding** panel with the **12 Curated Starter Examples** from Section 4.2 (clicking loads & immediately runs).
   - Add mode-aware **Inline Syntax Hint Bar** below `#exprInput` and **LaTeX Auto-Detect Banner** (`\frac`, `\sin`, etc. typed in Maxima mode -> one-click switch).
   - Render `error_span` with escaped monospace caret underline (`^~~~`) + **One-Click "Fix It" Suggestions** for `2x`, `x**2`, `pi`/`e`/`i`, `=` in `solve`, `sin x`, and unbalanced parentheses.
   - Replace jargon labels; move `Fast-Math` and `fpprec` into a collapsible **Advanced Settings** drawer with tooltips; add `CATEGORY_NAMES` for `matrix`, `complex`, `transform`, `assume`.
3. **Notebook Workflow, Keyboard & Accessibility**:
   - Add per-cell **Edit & Re-run**, **Delete**, and **Export Notebook (`.md` / `.tex`)**; fix `Use as input` for LaTeX mode and `Copy` for active tab with clipboard fallback; add chronological vs newest-first toggle.
   - Bind `Shift+Enter` and `Ctrl/Cmd+Enter` across `#exprInput` and `#paramsContainer`; bind `Escape` to close modals; add focus trap to `#rulesModal`.
   - Add `<label for="...">` on every form control, `aria-live="polite"` on preview & notebook, responsive mobile layout (<= 768px), and verify WCAG AA `>= 4.5:1` contrast.

**Required Tests:**
- Backend session tests in `tests/integration/webui_e2e.rs`: multi-step session (`a: x^2 + 1` -> `diff(a, x)` -> `integrate(%, x)`), `assume(a > 0)` affecting `integrate`/`limit`/`csign`, `forget` & `kill(all)`, and session eviction/TTL cap.
- Automated UI E2E tests (Playwright/headless browser or Node DOM harness): verify initial empty state renders all 12 starter cards; clicking each of the 12 examples produces the exact expected result without param wiping; typing `\frac{1}{x}` in Maxima mode triggers the LaTeX switch banner; typing `2x` shows the `2*x` Fix-It button and applying it parses cleanly; XSS payload in `#expr=` renders inert text with 0 script execution; keyboard `Shift+Enter` inside a param input triggers compute; `Escape` closes Rules modal.

**Phase 51 Passing Gate:**
- All 12 newbie starter examples pass end-to-end through the real HTTP server + browser UI.
- All backend session tests pass with bounded memory (LRU/TTL verified).
- Automated accessibility check (labels, `aria-live`, keyboard nav, WCAG AA contrast) and XSS immunity tests pass 100%.

---

### Phase 52 — Maxima 5.50 Language Parity & Cooperative Compute Cancellation

**Goal:** Close core Maxima 5.50 syntax/command gaps and eliminate thread/permit leaks on compute timeouts.

**Implementation Tasks:**
1. **Cooperative Cancellation (`sym-core`, `maxima-web`)**:
   - Add `CancelToken` (`Arc<AtomicBool>`) checked inside `egg` saturation `Runner` hook, `series`, `poly`, `integrate`, `bareiss`, and `solve` loops.
   - On `tokio::time::timeout` in `maxima-web/src/app.rs:155-194`, set `cancel.store(true, Ordering::Relaxed)` so the blocking worker aborts promptly and releases its `OwnedSemaphorePermit`. Configure a dedicated `spawn_blocking` thread stack size (`>= 8 MB`) for deep symbolic recursion.
2. **Maxima 5.50 Core Syntax & Function Parity**:
   - Parser (`sym-core/src/parser/maxima.rs`): Support `**` exponentiation, `lhs = rhs` equations (auto-converted to `lhs - rhs` in `solve`/`ode2`), lists `[a, b, c]`, `sec`, `csc`, `cot`, `asec`, `acsc`, `acot`, `asinh`, `acosh`, `atanh`, `atan2`, `abs`, `diff(expr, x, n)` ($n$-th order derivative), and noun quote `'`.
   - Core Maxima operations (`sym-core` + `service.rs`): Implement `trigexpand`, `trigreduce`, `trigsimp`, `radcan` (radical canonicalization), `demoivre`, `exponentialize`, `subst`, `float`, `divide`/`quotient`/`remainder`, `num`/`denom`, and finite `sum`/`product` evaluation.
   - Add `sym-core` `limit` directional syntax (`limit(expr, x, 0, plus)` / `minus`) in parser and WebUI dropdown.

**Required Tests:**
- **Real Timeout & Permit Recovery Test** (`tests/integration/web_hardening.rs`): Run a genuinely expensive computation with `timeout_ms: 150` and `concurrency: 1`; assert HTTP `408`, wait `<= 250ms`, then immediately send a normal request and assert HTTP `200` (proving the worker aborted and the permit was released, not leaked).
- Parser & CAS parity tests for `2**3`, `solve(x^2 = 9, x)`, `diff(x^4, x, 2) == 12*x^2`, `sec(x)`, `atan2(1, -1) == 3*%pi/4`, `trigexpand(sin(a+b))`, `trigreduce(sin(x)^2)`, `demoivre(exp(%i*x))`, `exponentialize(cos(x))`, `subst(x = 2, x^2 + 1) == 5`, `limit(1/x, x, 0, plus) == inf`.

**Phase 52 Passing Gate:**
- Real-timeout cancellation + permit-recovery test passes deterministically 10/10 runs.
- All new Maxima 5.50 syntax & function parity tests pass across both `sym-core` and `/api/compute`.
- Zero `stack overflow` or OOM panics on adversarial inputs (`2^2^...` depth 1000, `1e1000000000`, `taylor` order 10000, `orthopoly` degree 1000).

---

### Phase 53 — Automated Maxima 5.50 Golden Test Harness, Fuzzing & Release Readiness

**Goal:** Establish truthful, automated 1:1 Maxima 5.50 parity measurement, full AST proptest fuzzing, and clean documentation/ADR alignment.

**Implementation Tasks:**
1. **Upstream `.mac` Golden Parity Harness (`tests/integration/golden_parity.rs`)**:
   - Build a lightweight `.mac` test-pair parser in `tests/` that reads real test cases directly from `References/maxima-master/tests/` (`rtest1.mac`, `rtest_trig.mac`, `rtest_integrate.mac`, `rtest_limit.mac`, `rtest_taylor.mac`, `rtest_carg.mac`, `rtest_sign.mac`, `rtest_laplace.mac`, `rtestode.mac`, `rtest_algsys.mac`).
   - Generate a machine-readable **Parity Scoreboard** (`docs/audits/PARITY_SCOREBOARD.md`) reporting exact `Passed / Failed / Unsupported` counts per `.mac` file (no hand-picked 25-case inflation).
2. **Real AST Proptest Fuzzing (`tests/proptest/*.rs`)**:
   - Replace fixed-template generators with recursive `proptest` AST strategies covering arithmetic, trig, exp, ln, rational, and complex expressions; assert root-count completeness in `solve_correctness.rs`; eliminate silent `if let Some` passes.
3. **JSON Contract Tests & Full CI Gate**:
   - Add JSON schema contract tests verifying `/api/operations`, `/api/rules`, `/api/functions`, `/api/parse`, `/api/compute`, and `/api/health`.
   - Ensure CI runs: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, full 19-feature isolation matrix, `node --check maxima-web/static/js/*.js`, `python3 scripts/verify_css_classes.py`, Playwright UI E2E, and `docker build .`.
4. **Documentation & ADR-0012 Truth Alignment**:
   - Sync workspace version (`Cargo.toml` vs `README.md`), replace all `file:///e:/Rust/Max/...` links with relative repo links, align `README.md` status badges (`✅` vs `◐`), update `GLOSSARY.md` (remove `SymOpt` and false Jacobian/Hessian macro claims) and `agents.md` (fix dependency diagram & `agents.md` casing), and record final honest parity metrics.

**Required Tests:**
- Upstream `.mac` golden harness executing `>= 500` real Maxima 5.50 test pairs with tracked pass-rate thresholds per subsystem (zero silent regressions).
- Recursive AST proptest suite (`PROPTEST_CASES=500`) across `diff`, `integrate`, `solve`, `numerical_equivalence`, and `print_parse_roundtrip`.
- Full Playwright browser E2E suite against running `maxima-web` binary.

**Phase 53 Passing Gate:**
- Full CI workflow passes green end-to-end (`fmt`, `clippy`, `test`, 19-feature matrix, JS syntax, CSS verification, UI E2E, `docker build`).
- `docs/audits/PARITY_SCOREBOARD.md` generated automatically from `References/maxima-master/tests/` with `0` unexpected regressions.
- Zero broken local links (`file:///`) and zero ADR-0012 doc/code contradictions across `README.md`, `GLOSSARY.md`, `agents.md`, and `Cargo.toml`.

---

## 6. Master Phase Passing Gate Summary

| Phase | Focus Area | Primary Verification Commands | Hard Exit Criteria |
| :---: | :--- | :--- | :--- |
| **46** | Security (XSS), CI/Docker & WebUI Contract Hotfixes | `cargo test --workspace --all-features`, `node --check maxima-web/static/js/*.js`, `python3 scripts/verify_css_classes.py`, `docker build .` | 0 XSS sinks; CI & Docker green; 0 param wipes; Rules modal, `evaluate`, `float_type`, `fast_math`, TOML config, `/api/parse` isolation all verified |
| **47** | E-Graph Soundness, Numbers, Parsers, Codegen & Macros | `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0/0` & `0^0` safe; branch-cut rules flag-gated; `bfloat`/`Rat` safe; `parser/latex` UTF-8 safe; `codegen` & `sym-macros` (`f32`, arity, `fast_math`) verified |
| **48** | Assumptions, Complex & Multivariate Poly/Rat Engine | `cargo test -p tests --test assume_deduction --test complex_analysis --test poly_rat --test poly_expand` | DAG transitivity & `csign` sound; `Atan2`/`Abs` in `carg`/`cabs`; multivariate `gcd`, full `factor`, full `partfrac`, kernel `ratsimp` pass |
| **49** | Calculus Engine (`diff`, `series`, `limit`, `integrate`, `defint`) | `cargo test -p tests --test differentiation --test series --test limits --test integrate_elementary --test integrate_risch --test integrate_defint` | `diff(x!)` exact; Laurent precision exact; `is_zero_equivalent` & Gruntz MRV sound; Hermite + LRT + Risch + `defint` pole checks pass |
| **50** | Solvers, Bareiss Linear Algebra, SpecFn, Laplace & Service Wiring | `cargo test -p tests --test solvers_ode --test solvers_alg --test matrix_bareiss --test transform_laplace --test webui_e2e` | `ode_second` (`b!=0`), `alg` complex/parametric, $3\times 3$ multivariate Bareiss, `laplace`/`ilt` nouns, and full WebUI dispatch pass |
| **51** | Newbie UI/UX Overhaul, Session State & Accessibility | `cargo test --test webui_e2e`, Playwright E2E + A11y suite | All 12 newbie starter examples pass in browser; `error_span` + Fix-It hints working; session `%`/`a:`/`assume` bounded & tested; WCAG AA & ARIA pass |
| **52** | Maxima 5.50 Language Parity & Cooperative Cancellation | `cargo test --test web_hardening --test parser_maxima --test maxima_cas_parity` | Real 150ms timeout releases permit within 250ms (10/10 runs); `**`, `=`, lists, `trigexpand`/`trigreduce`/`radcan`/`subst`, directional `limit` pass |
| **53** | Upstream `.mac` Golden Harness, AST Fuzzing & Release Readiness | Full `.github/workflows/ci.yml` + `PROPTEST_CASES=500 cargo test --test '*'` | `>= 500` upstream Maxima 5.50 `.mac` cases tracked in `PARITY_SCOREBOARD.md`; 0 `file:///` links; 0 ADR-0012 doc/code contradictions |
