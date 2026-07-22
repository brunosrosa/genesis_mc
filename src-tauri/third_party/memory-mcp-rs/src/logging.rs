use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Transport mode for MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// stdio transport - for local MCP clients
    Stdio,
}

/// Initialize logging based on transport mode
pub fn init_logging(
    _mode: TransportMode,
    log_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL: stdio NEVER logs to stderr unless --log is explicitly enabled
    // Any stderr output during handshake causes "connection closed" in MCP clients
    if let Some(filename) = log_file {
        init_file_logging(filename)?;
    }
    Ok(())
}

/// Console-only logging (stderr)
fn init_console_logging() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
    Ok(())
}

/// File-only logging
fn init_file_logging(filename: String) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)?;

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer().with_writer(file).with_ansi(false))
        .init();
    Ok(())
}

/// Dual logging: both console (stderr) and file
fn init_dual_logging(filename: String) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)?;

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_writer(file).with_ansi(false))
        .init();
    Ok(())
}
