# Python Web API with FastAPI

## Target and Application Shape

Checked 2026-10-21. This example targets Python 3.14.x and FastAPI 0.156.8, with Uvicorn as the ASGI server. FastAPI 0.156.8 requires Python 3.10 or newer. The example assumes a project-root build context containing `requirements.lock` (all direct and transitive requirements pinned with hashes) and `app/main.py`, which exports `app`.

Python 3.14 and 3.15 are in bug-fix support; 3.11, 3.12, and 3.13 are in security-only support (3.13 left bug-fix support on 2026-10-01), and 3.10 reached end of life on 2026-10-01. Python 3.15.0 was released on 2026-10-09. This example stays on 3.14 to avoid assuming that every application dependency already publishes compatible 3.15 wheels; check wheel availability before changing the target. The lock file must include `fastapi[standard]` (or `fastapi-cli` and `uvicorn`) because the base `fastapi` package alone does not provide the `fastapi run` command and server.

The image runs one server process. Use one process per container when the surrounding platform provides replication; use multiple workers only when that is a deliberate deployment choice.

## Dockerfile

```dockerfile
FROM python:3.14-slim-trixie

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PATH="/opt/venv/bin:$PATH"

WORKDIR /app

RUN python -m venv /opt/venv

COPY requirements.lock ./
RUN pip install --no-cache-dir --require-hashes -r requirements.lock && rm requirements.lock

COPY app/ ./app/

USER 10001:10001
EXPOSE 8000
ENTRYPOINT ["fastapi", "run", "app/main.py"]
CMD ["--host", "0.0.0.0", "--port", "8000"]
```

The environment variables and dependency-installation commands have these purposes:

- `PYTHONDONTWRITEBYTECODE=1` tells Python not to create `.pyc` files when modules are imported at runtime. This avoids writes to the application filesystem; the tradeoff is that Python may need to compile source on startup.
- `PYTHONUNBUFFERED=1` disables buffering for standard output and error. Container logs are emitted promptly instead of waiting for a buffer to fill.
- `PATH="/opt/venv/bin:$PATH"` puts the virtual environment's executables, including `fastapi`, before system commands so the runtime uses the installed dependencies.
- `python -m venv /opt/venv` runs Python's standard-library virtual-environment module (`-m venv`) and creates the environment at `/opt/venv`. Keeping it outside `/app` separates installed packages from application source and gives the `PATH` setting a stable location.
- `pip install` installs dependencies into the active virtual environment. `--no-cache-dir` prevents pip's downloaded package cache from being retained in an image layer. `--require-hashes` requires each resolved package to match a hash in the lock file, detecting unexpected artifacts. `-r requirements.lock` tells pip to read the complete dependency set from that file.

Build from the directory containing the Dockerfile and the listed inputs. Add a `.dockerignore` that excludes virtual environments, local secrets, tests not needed at runtime, and build artifacts. Generate and update `requirements.lock` with the project's chosen lock workflow; do not hand-maintain a partial list of top-level packages.

Example `.dockerignore`:

```dockerignore
.git
.env*
.venv/
__pycache__/
*.py[cod]
tests/
```

If the builder supports cache mounts, cache pip's `/root/.cache/pip` instead of using `--no-cache-dir`. Keep the cache out of the image.

## Alternative: uv-Managed Dependencies

Checked 2026-10-21. This alternative uses uv 0.13.4, released 2026-10-20, with the same Python 3.14.x and FastAPI 0.156.8 targets. It assumes the build context contains `pyproject.toml`, a committed `uv.lock`, and `app/main.py`. Declare FastAPI and its server dependencies in the project's runtime dependencies. This example installs dependencies from the lockfile but does not install the application itself as a Python distribution; the application is imported from `/app` at runtime.

```dockerfile
FROM python:3.14-slim-trixie AS build

COPY --from=ghcr.io/astral-sh/uv:0.13.4 /uv /uvx /bin/

ENV UV_NO_CACHE=1 \
    UV_NO_DEV=1 \
    UV_COMPILE_BYTECODE=1 \
    UV_PYTHON_DOWNLOADS=0

WORKDIR /app
COPY pyproject.toml uv.lock ./
RUN uv sync --locked --no-install-project

FROM python:3.14-slim-trixie AS runtime

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PATH="/app/.venv/bin:$PATH"

WORKDIR /app
COPY --from=build /app/.venv /app/.venv
COPY app/ ./app/

USER 10001:10001
EXPOSE 8000
ENTRYPOINT ["fastapi", "run", "app/main.py"]
CMD ["--host", "0.0.0.0", "--port", "8000"]
```

The `ENV` and `RUN` instructions configure uv and the runtime environment:

- In the build stage, `UV_NO_CACHE=1` tells uv not to retain its package cache, avoiding cache files in the build layer. `UV_NO_DEV=1` excludes development dependency groups from the sync, keeping test and tooling packages out of the production environment. `UV_COMPILE_BYTECODE=1` compiles installed Python modules to bytecode during the build; this can improve startup time at the cost of some build time and image size. `UV_PYTHON_DOWNLOADS=0` prevents uv from downloading a managed interpreter that would not be included in the runtime stage; a project requiring a different Python version will fail during the build instead.
- In the runtime stage, `PYTHONDONTWRITEBYTECODE=1` prevents runtime imports from writing new `.pyc` files, and `PYTHONUNBUFFERED=1` makes application logs appear promptly. `PATH="/app/.venv/bin:$PATH"` selects the virtual environment copied from the build stage, so the `fastapi` command and imports use the locked dependencies.
- `uv sync` resolves the project environment from `pyproject.toml` and `uv.lock`. `--locked` requires the lock file to match the project metadata and fails instead of changing the lock during the image build. `--no-install-project` installs dependencies without installing the current project as a package; this keeps the expensive dependency layer reusable when only application source changes. The runtime imports `app/main.py` directly from `/app`.

Build from the project root, and add `.venv`, local secrets, tests not needed at runtime, and build output to `.dockerignore`. Run `uv lock` when dependency declarations change and commit the resulting lockfile. `uv sync --locked` fails if the lockfile is out of date. The builder and runtime use the same Python base tag so the virtual environment and any native extensions remain compatible; if native extensions need OS libraries, include those libraries in the runtime stage as well.

This multi-stage option keeps uv out of the final image and caches dependency installation independently from application source. uv 0.13.4 is pinned by release tag; use an image digest instead where immutable base inputs are required. `UV_NO_CACHE=1` avoids retaining uv's package cache in the build layer; if the builder supports cache mounts, omit it and cache uv's `/root/.cache/uv` instead. If the application is packaged as a project or needs workspace members, adjust the sync strategy to copy the required metadata and source before the final locked sync rather than using `--no-install-project`.

The runtime interface, non-root identity, and `fastapi run` command match the pip example. Do not use `uv run` as the production server command here; the virtual environment's executable is already on `PATH`.

## Choices and Tradeoffs

- The official slim Debian image avoids Alpine's musl compatibility differences. Install OS build packages in a separate build stage only if dependencies require compiling native extensions; copy only the resulting environment and required shared libraries into the runtime stage. For a dependency set that installs cleanly from wheels, this single stage is simpler and avoids copying virtual environments between potentially incompatible images.
- Installing the hash-locked requirements before the application source preserves the dependency layer across source edits. `--no-cache-dir` avoids retaining pip's download cache in the image.
- The virtual environment lives outside the application directory; its executable path is explicit. A numeric non-root identity avoids depending on account-management utilities in the slim base. Application files and installed dependencies remain root-owned and readable; give the runtime user ownership only of directories the application must write to.
- `fastapi run` is the production CLI command; `fastapi dev` enables development behavior such as reload and is not used here. The exec-form entrypoint and command let the server receive termination signals directly. Override the command arguments for a different port or worker count. When behind a trusted TLS-terminating proxy, configure proxy-header trust deliberately; do not trust arbitrary client-supplied forwarding headers.
- Runtime settings belong in environment variables read by the application. Never bake secrets into the image. The application must listen on the configured address and port; `EXPOSE` documents the default port but does not publish it.
- Add a `HEALTHCHECK` only if the application exposes a reliable health endpoint; use a suitable probe available in the image or configure the check in the runtime platform.

## Validation Recipe

The runnable sample for this guide uses the uv-managed alternative and is in [`examples/python-fastapi/`](../examples/python-fastapi/). From that directory, update the lock after dependency changes, build and run the image, verify both routes and the configured runtime identity, then stop the server cleanly:

```sh
uv lock
docker build -t fastapi-api .
docker image inspect --format '{{.Config.User}}' fastapi-api
docker run -d --name fastapi-api -p 8000:8000 fastapi-api
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /tmp/fastapi-root.html --write-out '%{http_code}\n' http://127.0.0.1:8000/ | grep -qx 200
grep -q 'Hello World' /tmp/fastapi-root.html
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:8000/healthz | grep -qx 200
docker stop --time 10 fastapi-api
docker rm fastapi-api
```

The image should report user `10001:10001`; the root route should return HTML containing `Hello World`, and `/healthz` should return HTTP 200. The final image contains the application and installed virtual environment, but not uv, project metadata, the lockfile, or local development files.

## References

- [Python version status](https://devguide.python.org/versions/)
- [FastAPI on PyPI](https://pypi.org/project/fastapi/)
- [FastAPI container deployment](https://fastapi.tiangolo.com/deployment/docker/)
- [uv 0.13.4 release](https://github.com/astral-sh/uv/releases/tag/0.13.4)
- [uv Docker integration guide](https://docs.astral.sh/uv/guides/integration/docker/)
- [Official Python image](https://hub.docker.com/_/python)