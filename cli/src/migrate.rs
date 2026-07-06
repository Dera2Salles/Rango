//! Rango migration CLI commands.
//!
//! Commands:
//!   rango makemigrations <name> [--sql "..."] [--dir migrations]
//!   rango migrate --database-url sqlite://rango.db [--dir migrations]
//!   rango showmigrations [--dir migrations]
//!   rango dbshell [--database-url sqlite://rango.db]

use crate::error::RangoCliError;
use std::fs;
use std::path::Path;


/// Create a new numbered migration file in the migrations directory.
pub fn make_migration(
    name: &str,
    sql: Option<String>,
    dir: &str,
) -> Result<(), RangoCliError> {
    fs::create_dir_all(dir).map_err(RangoCliError::IoError)?;

    let next_num = next_migration_number(dir)?;
    let filename = format!("{}/{:04}_{}.sql", dir, next_num, name);

    let is_sql_none = sql.is_none();
    let content = sql.unwrap_or_else(|| {
        format!(
            "-- Migration: {}\n-- Created by: rango makemigrations {}\n-- Edit this file with your SQL\n\n-- Example:\n-- CREATE TABLE my_table (\n--     id INTEGER PRIMARY KEY AUTOINCREMENT,\n--     name TEXT NOT NULL\n-- );\n",
            name, name
        )
    });

    fs::write(&filename, &content).map_err(RangoCliError::IoError)?;

    println!("✅ Migration created: {}", filename);
    if is_sql_none {
        println!("   Edit the file to add your SQL statements.");
    }
    Ok(())
}

fn next_migration_number(dir: &str) -> Result<u32, RangoCliError> {
    let path = Path::new(dir);
    let mut max = 0u32;
    if path.exists() {
        for entry in fs::read_dir(path).map_err(RangoCliError::IoError)? {
            let entry = entry.map_err(RangoCliError::IoError)?;
            let fname = entry.file_name();
            let s = fname.to_string_lossy();
            if s.ends_with(".sql") {
                if let Ok(num) = s[..4].parse::<u32>() {
                    if num > max {
                        max = num;
                    }
                }
            }
        }
    }
    Ok(max + 1)
}


/// Apply all pending migrations using sqlx's migrator.
pub fn run_migrate(database_url: &str, dir: &str) -> Result<(), RangoCliError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        apply_migrations(database_url, dir).await
    })
}

async fn apply_migrations(database_url: &str, dir: &str) -> Result<(), RangoCliError> {
    use sqlx::migrate::Migrator;
    use sqlx::AnyPool;

    sqlx::any::install_default_drivers();

    println!("🗄️  Connecting to: {}", database_url);
    let pool = AnyPool::connect(database_url)
        .await
        .map_err(|e| RangoCliError::DatabaseError(e.to_string()))?;

    let path = Path::new(dir);
    if !path.exists() {
        return Err(RangoCliError::MigrationsNotFound(dir.to_string()));
    }

    println!("📂 Applying migrations from: {}", dir);
    let migrator = Migrator::new(path)
        .await
        .map_err(|e| RangoCliError::DatabaseError(e.to_string()))?;

    migrator
        .run(&pool)
        .await
        .map_err(|e| RangoCliError::DatabaseError(e.to_string()))?;

    println!("✅ All migrations applied successfully.");
    Ok(())
}


pub fn show_migrations(dir: &str) -> Result<(), RangoCliError> {
    let path = Path::new(dir);
    if !path.exists() {
        println!("📂 No migrations directory found at '{}'.", dir);
        return Ok(());
    }

    let mut files: Vec<String> = fs::read_dir(path)
        .map_err(RangoCliError::IoError)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|f| f.ends_with(".sql"))
        .collect();

    files.sort();

    if files.is_empty() {
        println!("📭 No migration files found in '{}'.", dir);
        return Ok(());
    }

    println!("📋 Migrations in '{}':", dir);
    for f in &files {
        println!("   [ ] {}", f);
    }
    println!("");
    println!("Run 'rango migrate --database-url <URL>' to apply pending migrations.");
    Ok(())
}


/// Launch an interactive SQLite shell (requires sqlite3 in PATH).
pub fn dbshell(database_url: &str) -> Result<(), RangoCliError> {
    let db_path = if database_url.starts_with("sqlite://") {
        database_url.trim_start_matches("sqlite://").to_string()
    } else if database_url.starts_with("sqlite:") {
        database_url.trim_start_matches("sqlite:").to_string()
    } else {
        return launch_psql(database_url);
    };

    match std::process::Command::new("sqlite3").arg(&db_path).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Ok(()),
        Err(_) => {
            eprintln!(
                "❌ 'sqlite3' not found in PATH.\n   Install it with: apt install sqlite3 (Linux) or brew install sqlite3 (macOS)"
            );
            Err(RangoCliError::CommandNotFound("sqlite3".to_string()))
        }
    }
}

fn launch_psql(database_url: &str) -> Result<(), RangoCliError> {
    match std::process::Command::new("psql").arg(database_url).status() {
        Ok(_) => Ok(()),
        Err(_) => {
            eprintln!("❌ 'psql' not found in PATH.");
            Err(RangoCliError::CommandNotFound("psql".to_string()))
        }
    }
}
