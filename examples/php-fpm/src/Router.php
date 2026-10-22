<?php

namespace Example;

final class Router
{
    public static function dispatch(): void
    {
        $method = $_SERVER['REQUEST_METHOD'] ?? '';
        $path = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);

        if ($method === 'GET' && $path === '/') {
            header('Content-Type: text/html; charset=utf-8');
            echo '<!doctype html><html><body><h1>Hello World</h1></body></html>';
            return;
        }

        if ($method === 'GET' && $path === '/healthz') {
            header('Content-Type: text/plain; charset=utf-8');
            echo 'ok';
            return;
        }

        http_response_code($method === 'GET' ? 404 : 405);
        header('Content-Type: text/plain; charset=utf-8');
        echo 'not found';
    }
}