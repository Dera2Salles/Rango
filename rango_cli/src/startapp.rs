use crate::error::RangoCliError;
use std::fs;
use std::path::Path;

pub fn startapp(name: &str) -> Result<(), RangoCliError> {
    let app_dir = format!("src/{}", name);

    if Path::new(&app_dir).exists() {
        eprintln!("❌ App '{}' already exists in src/{}", name, name);
        return Err(RangoCliError::AppAlreadyExist(name.to_string()));
    }

    fs::create_dir_all(&app_dir).map_err(RangoCliError::IoError)?;

    let views = format!(
        r#"use rango::macros::{{view, context}};
use rango::responses::render;

#[view]
pub async fn index() {{
    render("{}s/index.html", context! {{
        app_name => "{}"
    }}).unwrap()
}}
"#,
        name, name
    );
    fs::write(format!("{}/views.rs", app_dir), views).map_err(RangoCliError::IoError)?;

    let urls = format!(
        r#"use rango::macros::urls;
use crate::{}::views;

urls!(
    path("/", views::index),
);
"#,
        name
    );
    fs::write(format!("{}/urls.rs", app_dir), urls).map_err(RangoCliError::IoError)?;

    let mod_rs = "pub mod views;\npub mod urls;\n";
    fs::write(format!("{}/mod.rs", app_dir), mod_rs).map_err(RangoCliError::IoError)?;

    let tmpl_dir = format!("templates/{}s", name);
    fs::create_dir_all(&tmpl_dir).map_err(RangoCliError::IoError)?;

    let tmpl_index = format!(
        r#"<!DOCTYPE html>
<html>
<head><title>{{ app_name }}</title></head>
<body>
  <h1>🤠 App : {{ app_name }}</h1>
  <p>Welcome in app <strong>{}</strong> !</p>
</body>
</html>
"#,
        name
    );
    fs::write(format!("{}/index.html", tmpl_dir), tmpl_index).map_err(RangoCliError::IoError)?;

    println!("✅ App '{}' created :", name);
    println!("   src/{}/views.rs", name);
    println!("   src/{}/urls.rs", name);
    println!("   src/{}/mod.rs", name);
    println!("   templates/{}s/index.html", name);
    println!("");
    println!("👉 Add to src/main.rs :");
    println!("   mod {};", name);
    println!("👉 Add to src/urls.rs :");
    println!(
        "   include(\"/{}\", {}::urls::get_rango_router),",
        name, name
    );

    Ok(())
}
