# 11. Configuration Reference

All framework settings live in a single `RangoConfig`, initialized once via `rango::init_config()` before `rango::start()`. Every field has a sensible default — override only what you need.

```rust
use rango::{RangoConfig, DatabaseConfig, init_config};

init_config(RangoConfig {
    debug: false,
    secret_key: std::env::var("RANGO_SECRET_KEY").unwrap(),
    database: Some(DatabaseConfig::postgres("localhost", 5432, "user", "pass", "mydb")),
    ..RangoConfig::default()
});
```

Or build everything from environment variables — see the table below:

```rust
init_config(RangoConfig::from_env());
```

`init_config()` panics if called more than once, and prints startup warnings (e.g. insecure secret key, `cors_allow_all` in production) via `RangoConfig::validate()`.

## `RangoConfig`

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `debug` | `bool` | `true` in debug builds | Enables the diagnostic error page & verbose logging |
| `allowed_hosts` | `Vec<String>` | `["127.0.0.1", "localhost"]` | `Host` header allow-list; empty or `"*"` disables the check |
| `templates_dir` | `String` | `"templates"` | MiniJinja template root |
| `static_dir` | `Option<String>` | `Some("static")` | Served at `/static`; `None` disables static serving |
| `database` | `Option<DatabaseConfig>` | `None` | See below; `None` runs without a DB |
| `secret_key` | `String` | insecure placeholder | **Change in production** — used for signing sessions/tokens |
| `cors_allow_all` | `bool` | `false` | Wide-open CORS — dev/public APIs only |
| `cors_allowed_origins` | `Vec<String>` | `[]` | Used when `cors_allow_all` is `false` |
| `bind_addr` | `String` | `"127.0.0.1:8000"` | Overridable via `.bind()` on `RangoBuilder` |
| `session` | `SessionConfig` | see below | |
| `security` | `SecurityConfig` | see below | |

## `DatabaseConfig`

Build with a shortcut constructor, then chain builder methods:

```rust
DatabaseConfig::sqlite("rango.db")
    .migrations("./migrations")
    .max_connections(10)
    .connect_timeout(30)
    .log_statements();

DatabaseConfig::postgres("localhost", 5432, "user", "password", "mydb");
DatabaseConfig::mysql("localhost", 3306, "user", "password", "mydb");
DatabaseConfig::from_url("postgres://user:pass@host/db");
```

| Field | Type | Default | Builder method |
|-------|------|---------|----------------|
| `url` | `String` | — | (constructor) |
| `max_connections` | `u32` | `5` | `.max_connections(n)` |
| `min_connections` | `u32` | `1` | `.min_connections(n)` |
| `connect_timeout_secs` | `u64` | `30` | `.connect_timeout(secs)` |
| `idle_timeout_secs` | `Option<u64>` | `Some(600)` | — |
| `max_lifetime_secs` | `Option<u64>` | `Some(1800)` | — |
| `migrations_path` | `Option<String>` | `Some("./migrations")` | `.migrations(path)` |
| `auto_migrate` | `bool` | `true` | `.no_auto_migrate()` |
| `log_statements` | `bool` | `false` | `.log_statements()` |
| `read_replica_url` | `Option<String>` | `None` | `.with_read_replica(url)` |

`sqlite(":memory:")` is a shortcut for an in-memory database (handy in tests).

## `SessionConfig`

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `secure` | `bool` | `false` in debug, `true` in release | Requires HTTPS |
| `cookie_name` | `String` | `"rango_session"` | |
| `max_age_secs` | `u64` | `86400` (1 day) | |
| `http_only` | `bool` | `true` | Blocks JS access to the cookie |
| `same_site` | `String` | `"Lax"` | `"Strict"`, `"Lax"`, or `"None"` |

## `SecurityConfig`

| Field | Type | Default | Header |
|-------|------|---------|--------|
| `content_type_nosniff` | `bool` | `true` | `X-Content-Type-Options: nosniff` |
| `x_frame_options` | `bool` | `true` | `X-Frame-Options: DENY` |
| `xss_protection` | `bool` | `true` | `X-XSS-Protection: 1; mode=block` |
| `referrer_policy` | `bool` | `true` | `Referrer-Policy: same-origin` |
| `hsts_max_age` | `Option<u64>` | `None` | `Strict-Transport-Security` (enable only over HTTPS) |
| `csp` | `Option<String>` | `None` | `Content-Security-Policy` |

## Environment variables (`RangoConfig::from_env()`)

| Variable | Default |
|----------|---------|
| `RANGO_DEBUG` | `true` in debug builds |
| `RANGO_SECRET_KEY` | insecure placeholder — **set this in production** |
| `RANGO_ADDR` | `127.0.0.1:8000` |
| `RANGO_TEMPLATES` | `templates` |
| `RANGO_STATIC` | `static` |
| `RANGO_ALLOWED_HOSTS` | `127.0.0.1,localhost` (comma-separated) |
| `RANGO_CORS_ALL` | `false` |
| `RANGO_CORS_ORIGINS` | *(empty)* (comma-separated) |
| `SESSION_SECURE` | `false` in debug, `true` in release |
| `SESSION_COOKIE_NAME` | `rango_session` |
| `SESSION_MAX_AGE` | `86400` |
| `SESSION_SAME_SITE` | `Lax` |
| `DATABASE_URL` | *(none — DB disabled if absent)* |
| `DATABASE_READ_URL` | *(none)* — read-replica URL |
| `DB_MAX_CONNECTIONS` | `5` |
| `DB_MIN_CONNECTIONS` | `1` |
| `DB_CONNECT_TIMEOUT` | `30` |
| `DB_MIGRATIONS_PATH` | `./migrations` |
| `DB_AUTO_MIGRATE` | `true` (set to `false` to disable) |
| `DB_LOG_STATEMENTS` | `false` |

A ready-to-copy `.env.example` documenting all of these is generated by `rango startproject`.

## Production checklist

`RangoConfig::validate()` (called automatically by `init_config()`) warns about:

- An insecure or short (`< 32` chars) `secret_key`
- `cors_allow_all = true` while `debug = false`
- Session cookies not marked `secure` while `debug = false`
- `allowed_hosts` containing `"*"`

Before deploying, also double check:

- [ ] `RANGO_DEBUG=false` (or `debug: false`) — hides internal error details from responses
- [ ] A strong, random `RANGO_SECRET_KEY` (64+ characters)
- [ ] `allowed_hosts` set to your real domain(s)
- [ ] `session.secure = true` (automatic once `debug = false`) and served over HTTPS
- [ ] `security.hsts_max_age` set once HTTPS is confirmed working
- [ ] `cors_allow_all = false`, with `cors_allowed_origins` listing only trusted origins

Next: [CLI Reference →](12-cli-reference.md)
