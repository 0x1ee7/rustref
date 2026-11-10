# 18 · Deployment: a container-ready HTTP service

## Scenario

You own `rustref-service`, a small HTTP service that will run on Kubernetes behind a
load balancer. Before it can ship, it has to:

- refuse to start with a bad configuration, and report *every* problem at once;
- keep secrets out of logs, `Debug` output and the image;
- answer liveness and readiness probes separately, and run readiness checks concurrently;
- shut down gracefully on SIGTERM so in-flight requests finish;
- expose build metadata so a running pod can be traced back to a commit;
- build into a small, non-root, reproducible image.

The crate is a library plus a thin binary (`rustref-service`). Tests drive the library
directly, so they run in-process and need no container runtime.

## Part 1: Configuration (`config`)

API (`src/config.rs`): `Config::from_env(lookup)`, `Secret<T>`, `ConfigError`, `Issue`,
`LogLevel`, `parse_bool`, `parse_duration`, `parse_list`.

- `lookup: impl Fn(&str) -> Option<String>` is injected. Tests pass a map, never the process env.
- Blank or whitespace-only values count as unset.
- Required: `API_TOKEN` (≥ 16 characters), `DATABASE_URL` (scheme `postgres`, `postgresql` or `sqlite`).
- Defaults: `LISTEN_ADDR=0.0.0.0:8080`, `APP_NAME=rustref-service`, `REQUEST_TIMEOUT=30s`,
  `MAX_CONNECTIONS=256`, `ALLOWED_HOSTS=` (empty), `DEBUG=false`, `LOG_LEVEL=info`.
- `ConfigError` aggregates every `Issue`. Messages never echo a secret value.
- `Secret<T>` has redacted `Debug` and `Display`; the value is reachable only via `expose()`.
- `parse_duration` accepts `500ms`, `30s`, `5m`, `1h`, `1.5s` and bare seconds; it rejects
  empty input, negatives, unknown units and prose such as `5 minutes`.

## Part 2: HTTP API (`api`)

API (`src/api.rs`): `router(AppState) -> axum::Router`, `AppState::new`, `with_check`,
`ReadinessCheck`, `CheckFuture`.

- `GET /healthz` → `200 {"status":"ok"}`. Never runs readiness checks.
- `GET /readyz` → runs all checks concurrently. `200` when all pass, otherwise `503`.
  Every check is reported, even when another fails.
- `GET /version` → app name, `CARGO_PKG_VERSION`, and the build git SHA.
- Unknown path → `404 {"error":"not_found","path":...}`. Wrong method → `405`.
- `HEAD` mirrors `GET` with an empty body. Responses carry `Content-Type` and `Content-Length`.
- Any `Fn() -> impl Future<Output = Result<(), String>>` is a `ReadinessCheck` via a blanket impl.

## Part 3: Graceful shutdown (`shutdown`)

API (`src/server.rs`): `serve(listener, router, shutdown_future)`.

- After the shutdown future resolves, no new connections are accepted, but in-flight
  requests finish. `serve` returns only once they have drained.
- Tests use real loopback sockets, raw HTTP and a `Notify`-synchronised slow handler, so
  there are no sleeps-as-synchronisation.

## Part 4: Command-line interface (`cli`)

API (`src/cli.rs`): `Cli`, `Command`, `run(args, lookup, out, err) -> ExitCode`.

- Subcommands: `serve [--dry-run]`, `check-config`, `version`.
- Exit codes: `0` success; `2` usage error or invalid configuration; `1` runtime failure
  (for example, the address cannot be bound).
- `run` writes to injected writers, so output is testable.
- `--dry-run` validates configuration and prints the plan without binding a socket.
- Every configuration issue is printed on its own `error: VAR: problem` line.

## Part 5: Build metadata (`build_info`)

API (`src/build_info.rs`): `version()`, `git_sha()`. The `build.rs` script is yours to write.

- `build.rs` must emit `cargo:rustc-env=BUILD_GIT_SHA=<sha>`, so `env!("BUILD_GIT_SHA")` compiles.
- Resolution order: a valid 40-hex `BUILD_GIT_SHA` environment variable, then
  `git rev-parse HEAD`, then the literal `unknown`.
- Declare `cargo:rerun-if-env-changed=BUILD_GIT_SHA` and `cargo:rerun-if-changed` on the git
  HEAD path, so the SHA refreshes after a commit.
- Tests read `build.rs` and this README via `CARGO_MANIFEST_DIR`. They also check the
  release profile below.

### Release profile

Profiles apply only at the workspace root, so this section documents the intended
`Cargo.toml` change; it is not added to the repository root.

```toml
[profile.release]
lto = "thin"
codegen-units = 1   # one codegen unit: better inlining and smaller binaries, at the cost of build time
strip = "symbols"
```

## Part 6: Container image (`container`)

Files: `Dockerfile` and `.dockerignore`, in this crate directory.

- Builder: a pinned `rust:1.98.1-slim-bookworm` stage named `builder`.
- Runtime: distroless `cc-debian12:nonroot`. No shell, no toolchain.
- Build: `cargo build --release --locked -p ex18_deployment --bin rustref-service`, with
  BuildKit cache mounts for the registry and `target/`.
- `ENV LISTEN_ADDR=0.0.0.0:8080`, `EXPOSE 8080`, `USER nonroot` (last), exec-form
  `ENTRYPOINT`/`CMD`, no secrets in `ENV`/`ARG`.
- No `HEALTHCHECK`: distroless has no shell to run one, so probes hit `/healthz` and `/readyz`.
- `.dockerignore` excludes `target` and `.git`.

**Build-context caveat.** The build context is the repository root, so the `COPY . .` in the
Dockerfile uses root-level context. BuildKit reads `.dockerignore` from the context root
only (or a `<Dockerfile>.dockerignore` next to the Dockerfile). A crate-local `.dockerignore`
is therefore ignored when you build from the repo root. Either move the excludes to the root
`.dockerignore`, or build with `-f exercises/18_deployment/Dockerfile.dockerignore`, as
appropriate for your setup.

## Concepts exercised

- Ownership and redacted wrappers for secrets; `Display` vs `Debug` contracts.
- Injected lookups for testability; `FromStr` and `Display` impls; `thiserror` error types.
- Aggregating errors instead of failing on the first.
- `axum` routing, `State`, fallbacks, `Arc`-shared state; `JoinSet` for concurrent checks.
- Graceful shutdown with `axum::serve().with_graceful_shutdown`.
- `clap` derive subcommands; `ExitCode`; writer injection.
- `build.rs`, `cargo:rustc-env`, and `rerun-if-*` directives.
- Multi-stage Docker builds, non-root users, exec-form entrypoints.

## Run

```sh
cargo test -p ex18_deployment                 # all parts
cargo test -p ex18_deployment part2           # one part (name prefix filter)
cargo solutions -p ex18_deployment            # reference solution
```

## Stretch ideas (not tested)

- Enforce `MAX_CONNECTIONS` with a `tokio::sync::Semaphore` in an accept loop. It is parsed and validated, but not enforced.
- Structured JSON logs via `tracing-subscriber` (`json` feature), scoped with `with_default`.
- Propagate `ALLOWED_HOSTS` into a `Host` header check middleware.
- Add a `/metrics` endpoint with a counter shared through `AppState`.
