# 2. Routing & Views

Rango centralizes routes in one file per app, similarly to Django's `urls.py`, instead of scattering `.route()` calls across your codebase.

## The `urls!` macro

```rust
use rango::macros::{urls, view};
use crate::{blog, api};

urls!(
    path("/", home_view),
    path("/about", about_view),
    include("/blog", blog::urls::get_rango_router),
    include("/api", api::urls::get_rango_router),
);
```

- `path("/route", view_fn)` — maps a URL to a view function (must be annotated with `#[view]`).
- `include("/prefix", router_fn)` — nests another router (usually another module's `get_rango_router`) under a prefix, exactly like Django's `include()`.

`urls!` expands to a `pub fn get_rango_router() -> rango::axum::Router` — mount it from `main.rs`:

```rust
mod urls;

#[tokio::main]
async fn main() {
    rango::start(urls::get_rango_router()).run().await;
}
```

Axum path parameter syntax works as usual inside route strings, e.g. `path("/posts/:id", post_detail)`.

## Writing views with `#[view]`

`#[view]` turns an async function into a valid Axum handler and generates the routing glue `urls!` needs (a hidden `<fn>_meta()` function returning a `MethodRouter`).

```rust
use rango::macros::view;
use rango::{Path, Json};

#[view]
pub async fn home() {
    rango::render("index.html", rango::json!({ "title": "Welcome" }))
}

#[view(method = "POST")]
pub async fn create_post(Json(body): Json<serde_json::Value>) {
    rango::responses::json_response(rango::json!({ "created": body }))
}

#[view(method = "GET,POST")]
pub async fn contact_form(/* ... */) {
    // handles both GET (show form) and POST (process it) in one handler
}
```

Rules:

- With **no** `method` argument, the view responds to **both `GET` and `POST`**.
- `method = "POST"` (or any comma-separated list of `GET,POST,PUT,PATCH,DELETE`) restricts which HTTP verbs are routed to the function.
- Standard Axum extractors work unmodified: `Path`, `Query`, `Json`, `Form`, `State`, `Extension`, etc. — Rango re-exports the common ones (`rango::Path`, `rango::Query`, `rango::Json`, `rango::State`).
- The function body is any expression that implements `IntoResponse` — return `rango::render(...)`, `rango::responses::json_response(...)`, `rango::redirect(...)`, a `Result<impl IntoResponse, RangoError>`, etc.

### `#[login_required]`

```rust
use rango::macros::login_required;

#[login_required]
pub async fn dashboard(/* ... */) {
    rango::render("dashboard.html", rango::json!({}))
}
```

`#[login_required]` currently expands to the same routing boilerplate as `#[view]` — it does **not**, by itself, block unauthenticated requests. Pair it with the `require_auth` middleware (see [Auth & Sessions](07-auth-and-sessions.md)) on the routes (or router) that need protection:

```rust
use axum::middleware::from_fn;
use rango::middleware::require_auth;

let router = urls::get_rango_router()
    .layer(from_fn(require_auth));
```

## Passing data to templates with `context!`

`context!` is sugar over `serde_json::json!` for readable template contexts:

```rust
use rango::macros::context;
use rango::responses::render;

#[view]
pub async fn blog_index() {
    let posts = vec!["Post 1", "Post 2"];
    render("blog/index.html", context! {
        posts => posts,
        author => "Dera",
    }).unwrap()
}
```

This is exactly equivalent to `rango::json!({ "posts": posts, "author": "Dera" })`.

## Nested apps

Each "app" module exposes its own `get_rango_router()`, so apps compose like Django's `include()`:

```rust
// src/blog/urls.rs
use rango::macros::urls;
use crate::blog::views;

urls!(
    path("/", views::index),
    path("/:id", views::detail),
);
```

```rust
// src/urls.rs
use rango::macros::urls;
use crate::blog;

urls!(
    include("/blog", blog::urls::get_rango_router),
);
```

A request to `/blog/42` is routed to `blog::views::detail`.

Next: [Templates →](03-templates.md)
