# Rango 🦀🦎

<p align="center">
    <img src="docs/rango.png" alt="Rango Logo" width="900">
</p>

> Build blazing-fast web applications with zero compromise on developer velocity.

Rango is a lightweight, ergonomic web framework built on top of Axum. It is carefully designed to provide a productive, Django-like development experience in Rust, eliminating boilerplate while maintaining bare-metal performance.

---

## ⚡ Key Features

- **Django-Inspired Routing 🛤️**: Centralize your URLs in a single, clean file using the `urls!` macro. Support for nested sub-routers via `include` and `path`.
- **Simplified View Handling 👁️**: Write clean, asynchronous handlers using `#[view]` attributes. Let Rango manage the underlying Axum routing plombery.
- **On-Demand Database Support 🗄️**: Seamless database integration with compile-time query validation, completely optional and feature-gated.
- **Ergonomic Contexts 🧪**: Create view contexts instantly with the `context!` macro for seamless JSON payloads and template rendering.
- **Blazing Fast Compilation 🚀**: Highly modular design. If you don't use the database or templates, they aren't compiled. Keep your binary lightweight.

---

## Project Structure

Coming Soon

## License

This project is licensed under the MIT License.
