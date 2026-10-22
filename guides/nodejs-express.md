# Node.js Web API with Express

## Target and Application Shape

Checked 2026-10-21. This example targets Node.js 24.x (24.22.0 at review) and Express 5.x. Node.js 24 moved from Active LTS to Maintenance LTS on 2026-10-20 and remains supported until 2028-04-30; Node.js 26 is scheduled to enter Active LTS on 2026-10-28, so re-evaluate the target after that date. Express 5 requires Node.js 18 or newer. The example assumes a project-root build context with `package.json`, a committed `package-lock.json`, and a production entrypoint at `src/index.js`. The application reads its listening port from `process.env.PORT`.

## Dockerfile

```dockerfile
# syntax=docker/dockerfile:1.7

FROM node:24-trixie-slim

ENV NODE_ENV=production \
    PORT=3000

WORKDIR /app

COPY package.json package-lock.json ./
RUN --mount=type=secret,id=npm-ca,target=/run/secrets/npm-ca,required=false \
    if [ -s /run/secrets/npm-ca ]; then \
        npm ci --omit=dev --cafile=/run/secrets/npm-ca; \
    else \
        npm ci --omit=dev; \
    fi && npm cache clean --force

COPY src/ ./src/

EXPOSE 3000
USER node
CMD ["node", "src/index.js"]
```

The environment variables and `RUN` commands are configured as follows:

- `NODE_ENV=production` tells Node.js libraries and frameworks to use production behavior, such as disabling development diagnostics or enabling production optimizations. It is also the image's default runtime mode.
- `PORT=3000` supplies the service's default listening port. The application reads it from `process.env.PORT`, and it can be overridden when the container starts.
- `npm ci` runs as root during the image build, so the application files and installed dependencies remain root-owned and readable by the runtime user. The process switches to the official image's `node` user only after these files have been copied and installed.
- `npm ci` installs dependencies from `package-lock.json` as a clean, lockfile-driven install. It fails if the manifest and lockfile disagree instead of updating the lockfile, making builds predictable.
- `npm cache clean --force` removes npm's downloaded-package cache in the same layer after installation so it does not inflate the image.
- `--omit=dev` leaves packages in `devDependencies` off disk in the production image. Build and test tools should stay in `devDependencies`; Express and other packages needed while serving requests belong in `dependencies`.

Build from the directory containing the Dockerfile and the listed inputs. Add a `.dockerignore` for `.git`, local environment files, `node_modules`, tests, and other files not needed at runtime. Keep Express and all runtime dependencies in `dependencies`, not `devDependencies`.

Example `.dockerignore`:

```dockerignore
.git
.env*
node_modules/
coverage/
test/
tests/
```

If the builder supports cache mounts, cache npm's `/root/.npm` directory used by the root build step rather than retaining it in an image layer.

The Dockerfile uses `RUN --mount=type=secret`, so builds require Docker BuildKit with Dockerfile frontend 1.7 or newer. The npm CA secret is optional and supports networks that intercept TLS with an organization-managed certificate. With normal public registry trust, the standard `docker build` command needs no secret. For an inspected network, use an approved CA bundle outside the build context instead of the plain build command:

```sh
docker build --secret id=npm-ca,src=/path/to/approved-ca-bundle.pem -t express-api .
```

The certificate is mounted only for the dependency-install step and is not copied into an image layer. Do not disable npm TLS verification with `strict-ssl=false`.

## Choices and Tradeoffs

- This is a single-stage image: a plain JavaScript service has no separate compile artifact. `npm ci --omit=dev` installs exactly the lockfile dependency tree without development dependencies. A multi-stage build becomes useful if the app is transpiled, bundles assets, or needs build-only tools.
- Dependencies are installed before source is copied so routine code changes reuse the npm install layer. The official Debian slim image is a pragmatic default for native npm modules; Alpine uses musl and can require different native binaries or additional build setup.
- The official image's `node` account runs the process, while application files and dependencies remain root-owned and readable. Give `node` ownership only of directories the application must write to. The service must bind to `0.0.0.0`, not only loopback, for container networking. `PORT` is a non-secret runtime setting; `EXPOSE` is metadata, not port publication.
- The exec-form command runs Node directly and supports signal-based shutdown. Handle termination signals and close HTTP servers gracefully in the application.
- Add a `HEALTHCHECK` only when a reliable application health route exists; Node's built-in HTTP client can probe it without adding `curl`.

## Validation Recipe

The runnable sample for this guide is in [`examples/nodejs-express/`](../examples/nodejs-express/). From that directory, build and run the image, verify its runtime identity, production dependency set, and both routes, then stop it cleanly:

```sh
docker build -t express-api .
docker image inspect --format '{{.Config.User}}' express-api
docker run --rm --entrypoint id express-api -u | grep -qx 1000
docker run -d --name express-api -p 3000:3000 express-api
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /tmp/express-root.html --write-out '%{http_code}\n' http://127.0.0.1:3000/ | grep -qx 200
grep -q 'Hello World' /tmp/express-root.html
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:3000/healthz | grep -qx 200
docker exec express-api npm ls --omit=dev --depth=0
docker stop --time 10 express-api
docker rm express-api
```

The image should report user `node` (UID 1000); `/` should return HTML containing `Hello World`, and `/healthz` should return HTTP 200. The running process binds to `0.0.0.0` and handles termination signals by closing the HTTP server.

## References

- [Node.js release schedule](https://nodejs.org/en/about/previous-releases)
- [Express 5 API reference](https://expressjs.com/en/5x/api.html)
- [Official Node.js image](https://hub.docker.com/_/node)