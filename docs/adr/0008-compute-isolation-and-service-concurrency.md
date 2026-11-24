# Compute Isolation and Service Concurrency Architecture

**Status**: Accepted

Context: The `maxima-web` service previously executed CPU-bound e-graph equality saturation and procedural CAS routines synchronously on Tokio worker threads without timeouts, panic handling, or concurrency limits. A pathological expression or engine panic could stall the Tokio runtime or crash the web server process. We decided to:
1. Isolate every symbolic computation within `tokio::task::spawn_blocking` and wrap the execution in `std::panic::catch_unwind(AssertUnwindSafe(...))` so engine panics return clean HTTP 500 errors instead of terminating the server.
2. Enforce strict wall-clock computation timeouts using `tokio::time::timeout` (default 8.0 seconds, configurable via CLI arguments or TOML configuration).
3. Throttle concurrent compute requests with a `tokio::sync::Semaphore` bounded by the available logical CPU cores, returning HTTP 503 Service Unavailable when the system is saturated.
4. Establish standardized HTTP status mappings: 200 for computed outcomes (including `unevaluated`, `divergent`, `no_solution`), 400 for malformed input or parameter schema violations, 404 for unknown `/api/*` endpoints, 408 for computation timeouts, 413 for oversized payloads, 500 for engine panics, and 503 for concurrency saturation.
