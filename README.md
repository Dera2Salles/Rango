# Rango 🦀🦎

<p align="center">
    <img src="docs/rango.png" alt="Rango Logo" width="900">
</p>

> Build blazing-fast web applications with zero compromise on developer velocity.

Rango is a lightweight, ergonomic web framework built on top of Axum. It is carefully designed to provide a productive, Django-like development experience in Rust, eliminating boilerplate while maintaining bare-metal performance.

---

## Key Features

- **Django-Inspired Routing ️**: Centralize your URLs in a single, clean file using the `urls!` macro. Support for nested sub-routers via `include` and `path`.
- **Simplified View Handling ️**: Write clean, asynchronous handlers using `#[view]` attributes. Let Rango manage the underlying Axum routing plumbing.
- **On-Demand Database Support ️**: Seamless database integration with compile-time query validation, completely optional and feature-gated.
- **Ergonomic Contexts **: Create view contexts instantly with the `context!` macro for seamless JSON payloads and template rendering.
- **Blazing Fast Compilation 🚀**: Highly modular design. If you don't use the database or templates, they aren't compiled. Keep your binary lightweight.

---

## ️ Project Structure

```text
rango_workspace/
├── rango/           # Core framework library (State, Middleware, Responses)
├── macros/          # Procedural macros (view, urls, context)
├── rango_cli/       # CLI tool for scaffolding projects and apps
└── docs/            # Documentation assets
```

---

## Installation

### 1. Add to your project

Add Rango to your `Cargo.toml` dependencies:

```toml
[dependencies]
rango = {{ version = "0.1.0", package = "rango-framework", features = ["db", "templates"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
sqlx = {{ version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "postgres", "any", "migrate"] }}
tokio = {{ version = "1.0", features = ["full"] }}
```

### 2. Install the CLI

To use the `rango` command-line tool for scaffolding:

```bash
cargo install --path rango_cli
```

---

## Quick Start

1. **Create a new project:**

   ```bash
   rango startproject my_cool_site
   cd my_cool_site
   ```

2. **Run the development server:**
   ```bash
   rango runserver
   ```

---

## Usage Guide

### Centralized Routing

Define your application's URL structure in `src/urls.rs`. Rango supports nesting routes just like Django's `include`.

```rust
use rango::macros::{urls, view};
use crate::{blog, api};

urls!(
    path("/", home_view),
    include("/blog", blog::urls::get_rango_router),
    include("/api", api::urls::get_rango_router),
);

#[view]
pub async fn home_view() {
    rango::render("index.html", rango::json!({
        "title": "Welcome to Rango"
    })).unwrap()
}
```

### Writing Views

Use the `#[view]` macro to turn async functions into valid Rango/Axum handlers. You can use standard Axum extractors like `Path`, `Query`, and `Json`.

```rust
use rango::macros::view;
use rango::responses::json_response;
use rango::{Path, Json};

#[view(method = "POST")]
pub async fn create_post(Json(body): Json<serde_json::Value>) {
    json_response(rango::json!({
        "message": "Post created successfully",
        "data": body
    }))
}
```

### Template Rendering

Rango uses **MiniJinja** for high-performance template rendering. Use the `context!` macro to pass data to your templates easily.

```rust
use rango::macros::{view, context};
use rango::responses::render;

#[view]
pub async fn blog_index() {
    let posts = vec!["Post 1", "Post 2"];
    render("blog/index.html", context! {
        posts => posts,
        author => "Dera"
    }).unwrap()
}
```

### Configuration

Initialize Rango with a configuration struct in your `main.rs`:

```rust
use rango::state::{RangoConfig, init_config};

#[tokio::main]
async fn main() {
    init_config(RangoConfig {
        debug: true,
        templates_dir: "templates".to_string(),
        database_url: Some("sqlite://db.sqlite3".to_string()),
        ..Default::default()
    });

    let router = urls::get_rango_router();
    rango::start(router).run().await;
}
```

### Database Models (Coming soon)

Rango provides a simple `RangoModel` trait for basic CRUD operations (when the `db` feature is enabled).

```rust
use rango::db::RangoModel;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: i64,
    pub title: String,
}

impl RangoModel for Post {
    fn table_name() -> &'static str { "posts" }
}

// In your view:
let posts = Post::all().await.unwrap();
```

### Middleware

Rango supports standard Axum middleware and provides some built-in helpers.

```rust
use rango::middleware::require_auth;
use axum::middleware::from_fn;

// In your router configuration:
let router = urls::get_rango_router()
    .layer(from_fn(require_auth));
```

---

## ️ CLI Commands

- `rango startproject <name>`: Scaffolds a new Rango project structure.
- `rango startapp <name>`: Creates a new "app" module with `views.rs`, `urls.rs`, and a template folder.
- `rango runserver`: Starts the project using `cargo run` with environment defaults.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
