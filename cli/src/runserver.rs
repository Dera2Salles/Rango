use crate::error::RangoCliError;
use std::process::Command;

pub fn runserver(addr: &str) -> Result<(), RangoCliError> {
    println!("🤠 Running rango server on http://{}", addr);
    println!("    (Ctrl+C to stop)\n");

    let mut child = Command::new("cargo")
        .args(["run"])
        .env("RANGO_ADDR", addr)
        .env("RUST_LOG", "rango=debug,tower_http=debug")
        .spawn()
        .map_err(RangoCliError::IoError)?;

    let status = child.wait().map_err(RangoCliError::IoError)?;

    if status.success() {
        Ok(())
    } else {
        Err(RangoCliError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Server process exited with non-zero status",
        )))
    }
}
