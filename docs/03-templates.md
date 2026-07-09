# 3. Templates

Rango renders server-side templates with [MiniJinja](https://github.com/mitsuhiko/minijinja), a Rust implementation of the Jinja2/Django template language. Requires the `templates` feature (enabled by default).

## Rendering

```rust
use rango::responses::render;

#[view]
pub async fn home() {
    render("index.html", rango::json!({
        "title": "Welcome to Rango",
        "posts": posts,
    })).unwrap()
}
```

- Templates are loaded from `RangoConfig.templates_dir` (default: `"templates"`).
- **Debug builds** reload templates from disk on every request (live reload while you edit).
- **Release builds** bundle templates into the binary at compile time (via `minijinja::include_source_bundle!`) and cache the environment — zero filesystem access at runtime.
- `render()` returns `Result<Response, RangoError>` (or panics if `.unwrap()`'d) — errors become a friendly HTML error page in debug mode ([see error handling](11-configuration-reference.md)), or a generic message in production.

## Template syntax

Standard Jinja2/Django-style syntax:

```jinja
{% extends "base.html" %}

{% block content %}
<h1>{{ title }}</h1>

<ul>
{% for post in posts %}
  <li>{{ post.title }} — {{ post.body|truncatewords(20) }}</li>
{% else %}
  <li>No posts yet.</li>
{% endfor %}
</ul>
{% endblock %}
```

MiniJinja ships with Jinja's usual built-ins (`{% if %}`, `{% for %}`, `{% extends %}`/`{% block %}`, `{% include %}`, `{% macro %}`, filters like `upper`, `lower`, `length`, `default`, `join`, `first`, `last`, `sort`, `dictsort`, `escape`/`safe`, etc.) — see the [MiniJinja filter docs](https://docs.rs/minijinja/latest/minijinja/filters/index.html) for the full built-in list.

## Django-style filters (Rango extensions)

Rango registers a handful of extra filters/functions inspired by `django.template.defaultfilters`, available in **every** template (including the admin panel):

| Filter | Example | Output |
|--------|---------|--------|
| `pluralize` | `{{ count }} item{{ count\|pluralize }}` | `"1 item"`, `"3 items"` |
| `pluralize("es")` | `{{ count }} box{{ count\|pluralize("es") }}` | `"1 box"`, `"3 boxes"` |
| `pluralize("y,ies")` | `{{ count }} categor{{ count\|pluralize("y,ies") }}` | `"1 category"`, `"3 categories"` |
| `truncatewords(n)` | `{{ body\|truncatewords(15) }}` | first 15 words + `…` |
| `yesno` | `{{ is_published\|yesno }}` | `"yes"` / `"no"` |
| `yesno("Live", "Draft")` | `{{ is_published\|yesno("Live", "Draft") }}` | `"Live"` / `"Draft"` |
| `intcomma` | `{{ 1234567\|intcomma }}` | `"1,234,567"` |
| `linebreaks` | `{{ comment\|linebreaks }}` | HTML-escaped text wrapped in `<p>`/`<br>` |
| `timesince` | `{{ created_at_unix\|timesince }}` | `"3 hours"`, `"2 days"`, ... |
| `default_if_none(fallback)` | `{{ bio\|default_if_none("No bio yet") }}` | explicit Django-style alias of MiniJinja's `default` |

Global function:

- `now()` — current Unix timestamp (seconds), handy combined with `timesince`: `{{ now()|timesince }}`.

These are registered in [`rango::template_filters`](../rango/src/template_filters.rs); if you need custom filters of your own, register them the same way on your own MiniJinja environment (or open an issue/PR to add more Django-isms upstream).

## Static files

Configure a static directory (default: `"static"`, served at `/static/*`):

```rust
init_config(RangoConfig {
    static_dir: Some("static".to_string()),
    ..RangoConfig::default()
});
```

```jinja
<link rel="stylesheet" href="/static/css/site.css">
```

Override the mount point/directory per-app with the builder:

```rust
rango::start(router)
    .with_static("/assets", "public")
    .run()
    .await;
```

## Flash messages in templates

If you're using the [messages framework](08-messages-and-cache.md) (`auth` feature), pass the queued messages into your base template context once per request:

```rust
render("base.html", rango::json!({
    "messages": rango::messages::get_messages_json(&session).await?,
}))
```

```jinja
{% for message in messages %}
  <div class="alert alert-{{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

The admin panel does exactly this out of the box — see its `admin_base.html` for a styled example (`.flash`, `.flash-success`, `.flash-error`, ...).

## Error pages

Rango ships built-in, styled error pages so you get something reasonable even before writing your own:

- **404** — `rango/src/static/404.html`, used automatically for any unmatched route. If your project defines a `templates/404.html`, it takes priority.
- **500 / debug page** — in `debug = true` mode, unhandled server errors render a detailed diagnostic page (request method/URI/headers, error message, Rango version, OS/arch) instead of a bare 500. In production (`debug = false`) a generic error page is shown and details are only logged server-side (see [`RangoError`](../rango/src/error.rs)).

Next: [Database & ORM →](04-database-orm.md)
