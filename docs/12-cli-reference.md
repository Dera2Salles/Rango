# 12. CLI Reference

Install once from the workspace root:

```bash
cargo install --path cli
```

This builds and installs the `rango` binary (package `rango_cli`).

## `rango startproject <name>`

Scaffolds a new project directory named `<name>` with a working SQLite-backed demo app (a `Post` model, one migration, a home view, and base templates). See [Getting Started](01-getting-started.md) for the full generated layout.

```bash
rango startproject my_site
```

## `rango startapp <name>`

Adds an app module to the **current** project (run from your project root): `src/<name>/{mod.rs, models.rs, views.rs, urls.rs}` plus a `templates/<name>s/index.html`.

```bash
rango startapp blog
```

After running it, wire the app into your project as printed by the command:

```rust
// src/main.rs
mod blog;
```

```rust
// src/urls.rs
use crate::blog;
urls!(
    include("/blog", blog::urls::get_rango_router),
);
```

## `rango runserver [addr]`

Runs `cargo run` with sensible defaults (`RANGO_ADDR`, `RUST_LOG=rango=debug,tower_http=debug`). Defaults to `127.0.0.1:8000`.

```bash
rango runserver
rango runserver 0.0.0.0:3000
```

## `rango makemigrations <name> [--sql "..."] [--dir migrations]`

Creates a new, numbered migration file (`NNNN_<name>.sql`) in the migrations directory. Pass `--sql` to write content immediately, or leave it out to get a commented placeholder to fill in by hand.

```bash
rango makemigrations create_posts --sql "CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL);"
rango makemigrations add_comments_table
```

## `rango migrate --database-url <url> [--dir migrations]`

Applies every pending migration in the directory using SQLx's migrator.

```bash
rango migrate --database-url sqlite://rango.db
rango migrate --database-url postgres://user:pass@localhost/mydb --dir migrations
```

`--database-url` also reads from the `DATABASE_URL` environment variable if omitted.

## `rango showmigrations [--dir migrations]`

Lists every `.sql` file found in the migrations directory.

```bash
rango showmigrations
```

## `rango dbshell [--database-url <url>]`

Opens an interactive database shell — `sqlite3` for SQLite URLs, `psql` for everything else. Both must already be installed and on your `$PATH`.

```bash
rango dbshell --database-url sqlite://rango.db
rango dbshell --database-url postgres://user:pass@localhost/mydb
```

## `rango sql-schema <sql>`

Echoes back the given SQL — a placeholder command reserved for future binary-introspection support (printing a model's generated schema without writing a Rust harness). Currently equivalent to `echo`.

```bash
rango sql-schema "CREATE TABLE demo (id INTEGER PRIMARY KEY);"
```

---

Back to the [documentation index](README.md).
