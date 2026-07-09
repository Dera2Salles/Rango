# 4. Database & ORM

Requires the `db` feature. Rango's ORM is built on SQLx's `Any` driver, so the exact same model code runs against **SQLite**, **PostgreSQL**, and **MySQL/MariaDB** — only your connection URL changes.

```toml
rango = { version = "0.1.0", package = "rango-framework", features = ["db", "templates"] }
```

## Defining models

```rust
use rango::macros::model;

#[model]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub published: bool,
}
```

The `#[model]` macro derives, for free:

- `sqlx::FromRow`, `Serialize`, `Deserialize`, `Clone`, `Debug`
- The `RangoModel` trait — CRUD, `QuerySet` access, bulk operations
- The `RangoAdminMetadata` trait — powers the [admin panel](05-admin-panel.md)
- The `RangoSchema` trait — generates `CREATE TABLE` / index SQL for you

### Table & column options

```rust
#[model(table = "blog_posts")]
pub struct Post {
    #[rango(id)]
    pub post_id: i64,

    #[rango(unique)]
    pub slug: String,

    #[rango(index)]
    pub author_id: i64,

    #[rango(nullable)]
    pub subtitle: Option<String>,

    #[rango(default = "0")]
    pub views: i64,

    pub title: String,
}
```

| Attribute | Effect |
|-----------|--------|
| `#[model(table = "...")]` | Override the table name (default: pluralized, lowercased struct name — `Post` → `posts`) |
| `#[rango(id)]` | Marks this field as the primary key (default: a field named `id`) |
| `#[rango(unique)]` | Adds a `UNIQUE` constraint + index in generated migration SQL |
| `#[rango(index)]` | Adds a plain index in generated migration SQL |
| `#[rango(nullable)]` | Column allows `NULL` (pair with `Option<T>` fields) |
| `#[rango(default = "...")]` | SQL-level `DEFAULT` clause |

Supported Rust → SQL type mapping: `String`/`&str` → `TEXT`, integer types → `INTEGER`, `f32`/`f64` → `REAL`, `bool` → `BOOLEAN`, `Vec<u8>` → `BLOB`, `chrono` types → `TIMESTAMP`, `Uuid` → `TEXT`, and `Option<T>` variants of all the above.

## Basic CRUD

```rust
use rango::RangoModel;

// Create
let mut post = Post { id: 0, title: "Hello".into(), body: "...".into(), published: false };
post.save().await?;              // INSERT — id is populated after save()

// Read
let all = Post::all().await?;
let one = Post::get_by_id(post.id).await?;         // Option<Post>
let one = Post::get_or_404(post.id).await?;         // Post, or RangoError::NotFound
let count = Post::count().await?;

// Update
post.title = "Updated".into();
post.save().await?;              // UPDATE (id != 0 ⇒ update, not insert)

// Delete
post.delete().await?;
Post::delete_by_id(42).await?;
```

### Extra `RangoModel` convenience methods

```rust
// Re-fetch the row from the DB and overwrite `self` in place.
post.refresh_from_db().await?;

// Get-or-insert.
let (user, created) = User::get_or_create("email = 'a@b.com'", User { .. }).await?;

// Insert-or-update by a lookup condition (overwrites the matched row's fields).
let (user, created) = User::update_or_create("email = 'a@b.com'", User { .. }).await?;

// Sequential bulk operations (see "Transactions" below for the atomicity caveat).
let created = Post::bulk_create(vec![post1, post2, post3]).await?;
Post::bulk_update(&mut posts).await?;
let deleted = Post::bulk_delete(&[1, 2, 3]).await?;

// Ordering shortcuts.
let newest = Post::latest("created_at").await?;   // Option<Post>, MAX(created_at) first
let oldest = Post::earliest("created_at").await?; // Option<Post>, MIN(created_at) first
```

`bulk_create` / `bulk_update` call `save()` on each item sequentially over the shared pool — they are **not** wrapped in a single database transaction. If you need true atomicity across multiple writes, use [`with_transaction`](#transactions) with raw queries.

## `QuerySet` — the query builder

`Model::objects()` returns a lazy, chainable `QuerySet<Model>`:

```rust
let posts = Post::objects()
    .filter_eq("published", true)
    .order_by_desc("created_at")
    .limit(10)
    .all()
    .await?;
```

### Typed, safely-parameterized filters

Every value passed to these helpers is sent to the database as a **bound parameter** — never interpolated into SQL, so they're always safe with user-supplied input:

```rust
Post::objects().filter_eq("status", "published");
Post::objects().filter_ne("status", "draft");
Post::objects().filter_gt("views", 100);
Post::objects().filter_gte("views", 100);
Post::objects().filter_lt("views", 100);
Post::objects().filter_lte("views", 100);
Post::objects().filter_like("title", "Rust%");           // raw LIKE pattern
Post::objects().filter_icontains("title", "rust");        // case-insensitive contains
Post::objects().filter_in("id", vec![1, 2, 3]);           // WHERE id IN (?, ?, ?)
Post::objects().filter_null("subtitle");                  // WHERE subtitle IS NULL
Post::objects().filter_not_null("subtitle");
Post::objects().filter_between("views", 10, 100);         // inclusive range
```

Chain as many as you like — they combine with `AND`:

```rust
let results = Post::objects()
    .filter_eq("published", true)
    .filter_gt("views", 1000)
    .filter_icontains("title", &search_query)   // `search_query` is user input — safe here
    .order_by_desc("views")
    .all()
    .await?;
```

Any value implementing `Into<SqlValue>` works — `&str`, `String`, `i64`, `i32`, `u32`, `f64`, `f32`, `bool`, and `Option<T>` of those.

### Developer-authored SQL (raw filters)

For SQL fragments **you** control (never user input), use the raw variants:

```rust
Post::objects().filter_raw("status = 'active'");
Post::objects().exclude("status = 'archived'");
```

### `Q` objects — combinable conditions

```rust
use rango::Q;

let q = Q::new("status = 'active'") & Q::new("views > 100");
let q2 = (Q::new("role = 'admin'") | Q::new("role = 'staff'")).not();

let staff_or_admin = User::objects().filter_q(q2).all().await?;
```

`Q::new()` takes developer-authored SQL only — combine with `&` (AND), `|` (OR), `.not()` (negate).

### Ordering, limiting, pagination

```rust
Post::objects().order_by("title");            // ASC
Post::objects().order_by_desc("created_at");  // DESC
Post::objects().limit(20).offset(40);

// Full pagination — returns a `Page<Post>` with items + metadata
let page = Post::objects()
    .filter_eq("published", true)
    .paginate(page_number, 20)
    .await?;

println!("{} of {} pages, {} total", page.page, page.num_pages, page.total);
for post in &page.items { /* ... */ }
```

Combine with [`Paginator`](../rango/src/paginator.rs) to drive pagination widgets in templates:

```rust
use rango::Paginator;

let page = Post::objects().paginate(page_number, 20).await?;
let paginator = Paginator::from_page(&page);
render("blog/index.html", rango::json!({
    "posts": page.items,
    "paginator": paginator.to_json(),
}))
```

### Joins, grouping, distinct

```rust
Post::objects()
    .left_join("users", "users.id = posts.author_id")
    .only("posts.*, users.name AS author_name")
    .filter_eq("posts.published", true)
    .all()
    .await?;

Post::objects().group_by("author_id").having("COUNT(*) > 5");
Post::objects().distinct().values("category").await?;
```

### Aggregates

```rust
let total_views: Option<f64> = Post::objects().filter_eq("published", true).sum("views").await?;
let avg_views: Option<f64>   = Post::objects().avg("views").await?;
let min_views: Option<f64>   = Post::objects().min_of("views").await?;
let max_views: Option<f64>   = Post::objects().max_of("views").await?;
let count: i64               = Post::objects().count().await?;
let any: bool                = Post::objects().filter_eq("published", true).exists().await?;
```

For ad-hoc aggregate SQL you write yourself, use the free functions `rango::db::aggregate()` / `aggregate_float()`.

### Projections: `values()` / `values_list()`

```rust
// Vec<serde_json::Value> — full-row-shaped objects with just the selected columns
let rows = Post::objects().filter_eq("published", true).values("id, title").await?;

// Flat Vec<serde_json::Value> of a single column
let titles = Post::objects().values_list("title").await?;
```

### `first()` / `last()`

```rust
let newest = Post::objects().order_by_desc("created_at").first().await?;
let last_row = Post::objects().last().await?; // implicit "id DESC" if no order_by() set
```

### Raw updates / deletes on a filtered set

```rust
// Developer-authored SET clause; WHERE conditions built via filter_eq/etc. stay parameterized.
Post::objects().filter_eq("author_id", 5).update("published = true").await?;

Post::objects().filter_lt("views", 1).delete().await?;
```

## Transactions

```rust
use rango::db::with_transaction;

with_transaction(|tx| Box::pin(async move {
    sqlx::query("UPDATE accounts SET balance = balance - ? WHERE id = ?")
        .bind(100).bind(1)
        .execute(&mut **tx)
        .await
        .map_err(rango::RangoError::from)?;

    sqlx::query("UPDATE accounts SET balance = balance + ? WHERE id = ?")
        .bind(100).bind(2)
        .execute(&mut **tx)
        .await
        .map_err(rango::RangoError::from)?;

    Ok(())
})).await?;
```

The closure receives `&mut sqlx::Transaction<'static, sqlx::Any>`; the transaction commits automatically if the closure returns `Ok`, and rolls back if it returns `Err`.

## Migrations

Rango uses SQLx's file-based migrator. Migration files live in `migrations/*.sql`, numbered sequentially:

```
migrations/
├── 0001_initial.sql
└── 0002_add_comments.sql
```

Generate the boilerplate for a new model with the [CLI](12-cli-reference.md):

```bash
rango makemigrations create_posts --sql "$(cat schema.sql)"
```

Or use a model's generated schema helpers directly to write the file yourself:

```rust
println!("{}", Post::generate_migration_sql());
for stmt in Post::generate_index_sql() { println!("{}", stmt); }
```

Migrations run automatically on startup when `DatabaseConfig.auto_migrate` is `true` (the default):

```rust
DatabaseConfig::sqlite("rango.db").migrations("./migrations")
```

Or apply them manually:

```bash
rango migrate --database-url sqlite://rango.db
rango showmigrations
```

## Raw SQL escape hatches

When the query builder doesn't fit your needs, drop to raw SQL — still through the shared pool:

```rust
use rango::db::{query, query_as, execute};

execute("DELETE FROM sessions WHERE expires_at < NOW()").await?;

let rows: Vec<(i64, String)> = query_as("SELECT id, title FROM posts WHERE views > ?")
    .bind(1000)
    .fetch_all(rango::db::db()?)
    .await?;
```

## Security notes

- All typed `filter_*` helpers and `filter_param` bind values as real SQL parameters (via a small internal `SqlValue` enum) — they are safe with untrusted/user-supplied input.
- `filter_raw`, `exclude`, `Q::new`, `.update(assignments)`, `.join()`/`.having()`/`.group_by()` take **developer-authored SQL fragments** — never interpolate request data into them directly. Use the typed filters (or bind via `filter_param`) for anything coming from a request.

Next: [Admin Panel →](05-admin-panel.md)
