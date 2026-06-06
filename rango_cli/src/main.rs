mod command;
mod error;
mod runserver;
mod startapp;
mod startproject;

use clap::Parser;

use crate::command::{Cli, Commands};
use crate::runserver::runserver;
use crate::startapp::startapp;
use crate::startproject::startproject;

fn main() -> Result<(), error::RangoCliError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Startapp { name } => startapp(&name),
        Commands::Startproject { name } => startproject(&name),
        Commands::Runserver { addr } => runserver(&addr),
    }
}
