# Rust Web Service with Axum

## Target and Application Shape

Checked 2026-10-21. This example targets Rust 1.99.1, the latest stable patch released on 2026-10-15, Axum 0.8.9, and cargo-chef 0.1.78. Axum 0.8.9 declares a minimum Rust version of 1.80. The application is a Tokio-based HTTP service and assumes a project-root build context with `Cargo.toml`, a committed `Cargo.lock`, and a binary target named `rust-api` at `src/main.rs` or `src/bin/rust-api.rs`. The application reads `PORT`, defaulting to 8080, and binds to `0.0.0.0`.

The service must handle `SIGTERM` and use Axum's graceful-shutdown support, for example with a Tokio signal future that listens for `tokio::signal::unix::SignalKind::terminate()`; otherwise, Docker may wait for its stop timeout before killing the PID 1 process.

## Dockerfile

```dockerfile
# syntax=docker/dockerfile:1.7

FROM lukemathwalker/cargo-chef:0.1.78-rust-1.99.1-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . ./
RUN --mount=type=secret,id=cargo-ca,target=/run/secrets/cargo-ca,required=false \
    if [ -s /run/secrets/cargo-ca ]; then \
        CARGO_HTTP_CAINFO=/run/secrets/cargo-ca cargo chef prepare --bin rust-api --recipe-path recipe.json; \
    else \
        cargo chef prepare --bin rust-api --recipe-path recipe.json; \
    fi

FROM chef AS builder
COPY --from=planner /app/recipe.json ./recipe.json
RUN --mount=type=secret,id=cargo-ca,target=/run/secrets/cargo-ca,required=false \
    if [ -s /run/secrets/cargo-ca ]; then \
        CARGO_HTTP_CAINFO=/run/secrets/cargo-ca cargo chef cook --release --locked --bin rust-api --recipe-path recipe.json; \
    else \
        cargo chef cook --release --locked --bin rust-api --recipe-path recipe.json; \
    fi
COPY . ./
RUN --mount=type=secret,id=cargo-ca,target=/run/secrets/cargo-ca,required=false \
    if [ -s /run/secrets/cargo-ca ]; then \
        CARGO_HTTP_CAINFO=/run/secrets/cargo-ca cargo build --release --locked --bin rust-api; \
    else \
        cargo build --release --locked --bin rust-api; \
    fi

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust-api /usr/local/bin/rust-api

ENV PORT=8080
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/rust-api"]
```

The `RUN` commands and runtime environment are configured as follows:

- `cargo chef prepare` analyzes the workspace manifests and lock file to create a recipe describing the files needed to build its dependencies. `--bin rust-api` limits the recipe to the named application binary and its required workspace crates. `--recipe-path recipe.json` writes the recipe at the specified path so the builder stage can use it.
- `cargo chef cook` builds dependencies from the generated recipe before application source is copied, allowing Docker to reuse that costly layer for source-only changes. `--release` uses Cargo's optimized release profile, matching the later application build. `--locked` requires the lock file to remain unchanged, and `--bin rust-api` keeps the dependency build aligned with the selected binary. `--recipe-path recipe.json` selects the planner's recipe.
- `cargo build` compiles the service. `--release` selects optimized release artifacts. `--locked` requires the existing `Cargo.lock` to remain unchanged and fails if dependency resolution would update it. `--bin rust-api` selects the executable target named `rust-api` instead of building every target in the package or workspace.
- In the runtime stage, `apt-get update` refreshes Debian's package index so installation sees current package metadata. `&&` makes each following command run only if the previous one succeeds. `apt-get install -y` installs the named packages without prompting for confirmation; `--no-install-recommends` omits optional recommended packages. `ca-certificates` supplies trusted root certificates for outbound TLS. `rm -rf /var/lib/apt/lists/*` removes downloaded package-index files; running it in the same `RUN` keeps them out of the resulting layer.
- `PORT=8080` supplies the service's default listening port. The application must read this environment variable and bind to `0.0.0.0`; deployments can override the port at runtime.

Build from the directory containing the workspace's root manifest and lock file. Add a `.dockerignore` that excludes `.git`, local configuration and secrets, and `target/`; do not exclude workspace member manifests or source needed by the planner and build.

Example `.dockerignore`:

```dockerignore
.git
.env*
target/
```

If the builder supports cache mounts, cache Cargo's registry and Git checkouts under `$CARGO_HOME/registry` and `$CARGO_HOME/git`.

The optional `cargo-ca` secret supports registries reached through TLS inspection. The Dockerfile's `RUN --mount=type=secret` instructions require Docker BuildKit with Dockerfile frontend 1.7 or newer. A normal build needs no secret when crates.io's certificate chain is trusted. For an inspected network, pass an approved CA bundle outside the build context; the bundle is mounted only for Cargo operations and is not stored in the image. Do not disable Cargo TLS verification.

## Choices and Tradeoffs

- Rust has no separate interpreter runtime. The final image needs the compiled executable and its shared runtime libraries, but not the Rust toolchain, Cargo, or crate source. Debian slim already includes `libgcc-s1` for binaries that need GCC's runtime support; this example adds only CA certificates for outbound TLS. If the application has other dynamic library dependencies, add their runtime packages too. A `gcr.io/distroless/cc-debian13` runtime is a smaller glibc-compatible alternative, with fewer debugging tools available.
- The `cargo-chef` planner derives a recipe from Cargo manifests and the lock file. The builder compiles dependencies from that recipe before copying application source, so source-only changes can reuse the expensive dependency build layer. The `chef`, `planner`, and `builder` stages use the same cargo-chef image and therefore the same Rust toolchain. The separate runtime stage uses Debian trixie-slim, matching the build image's Debian release. To avoid a community-provided build image, start the chef stage from `rust:1.99.1-trixie` and install cargo-chef with `cargo install --locked cargo-chef --version 0.1.78`; this reduces reliance on a prebuilt third-party image at the cost of compiling the helper during the build.
- `Cargo.lock` records exact dependency versions and should be committed for an application. `cargo build --locked` fails rather than changing the lock file if dependency resolution would alter it. Update dependencies deliberately and commit the resulting lock-file change.
- `--release` uses Cargo's optimized release profile. The binary target name in `--bin rust-api` must match the name declared by the application manifest. Workspaces with custom build scripts or nonstandard manifest layouts should be checked against cargo-chef's recipe behavior.
- `PORT=8080` documents and supplies the default runtime configuration; the application must read it and bind to `0.0.0.0`. The numeric user can execute the root-owned binary without write access to the application directory. Provide a dedicated writable path with explicit ownership only if the service needs one.
- This uses cargo-chef as a build-cache helper, not as a runtime dependency. A simpler build that copies the entire source tree before `cargo build` may be appropriate for small projects where compile time is not a concern. A musl/static build can avoid glibc runtime dependencies, but requires a compatible target toolchain and validation of all native crates.
- The runtime image has no shell-based health probe or `curl`; use a platform-native check or include a purpose-built health probe if the application exposes a reliable health endpoint.

## Validation Recipe

The runnable sample for this guide is in [`examples/rust-axum/`](../examples/rust-axum/). From that directory, build and run the image, verify its runtime identity and contents, exercise both routes, then stop it cleanly. The Dockerfile uses BuildKit secrets and therefore requires Docker BuildKit with Dockerfile frontend 1.7 or newer. Normal public registry access needs no secret; for TLS-inspecting networks, pass an approved CA bundle outside the build context:

```sh
docker build -t rust-axum-api .
# For TLS inspection, use this command instead:
# docker build --secret id=cargo-ca,src=/path/to/approved-ca-bundle.pem -t rust-axum-api .
docker image inspect --format '{{.Config.User}}' rust-axum-api
docker run --rm --entrypoint sh rust-axum-api -c 'test -x /usr/local/bin/rust-api && test -s /etc/ssl/certs/ca-certificates.crt && ! command -v cargo && ! command -v rustc && test ! -d /app'
docker run -d --name rust-axum-api -p 8080:8080 rust-axum-api
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /tmp/rust-root.html --write-out '%{http_code}\n' http://127.0.0.1:8080/ | grep -qx 200
grep -q 'Hello World' /tmp/rust-root.html
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:8080/healthz | grep -qx 200
docker stop --time 10 rust-axum-api
docker inspect --format '{{.State.ExitCode}}' rust-axum-api | grep -qx 0
docker rm rust-axum-api
```

The image should report user `10001:10001`; the runtime contains the executable and system CA certificates, but not Cargo, rustc, or application source. `/` should return HTML containing `Hello World`, `/healthz` should return HTTP 200, and the process should exit successfully after SIGTERM-driven graceful shutdown.

## References

- [Rust release announcements](https://blog.rust-lang.org/releases/)
- [Axum on crates.io](https://crates.io/crates/axum)
- [Axum documentation](https://docs.rs/axum/0.8.9/axum/)
- [cargo-chef](https://github.com/LukeMathWalker/cargo-chef)
- [Cargo build command](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
- [Cargo.toml and Cargo.lock](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html)
- [Official Debian image](https://hub.docker.com/_/debian)