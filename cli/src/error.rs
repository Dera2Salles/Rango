use thiserror::Error;

#[derive(Debug, Error)]
pub enum RangoCliError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Project '{0}' already exists")]
    ProjectAlreadyExist(String),

    #[error("App '{0}' already exists")]
    AppAlreadyExist(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Migrations directory not found: {0}")]
    MigrationsNotFound(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),
}
