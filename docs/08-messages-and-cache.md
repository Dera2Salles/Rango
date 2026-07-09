# 8. Messages & Caching

## Flash messages (`rango::messages`)

Requires the `auth` feature (messages are stored in the session). Mirrors `django.contrib.messages`: queue a message, redirect, and it's shown exactly once on the next page render.

```rust
use rango::messages;
use tower_sessions::Session;

#[view(method = "POST")]
pub async fn create_post(session: Session, /* ... */) {
    // ... save the post ...
    messages::success(&session, "Post created successfully!").await?;
    rango::redirect("/posts")
}
```

### Levels

```rust
messages::debug(&session, "...").await?;
messages::info(&session, "...").await?;
messages::success(&session, "...").await?;
messages::warning(&session, "...").await?;
messages::error(&session, "...").await?;

// Or generically:
messages::add_message(&session, messages::MessageLevel::Warning, "Careful!").await?;
```

### Displaying messages

Fetch (and clear) pending messages right before rendering — this gives one-shot "flash" semantics:

```rust
#[view]
pub async fn posts_list(session: Session) {
    let messages_json = messages::get_messages_json(&session).await?;
    render("posts/list.html", rango::json!({
        "messages": messages_json,
        // ...
    }))
}
```

```jinja
{% for message in messages %}
  <div class="alert alert-{{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

The admin panel already wires this pattern into `admin_base.html` (`.flash`, `.flash-success`, `.flash-error`, `.flash-warning`, `.flash-info` CSS classes) — copy that pattern into your own base template.

`get_messages()` (non-JSON variant) returns `Vec<Message>` if you'd rather work with the typed struct directly:

```rust
pub struct Message {
    pub level: MessageLevel, // Debug | Info | Success | Warning | Error
    pub text: String,
}
```

## Caching (`rango::cache`)

A lightweight, thread-safe, in-process cache with per-key TTL — no feature flag required, mirrors `django.core.cache.cache`. Good for memoizing expensive computations within a single server process; for multi-instance deployments, back a similar API with Redis instead.

```rust
use rango::cache::cache;
use std::time::Duration;

// Store for 60 seconds
cache().set("home_stats", rango::json!({ "views": 42 }), Some(Duration::from_secs(60)));

// Store forever (until overwritten/cleared)
cache().set("site_config", rango::json!({ "maintenance": false }), None);

if let Some(stats) = cache().get("home_stats") {
    println!("{}", stats);
}

cache().delete("home_stats");
cache().clear();
println!("{} entries cached", cache().len());
```

### Compute-and-cache in one call

```rust
let value = cache().get_or_set("expensive_calc", Duration::from_secs(300), || {
    rango::json!({ "result": expensive_computation() })
});
```

### Counters

```rust
let views = cache().incr("post:42:views", 1); // creates at 0 if absent, returns new value
```

Expired entries are evicted lazily on the next `get()`/`incr()` for that key — there's no background sweeper thread.

Next: [Middleware & Security →](09-middleware-and-security.md)
