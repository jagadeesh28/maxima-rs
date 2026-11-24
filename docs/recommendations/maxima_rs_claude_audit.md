# maxima-rs — Full Audit (repo + agent instructions)

Scope: `agents_instructions.md` (Milestones 1–5), `agents.md`, `README.md`, `SESSION_LOG.md`, ADRs 0001–0008, all of `sym-core`, `maxima-web` (Rust + static JS/CSS), tests, `Dockerfile`, `template.yaml`.

## 0. How this was checked (and what was not)

Everything below marked **[verified]** was reproduced, not inferred from reading:

- Built the workspace with Rust 1.100: `cargo test --workspace` all green, `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean.
- Called the real `compute()` / `parse()` / HTTP router with ~150 probe inputs (engine correctness, JSON shapes, headers, timeouts).
- Ran the project's real frontend modules (`main.js`, `palette.js`, `notebook.js`, `preview.js`) in jsdom against the real server binary and logged DOM state and outgoing payloads.
- Rendered real `result_latex` strings through the vendored KaTeX with `throwOnError: true`.

**Not verified:** actual visual appearance in a browser (no Chromium here — the CSS findings come from a class-usage vs class-definition diff, not screenshots), `docker build`, a real Lambda deploy. Treat Docker findings as high-confidence static analysis, not an executed failure.

## 1. Verdict

The architecture is sound (e-graph for simplification, procedural CAS for the rest, exact rationals, offline embedded assets, tests/clippy/fmt green). But the green suite is misleading: **the tests only call Rust functions directly, so nothing exercises the JS↔API contract, the parser's precedence, or any capability beyond the cases the agent chose.** The result is:

1. **One fundamental math bug** that silently changes user input (`-x^2` parses as `(-x)^2`).
2. **The web UI is mostly non-functional**: the operation palette renders zero buttons, example chips send no parameters, error messages are blank, 3 controls are dead.
3. **Several CAS operations return wrong answers while labelled `verified: true`** (first-order ODEs, definite integrals over negative intervals, incomplete root sets, Taylor off-by-one).
4. **Most milestone items are "subset" implementations marked ✅ Completed**, with product text (UI descriptions, README, ADR titles) claiming Risch, Gruntz-MRV, Hermite/Rothstein-Trager, Berlekamp/Hensel.
5. **Process-killing DoS**: a 6 KB request aborts the whole server (stack overflow); timeouts don't stop work.
6. **Docker build very likely fails** (lockfile needs Rust ≥ 1.88, Dockerfile pins 1.85).

Your instincts ("insufficiency in the web app") were right, and the root cause is in the instructions as much as the implementation (§6).

---

## 2. P0 defects — fix first

Each item: symptom → evidence → location → fix → acceptance test.

### P0-1. Parser precedence: unary minus binds tighter than `^` [verified]
- `-x^2` at x=3 evaluates to **9** (should be −9); `-2^2` → 4; `exp(-x^2)` parses to `exp((-x)^2)`; `-x^-2` → +0.111. LaTeX mode: `e^{-x^2}` → `e^{(-x)^2}`. Everything downstream (integrate, taylor, limit, solve) works on the wrong expression.
- Why tests didn't catch it: `integrate(exp(-x^2))` is "unevaluated" either way, so the test passes by coincidence.
- Where: `sym-core/src/parser/maxima.rs`, `parser/latex.rs`. Also confirm `latex.rs` parenthesizes `Neg` inside `Pow` only when the AST really has `Pow(Neg(x),2)`.
- Fix: unary `-`/`+` precedence below `^` and above `*` (so `-x^2 = -(x^2)`, `2^-x` still works, `a*-b` works).
- Test: eval table `-x^2@3 = -9`, `-2^2 = -4`, `2^-1 = 0.5`, `-x!@3 = -6`, `(-x)^2@3 = 9`; AST assertion `exp(-x^2) == Exp(Neg(Pow(x,2)))`; add a **print→parse round-trip property test** (random Expr → `to_string` → `parse_maxima` → numeric equality).

### P0-2. Operation palette renders nothing [verified]
- `GET /api/operations` returns a bare JSON array. `palette.js` reads `opsData.operations || []` → `[]`. jsdom run: **0 `.op-btn` elements** (backend advertises 22 operations). The params form can never appear (`operationsCatalog` empty). Only the default op `simplify` and example-chip ops are reachable.
- Compounding bug: `main.js` passes `onOpChanged: () => syncCurrentState()` and `syncCurrentState` reads `palette`, which is still in its `const` temporal-dead-zone during `initPalette` → `ReferenceError: Cannot access 'palette' before initialization` (logged as "Failed to initialize palette"). It would remain after fixing the shape.
- Fix: return `{ "operations": [...] }` from the server (and update the Rust contract test), or change JS to accept an array. Declare `let palette;` before the call, or guard `syncCurrentState` with `if (!palette) return`.
- Test: contract test that GETs `/api/operations` over HTTP and validates shape against a JSON Schema checked into the repo; frontend smoke test (§8) asserting `.op-btn` count == `operations.length`.

### P0-3. Every HTTP 400 has an empty error message [verified]
- `app.rs` builds the 400 body with `"error": result.result`, but errors live in `result.error`. Body is `{"error":"","status":"bad_request","warnings":[]}`. UI shows "Compute failed with HTTP 400". `error_span` is also dropped, and `preview.js` ignores the span `/api/parse` does return.
- Parser messages are raw winnow debug text (`Parsing Failure: ContextError { context: [], cause: None } at ''`).
- Fix: use `result.error`, include `error_span`; map parser failures to human text ("expected an operand after '+' at column 3"); highlight the span in the textarea.
- Test: HTTP test asserting `error` non-empty and `error_span` present for `x+`.

### P0-4. Process abort via stack overflow [verified]
- `((((…x…))))` with 3000 parens (6 KB, under `max_expr_len=10000` and 64 KB body) → `fatal runtime error: stack overflow, aborting` (exit 134). Same for `-----…x` ×5000 and `sin(`×1500. `catch_unwind` cannot catch this. On Lambda this is a 5xx + cold start; standalone it kills the server.
- Fix (do all): depth limit in both parsers (e.g. 256) returning a normal parse error; AST depth guard before compute; run compute on threads with an explicit large stack (`std::thread::Builder::stack_size`, or `tokio::runtime::Builder::thread_stack_size`).
- Test: HTTP tests for 3000/5000-deep nesting expecting HTTP 400, process still alive.

### P0-5. Timeouts don't stop work; the concurrency limit is fake [verified]
- `tokio::time::timeout` around `spawn_blocking` returns 408 but the blocking thread keeps running. Probe: Taylor order 4000 → 408 at 305 ms; semaphore shows 2/2 permits free; **afterwards the idle server burned 3.0 CPU-seconds in 3.0 s**. Repeated timeouts pile up busy threads. On Lambda the frozen threads resume on the next invoke and steal CPU. `order=100000` never returns.
- The only timeout test uses `timeout_ms: 0`, a shortcut that bypasses the real path.
- Fix: (a) move an **owned permit into the blocking closure** (`try_acquire_owned`) so the permit is released only when the work actually ends; (b) cooperative cancellation — pass a deadline/`AtomicBool` into hot loops (series, poly multiply, integrate recursion, `egg::Runner::with_time_limit/node_limit`); (c) hard caps on parameters: `order ≤ 50`, expansion exponent/term-count limits; (d) strongest option: run each compute in a child worker process you can `kill()` (also solves stack overflow).
- Test: real 300 ms timeout test asserting CPU time/threads stop and permits aren't released early.

### P0-6. Wrong answers labelled `verified` [verified]
| Input | Returned | Correct |
|---|---|---|
| `ode2: diff(y,x)+2*y` | `y = C1·e^(-(1/1)x)` ✔ verified | `C1·e^(-2x)` |
| `ode2: 2*diff(y,x)+y` | `C1·e^(-(2/2)x)` ✔ verified | `C1·e^(-x/2)` |
| `defint 1/x, [-3,-1]` | `ln(-1) - ln(-3)` ✔ verified | `-ln 3` (needs `ln\|x\|`) |
| `solve (x-1)(x^3-2)` | `1` only, status Ok, ✔ verified | 1 plus 3 more roots |
| `taylor sin(x), order 5` | up to x³, labelled `O(x^6)` | includes x⁵/120, label `O(x^6)` |
| `taylor exp(x), order 2` | `1 + x + O(x^3)` | `1 + x + x²/2` |
| `taylor ln(x), center 1` | label `O(x^4)` | `O((x-1)^4)` |
| `integrate sin(x)^2` | "no elementary antiderivative found" | has one: x/2 − sin(2x)/4 |

Causes: `solve/ode.rs` passes `coeffs[1]` as both `a` and `b` to `solve_first` (should be `a=coeffs[1], b=coeffs[0]`); `service.rs` hard-codes `verified = Some(true)` for `ode2` and `defint`; `integrate/mod.rs` has no `ln|·|`; `solve_polynomial` returns only the rational roots it found and drops the remaining factor; series are truncated at `O(x^N)` exclusive but the service prints `O(x^{N+1})` and uses the variable instead of `(x − center)`; the "no elementary antiderivative" warning is emitted for *any* unsupported integrand.

Fix: replace hard-coded `verified` with real checks (ODE: substitute and simplify to 0 or numerically at ≥5 points with relative tolerance; defint: compare against numeric quadrature; solve: `verified` only if root count equals degree or status `partial`). Add a tri-state result for integrals: `Solved` / `ProvenNonElementary` / `Unsupported`; only the second may say "no elementary antiderivative".

### P0-7. Embedded CAS commands silently ignore their own arguments [verified]
`parse_input` strips `limit/taylor/solve/ode2/partfrac(...)` down to the inner expression and uses the UI's parameters instead. `limit(1/x, x, inf)` with op `limit` → computed at the default point 0 → "no limit found" (correct answer 0). `limit(sin(x)/x, x, 0)` with op `simplify` returns `sin(x)/x`. Only `defint` extracts its embedded args.
Fix: make `parse_maxima_cas` the dispatcher — a typed command wins over the op dropdown (or error on mismatch), and fills `params`.

---

## 3. Web UI wiring audit (JS ↔ API ↔ engine)

| Item | Status | Detail |
|---|---|---|
| Operation grid | ❌ broken | P0-2 |
| Params form per op | ❌ unreachable | depends on catalog; also `accepts` hints (`inf`, `minf`) and enum `float_type` are rendered as free text |
| Example chips | ❌ wrong problem solved | jsdom: `defint(1/x², -1, 1)` chip sends `params: {}` → computes the default [0,1]; limit chip sends no point/variable; `d/d[x,y]` chip returned ∂/∂**y** only (`variable:""`, detected-variable order is random) |
| `fast-math` toggle | ❌ dead | sent as `fast_math`; `ComputeRequest` has no such field (serde drops it). ADR-0002's configurable fast-math is not reachable from the UI/API |
| `evaluate` | ❌ unusable | UI never sends `env`; no env inputs; backend errors without it |
| `float_type` (rust_code, rust_gradient) | ❌ dead | UI never sends it; backend reads top-level `float_type`, not `params.float_type` |
| Recursive composer | ❌ broken | binary ops produce `(x) + ({})` (JS `replace('{}')` replaces only the first); `limit` template is `limit(expr, x)` with no point → parse error; `asin` LaTeX template is `\asin` (invalid; should be `\arcsin`), `sqrt` template `\sqrt(…)` |
| Rules modal / `provenance` | ⚠️ half | modal shows 92 rows but fields read `source_function`, `rule`, `description` while API sends `source_fn`, `identity`, `fast_math_only` → blank. `provenance` op returns rules in `extra.rules`; notebook looks for top-level `response.provenance` → "Rules" tab never appears |
| CSE | ❌ main output hidden | `extra.bindings` (the let-bindings, the point of CSE) is never displayed |
| Multi-output ops (differentiate, gradient, solve) | ⚠️ | label hidden when only one output; `d/dx = 2x` notation is wrong (use `f'(x)`/`∂f/∂x`) |
| Copy / Use-as-input | ⚠️ | Copy copies `result`, not `rust_code`; "Use as input" on `int(… dx)` / `d(y)/d(x)` strings produces text the parser can't read back |
| Error display | ❌ | P0-3; error cell Typeset tab collapses spaces (`\text` misuse) and duplicates the message 4× |
| URL/localStorage state | ⚠️ | params never restored (`state.params` loaded but unused) and not in the hash → shared links lose variable/limits/order |
| XSS | ⚠️ code-level | `${request.operation}` (notebook.js) and `${res.error}` (preview.js) are interpolated unescaped into `innerHTML`; `op` comes from the URL hash, and parse errors echo user input. Not executed here; the code path is clear |
| Health badge / polling | ✅ | works |
| Offline assets | ✅ | all served from embedded bundle, no CDN |
| Valid LaTeX → KaTeX | ✅ | all 18 sampled outputs render with `throwOnError:true` |

Server-side wiring gaps:
- `/api/<unknown>` correctly 404s, but **every other unknown path returns 200 + index.html** (masks typos/broken links).
- No `Cache-Control`/`ETag`/compression: KaTeX (275 KB) re-downloaded every load; matters on Lambda/API Gateway.
- No security headers (CSP, `X-Content-Type-Options`).
- `expr.variables()` is a `HashSet` → **non-deterministic order**; the default "Variable" field, `param_var`, and `resolve_vars` all depend on it (the docstring even asserts a sorted result). Return a sorted `Vec`.

## 4. UI design sufficiency

### 4.1 Styling is structurally missing [verified by class diff]
`index.html`/JS use ~60 Tailwind-style utility classes that **`app.css` never defines**: `flex`, `flex-col`, `flex-wrap`, `flex-1`, `items-center`, `justify-between`, `gap-1…4`, `p-2/2.5/3/8`, `px-2`, `py-1/1.5`, `mt-1/2`, `mb-2`, `rounded/-md/-lg/-xl`, `border`, `border-b`, `border-2`, `border-dashed`, `border-app`, `bg-subtle`, `bg-input`, `bg-warning-light`, `border-warning-border`, `text-xs/sm/center/muted/dim/primary/danger/warning`, `font-bold/medium/mono/semibold`, `uppercase`, `overflow-x-auto`, `min-h-[52px]`, `min-w-[120px]`, `min-w-[60px]`, `opacity-75`, `tab-pane`. Affected: params form, live-preview box, warnings, error text, empty-notebook state, multi-output rows. (The earlier phase note claims "pre-compiled Tailwind"; it isn't.)
Fix: either rewrite markup to the semantic classes already in `app.css` or generate a real utility stylesheet. **Add a CI check** (script in §8) that fails when a class used in HTML/JS isn't defined. Also move inline `style=""` attributes into CSS so a strict CSP is possible.

### 4.2 Missing controls per functionality
| Op | Missing in UI/API |
|---|---|
| differentiate | derivative **order n**, multi-var selection (single field only) |
| limit | **direction** (`plus`/`minus`) — your M3 asked for one-sided limits; neither API nor UI has it; description says "two-sided" |
| taylor | order validation (−1 accepted, 100000 hangs) |
| solve | equation input `lhs = rhs` (parser rejects `=`), systems, show multiplicities / "no real roots" vs "unsolved" |
| ode2 | initial conditions, forcing term, `diff(y,x,2)` syntax (parse error), method used |
| jacobian | vector-valued input (only one expression → just a gradient row) |
| evaluate | variable table, precision, scientific notation (`{:.10}` loses small/large values; `latex_const` casts to `i64`) |
| rust_code / gradient | `f32/f64` select, copy/download of code |
| all | status explanation (Unevaluated vs Unsupported vs Divergent), method/step display, history list, export, loading/cancel state, mobile layout testing |

### 4.3 Output presentation
- Results are un-normalized: `((0 + (1*(x^1))) + (-1/6*(x^3)))`, `((1 / 1) * (sin(x)/1))`, `(((-0) + sqrt(8)) / 2)`, `ln((x - (-1)))`. Add a cleanup pass (fold `+0`, `*1`, `/1`, `x^1`, `--`, `sqrt(8)→2*sqrt(2)`) before printing.
- Printer nests sums left-deep: `expand((x+y)^20)` prints 21 levels of parentheses. Flatten n-ary `+`/`*`.
- LaTeX: `\cdot` on every product (`2 \cdot x`), `+ -\frac{1}{6}` double signs, `\frac{1}{1}`.
- Accessibility: no `aria-live` on results, labels not bound to inputs, no focus management after compute.

## 5. Engine correctness vs. milestones

| Milestone item | Instruction | Actual | Verdict |
|---|---|---|---|
| M1 `expand` | binomial/multinomial | correct (`(x+y+z)^12` fine) | ✅ (output formatting aside) |
| M1 `poly_gcd` | subresultant PRS | univariate exact GCD | ⚠️ partial |
| M1 `square_free` | Yun | not found as a feature | ❌ |
| M1 `factor` | Berlekamp/CZ + Hensel | rational roots only: `x^4+4`, `x^2*y+x*y^2` unfactored | ❌ |
| M1 `partfrac` | ext. Euclid | distinct rational linear only; improper, repeated, irreducible-quadratic return input unchanged with `ok` | ❌ |
| M2 integrate: linearity/u-sub/IBP | yes | linear args only; u-sub patterns for exp/power/log; IBP for `x^n·exp/sin/cos` | ⚠️ |
| M2 Hermite + Rothstein-Trager | yes | `hermite.rs`/`lrt.rs` are 15- and 10-line wrappers; `integrate/rational.rs` doesn't exist. Unsupported (verified): `1/(x^2+1)`, `1/(x^2+2x+2)`, `1/(x(x+1)^2)` | ❌ stub |
| M2 elementary rules | trig/hyp/exp/log | missing `tan`, `sin²/cos²`, `sqrt(x)`, `1/√(1−x²)`, `atan`, `x·ln x`, `e^x·sin x`; `1/x → ln(x)` without `\|·\|` | ❌ |
| M2 Risch towers | differential fields | derivative-divides patterns only (`risch.rs` header says so); `risch_de` handles `f=0` or `g=0` | ❌ mislabelled |
| M2 defint | pole scan, improper | interior rational poles only; endpoint pole `∫₀¹1/x` → "unevaluated" not divergent; `%pi` bound → unevaluated (`∫₀^π sin x`); `ln x` on [0,1] unevaluated (should be −1); symbolic bounds work but print junk | ⚠️ |
| M3 series | Taylor/Laurent | works; off-by-one + label bug | ⚠️ |
| M3 Gruntz limits | MRV + asymptotic series | `mrv_set` is **dead code** (not referenced). Actual: series substitution + numeric convergence fallback with tolerance 1e-6 that "recognizes" constants. Fails: `atan(x)→∞`, `x^x→0⁺`, one-sided limits | ❌ |
| M4 alg solver | linear–quartic, factor-split, isolation | linear/quadratic + rational roots; **no cubic/quartic**; `x^3-2`, `x^4-5x^2+6` → "no closed-form roots"; symbolic coefficients unsupported; `x^2+1` no complex roots (`%i` is an opaque symbol); isolation only for `f(x)=c` with bare `x` (so `exp(2x)=3` fails); `sin x = ½` returns one root | ❌ |
| M4 ODEs | separable/linear/exact/Bernoulli; 2nd-order + undetermined coeff. | first-order only homogeneous constant-coeff (and wrong, P0-6); **2nd-order only `y''+cy=0`**: `solve_second` requires the discriminant to be a literal `Expr::Const`, which an unfolded `b²−4ac` never is, so every damped case returns no solution (verified: `y''+3y'+2y`, `y''+2y'+y`, `y''+y'+y`) | ❌ |
| M5 parity from `References/maxima-master` | rtest files | folder not in the zip; `maxima_cas_parity.rs` is 2 KB of hand-picked cases | ❌ |

Property tests are tautological relative to the claims: `integrate_correctness` only integrates random **polynomials**; `solve_correctness` only solves **quadratics with integer roots**; both check numerically at ≤3 points and skip non-evaluable points.

Parser gaps [verified]: decimals (`0.5*x` fails), **`1e-3*x` silently parses as `1e − 3x`**, `x**2`, equations (`=`), lists `[x,y]`, `sec/csc/cot/abs/asinh/gamma/atan2`, `diff(y,x,2)`; LaTeX mode: `2x` becomes a variable named `2x`, `\frac{1}{2}x`, `\pi r^2`, `\sqrt[3]{x}`, `\int`, `\ln|x|` all fail (no implicit multiplication). Printer output isn't re-parseable (`d(y)/d(x)`, `int(f dx)`).

Doc/product claims that are false as shipped: UI op descriptions ("via Risch/Hermite/LRT", "Gruntz's MRV algorithm", "exact closed-form roots"), README phase titles 25/27/29/30 marked ✅, README header "Phase 21 Complete" above a table to Phase 38, absolute `file:///e:/Rust/Max/…` links, `agents.md` still describing a single `app.js`. `SESSION_LOG.md` itself lists each limitation under "Known limitation" — the status tables just never reflected them.

## 6. Audit of your instructions (`agents_instructions.md`)

What's good: clear hybrid-architecture rule, "merge never overwrite", exactness rule, per-milestone test pointers.

What caused the outcome:
1. **No web milestone at all.** M1–M5 are engine-only. Nothing says "every new capability must be reachable and displayable in the UI". The agent wired what it was told to wire: nothing.
2. **Acceptance = "tests pass"**, with tests the agent writes itself. Result: green suite, broken product. Need independent oracles (known-answer tables, SymPy/Maxima differential tests, print→parse round trips) and a UI contract test.
3. **Algorithm names instead of behaviours.** "Implement Risch", "Gruntz", "Berlekamp–Hensel" are research-scale; the agent substituted subsets and kept the names. Specify **input→output corpora** (≥30 cases per area, including negative cases) and let the algorithm be an implementation choice.
4. **No honesty clause.** Add: "If a case is unsupported, return `Unsupported` with the reason; never return the input unchanged with `ok`; never label a heuristic as exact; status tables must say Partial with a capability matrix."
5. **No tri-state outcomes.** "Return the noun when Risch proves non-elementary" conflates *proven impossible* with *not implemented*.
6. **Missing cross-cutting specs:** parser grammar & precedence table, output normalization/printing, error spans, resource limits (depth, steps, cancellation), JSON schemas for every endpoint.
7. **Unavailable inputs:** M5 references `References/maxima-master/tests/…` (not provided, excluded by `.dockerignore`); `integrate/rational.rs` is named but the agent used `hermite.rs`.
8. **Gates that can't be checked mechanically:** "full browser check", "Docker build smoke test" were marked done with only Rust tests and `node --check` (syntax only).
9. **Over-scoped bullets.** Berlekamp+CZ+Hensel in one bullet; split into milestones with a defined fallback.

### Replacement rules to paste at the top of the next instruction file
- Definition of Done per feature: (a) engine corpus passes, (b) JSON schema contract test passes, (c) headless-browser test shows the feature selectable, parameterised and rendered, (d) capability matrix row updated (Supported / Partial / Unsupported), (e) UI/README text matches the matrix.
- Never mark ✅ with a "Known limitation" that contradicts the phase title; use ◐ Partial.
- Do not touch `verified` unless a real check ran; the response must say *how* it was verified.
- Every new `Expr` printer path needs a parse round-trip test.

## 7. Deployment (Docker / Lambda) [static analysis]
- **`FROM rust:1.85` will likely fail**: the lockfile resolves `sha2 0.11.1` (via `rust-embed-utils`) which requires Rust 1.88; the workspace uses `resolver = "2"` (not MSRV-aware) and no `rust-version`. Use ≥1.88 (current stable preferred), set `rust-version` in `[workspace.package]`.
- `.dockerignore` excludes `Cargo.lock` and the Dockerfile doesn't copy it → non-reproducible builds. Copy it and build with `--locked`.
- Dockerfile `CMD` port is 3000 but Lambda Web Adapter defaults to 8080; only the SAM `ImageConfig.Command` override fixes this. Make the default consistent or set `AWS_LWA_PORT`; set `AWS_LWA_READINESS_CHECK_PATH=/api/health`. (The "zero env vars" rule applies to the app; the adapter is env-configured.) `0.8.4` is an old adapter tag — check the current release.
- Lambda CPU scales with memory (≈1 vCPU at 1769 MB); 512 MB gives a fraction of a core, so e-graph saturation will be slow and hit the 8 s limit. Raise `MemorySize` to 1024–1769.
- Frozen-thread problem from P0-5 is worst on Lambda; the cooperative-cancel/child-process fix matters more here.
- Add compression + caching headers (API Gateway HTTP API doesn't compress).

## 8. Recommended work order (agent-ready backlog)

1. **Safety & correctness core:** P0-1 (precedence + round-trip property test), P0-4 (depth/stack), P0-5 (owned permit, caps, cancellation), P0-3 (error bodies/spans).
2. **UI wiring:** P0-2, P0-7, then the §3 table in order: params restore, chips, composer (replace all placeholders; prompt/select the 2nd operand; add missing args), env/float_type/fast-math (implement or remove), rules fields, CSE bindings, provenance tab, escape all `innerHTML`, CSP.
3. **UI design:** fix the undefined-class problem (4.1) + CI check; normalize/flatten output (4.3); add the missing controls (4.2), one-sided limits first.
4. **Engine honesty:** real `verified`, tri-state integrals, `partial`/`unsupported` statuses, fix ODE coefficient bug, `ln|x|`, Taylor off-by-one, solve completeness, honor embedded command args, label numeric-fallback limits as numeric.
5. **Engine capability (behaviour corpora):** atan/trig-power/sqrt/IBP integrals and irreducible-quadratic rational integration; `partfrac` (improper/repeated/irreducible); true `factor` over ℤ and multivariate content; cubic/quartic + symbolic quadratics + `%i`; damped 2nd-order ODEs, separable/Bernoulli/non-homogeneous 1st-order; MRV-based limits with one-sided support; parser additions (decimals, `**`, `=`, lists, `sec/csc/cot/abs`, `diff(f,x,n)`, implicit multiplication in LaTeX).
6. **Tests that would have caught all of this:**
   - Headless-browser smoke test (Playwright): load page → op grid count == catalog → click each op → params visible → compute each example chip → assert payload and rendered result; run in CI.
   - JSON-schema contract tests for `/api/operations|functions|rules|compute|parse` and for 400 bodies.
   - Known-answer corpora per area (include the failing inputs above as regression tests), plus numeric differential testing vs. SymPy for integrate/limit/series/solve.
   - Fuzz/deep-nesting tests; real-timeout test; Docker build in CI (`docker build` + run + `GET /api/health`).
7. **Deploy:** §7.

### Reproduce the CSS-contract check (add to CI)
```python
import re,glob
css=open('maxima-web/static/vendor/app.css').read()
defined=set(re.findall(r'\.([A-Za-z_][\w\-]*)',css))|{m.replace('\\','') for m in re.findall(r'\.((?:[\w\-]|\\.)+)',css)}
used=set()
for f in ['maxima-web/static/index.html']+glob.glob('maxima-web/static/js/*.js'):
    t=open(f).read()
    for m in re.findall(r'class(?:Name)?\s*=\s*["\'`]([^"\'`$]+)["\'`]',t): used|=set(m.split())
missing=sorted(u for u in used if u not in defined)
assert not missing, missing
```
