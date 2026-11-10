# syntax=docker/dockerfile:1.7
#
# Build from the repository root (the build context):
#   docker build -f solutions/18_deployment/Dockerfile -t rustref-service-solution .
#
# The solution workspace is `solutions/`; its tests live under `exercises/` and are
# needed at manifest-load time, so they are copied alongside.

FROM rust:1.98.1-slim-bookworm AS builder
ARG BUILD_GIT_SHA=unknown
WORKDIR /src
COPY solutions/ solutions/
COPY exercises/18_deployment/tests/ exercises/18_deployment/tests/
WORKDIR /src/solutions
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/solutions/target \
    BUILD_GIT_SHA=${BUILD_GIT_SHA} cargo build --release --locked -p ex18_deployment --bin rustref-service \
 && cp target/release/rustref-service /usr/local/bin/rustref-service

# Distroless: no shell, no package manager, runs as the `nonroot` user (uid 65532).
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /usr/local/bin/rustref-service /usr/local/bin/rustref-service
ENV LISTEN_ADDR=0.0.0.0:8080
EXPOSE 8080
USER nonroot
# No HEALTHCHECK: distroless has no shell or curl. Probe GET /healthz and GET /readyz over HTTP.
# Secrets (API_TOKEN, DATABASE_URL) are injected at runtime, never baked in.
ENTRYPOINT ["/usr/local/bin/rustref-service"]
CMD ["serve"]
