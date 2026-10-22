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