use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rango")]
#[command(about = "🤠 Rango CLI — Framework Django-like for Rust")]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Startapp {
        name: String,
    },
    Startproject {
        name: String,
    },
    Runserver {
        #[arg(default_value = "127.0.0.1:8000")]
        addr: String,
    },
}
