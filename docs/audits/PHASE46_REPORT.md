# Phase 46 Sign-Off Report: Security, Build/CI Integrity & WebUI Contract Hotfixes (Stop-Ship)

## 1. Summary of Deliverables & Changes
- `maxima-web/static/js/{preview,notebook,main,palette,api}.js` & `maxima-web/static/index.html` & `maxima-web/static/vendor/app.css`:
  - Eliminated DOM XSS sinks in `preview.js`, `notebook.js`, and `main.js` using `textContent` and `escapeHtml`.
  - Added `AbortController` + sequence counter in `preview.js`, safe non-JSON error parsing in `api.js`, preserved user/example parameters across live preview variable detection in `palette.js`, added `env` input rendering for `evaluate`, rendered `enum` `accepts` as `<select>`, fixed binary composer `{}` placeholder replacement, fixed `d/d[x,y]` chip `data-params`, fixed Rules modal/tab fields (`source_fn`, `identity`, `fast_math_only`, `response.extra.rules`), defined `.opacity-75`/`.border-danger`/`.border-info` in `app.css`, and removed dead `auto-render.min.js`.
- `maxima-web/src/{app,api,config,main,service}.rs`:
  - Added `Content-Security-Policy`, `X-Content-Type-Options: nosniff`, and `X-Frame-Options: DENY` security headers via Axum middleware in `app.rs`.
  - Extracted `run_isolated` helper and wrapped `/api/parse` in `spawn_blocking` + `catch_unwind` + `timeout` + semaphore alongside `/api/compute`.
  - Changed `Cli` fields in `config.rs` to `Option<T>` so `--config <toml>` values are preserved; added `with_graceful_shutdown` in `main.rs`.
  - Added `fast_math: bool` to `ComputeRequest` (`api.rs`), wired to `SaturationConfig`; unified `params` vs top-level lookup (`variable`, `variables`, `float_type`, `charpoly`, `residue`); rejected invalid `float_type` and Rust keyword/invalid identifiers with HTTP `400`; enforced bounds (`taylor` order `0..=64`, `variables <= 16`, `env <= 64`, `params <= 32`, param strings `<= 1024` bytes); gated `parse_maxima_cas` on `input_format != "latex"`; eliminated double-parse on error; updated `resp.operation` on auto-dispatch; supported `[f1, f2]` vector inputs in `jacobian`; fixed `limit` composer arity (`3`).
- `sym-core/src/expr.rs`:
  - Sorted `Expr::variables()` deterministically.
- `Cargo.toml`, `maxima-rs/Cargo.toml`, `maxima-web/Cargo.toml`, `tests/Cargo.toml`, `.github/workflows/ci.yml`, `.dockerignore`, `Dockerfile`, `template.yaml`, `scripts/verify_css_classes.py`:
  - Centralized `axum`, `tokio`, `serde`, `serde_json` in `[workspace.dependencies]` and added `rust-version = "1.88"` in `[workspace.package]`.
  - Added `tensor = ["sym-core/tensor"]` (bundled in `full`) to `maxima-rs/Cargo.toml`.
  - Updated `.github/workflows/ci.yml` to run `node --check` on `maxima-web/static/js/*.js`, `verify_css_classes.py`, and expanded `feature-matrix` (`bfloat`, `matrix`, `complex`, `specfn`, `transform`, `assume`, plus `maxima-rs`).
  - Updated `Dockerfile` to `rust:1.88-bookworm`, copied `Cargo.lock` (removed from `.dockerignore`), enabled `--locked`, set `AWS_LWA_READINESS_CHECK_PATH=/api/health`, aligned port `8080`, and raised `template.yaml` `MemorySize` to `1536` MB.
  - Expanded `scripts/verify_css_classes.py` to inspect template literals and `classList` calls.
- `tests/integration/web_hardening.rs`:
  - Added `test_security_headers_present`, `test_parse_isolated_bounds_and_timeout`, `test_phase46_compute_contract_validation`, and `test_cli_toml_config_precedence`.

## 2. Issues & Audit Findings Remediated
| Defect ID | File / Crate | Defect Type | Root Cause | Fix & Verification Test |
| :--- | :--- | :--- | :--- | :--- |
| P46-01 | `maxima-web/static/js/{preview,notebook,main}.js` | DOM XSS | Unescaped `res.error`, `err.message`, `request.operation`, `statusLabel` in `innerHTML` | Replaced with `textContent` / `escapeHtml`; verified via JS syntax check & code audit |
| P46-02 | `maxima-web/src/app.rs` | Security Headers & Isolation | Missing `CSP`/`X-Content-Type-Options`/`X-Frame-Options`; `/api/parse` unisolated | Added `security_headers` middleware and `run_isolated`; verified in `web_hardening.rs` |
| P46-03 | `maxima-web/src/{config,main,service}.rs` | Config & Contract | `Cli` defaults overwrote TOML; unvalidated `float_type`, Rust idents, `taylor` order, missing `fast_math` | Switched `Cli` to `Option<T>`, added validation & bounds; verified in `web_hardening.rs` |
| P46-04 | `maxima-web/static/js/palette.js`, `index.html`, `app.css` | Broken UI Features | Param wipe on preview, missing `env`/`enum` UI, `{}` binary wrap bug, missing CSS classes | Preserved params, added `env`/`<select>`, fixed `{}` & CSS; verified with `verify_css_classes.py` |
| P46-05 | `.github/workflows/ci.yml`, `Dockerfile`, `Cargo.toml`, `maxima-rs/Cargo.toml` | CI / Docker / Cargo | Stale `app.js` check, `rust:1.85` without `--locked`, missing `tensor` in `maxima-rs` | Updated CI, `Dockerfile` (`1.88-bookworm` + `--locked`), `[workspace.dependencies]`, `tensor` |

## 3. Mandatory Edge-Case Coverage Checklist
- [x] Numerical & Constants (`taylor` order `0..=64` bounds)
- [x] Operator Precedence & Codegen (Rust identifier & keyword injection rejected with `400`)
- [x] Cardinality & Boundaries (`variables <= 16`, `env <= 64`, `params <= 32`, `jacobian` `[f1, f2]` vector)
- [x] Domain Algorithms (`fast_math` wired to `SaturationConfig`, `Expr::variables()` sorted)
- [x] Parsers & I/O (`/api/parse` isolated with `413`/`408`/`503`, `parse_maxima_cas` gated on `!= "latex"`)
- [x] Concurrency, Security & Timeouts (`CSP`, `nosniff`, `DENY`, XSS escape, `with_graceful_shutdown`)
- [x] WebUI & Persistence (`node --check` on all JS, `verify_css_classes.py` 0 missing, `Cli` TOML precedence)

## 4. Passing Gate Verification
| Command | Expected Exit Code | Actual Exit Code | Result Summary |
| :--- | :---: | :---: | :--- |
| `cargo fmt --all -- --check` | `0` | `0` | Clean formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `0` | `0` | 0 warnings |
| `cargo test --workspace --all-features` | `0` | `0` | All unit, integration, proptest, and doc tests passed |
| `cargo build -p maxima-rs --no-default-features --features tensor` | `0` | `0` | `maxima-rs` `tensor` feature verified |
| `E:\Anaconda\python.exe scripts/verify_css_classes.py` | `0` | `0` | 107 used, 114 defined, 0 missing |
| `node --check maxima-web/static/js/*.js` | `0` | `0` | All 6 JS modules valid |

## 5. Git Checkpoint Verification
- **Local Commit**: Pending creation in final step of Phase 46
- **Push Status**: Deferred pending explicit user confirmation
