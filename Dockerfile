# ---- Stage 1: build ---------------------------------------------------------
# A full Rust toolchain is ~1.5GB. We only want the 5MB binary it produces, so
# we build here and throw this whole stage away.
FROM rust:1-slim AS builder

WORKDIR /src

# Copy only what the build needs. tests/ is excluded because `cargo build`
# does not compile tests, and target/ is excluded by .dockerignore.
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

# ---- Stage 2: run -----------------------------------------------------------
# debian-slim rather than distroless: it keeps a shell, which makes
# `kubectl exec` into a running pod possible while experimenting.
FROM debian:stable-slim

# Run as a non-root user. specdrift only reads system info; it needs no
# privileges, and Kubernetes security policies will reject root containers.
RUN useradd --create-home --shell /bin/bash drift

COPY --from=builder /src/target/release/specdrift /usr/local/bin/specdrift

USER drift
WORKDIR /home/drift

ENTRYPOINT ["/usr/local/bin/specdrift"]
CMD ["--help"]
