# .NET Web API with ASP.NET Core

## Target and Application Shape

Checked 2026-10-21. This example targets .NET 10.0.x, the current LTS release line. Microsoft listed 10.0.13 as the latest patch on 2026-10-13; use the current patch when rebuilding. The example assumes a project-root context with one `Api.csproj`, a committed `packages.lock.json`, and an ASP.NET Core API that targets `net10.0` and produces `Api.dll`.

## Dockerfile

```dockerfile
FROM mcr.microsoft.com/dotnet/sdk:10.0-noble AS build

WORKDIR /src
COPY Api.csproj packages.lock.json ./
RUN dotnet restore Api.csproj --locked-mode

COPY . ./
RUN dotnet publish Api.csproj \
    --no-restore \
    --configuration Release \
    --output /out \
    /p:UseAppHost=false

FROM mcr.microsoft.com/dotnet/aspnet:10.0-noble

WORKDIR /app
COPY --from=build /out/ ./

ENV ASPNETCORE_HTTP_PORTS=8080
USER $APP_UID
EXPOSE 8080
ENTRYPOINT ["dotnet", "Api.dll"]
```

The environment variable and `RUN` arguments are used for these reasons:

- `dotnet restore Api.csproj` resolves and downloads the NuGet dependencies for the named project. `--locked-mode` requires the committed `packages.lock.json` to match the project; it fails on drift instead of rewriting the lock file during an image build.
- `dotnet publish Api.csproj` builds and prepares the named project for deployment. `--no-restore` reuses the earlier locked restore rather than resolving packages a second time. `--configuration Release` selects production build settings, including compiler optimizations. `--output /out` puts the publish artifacts in a known directory for the runtime stage to copy. `/p:UseAppHost=false` sets an MSBuild property that omits the platform-specific app-host executable; the image starts the framework-dependent application with `dotnet Api.dll` instead.
- `ASPNETCORE_HTTP_PORTS=8080` explicitly restates the ASP.NET Core container image's .NET 8+ default HTTP port. It matches `EXPOSE 8080` and can be overridden when the container starts.
- `USER $APP_UID` opts into the non-root `app` identity (UID 1654) included in the standard .NET Linux image; that image otherwise starts as root.

Build from the directory containing the project file, lock file, and Dockerfile. The lock file must be generated and committed by NuGet's package-lock workflow. Add a `.dockerignore` for `bin/`, `obj/`, `.git`, IDE files, local configuration, and secrets.

If present, copy restore-affecting files such as `nuget.config`, `Directory.Build.props`, `Directory.Packages.props`, and `global.json` before `dotnet restore`. For a solution or a project with project references, copy every referenced `.csproj` and its relevant restore inputs before restoring; otherwise the dependency layer may be incomplete or fail.

Example `.dockerignore`:

```dockerignore
.git
.vs/
.env*
bin/
obj/
*.user
```

If the builder supports cache mounts, cache NuGet packages at `/root/.nuget/packages`.

## Choices and Tradeoffs

- The SDK stage restores, builds, and publishes; the ASP.NET runtime stage contains only published output and the runtime. Framework-dependent deployment keeps the runtime image smaller than a self-contained deployment; the runtime must match the target framework.
- Copying the project and lock file before source preserves the restore layer across code changes. `--locked-mode` makes a stale lock file fail rather than silently changing dependency resolution.
- Standard .NET Linux runtime images include the non-root `app` user but default to root; this example explicitly opts into `$APP_UID`. Prepare any required writable paths for that identity. The `10.0-noble-chiseled` ASP.NET image is a smaller, non-root-by-default alternative, but has no shell or package manager and is less convenient to debug.
- `ASPNETCORE_HTTP_PORTS` sets the HTTP listener to 8080, matching the documented port. Override it at runtime when needed. Secrets and environment-specific settings should be supplied at runtime, not copied into the image. The exec-form entrypoint lets .NET handle termination signals directly.
- The .NET SDK can also publish container images without a Dockerfile. That is a valid alternative for teams that do not need custom OS packages or other Dockerfile-specific build steps; this guide focuses on the Dockerfile workflow.
- Add a `HEALTHCHECK` only if there is a reliable application health endpoint and a suitable probe available in the runtime image; otherwise configure a platform-native probe.

## Validation Recipe

The runnable sample for this guide is in [`examples/dotnet-aspnet-core/`](../examples/dotnet-aspnet-core/). From that directory, create or check the NuGet lock file, build and run the image, verify its runtime identity and published output, exercise both routes, then stop it cleanly:

```sh
dotnet restore Api.csproj --use-lock-file
docker build -t aspnet-api .
docker image inspect --format '{{.Config.User}}' aspnet-api
test -z "$(docker run --rm --entrypoint dotnet aspnet-api --list-sdks)"
docker run --rm --entrypoint dotnet aspnet-api --list-runtimes | grep -q 'Microsoft.AspNetCore.App 10.'
docker run --rm --entrypoint sh aspnet-api -c 'test -f /app/Api.dll && test ! -d /src && test ! -d /usr/share/dotnet/sdk'
docker run -d --name aspnet-api -p 8080:8080 aspnet-api
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /tmp/aspnet-root.html --write-out '%{http_code}\n' http://127.0.0.1:8080/ | grep -qx 200
grep -q 'Hello World' /tmp/aspnet-root.html
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:8080/healthz | grep -qx 200
docker stop --time 10 aspnet-api
docker rm aspnet-api
```

The image should report user ID `1654`; `dotnet --info` should show the ASP.NET runtime without an SDK. `/` should return HTML containing `Hello World`, `/healthz` should return HTTP 200, and the process should stop cleanly on the container's termination signal.

## References

- [.NET support policy](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core)
- [.NET container configuration](https://learn.microsoft.com/en-us/dotnet/core/containers/publish-configuration)
- [NuGet package lock files](https://learn.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files#locking-dependencies)
- [Microsoft .NET container images](https://mcr.microsoft.com/en-us/catalog?search=dotnet)