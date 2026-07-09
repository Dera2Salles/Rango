# 10. Signals

Rango's `signals` module gives you decoupled event hooks — like Django's `pre_save` / `post_save` / `pre_delete` / `post_delete` signals. No feature flag required.

## Built-in signals

Every model saved/deleted through `#[model]`-generated code fires these automatically, with the model's JSON representation as the payload:

```rust
use rango::signals::{PRE_SAVE, POST_SAVE, PRE_DELETE, POST_DELETE};

POST_SAVE.connect(|value: &serde_json::Value| {
    tracing::info!("Something was saved: {}", value);
});

PRE_DELETE.connect(|value: &serde_json::Value| {
    tracing::warn!("About to delete: {}", value);
});
```

Register your listeners once at startup, before `rango::start()`:

```rust
#[tokio::main]
async fn main() {
    rango::signals::POST_SAVE.connect(|v| {
        // e.g. invalidate a cache entry, send a webhook, write an audit log...
    });

    init_config(RangoConfig::default());
    rango::start(urls::get_rango_router()).run().await;
}
```

There are also two request-lifecycle signals available for you to fire manually if useful in custom middleware: `REQUEST_STARTED` and `REQUEST_FINISHED` (both `Signal<String>`).

## Defining your own signals

```rust
use rango::signals::Signal;

pub static ORDER_PLACED: Signal<Order> = Signal::new();

// Somewhere in your checkout view:
ORDER_PLACED.send(&order);

// Somewhere at startup:
ORDER_PLACED.connect(|order| {
    // send confirmation email, update inventory, etc.
});
```

Signals are synchronous by design (listeners run inline, in registration order, on the thread that calls `.send()`) — keep listeners fast, or spawn a task from inside one if you need to do async work:

```rust
ORDER_PLACED.connect(|order| {
    let order = order.clone();
    tokio::spawn(async move {
        send_confirmation_email(&order).await;
    });
});
```

## `SignalRegistry` — dynamic, string-keyed signals

For cases where the signal name/type isn't known at compile time (e.g. plugin systems), use a registry of JSON-payload channels instead of a typed `static Signal<T>`:

```rust
use rango::signals::SignalRegistry;

let mut registry = SignalRegistry::new();

registry.connect("user.registered", |payload| {
    println!("New user: {}", payload);
});

registry.send("user.registered", rango::json!({ "id": 1, "email": "a@b.com" }));
```

Next: [Configuration Reference →](11-configuration-reference.md)
