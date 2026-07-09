# 6. Forms & Validation

Rango's `forms` and `validators` modules provide Django's `forms.Form` ergonomics — no feature flag required, works everywhere.

## `Form` — validating request data

```rust
use rango::forms::Form;
use axum::extract::Form as AxumForm;
use std::collections::HashMap;

#[view(method = "POST")]
pub async fn register(AxumForm(data): AxumForm<HashMap<String, String>>) {
    let mut form = Form::new(data);

    let username = form.field("username")
        .required()
        .min_length(3)
        .max_length(150)
        .validate_username()
        .get();

    let email = form.field("email")
        .required()
        .validate_email()
        .get();

    let password = form.field("password")
        .required()
        .validate_password()
        .get();

    form.field("password_confirm")
        .required()
        .matches_field("password");

    if !form.is_valid() {
        return rango::responses::bad_request(&form.errors().to_string());
    }

    // username / email / password are `Option<String>` — safe to unwrap after is_valid()
    rango::responses::json_response(rango::json!({ "created": username }))
}
```

You can also build a `Form` straight from a JSON body:

```rust
let form = Form::from_json(&json_value);
```

### `FieldBuilder` methods

Chain any combination — every call that fails records an error against that field but doesn't stop the chain:

| Method | Checks |
|--------|--------|
| `.required()` | non-empty (after trimming) |
| `.min_length(n)` / `.max_length(n)` | string length |
| `.validate_email()` | basic `local@domain.tld` shape |
| `.validate_url()` | starts with `http://` or `https://` |
| `.validate_username()` | 3–150 chars, alphanumeric + `_`/`-` |
| `.validate_password()` | 8+ chars, upper+lower+digit |
| `.matches_field(other)` | equals another field's raw value (e.g. password confirmation) |
| `.custom(\|value\| Result<(), String>)` | your own validation function |

Terminal methods (consume the builder):

| Method | Returns |
|--------|---------|
| `.get()` | `Option<String>` — `None` if empty |
| `.get_or(default)` | `String`, falling back to `default` if empty |
| `.parse::<T>()` | `Option<T>` via `T: FromStr` |

### Reading errors

```rust
form.is_valid();               // bool
form.errors();                 // &ValidationErrors
form.errors().to_json();       // serde_json::Value — great for JSON APIs
form.errors().into_error();    // RangoError::ValidationError(...)
```

## `Validator` — standalone field checks

Use these directly (e.g. inside `#[model]` business logic, or your own form-like abstractions) without going through `Form`:

```rust
use rango::validators::{Validator, ValidationErrors};

let mut errors = ValidationErrors::new();

if let Err(msg) = Validator::email("not-an-email") {
    errors.add("email", &msg);
}
if let Err(msg) = Validator::min_length(&password, 8) {
    errors.add("password", &msg);
}

if !errors.is_empty() {
    return Err(errors.into_error());
}
```

### Available validators

**Strings:** `required`, `min_length`, `max_length`, `exact_length`, `alphanumeric`, `ascii`, `username`, `password_strength`, `matches(value, other, field_name)`

**Format:** `email`, `url`

**Numeric:** `min_value(v, min)`, `max_value(v, max)`, `range(v, min, max)` — generic over any `PartialOrd + Display`

**Files:** `file_extension(filename, &["jpg", "png"])`, `max_file_size(bytes, max_bytes)`

All validators return `Result<(), String>` where the `Err` is a ready-to-display message.

Next: [Auth & Sessions →](07-auth-and-sessions.md)
