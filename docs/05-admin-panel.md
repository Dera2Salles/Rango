# 5. Admin Panel

Requires both `db` and `templates` features. Register your `#[model]` structs and get a themed, functional CRUD UI for free — like Django's `django.contrib.admin`.

## Setup

```rust
use rango::RangoAdmin;
use crate::models::{Post, Comment};

#[tokio::main]
async fn main() {
    // ... init_config(...) ...

    let mut admin = RangoAdmin::new();
    admin.register::<Post>();
    admin.register::<Comment>();

    let router = urls::get_rango_router()
        .nest("/admin", admin.router());

    rango::start(router).run().await;
}
```

Visit `/admin` for the dashboard, `/admin/post` for the `Post` list, `/admin/post/add` to create a new one, `/admin/post/:id` to edit.

## What you get

- **Dashboard** (`/admin`) — a card per registered model with its live record count.
- **List view** (`/admin/<model>`) — a searchable, paginated, sortable-by-column table.
- **Add / Edit form** (`/admin/<model>/add`, `/admin/<model>/:id`) — auto-generated from your struct's fields, with the right input type per Rust type (`checkbox` for `bool`, `number` for integers/floats, `text` otherwise).
- **Delete** — per-row delete button, and multi-row **bulk delete** via checkboxes.
- **Search** — `?q=...` on the list URL performs a case-insensitive substring match across every field of every row.
- **Pagination** — `?page=N&per_page=M` (default 25/page, max 200/page).

## Search

The list view's search box performs a server-side, case-insensitive substring match. By default (`RangoAdminOps::search`), it scans **every field** of every row (via each model's JSON representation) — no per-model configuration needed:

```
GET /admin/post?q=rust
```

For very large tables you may want to override search with a real `WHERE ... LIKE` query — implement your own `RangoAdminOps` (see below) instead of using `ModelAdmin<T>` directly.

## Pagination

The list view calls `RangoAdminOps::list_paginated(page, per_page, query)` under the hood, which:

- delegates to `QuerySet::paginate()` (an efficient `LIMIT`/`OFFSET` + `COUNT(*)` pair) when there's no search term, or
- performs the search first, then paginates the in-memory results, when there is one.

```
GET /admin/post?page=2&per_page=50
```

## Bulk delete

Check the row checkboxes (or "select all" in the header), then **Delete selected**. This posts a comma-separated list of primary keys to `/admin/<model>/bulk-delete`, which calls `Model::bulk_delete(&ids)` — a single parameterized `DELETE ... WHERE id IN (...)` query.

## Customizing the admin

`RangoAdmin::register::<T>()` wraps your model in a generic `ModelAdmin<T>`, which implements `RangoAdminOps` using your model's `RangoModel` + `RangoAdminMetadata` impls (both derived by `#[model]`). If you need custom behavior (e.g. a smarter search, computed columns, permission checks), implement `RangoAdminOps` yourself and register an `Arc<dyn RangoAdminOps>`:

```rust
use rango::db::{RangoAdminOps, AdminField};
use std::sync::Arc;

struct PostAdmin;

#[rango::axum::async_trait]
impl RangoAdminOps for PostAdmin {
    fn model_name(&self) -> &'static str { "Post" }
    fn fields(&self) -> Vec<AdminField> { Post::fields() }
    // ... list, list_paginated, get, save, delete, bulk_delete, search ...
}

let mut admin = RangoAdmin::new();
admin.models.push(Arc::new(PostAdmin));
```

## Theming

The admin panel's HTML/CSS lives in [`rango/src/templates/admin_*.html`](../rango/src/templates/) and is compiled into the framework binary (no filesystem dependency at runtime). It ships with:

- A dark, glassmorphism-styled dashboard and sidebar
- Flash message support (`.flash-success`, `.flash-error`, `.flash-warning`, `.flash-info`) if you pass a `messages` context key (see [Messages](08-messages-and-cache.md))
- Django-style template filters available (`intcomma`, `pluralize`, etc. — see [Templates](03-templates.md))

There's currently no user-facing theming API — if you need a fully custom admin UI, build your own views against the ORM directly (the admin panel is just regular Axum routes + MiniJinja templates internally, nothing magic).

Next: [Forms & Validation →](06-forms-and-validation.md)
