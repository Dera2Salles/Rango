use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rango")]
#[command(about = "🤠 Rango CLI — Framework Django-like for Rust")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new Rango project.
    Startproject {
        name: String,
    },
    /// Create a new app inside an existing project.
    Startapp {
        name: String,
    },
    /// Start the development server.
    Runserver {
        #[arg(default_value = "127.0.0.1:8000")]
        addr: String,
    },
    /// Generate a new migration file from a model SQL definition.
    Makemigrations {
        /// Name for the migration (e.g. "create_users").
        name: String,
        /// SQL content for the migration (use --sql or pipe from a file).
        #[arg(long)]
        sql: Option<String>,
        /// Migrations directory (default: ./migrations).
        #[arg(long, default_value = "migrations")]
        dir: String,
    },
    /// Apply all pending migrations.
    Migrate {
        /// Database URL (e.g. sqlite://rango.db or postgres://...).
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
        /// Migrations directory (default: ./migrations).
        #[arg(long, default_value = "migrations")]
        dir: String,
    },
    /// Show the list of migrations and their status.
    ShowMigrations {
        /// Migrations directory (default: ./migrations).
        #[arg(long, default_value = "migrations")]
        dir: String,
    },
    /// Open an interactive SQLite shell.
    Dbshell {
        /// Database URL.
        #[arg(long, env = "DATABASE_URL", default_value = "sqlite://rango.db")]
        database_url: String,
    },
    /// Print the SQL that would be generated for a model (from its migration_sql()).
    /// Usage: provide the SQL directly; future versions will introspect binaries.
    SqlSchema {
        /// Raw SQL to display / validate.
        sql: String,
    },
}
