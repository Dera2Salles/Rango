use crate::error::RangoCliError;
use std::fs;
use std::path::Path;

pub fn startproject(name: &str) -> Result<(), RangoCliError> {
    let project_root = name;

    if Path::new(project_root).exists() {
        return Err(RangoCliError::ProjectAlreadyExist(name.to_string()));
    }

    fs::create_dir_all(format!("{}/src", name)).map_err(RangoCliError::IoError)?;
    fs::create_dir_all(format!("{}/templates", name)).map_err(RangoCliError::IoError)?;
    fs::create_dir_all(format!("{}/static", name)).map_err(RangoCliError::IoError)?;
    fs::create_dir_all(format!("{}/migrations", name)).map_err(RangoCliError::IoError)?;

    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
rango = {{ version = "0.1.0", package = "rango-framework" }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}
"#,
        name
    );
    fs::write(format!("{}/Cargo.toml", name), cargo_toml).map_err(RangoCliError::IoError)?;

    let gitignore = r#"/target
**/*.rs.bk
Cargo.lock

# Environnement
.env
.env.local
"#;
    fs::write(format!("{}/.gitignore", name), gitignore).map_err(RangoCliError::IoError)?;

    let env_example = r#"# Rango Framework Configuration
RANGO_ADDR=127.0.0.1:8000
DATABASE_URL=sqlite://rango.db
RUST_LOG=rango=debug,tower_http=debug
"#;
    fs::write(format!("{}/.env.example", name), env_example).map_err(RangoCliError::IoError)?;

    let main_rs = r#"mod urls;

#[tokio::main]
async fn main() {
    let router = urls::get_rango_router();

    rango::start(router)
        .bind("127.0.0.1:8000")
        .with_static("/static", "./static")
        .with_cors()
        .run()
        .await;
}
"#;
    fs::write(format!("{}/src/main.rs", name), main_rs).map_err(RangoCliError::IoError)?;

    let urls_rs = r#"use rango::macros::{rango_urls, view};

// Declare your views here and use include() for the apps
rango_urls!(
    path("/", home),
);

// View startup inline
#[view]
pub async fn home() {
    "<h1>🤠 Rango works !</h1>"
}
"#;
    fs::write(format!("{}/src/urls.rs", name), urls_rs).map_err(RangoCliError::IoError)?;

    let base_html = r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>{% block title %}Rango{% endblock %}</title>
</head>
<body>
  {% block content %}{% endblock %}
</body>
</html>
"#;
    fs::write(format!("{}/templates/base.html", name), base_html)
        .map_err(RangoCliError::IoError)?;

    println!(" Project '{}' created !", name);
    println!("   cd {}", name);
    println!("   cargo run");

    Ok(())
}
