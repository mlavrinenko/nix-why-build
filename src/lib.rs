use thiserror::Error;

/// Errors that can occur in this library.
#[derive(Debug, Error)]
pub enum AppError {
    /// An I/O operation failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// Greets the given name.
///
/// # Errors
///
/// Returns [`AppError::Io`] if writing to stdout fails.
pub fn greet(name: &str) -> Result<(), AppError> {
    use std::io::Write;
    log::info!("greeting {name}");
    writeln!(std::io::stdout(), "Hello from {name}!")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        // greet writes to stdout; just ensure it doesn't error
        greet("test").expect("greet should succeed");
    }
}
