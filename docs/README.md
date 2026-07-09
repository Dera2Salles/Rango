# Rango Documentation

Rango is a Django-inspired, async web framework for Rust, built on top of **Axum** and **Tokio**. It gives you centralized routing, a batteries-included ORM, server-rendered templates, an auto-generated admin panel, sessions/auth, forms & validators, flash messages, caching, and a `rango` CLI — all while staying as fast and type-safe as hand-rolled Axum.

This directory is the full reference manual. If you're brand new, read it top to bottom in order; otherwise jump straight to the topic you need.

## Table of Contents

| # | Guide | What you'll learn |
|---|-------|--------------------|
| 1 | [Getting Started](01-getting-started.md) | Installing the CLI, scaffolding a project, running the dev server |
| 2 | [Routing & Views](02-routing-and-views.md) | The `urls!` macro, `#[view]`, `#[login_required]`, `context!` |
| 3 | [Templates](03-templates.md) | Rendering MiniJinja templates, built-in Django-style filters, static files |
| 4 | [Database & ORM](04-database-orm.md) | `#[model]`, `QuerySet`, typed filters, aggregates, transactions, migrations |
| 5 | [Admin Panel](05-admin-panel.md) | Auto-generated CRUD UI, search, pagination, bulk actions |
| 6 | [Forms & Validation](06-forms-and-validation.md) | `Form`, field validators, error handling |
| 7 | [Auth & Sessions](07-auth-and-sessions.md) | Login/logout, password hashing, CSRF, sessions |
| 8 | [Messages & Caching](08-messages-and-cache.md) | Flash messages, in-memory cache |
| 9 | [Middleware & Security](09-middleware-and-security.md) | CORS, rate limiting, security headers, host validation |
| 10 | [Signals](10-signals.md) | Decoupled event hooks (`PRE_SAVE`, `POST_SAVE`, ...) |
| 11 | [Configuration Reference](11-configuration-reference.md) | Every `RangoConfig` field & environment variable |
| 12 | [CLI Reference](12-cli-reference.md) | `rango startproject`, `makemigrations`, `migrate`, ... |

## Quick example

```rust
use rango::macros::{urls, view};

urls!(
    path("/", home),
);

#[view]
pub async fn home() {
    rango::render("index.html", rango::json!({ "title": "Welcome to Rango" }))
}
```

```rust
// src/main.rs
mod urls;

use rango::{RangoConfig, DatabaseConfig, init_config};

#[tokio::main]
async fn main() {
    init_config(RangoConfig {
        database: Some(DatabaseConfig::sqlite("rango.db").migrations("./migrations")),
        ..RangoConfig::default()
    });

    rango::start(urls::get_rango_router()).run().await;
}
```

## Feature flags

Rango is modular — you only pay for what you use in compile time and binary size.

| Feature | Default | Enables |
|---------|---------|---------|
| `templates` | ✅ on | MiniJinja rendering, `rango::render`, built-in filters |
| `db` | off | SQLx-backed ORM (`#[model]`, `QuerySet`), migrations |
| `auth` | off | Sessions, login/logout, password hashing, CSRF, flash messages |

Enable them in your `Cargo.toml`:

```toml
[dependencies]
rango = { version = "0.1.0", package = "rango-framework", features = ["db", "templates", "auth"] }
```

`db` + `templates` together also unlock the auto-generated **admin panel** (see [Admin Panel](05-admin-panel.md)).

## Contributing

Want to help improve Rango itself? See [CONTRIBUTING.md](../CONTRIBUTING.md) for the development setup, coding guidelines, and PR workflow.
