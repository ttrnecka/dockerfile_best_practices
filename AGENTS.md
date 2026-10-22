# Plan

The goal of this project is to help DevOps engineers build Docker images for different applications and programming languages. The eventual deliverable should provide an example Dockerfile for each selected application/language combination, explaining the practices used and the reasoning behind them.

## Phase 1: General Dockerfile Practices

Create one Markdown document describing language- and application-agnostic Dockerfile best practices. Include short Dockerfile examples and explain why each practice is useful.

Keep this phase general rather than tied to specific Docker or BuildKit versions. Sources are not required. Limit the scope to Dockerfiles; do not cover Compose or broader deployment and orchestration practices.

Consider, but do not limit the guidance to, these areas:

- Stability and predictability
- Security
- Image size
- Build and runtime performance

Phase 1 is complete when the single document covers the general practices with examples and explanations.

## Future Phases

### Phase 2: Application- and Language-Specific Guidance

Turn the general practices into focused guidance for a small, prioritized set of common application and language/runtime combinations. Keep the existing general guide as the shared foundation; add one guide per combination and link to them from the general guide rather than duplicating all general advice.

Start with these combinations, in priority order:

1. Python web API (FastAPI)
2. Node.js web API (Express)
3. Go web service (standard library HTTP server)
4. Java web API (Spring Boot)
5. .NET web API (ASP.NET Core)
6. PHP web application (PHP-FPM)
7. Rust web service (Axum)

This set covers common service and web workloads, both compiled and interpreted runtimes, and distinct dependency-installation and build-artifact patterns. Rust adds a distinct Cargo compilation and dependency-layer caching example to complement the Go build. Treat this as an initial scope, not an exhaustive language survey. Add a combination only when it demonstrates a materially different Dockerfile concern or addresses a clearly important gap; avoid near-duplicate frameworks in the first pass.

For each combination, define the application shape and intended runtime, then document:

- The supported language/runtime and framework versions targeted by the example. Prefer a currently supported release line and state the exact versions or version ranges; record the date these choices were checked.
- A representative production Dockerfile, including the build context assumptions, dependency installation, cache-friendly instruction ordering, runtime stage, runtime user, configuration interface, and startup command.
- Whether a multi-stage build is useful for this combination and what must remain in the final image.
- The key choices and tradeoffs, including any meaningful differences from the general guide. Explain ecosystem-specific details rather than repeating generic guidance.
- A brief validation recipe describing how to build and smoke-test the image. Executable commands are required only when their application source and build context are available; Phase 3 supplies repository-owned sample applications for repeatable checks.

Use placeholders only where a value is genuinely application-specific, and mark them clearly. Avoid presenting one Dockerfile as universally optimal: call out relevant alternatives when they change security, image size, build behavior, or runtime behavior. Keep examples compatible with the stated target versions and avoid relying on version-specific builder features unless the guide identifies the required builder version.

Phase 2 is complete when all seven initial guides follow a consistent structure, identify their target versions and assumptions, explain their important tradeoffs, and are linked from the general guide. Each example should be reviewed for Dockerfile correctness and consistency with the general practices; executable sample applications and repeatable build/run validation remain Phase 3 work.

### Phase 3: Build and Runtime Examples

Implement a very small, independently buildable sample for each Phase 2 combination under `examples/`, following the existing priority order. Complete these subphases in order, finishing and validating one sample before starting the next; do not implement all seven as a single batch.

#### Phase 3.1: Python FastAPI

Implement and validate `examples/python-fastapi/`.

#### Phase 3.2: Node.js Express

Implement and validate `examples/nodejs-express/`.

#### Phase 3.3: Go HTTP Service

Implement and validate `examples/go-http-service/`.

#### Phase 3.4: Java Spring Boot

Implement and validate `examples/java-spring-boot/`.

#### Phase 3.5: .NET ASP.NET Core

Implement and validate `examples/dotnet-aspnet-core/`.

#### Phase 3.6: PHP-FPM

Implement and validate `examples/php-fpm/` using the PHP-FPM guide's target versions and runtime choices.

#### Phase 3.7: Rust Axum

Implement and validate `examples/rust-axum/`.

Each sample directory is its own Docker build context and contains its Dockerfile, minimal application, and any required manifests or configuration. Keep samples focused on container behavior; omit unrelated tools and services. Match each sample's language, framework, versions, and production runtime choices to its corresponding Phase 2 guide rather than repeating version numbers here. Follow that guide for ports and runtime settings unless the sample requires a documented difference.

Every HTTP server binds to `0.0.0.0` and implements these routes:

- `GET /` returns HTTP 200 with an HTML response containing `Hello World`.
- `GET /healthz` returns HTTP 200.

PHP-FPM is the exception to the HTTP-server contract: FPM speaks FastCGI, not HTTP. Implement equivalent GET behavior in the PHP application, then test both routes directly over FastCGI with `cgi-fcgi` from a disposable test client that shares the FPM container's network namespace. Send `REQUEST_METHOD=GET`, the route in `REQUEST_URI`, and the PHP entry point's in-container path in `SCRIPT_FILENAME`. Do not add an HTTP frontend or publish port 9000 to the host. Check the CGI response headers and body; absent an explicit `Status` header, CGI defines the response status as 200. Also run `php-fpm -tt` against the built image and stop the FPM container cleanly. Keep the test client outside the runtime image.

Each subphase is complete only when its sample builds and runs as a container, repeatable smoke-test instructions verify both routes using the appropriate HTTP or FastCGI contract and stop the container cleanly, relevant runtime-image and runtime-user expectations from its guide are verified, and the corresponding guide links to the sample with usable build and test commands.

Phase 3 is complete when all seven subphases are complete and all seven images pass their build and runtime checks. Keep the scope to Dockerfiles and sample applications; do not add Compose or broader deployment and orchestration material.

### Phase 4: Coverage and Maintenance

Review the selected combinations for gaps, refine examples based on practical build and run checks, and keep version-specific guidance current as supported runtimes and container tooling change.