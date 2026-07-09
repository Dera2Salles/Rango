# 9. Middleware & Security

`rango::start(router).run()` already layers on a sensible default stack (request logging, security headers, host validation, CORS, sessions when `auth` is enabled, and a debug-error page in debug mode). This page covers what's included and how to extend it.

## What `RangoBuilder` sets up automatically

| Layer | Behavior | Toggle |
|-------|----------|--------|
| Request logger | Logs `METHOD URI → STATUS (Nms)` via `tracing` | always on |
| Static files | Serves `RangoConfig.static_dir` at `/static` | `.with_static(prefix, dir)` overrides |
| CORS | Off by default; wide-open if `cors_allow_all`, or origin-restricted via `cors_allowed_origins` | `.with_cors()` forces wide-open |
| Sessions | `tower-sessions` `MemoryStore`, configured from `RangoConfig.session` | requires `auth` feature |
| Host header validation | Rejects requests with an unrecognized `Host` header (400) | `.without_host_validation()` |
| Security headers | See below | `.without_security_headers()` |
| Debug error page | Renders a diagnostic page for 5xx responses when `debug = true` | tied to `RangoConfig.debug` |
| 404 fallback | Styled 404 page (or your `templates/404.html`) for unmatched routes | always on |

```rust
rango::start(router)
    .bind("0.0.0.0:8000")
    .with_static("/assets", "public")
    .with_cors()
    .without_host_validation()
    .run()
    .await;
```

## Security headers

Controlled by `RangoConfig.security` (`SecurityConfig`):

```rust
use rango::state::SecurityConfig;

init_config(RangoConfig {
    security: SecurityConfig {
        content_type_nosniff: true,          // X-Content-Type-Options: nosniff
        x_frame_options: true,                 // X-Frame-Options: DENY
        xss_protection: true,                  // X-XSS-Protection: 1; mode=block
        referrer_policy: true,                 // Referrer-Policy: same-origin
        hsts_max_age: Some(63072000),          // Strict-Transport-Security (2 years) — enable in prod, HTTPS only
        csp: Some("default-src 'self'".into()),// Content-Security-Policy
    },
    ..RangoConfig::default()
});
```

`hsts_max_age` and `csp` are `None` (disabled) by default — turn them on once you're serving over HTTPS.

## Host header validation

```rust
init_config(RangoConfig {
    allowed_hosts: vec!["example.com".into(), "www.example.com".into()],
    ..RangoConfig::default()
});
```

Leave `allowed_hosts` empty or include `"*"` to disable the check (useful in local dev — this is the default: `["127.0.0.1", "localhost"]`).

## CORS

```rust
init_config(RangoConfig {
    cors_allow_all: false,
    cors_allowed_origins: vec!["https://myapp.com".into()],
    ..RangoConfig::default()
});
```

Or reach for the lower-level helpers directly in your own router:

```rust
use rango::middleware::{cors_layer, cors_layer_for};

router.layer(cors_layer());                                  // any origin — dev/public APIs only
router.layer(cors_layer_for(vec!["https://myapp.com"]));      // specific origins
```

## Rate limiting

A simple in-memory token-bucket limiter, keyed by client IP (`X-Forwarded-For` or connection info):

```rust
use rango::middleware::{RateLimiter, rate_limit_middleware};

let limiter = RateLimiter::new(100, 60); // 100 requests / 60 seconds per IP

let router = router.layer(axum::middleware::from_fn(rate_limit_middleware(limiter)));
```

For distributed deployments (multiple server processes), back this with Redis instead — the in-memory version only tracks state within a single process.

## Authentication middleware

See [Auth & Sessions](07-auth-and-sessions.md#protecting-routes) for `require_auth` and `csrf_middleware`.

## Writing your own middleware

Rango middleware is just standard Axum `middleware::from_fn` — nothing framework-specific:

```rust
use axum::{extract::Request, middleware::Next, response::Response};

async fn my_middleware(req: Request, next: Next) -> Response {
    // ... before ...
    let response = next.run(req).await;
    // ... after ...
    response
}

let router = router.layer(axum::middleware::from_fn(my_middleware));
```

Next: [Signals →](10-signals.md)
