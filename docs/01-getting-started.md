# 1. Getting Started

## Install the CLI

The `rango` CLI scaffolds projects and apps, and wraps common database operations — it plays the same role as Django's `manage.py` / `django-admin`.

```bash
git clone https://github.com/Dera2Salles/Rango.git
cd Rango
cargo install --path cli
```

This installs a `rango` binary on your `$PATH`. Verify it:

```bash
rango --help
```

## Create a project

```bash
rango startproject my_site
cd my_site
```

This generates:

```
my_site/
├── Cargo.toml
├── .env.example        # every config variable, documented
├── migrations/
│   └── 0001_initial.sql
├── src/
│   ├── main.rs          # init_config() + rango::start()
│   ├── models.rs        # #[model] structs
│   └── urls.rs          # top-level routing table
├── templates/
│   ├── base.html
│   └── welcome.html
└── static/
```

## Run it

```bash
cargo run
# or, from anywhere inside the project:
rango runserver
```

By default Rango binds to `127.0.0.1:8000` and serves a SQLite-backed demo `Post` model with automatic migrations. Open `http://127.0.0.1:8000` in your browser.

## Add an app

Django-style "apps" are just Rust modules with their own `models.rs` / `views.rs` / `urls.rs` / templates directory:

```bash
rango startapp blog
```

This creates `src/blog/{mod.rs, models.rs, views.rs, urls.rs}` and `templates/blogs/index.html`. Wire it into your project:

```rust
// src/main.rs
mod blog;
mod models;
mod urls;
```

```rust
// src/urls.rs
use rango::macros::urls;
use crate::blog;

urls!(
    path("/", home),
    include("/blog", blog::urls::get_rango_router),
);
```

## Configure a database

Rango supports SQLite, PostgreSQL, and MySQL/MariaDB through a single `Any`-backed SQLx pool. Pick one in `src/main.rs`:

```rust
use rango::{RangoConfig, DatabaseConfig, init_config};

init_config(RangoConfig {
    database: Some(
        DatabaseConfig::sqlite("rango.db")
            .migrations("./migrations"),   // auto-applies on startup
    ),
    ..RangoConfig::default()
});
```

Or read everything from `.env` (copy `.env.example` to `.env` first):

```rust
init_config(RangoConfig::from_env());
```

See the [Configuration Reference](11-configuration-reference.md) for every available variable and default.

## Project layout at a glance

| Path | Purpose |
|------|---------|
| `src/main.rs` | Entry point: `init_config()` then `rango::start(router).run().await` |
| `src/urls.rs` | Top-level route table (`urls!` macro) |
| `src/models.rs` / `src/<app>/models.rs` | `#[model]` structs |
| `migrations/*.sql` | Numbered SQL migrations, applied via `sqlx::migrate` |
| `templates/` | MiniJinja (`.html`) templates |
| `static/` | Served at `/static/*` |
| `.env` / `.env.example` | Runtime configuration via environment variables |

Next: [Routing & Views →](02-routing-and-views.md)
