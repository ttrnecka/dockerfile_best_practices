# PHP Web Application with PHP-FPM

## Target and Application Shape

Checked 2026-10-21. This example targets PHP 8.5.x, currently in active support, and Composer 2.10.3. It assumes a project-root build context with `composer.json`, `composer.lock`, application source under `public/` and `src/`, and a framework or front controller that uses Composer's `vendor/` directory.

PHP-FPM serves FastCGI, not HTTP. A compatible web server or reverse proxy must speak FastCGI to it. Do not publish the FPM port directly to untrusted networks.

## Dockerfile

```dockerfile
FROM php:8.5-cli-trixie AS dependencies

COPY --from=composer:2.10.3 /usr/bin/composer /usr/bin/composer

WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends unzip \
    && rm -rf /var/lib/apt/lists/*

COPY composer.json composer.lock ./
RUN composer install \
    --no-dev \
    --prefer-dist \
    --no-interaction \
    --no-progress \
    --no-scripts \
    --no-autoloader

COPY . ./
RUN composer dump-autoload --no-dev --classmap-authoritative --no-scripts

FROM php:8.5-fpm-trixie AS runtime

WORKDIR /var/www/html
RUN mv "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini"
RUN printf 'opcache.validate_timestamps=0\n' > "$PHP_INI_DIR/conf.d/zz-opcache.ini"

COPY --from=dependencies /app/ ./

EXPOSE 9000
CMD ["php-fpm"]
```

The PHP build-stage commands configure Composer dependencies and runtime PHP settings:

- Installing `unzip` in the dependency stage lets Composer extract preferred distribution archives. The apt index is refreshed and removed in the same layer.
- `composer install` installs the exact dependency versions in `composer.lock`. `--no-dev` omits `require-dev` packages from the production image. `--prefer-dist` prefers packaged distribution archives over source checkouts. `--no-interaction` prevents prompts during automated builds. `--no-progress` keeps download progress bars out of build logs. `--no-scripts` skips Composer lifecycle scripts before application source has been copied; if the application needs such scripts, run the required reviewed scripts explicitly after copying source. `--no-autoloader` defers autoloader generation until the application's source is present.
- `composer dump-autoload` generates the autoloader after source has been copied. `--no-dev` excludes development dependencies from its generated mappings. `--classmap-authoritative` uses the generated class map as authoritative, improving class lookup at runtime; all autoloadable classes must therefore be present when this command runs. `--no-scripts` continues to suppress project-defined Composer scripts.
- `mv "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini"` activates PHP's bundled production configuration by moving it to the filename PHP loads by default. `$PHP_INI_DIR` expands to the PHP configuration directory provided by the official image.

This Dockerfile intentionally sets no `ENV` variables. Supply application-specific settings and secrets at runtime; add PHP or FPM environment defaults only when the application contract requires them.

Build from the application root containing the Dockerfile and Composer files. The context is copied after dependency installation so root-level framework files and configuration are available when Composer builds the autoloader. Add a `.dockerignore` for `.git`, local environment files, `vendor/`, tests, and generated artifacts. Add required PHP extensions using the official image's `docker-php-ext-*` helpers and retain only their runtime libraries.

Example `.dockerignore`:

```dockerignore
.git
.env*
vendor/
tests/
```

If the builder supports cache mounts, cache Composer's `/root/.cache/composer` directory.

## Choices and Tradeoffs

- Composer 2.10.3 runs in a PHP 8.5 CLI stage, matching the PHP version and Debian release used by the FPM runtime; only the Composer executable is copied into that build stage. Composer and its tooling stay out of the runtime image. The lock file is copied before source for cache reuse. The example disables scripts during dependency installation; projects whose install scripts generate required files must handle those deliberately after source is copied.
- If `composer.json` requires PHP extensions, install those extensions in the dependency stage as well as the runtime image so Composer's platform checks reflect the actual runtime. Do not bypass these checks globally with `--ignore-platform-reqs`.
- The official image's production `php.ini` is enabled. The `opcache.validate_timestamps=0` override avoids checking whether immutable application files changed on disk; deploy a new image for code changes. FPM pool settings belong in a dedicated configuration file under `/usr/local/etc/php-fpm.d/`; PHP application configuration and secrets are provided at runtime.
- The official FPM image uses a root master process to manage workers, which run as `www-data` by default. Application code and dependencies remain root-owned and readable by the workers. Give `www-data` ownership only of application-specific writable directories, such as `storage/` or `var/`, when needed. This is a deliberate FPM process-model exception to the non-root container default. Running the entire FPM process as non-root requires a tested pool configuration and writable runtime paths; do not simply change `USER` and assume the image still starts correctly.
- Port 9000 is FastCGI, not HTTP. `EXPOSE` is documentation only; do not use host port publishing for FPM. Put it on a private network with a trusted FastCGI-speaking frontend. The exec-form command runs FPM in the foreground as the container's main process. A health check should use an intentional FPM status/ping or application-aware FastCGI probe; an HTTP probe belongs on the frontend.
- The official FPM image sets `STOPSIGNAL SIGQUIT` for graceful worker shutdown. Retain its process setup unless a replacement has been tested.
- A multi-stage build is useful for Composer and generated dependencies. It does not remove PHP extension runtime libraries; those must still be present in the final image.

## Validation Recipe

The runnable sample for this guide is in [`examples/php-fpm/`](../examples/php-fpm/). From that directory, build and validate FPM configuration, then start FPM without publishing port 9000. The disposable Debian client installs `cgi-fcgi` and shares the FPM container's network namespace to test both application routes directly:

```sh
docker build -t php-fpm-app .
docker run --rm --entrypoint php-fpm php-fpm-app -tt 2>&1 | grep -E 'user = www-data|group = www-data'
docker run --rm --entrypoint sh php-fpm-app -ec '! command -v composer && test -f /var/www/html/public/index.php'
docker run -d --name php-fpm-app php-fpm-app
docker top php-fpm-app -eo user,args
docker run --rm --network container:php-fpm-app debian:trixie-slim sh -ec '
    apt-get update >/dev/null && apt-get install -y --no-install-recommends libfcgi-bin >/dev/null
    test_route() {
        route="$1"
        expected="$2"
        response="$(REQUEST_METHOD=GET REQUEST_URI="$route" SCRIPT_FILENAME=/var/www/html/public/index.php cgi-fcgi -bind -connect 127.0.0.1:9000)"
        printf "%s\n" "$response" | grep -qi "^Content-Type:"
        if printf "%s\n" "$response" | grep -q "^Status:"; then exit 1; fi
        printf "%s\n" "$response" | grep -q "$expected"
    }
    test_route / "Hello World"
    test_route /healthz "^ok$"
'
docker stop --time 10 php-fpm-app
docker inspect --format '{{.State.ExitCode}}' php-fpm-app | grep -qx 0
docker rm php-fpm-app
```

The CGI responses must include a `Content-Type` header and the expected body. No `Status` header means CGI status 200; the test rejects an explicit status header. Confirm `docker top` shows the FPM master as root and its pool worker as `www-data`. The test client is separate from the runtime image, and port 9000 is never published to the host.

## References

- [PHP supported versions](https://www.php.net/supported-versions.php)
- [Official PHP image](https://hub.docker.com/_/php)
- [PHP-FPM documentation](https://www.php.net/manual/en/install.fpm.php)
- [Composer documentation](https://getcomposer.org/doc/)