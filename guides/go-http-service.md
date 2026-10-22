# Go HTTP Service

## Target and Application Shape

Checked 2026-10-21. This example targets Go 1.27.3, the current patch release on the supported 1.27 line. Go supports a release until two newer major releases are available. The service uses the standard library HTTP server and assumes a project-root context with `go.mod`, `go.sum`, and source under `cmd/api/`; the executable reads `PORT`, defaulting to 8080.

## Dockerfile

```dockerfile
FROM golang:1.27.3-trixie AS build

ENV GOTOOLCHAIN=local

WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download

COPY . ./
RUN CGO_ENABLED=0 GOOS=linux go build \
    -trimpath \
    -ldflags="-s -w" \
    -o /out/server ./cmd/api

FROM scratch

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /out/server /server

ENV PORT=8080
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/server"]
```

The environment variable and `RUN` commands are configured as follows:

- `GOTOOLCHAIN=local` makes the build use the toolchain in the pinned Go image and fail if `go.mod` requires a newer one, rather than silently downloading another toolchain.
- `go mod download` downloads the modules declared by the project before source is copied. This makes dependency resolution a separate cacheable build step; `go.sum` records expected module checksums so downloaded content can be verified.
- `CGO_ENABLED=0` disables cgo for this build, allowing a statically linked binary suitable for the `scratch` runtime image. It is not appropriate if the application or one of its dependencies requires cgo.
- `GOOS=linux` selects Linux as the target operating system, matching the runtime image.
- `go build` compiles the `./cmd/api` package, the module-relative path to the service's main package.
- `-trimpath` removes local filesystem paths from the compiled binary, improving build reproducibility and avoiding embedding build-machine paths.
- `-ldflags="-s -w"` passes options to the Go linker: `-s` omits the symbol table and `-w` omits DWARF debugging information. Both reduce binary size, but make post-build debugging harder.
- `-o /out/server` writes the executable to `/out/server`, a known location that can be copied into the runtime stage.
- `PORT=8080` provides the service's default listening port. The application reads this value at runtime, and operators can override it for a different container port.

Build from the module root containing `go.mod`, `go.sum`, and the Dockerfile. The build copies the module tree so local packages outside `cmd/` are available. Commit both module files, including an empty `go.sum` if the module has no external dependencies. Add a `.dockerignore` to keep local build output, VCS metadata, and secrets out of the context.

Example `.dockerignore`:

```dockerignore
.git
.env*
bin/
dist/
*.test
```

If the builder supports cache mounts, cache the Go module download directory (`/go/pkg/mod`) and build cache (`/root/.cache/go-build`).

## Choices and Tradeoffs

- This is a natural multi-stage build: the Go compiler and module cache are absent from the final image. `scratch` is appropriate because `CGO_ENABLED=0` produces a static binary; the CA bundle is copied so outbound TLS verification continues to work.
- Disabling cgo is not universal. If the service uses cgo or needs OS libraries, choose a compatible runtime image and copy the required shared libraries instead of using `scratch`. `scratch` also has no `/tmp`; if the service or a library writes temporary files, provide a writable temporary directory or choose a runtime image that includes one. Embed time-zone data with `import _ "time/tzdata"` when named time zones must work without system zoneinfo.
- Copying module files before source allows dependency downloads to be cached independently. `go.sum` verifies module content; keep it under source control.
- The numeric non-root identity works without an `/etc/passwd` entry in `scratch`. The application must bind to `0.0.0.0`; `PORT` is a non-secret runtime setting. The exec-form entrypoint makes the server PID 1. For graceful connection draining, handle `SIGTERM` with `signal.NotifyContext` and call `http.Server.Shutdown`.
- `scratch` has no shell or HTTP client for a Dockerfile `HEALTHCHECK`; use a platform-native probe or include a purpose-built probe in the image.
- For multi-platform builds, an alternative is to compile on `$BUILDPLATFORM` and pass `TARGETOS`/`TARGETARCH` to Go. This requires a builder that supports automatic platform build arguments and should be tested for every target architecture.

## Validation Recipe

The runnable sample for this guide is in [`examples/go-http-service/`](../examples/go-http-service/). From that directory, build and run the image, verify its runtime identity and minimal filesystem, exercise both routes, then stop it cleanly:

```sh
docker build -t go-http-api .
docker image inspect --format '{{.Config.User}}' go-http-api
docker create --name go-http-api-inspect go-http-api
docker export go-http-api-inspect | tar -tf - | grep -E '^(server|etc/ssl/certs/ca-certificates.crt)$'
docker rm go-http-api-inspect
docker run -d --name go-http-api -p 8080:8080 go-http-api
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /tmp/go-root.html --write-out '%{http_code}\n' http://127.0.0.1:8080/ | grep -qx 200
grep -q 'Hello World' /tmp/go-root.html
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:8080/healthz | grep -qx 200
docker stop --time 10 go-http-api
docker rm go-http-api
```

The image should report user `65532:65532`; the exported `scratch` filesystem should contain the server and CA bundle but no compiler or application source. `/` should return HTML containing `Hello World`, `/healthz` should return HTTP 200, and the server handles the stop signal with graceful shutdown.

## References

- [Go release history and support policy](https://go.dev/doc/devel/release)
- [Official Go image](https://hub.docker.com/_/golang)
- [Go modules reference](https://go.dev/ref/mod)