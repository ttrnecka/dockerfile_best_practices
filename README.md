# General Dockerfile Best Practices

This repository provides language-agnostic Dockerfile practices, seven ecosystem-specific guides, and an independently buildable sample for each guide. The guide-specific version targets, image choices, and validation recipes are documented alongside each example.

## Application Guides and Samples

Use the general practices here as a foundation, then consult the matching guide for ecosystem-specific decisions and build/run checks. Each sample directory is its own Docker build context; run its guide's commands from that directory.

| Application | Guide | Buildable sample |
| --- | --- | --- |
| Python web API (FastAPI) | [Guide](guides/python-fastapi.md) | [Sample](examples/python-fastapi/) |
| Node.js web API (Express) | [Guide](guides/nodejs-express.md) | [Sample](examples/nodejs-express/) |
| Go web service (standard library HTTP server) | [Guide](guides/go-http-service.md) | [Sample](examples/go-http-service/) |
| Java web API (Spring Boot) | [Guide](guides/java-spring-boot.md) | [Sample](examples/java-spring-boot/) |
| .NET web API (ASP.NET Core) | [Guide](guides/dotnet-aspnet-core.md) | [Sample](examples/dotnet-aspnet-core/) |
| PHP web application (PHP-FPM) | [Guide](guides/php-fpm.md) | [Sample](examples/php-fpm/) |
| Rust web service (Axum) | [Guide](guides/rust-axum.md) | [Sample](examples/rust-axum/) |

The HTTP samples implement `GET /` and `GET /healthz`. PHP-FPM speaks FastCGI rather than HTTP; its guide describes direct FastCGI checks and intentionally does not publish port 9000 to the host.

The snippets are illustrative, not complete buildable images. Replace example image names, files, and commands with those required by the application.

## Choose Predictable Inputs

Use a maintained base image that provides only the operating system and runtime capabilities the application needs. Prefer an explicit release tag over a floating tag such as `latest`, so a rebuild does not silently switch to a different base. Pin an image digest as well when an exact immutable input is required.

For guide examples, use a major release tag for LTS runtimes and a minor or patch release tag otherwise, and include an explicit operating-system variant when the image publishes one. Production builds that require immutable base-image inputs should pin the digest alongside the readable tag.

Treat a pinned digest as an input to maintain, not a value to set once and forget. On a regular schedule, review upstream releases and security notices, update the digest, rebuild the image, and run the application's tests and image checks before merging the change. Make urgent updates sooner when a relevant security fix is released. Automating update proposals is useful, but review and validate each proposed update before releasing it.

```dockerfile
FROM registry.example/runtime:1.2
```

Apply the same principle to application dependencies. Where the package manager supports it, commit its lockfile and install in a locked or frozen mode. This makes builds more repeatable and makes dependency changes deliberate.

## Set an Explicit Working Directory

Set `WORKDIR` before copying files or running application commands. Use an absolute path so instructions do not depend on the base image's default directory.

```dockerfile
WORKDIR /app
COPY app-files/ ./
```

This makes file locations easier to understand and reduces accidental writes elsewhere in the image.

## Copy Only What the Image Needs

Prefer `COPY` for files from the build context. Be deliberate about which files enter the image; avoid copying local configuration, credentials, tests, or other material that the image does not need. Use a `.dockerignore` file to exclude unnecessary files and sensitive material from the build context, especially when broad `COPY` instructions are used. This reduces context transfer and helps prevent accidental inclusion, but do not put secrets in the build context in the first place.

```dockerfile
COPY app/ ./app/
COPY config/defaults/ ./config/
```

For example, tailor these `.dockerignore` patterns to the files the build actually needs:

```dockerignore
.git
.env
*.pem
*.key
tests/
```

Keeping the inputs explicit reduces accidental content in the image and makes the Dockerfile easier to audit. For remote inputs, use an explicit download and verify their integrity rather than relying on an opaque URL fetch.

## Order Instructions to Reuse Build Cache

Place inputs that change infrequently before inputs that change often. For example, copy dependency manifests and install dependencies before copying application source. A source-only change can then reuse the dependency-installation layer.

```dockerfile
COPY dependency-manifest dependency-lock ./
RUN package-manager install --locked
COPY src/ ./src/
```

Use the equivalent manifest files and locked-install option for the chosen ecosystem. Docker reuses a layer only while its instruction and relevant inputs remain unchanged, so keep expensive, stable steps early and frequently edited files later.

Where the builder supports persistent cache mounts or an equivalent cache facility, cache the package manager's download cache across builds. The cache may be stored on the build host and can avoid downloading the same packages again; it is separate from the image contents and should not be copied into the final image. The exact setup depends on the builder and package manager.

## Separate Build Tools from Runtime Contents

Use multiple stages when building or testing requires tools that the running application does not need. Copy the built artifact into a clean runtime stage instead of carrying compilers, source trees, and temporary build files into the final image.

```dockerfile
FROM registry.example/build-toolchain:1.2 AS build
WORKDIR /src
COPY source/ ./source/
COPY build-config ./build-config
RUN ./build.sh --output /out/app

FROM registry.example/runtime:1.2
WORKDIR /app
COPY --from=build /out/app ./app
ENTRYPOINT ["./app"]
```

The runtime stage must still contain any shared libraries and other runtime dependencies the artifact needs. A smaller final image usually has fewer files to maintain and transfer, but do not remove dependencies merely to chase size.

## Keep the Final Image Focused

Install only runtime dependencies in the final stage. If a package manager leaves downloaded indexes or caches behind, clean them in the same `RUN` instruction that creates them; deleting files in a later layer does not remove them from earlier image layers.

Do not combine every command into one large `RUN` just to reduce the number of layers. Prefer readable instructions and useful cache boundaries. Combine operations when doing so prevents temporary files from being retained in the image.

## Install OS Packages

For Debian-based images, refresh package indexes, install only the needed packages without recommended extras, and remove the indexes in the same layer. Use the equivalent package manager and cleanup commands for other distributions.

```dockerfile
RUN apt-get update \
    && apt-get install -y --no-install-recommends package-name \
    && rm -rf /var/lib/apt/lists/*
```

## Run with Least Privilege

Run the application as a non-root user unless it has a specific need for elevated privileges. Use a suitable user provided by the base image or create one using that image's conventions. Ensure application files and any required writable directories have appropriate ownership and permissions.

```dockerfile
COPY app/ ./app/
RUN mkdir -p /app/data && chown 10001:10001 /app/data
USER 10001:10001
```

Keep application code and dependencies root-owned and readable by the runtime user so a compromised process cannot rewrite its own executable files. Give the runtime user ownership only of directories it must write to, such as data, cache, or upload directories. The numeric ID is illustrative. Choose an identity that works with the application and the environment where the image will run.

## Keep Configuration Separate from Secrets

Use `ARG` for non-secret values needed only while building and `ENV` for non-secret defaults that the application needs at runtime. Prefer runtime configuration for values that differ between environments.

Never put credentials, tokens, or private keys in the Dockerfile, build arguments, environment variables baked into the image, or files copied into an image layer. For example, do not pass a token as a build argument and use it in a `RUN` command; build arguments and the resulting image history or layers are not a secret store.

For a private dependency download during a build, use the builder's dedicated secret facility to make the credential temporarily available to that build step, and have the package manager read it without copying it into an image layer. The mechanism and syntax depend on the builder. At runtime, if the application supports reading a secret from a file, configure the expected path while keeping the secret itself outside the image:

```dockerfile
ENV DB_PASSWORD_FILE=/run/secrets/db_password
```

The runtime environment must provide that file; do not create it with `COPY` or bake its contents into `ENV`.

## Define a Clear Runtime Command

Use the exec form of `ENTRYPOINT` and `CMD` for the normal application process. It starts the application directly, allowing it to receive operating-system signals such as termination requests.

```dockerfile
ENTRYPOINT ["/app/server"]
CMD ["--port", "8080"]
```

`ENTRYPOINT` sets the main executable, giving the image a consistent default purpose. `CMD` supplies default arguments to that executable; users can replace those arguments when starting a container, for example to run the same server with different options. Replacing the executable itself requires explicitly overriding the entrypoint. If users should normally replace the whole command, use `CMD` without an `ENTRYPOINT` instead. Avoid shell-form commands unless shell behavior is explicitly required.

Exec form starts the application directly, but does not guarantee graceful shutdown. Verify that the PID 1 process handles `SIGTERM` and exits promptly; `docker run --init` or a minimal init such as tini can help forward signals and reap child processes when needed.

## Document Runtime Interfaces Carefully

Use `EXPOSE` to document the port the application listens on, when it has one. It is image metadata; it does not publish the port by itself.

```dockerfile
EXPOSE 8080
```

Add a Dockerfile `HEALTHCHECK` only when the application has a meaningful, reliable health probe. A probe should test the application's own health and should not depend on unrelated external services.

## Add Useful Image Metadata

Use `LABEL` to attach non-sensitive information that helps identify and trace an image, such as its source repository, source revision, or description. Keep values accurate when publishing the image; labels are metadata, not a place for credentials or other secrets.

```dockerfile
LABEL org.opencontainers.image.source="https://example.com/team/app" \
    org.opencontainers.image.revision="example-revision" \
    org.opencontainers.image.description="Example application image"
```

For a revision label supplied at build time, pass a non-secret source-control revision as an argument:

```dockerfile
ARG GIT_SHA
LABEL org.opencontainers.image.revision="${GIT_SHA}"
```

## Review the Final Image

Before distributing an image, check that its final stage contains only the intended runtime files, that no secrets or build-only material were included, and that the configured user and command match the application's needs. Build success alone does not verify runtime behavior: start the final image and smoke-test that the application starts and responds as expected, then confirm it shuts down cleanly. Rebuild periodically from updated, trusted inputs and verify that updates do not break the application.