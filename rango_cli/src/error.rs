use std::fmt;

#[derive(Debug)]
pub enum RangoCliError {
    ProjectAlreadyExist(String),
    AppAlreadyExist(String),
    IoError(std::io::Error),
    RunError,
}

impl fmt::Display for RangoCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangoCliError::ProjectAlreadyExist(name) => {
                write!(
                    f,
                    "The project '{}' already exists in the current directory.",
                    name
                )
            }
            RangoCliError::AppAlreadyExist(name) => {
                write!(
                    f,
                    "The app '{}' already exists in the src/ directory.",
                    name
                )
            }
            RangoCliError::IoError(err) => {
                write!(f, "System Error (I/O): {}", err)
            }
            RangoCliError::RunError => {
                write!(f, "The server stopped unexpectedly.")
            }
        }
    }
}
