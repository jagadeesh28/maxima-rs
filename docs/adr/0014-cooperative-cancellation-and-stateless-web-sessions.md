# Cooperative Compute Cancellation and Stateless Client-Carried Web Sessions

**Status**: Accepted — extends [ADR-0007](0007-webui-cas-typed-dispatch-and-offline-architecture.md), [ADR-0008](0008-compute-isolation-and-service-concurrency.md), and [ADR-0012](0012-tri-state-symbolic-outcomes-and-verification-honesty.md)

**Context**:
Two architectural gaps were surfaced in `maxima-web` and `sym-core`:
1. **Timeout Permit Leakage & Stack Depth**: Wrapping `tokio::task::spawn_blocking` in `tokio::time::timeout` drops the `JoinHandle` on timeout while the blocking CAS thread continues running CPU-bound loops (`egg` saturation, `series`, `poly`, `integrate`, `bareiss`, `solve`) and holds its `OwnedSemaphorePermit` indefinitely, permanently exhausting server concurrency into HTTP 503. An alternative proposal (`maxima_rs_claude_audit.md`) suggested spawning killable child worker processes per request, while `NEON_audit_recommendations.md` proposed cooperative in-process cancellation.
2. **Notebook Session Continuity vs Lambda Statelessness**: Users expect Maxima notebook continuity (`%`, `%oN`, variable bindings `a: expr`, scoped `assume`/`facts`/`forget`, `kill(all)`), while `maxima-web` also targets containerized AWS Lambda deployment (`template.yaml`) where cold starts and multi-instance routing invalidate in-memory server session maps.

**Decision**:
1. **In-Process Cooperative `CancelToken` Over Subprocess Pools**:
   - Introduce a lightweight `CancelToken` (`Arc<AtomicBool>`) checked inside `egg` saturation `Runner` hooks and every potentially long-running procedural CAS loop (`poly`, `series`, `limit`, `integrate`, `solve`, `matrix::bareiss`).
   - On `tokio::time::timeout`, `maxima-web` flips `cancel.store(true, Ordering::Relaxed)` so the worker aborts promptly, cleans up stack state, and releases its `OwnedSemaphorePermit`.
   - Configure dedicated compute worker threads with `>= 8 MB` stack sizes alongside strict resource bounds (`RecursionDepthGuard` depth 256, `taylor` order `0..=64`, `orthopoly` degree `<= 100`, `variables <= 16`, `env <= 64`).
   - Rejected per-request OS subprocess spawning to preserve single-binary zero-IPC simplicity, `Windows`/`Linux`/`macOS` parity, `wasm32` compatibility in `sym-core`, and low AWS Lambda cold-start overhead.
2. **Stateless Client-Carried `session_context` JSON**:
   - Represent notebook session continuity (`last_result` / `%`, numbered `%oN` outputs, user variable bindings, scoped `Assumptions` facts) via an optional, schema-validated `session_context` JSON object carried in `/api/compute` requests and returned in responses, rather than an authoritative server-side session store.
   - Enforce strict server-side bounds on `session_context` entry count, serialized payload size (64 KB request cap), and recursion depth before deserializing into scoped `Assumptions` and bindings.

**Consequences**:
- Guarantees deterministic permit recovery after timeouts without process-management or serialization overhead.
- Keeps `maxima-web` 100% horizontally stateless across AWS Lambda invocations and standalone server restarts while enabling stateful Maxima notebook workflows in the WebUI.
- Requires every new CPU-intensive CAS loop added to `sym-core` to accept and poll `&CancelToken` (or `Option<&CancelToken>`).
