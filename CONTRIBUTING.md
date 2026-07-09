# Contributing to Rango

Thanks for taking the time to contribute! Rango is a Django-inspired async web framework for Rust, and it grows through community contributions — bug fixes, new features, documentation, and examples are all welcome.

This guide covers everything you need to get a change from "idea" to "merged."

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Project Layout](#project-layout)
- [Development Setup](#development-setup)
- [Building & Testing](#building--testing)
- [Coding Guidelines](#coding-guidelines)
- [Working on Feature-Gated Code](#working-on-feature-gated-code)
- [Documentation](#documentation)
- [Commit & PR Workflow](#commit--pr-workflow)
- [Reporting Bugs](#reporting-bugs)
- [Proposing Features](#proposing-features)
- [Security Issues](#security-issues)
- [License](#license)

## Code of Conduct

Be respectful, constructive, and patient. Assume good faith. Disagreements about design are fine and expected — personal attacks are not. Maintainers may edit, close, or reject issues/PRs that don't meet this bar.

## Project Layout

Rango is a Cargo workspace with three members:

```
Rango/
├── rango/     # The framework library (published as `rango-framework`, lib name `rango`)
│   └── src/
│       ├── db.rs               # ORM: RangoModel, QuerySet, SqlValue, migrations, admin ops
│       ├── admin.rs            # Auto-generated admin panel routes/handlers
│       ├── auth.rs / csrf.rs    # Sessions, password hashing, CSRF (feature: auth)
│       ├── messages.rs         # Flash messages (feature: auth)
│       ├── cache.rs            # In-memory cache
│       ├── forms.rs / validators.rs
│       ├── middleware.rs       # CORS, security headers, rate limiting, host validation
│       ├── responses.rs        # render(), redirects, JSON helpers
│       ├── template_filters.rs # Django-style MiniJinja filters (feature: templates)
│       ├── state.rs            # RangoConfig and friends
│       ├── signals.rs          # Signal<T> / SignalRegistry
│       ├── templates/          # Embedded admin panel templates
│       └── static/              # Embedded 404 / debug pages
├── macros/    # Procedural macros: #[model], #[view], #[login_required], urls!, context!
│   └── src/
├── cli/       # The `rango` CLI binary (package `rango_cli`)
│   └── src/
└── docs/      # User-facing documentation (docs/README.md is the index)
```

If you're not sure where a change belongs, look at how a similar existing feature is implemented first — consistency matters more than personal preference here.

## Development Setup

You need a recent stable Rust toolchain (Rango tracks stable, not nightly):

```bash
rustup update stable
git clone https://github.com/Dera2Salles/Rango.git
cd Rango
cargo build --workspace --all-features
```

To try the CLI against your local checkout without installing it globally:

```bash
cargo run -p rango_cli -- startproject demo
```

## Building & Testing

Rango is feature-gated (`templates` default-on, `db` and `auth` opt-in). **Always test the feature combination relevant to your change**, and ideally the full matrix before opening a PR:

```bash
# Default features only (what CI's basic job runs)
cargo build --workspace
cargo test --workspace

# Everything on — the combination most likely to reveal cfg-gating mistakes
cargo build --workspace --all-features
cargo test --workspace --all-features

# A specific combination, e.g. if your change only touches the ORM
cargo build -p rango-framework --features "db,templates"
```

Doctests are part of the test suite — run `cargo test --all-features` (not just `cargo build`) before submitting. If you add a `rust` code block to a doc comment that's illustrative pseudo-code rather than a real, runnable example (references an undefined type/variable on purpose, for clarity), mark it ` ```rust,ignore ` so it doesn't fail `cargo test`.

Lint before pushing:

```bash
cargo fmt --all
cargo clippy --workspace --all-features -- -D warnings
```

CI (`.github/workflows/rust.yml`) currently runs `cargo build --verbose` and `cargo test --verbose` with default features on every push/PR to `main`. If your change is feature-gated, please also test it locally with `--all-features` since CI doesn't cover that yet — improving CI's feature-matrix coverage is itself a welcome contribution.

## Coding Guidelines

- **Follow existing patterns.** This codebase has clear conventions (module-level `#[cfg(feature = "...")]`, `RangoResult<T>` return types, `tracing::debug!/warn!/error!` for logging, builder-style config structs). New code should look like it belongs.
- **Feature-gate consistently.** If a module depends on `sqlx`, `tower-sessions`, or `minijinja`, gate the whole module (`#[cfg(feature = "db")] pub mod db;` in `lib.rs`) rather than sprinkling `#[cfg]` on individual items where avoidable — that keeps `cargo build` (no features) fast and honest about what's actually optional.
- **Never break the no-feature build.** `cargo build -p rango-framework` (no features at all, or just default `templates`) must always succeed.
- **Security-sensitive code needs a `# Security` doc comment.** See `db.rs`'s `Q`/`filter_raw` docs and `csrf.rs` for the expected tone: state plainly what is/isn't safe with user input.
- **Prefer safe, parameterized APIs over raw SQL/string interpolation.** When adding ORM functionality, favor the `SqlValue`-backed typed filters (`filter_eq`, `filter_in`, etc.) pattern over new raw-SQL-accepting methods. If a raw-SQL method is unavoidable, document it as developer-authored-only, matching `filter_raw`'s doc comment.
- **Errors go through `RangoError`.** Add a new variant in `error.rs` rather than `.unwrap()`/`panic!` in library code (tests and examples are more lenient).
- **Run `cargo fmt`** — no bespoke formatting rules beyond rustfmt's defaults.
- **Keep `Cargo.toml` dependency additions justified.** New dependencies should be `optional = true` behind the relevant feature unless they're needed unconditionally.

## Working on Feature-Gated Code

When adding something behind `db` or `auth`:

1. Add the module/items behind `#[cfg(feature = "...")]` in `rango/src/lib.rs`, matching the existing style (see `messages.rs` for a recent example — session-backed, gated behind `auth`).
2. Re-export the public API from `lib.rs` under the same `#[cfg(...)]`.
3. Build and test with and without the feature enabled (see [Building & Testing](#building--testing)).
4. If your feature touches the `#[model]` macro (`macros/src/model.rs`) or any trait it implements (`RangoModel`, `RangoAdminMetadata`, `RangoSchema`), double-check the generated code by building a project through the `rango` CLI (`rango startproject`) against your local checkout — see the CLI's own `[dependencies]` path override trick, or add a temporary `[patch.crates-io]` / path dependency while testing.

## Documentation

User-facing docs live in [`docs/`](docs/README.md), one topic per file (routing, templates, ORM, admin, forms, auth, messages/cache, middleware, signals, config, CLI). If your change adds or alters public API:

- Update the relevant `docs/*.md` page(s) with a short example.
- Update doc comments (`///`) on the Rust items themselves — these are what `cargo doc` and IDE tooltips show.
- If you're adding an entirely new subsystem, add a new numbered `docs/NN-topic.md` page and link it from `docs/README.md`'s table of contents.
- If your change affects the root `README.md`'s quick-start examples or feature list, update those too — they should never fall out of sync with what actually compiles.

## Commit & PR Workflow

1. Fork the repo and create a branch off `main`: `git checkout -b fix/short-description` or `feat/short-description`.
2. Make focused commits — one logical change per commit is easier to review than one giant diff. Write commit messages in the imperative mood ("Add `filter_in` to QuerySet", not "Added" or "Adds").
3. Before opening the PR:
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-features -- -D warnings`
   - `cargo test --workspace --all-features`
   - Update `docs/` if you touched public API (see above).
4. Open the PR against `main` with:
   - A clear description of **what** changed and **why**.
   - Any breaking changes called out explicitly (e.g. a trait gaining a new required method, like `RangoAdminOps::bulk_delete` did).
   - Screenshots for admin-panel/template changes are appreciated but not required.
5. Be responsive to review feedback — small, iterative fixups are easier to review than force-pushed rewrites once a review is underway.

PRs that only reformat code, rename things without functional change, or reorganize files wholesale are much easier to review (and merge) if kept separate from functional changes.

## Reporting Bugs

Open a GitHub issue with:

- Rango version / commit hash, and which features you enabled (`db`, `templates`, `auth`).
- Minimal reproduction — ideally a few lines of model/view/route code, not your whole app.
- What you expected vs. what happened, including the exact error message/panic output.
- Your database backend (SQLite/PostgreSQL/MySQL) if the bug is ORM-related.

## Proposing Features

Rango's north star is *"Django's productivity, Rust's performance and safety."* When proposing a new feature (especially a new Django-parity feature), briefly describe:

- The Django equivalent (if any) and how closely you intend to mirror its API.
- Which crate/module it belongs in, and which feature flag (if any) should gate it.
- Whether it needs a new `docs/*.md` page.

Small, incremental PRs (e.g. "add `filter_between` to QuerySet") are much easier to merge than large speculative ones — feel free to open an issue to discuss direction before investing in a big patch.

## Security Issues

Please **do not** open a public issue for security vulnerabilities (e.g. SQL injection, auth bypass, CSRF bypass, timing attacks). Instead, contact the maintainer directly (see the repository's GitHub profile for contact details) so a fix can be prepared before public disclosure.

## License

By contributing, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).
