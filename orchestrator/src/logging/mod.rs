//! Logging module for CAGE Orchestrator
//!
//! Provides structured logging with tracing, supporting JSON output for production
//! and pretty printing for development.

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize the logging/tracing system
pub fn init_logging(log_level: &str) -> Result<()> {
    use std::fs::OpenOptions;
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    // Parse log level
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // Build filter from level or RUST_LOG env var
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("cage_orchestrator={}", level)));

    // Create log directory if it doesn't exist
    std::fs::create_dir_all("/var/log/cage").unwrap_or_else(|_| {
        std::fs::create_dir_all("./logs").expect("Failed to create logs directory");
    });

    // Open log file for appending
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/var/log/cage/orchestrator.log")
        .or_else(|_| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open("./logs/orchestrator.log")
        })
        .expect("Failed to open log file");

    // Check if we're in a terminal (for pretty printing) or not (for JSON)
    let is_terminal = atty::is(atty::Stream::Stdout);

    if is_terminal {
        // Development: pretty colored output to stdout + JSON to file
        let stdout_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE)
            .pretty()
            .with_writer(std::io::stdout);

        let file_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .json()
            .with_writer(log_file.and(std::io::stderr.with_max_level(Level::ERROR)));

        tracing_subscriber::registry()
            .with(filter)
            .with(stdout_layer)
            .with(file_layer)
            .init();
    } else {
        // Production: JSON output to both stdout and file
        let combined_writer = log_file.and(std::io::stdout);
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .json()
            .with_writer(combined_writer);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }

    Ok(())
}

/// Check if stdout is a terminal
mod atty {
    pub enum Stream {
        Stdout,
    }

    pub fn is(_stream: Stream) -> bool {
        // Simple check using libc
        #[cfg(unix)]
        {
            unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
        }
        #[cfg(windows)]
        {
            // On Windows, check using winapi
            use std::os::windows::io::AsRawHandle;
            let handle = std::io::stdout().as_raw_handle();
            unsafe {
                let mut mode = 0;
                windows_sys::Win32::System::Console::GetConsoleMode(
                    handle as *mut _,
                    &mut mode,
                ) != 0
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }
}
