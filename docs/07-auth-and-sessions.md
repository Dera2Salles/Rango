# 7. Auth & Sessions

Requires the `auth` feature, which pulls in `tower-sessions`, `argon2`, `rand`, `subtle`, and `hex`:

```toml
rango = { version = "0.1.0", package = "rango-framework", features = ["auth", "db", "templates"] }
```

Sessions are wired up automatically by `rango::start()` — no extra setup beyond `RangoConfig.session` (see [Configuration Reference](11-configuration-reference.md)). Every handler can extract `tower_sessions::Session` as an Axum argument.

## Password hashing

Rango uses **Argon2id** with OWASP-recommended parameters (64 MB memory cost, 2 iterations, 1 degree of parallelism):

```rust
use rango::auth::{hash_password, verify_password};

let hash = hash_password("correct horse battery staple")?;
// store `hash` in your `User` model...

let ok = verify_password("correct horse battery staple", &hash); // bool
```

`verify_password` uses constant-time comparison internally and returns `false` (never panics/errors) for malformed hashes.

## Login / logout

```rust
use rango::auth;
use tower_sessions::Session;

#[view(method = "POST")]
pub async fn login(session: Session, /* form data */) {
    // ... look up user, verify_password(...) ...
    auth::login(&session, user.id).await?;   // rotates the session ID (prevents fixation)
    rango::redirect("/dashboard")
}

#[view(method = "POST")]
pub async fn logout(session: Session) {
    auth::logout(&session).await?;           // flushes the session entirely
    rango::redirect("/")
}
```

Store arbitrary serializable user data alongside the ID if you want to avoid a DB round-trip on every request:

```rust
auth::login_with_data(&session, user.id, &UserSummary { name: user.name.clone() }).await?;

let data: Option<serde_json::Value> = auth::get_user_data(&session).await?;
```

## Checking authentication

```rust
let user_id: Option<i64> = auth::get_user_id(&session).await?;
let logged_in: bool = auth::is_authenticated(&session).await;
```

### Protecting routes

`require_auth` is a ready-made Axum middleware — apply it to a whole router or a specific route:

```rust
use axum::middleware::from_fn;
use rango::middleware::require_auth;

let protected = Router::new()
    .route("/dashboard", get(dashboard))
    .layer(from_fn(require_auth));
```

Without the `auth` feature, `require_auth` instead checks for a `Bearer` token in the `Authorization` header — handy for API-only crates that skip sessions entirely.

> `#[login_required]` (the macro) currently only generates the same routing boilerplate as `#[view]` — actual enforcement is done by the `require_auth` middleware shown above, applied at the router/route level. See [Routing & Views](02-routing-and-views.md#login_required).

## Password strength

```rust
use rango::auth::validate_password_strength;

if let Err(msg) = validate_password_strength(&new_password) {
    return rango::responses::bad_request(&msg);
}
```

(Same rules as `Validator::password_strength` / `Form::validate_password()` — 8+ characters, upper+lower+digit.)

## CSRF protection

Rango implements the Synchronizer Token Pattern: a random 256-bit token is stored in the session and must be echoed back on state-changing requests.

```rust
use rango::csrf;

#[view]
pub async fn show_form(session: Session) {
    let token = csrf::get_csrf_token(&session).await?;
    render("form.html", rango::json!({ "csrf_token": token }))
}
```

```jinja
<form method="POST">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  ...
</form>
```

Apply the CSRF middleware to enforce it on `POST`/`PUT`/`PATCH`/`DELETE` requests (checks the `X-CSRF-Token` header):

```rust
use rango::middleware::csrf_middleware;

let router = urls::get_rango_router().layer(from_fn(csrf_middleware));
```

```rust
// Regenerate the token after login/logout to prevent CSRF token fixation:
csrf::regenerate_csrf_token(&session).await?;
```

Token comparison uses `subtle::ConstantTimeEq` to avoid timing attacks.

## Random tokens

```rust
let api_key = rango::auth::generate_token(32); // 32 cryptographically-random hex chars
```

Next: [Messages & Caching →](08-messages-and-cache.md)
