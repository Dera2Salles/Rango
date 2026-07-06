mod command;
mod error;
mod migrate;
mod runserver;
mod startapp;
mod startproject;

use clap::Parser;

use crate::command::{Cli, Commands};
use crate::migrate::{make_migration, run_migrate, show_migrations, dbshell};
use crate::runserver::runserver;
use crate::startapp::startapp;
use crate::startproject::startproject;

fn main() -> Result<(), error::RangoCliError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Startapp { name } => startapp(&name),
        Commands::Startproject { name } => startproject(&name),
        Commands::Runserver { addr } => runserver(&addr),
        Commands::Makemigrations { name, sql, dir } => make_migration(&name, sql, &dir),
        Commands::Migrate { database_url, dir } => run_migrate(&database_url, &dir),
        Commands::ShowMigrations { dir } => show_migrations(&dir),
        Commands::Dbshell { database_url } => dbshell(&database_url),
        Commands::SqlSchema { sql } => {
            println!("{}", sql);
            Ok(())
        }
    }
}
