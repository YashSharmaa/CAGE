//! CAGE CLI - Command-line interface for CAGE orchestrator
//!
//! Provides a simple CLI for executing code, managing files, and sessions

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// CAGE CLI - Execute code safely in isolated sandboxes
#[derive(Parser)]
#[command(name = "cage")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "CLI for CAGE - Contained AI-Generated Code Execution", long_about = None)]
struct Cli {
    /// CAGE API URL
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    api_url: String,

    /// API Key for authentication
    #[arg(short, long, default_value = "dev_cli")]
    key: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute code in a sandbox
    Execute {
        /// Programming language
        #[arg(short, long, default_value = "python")]
        language: String,

        /// Code to execute (or path to file with @ prefix)
        code: String,

        /// Maximum execution time in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Use persistent interpreter mode
        #[arg(short, long)]
        persistent: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Upload a file to workspace
    Upload {
        /// Local file path
        file: PathBuf,

        /// Target path in workspace
        #[arg(short, long, default_value = "/")]
        path: String,
    },

    /// Download a file from workspace
    Download {
        /// File path in workspace
        file: String,

        /// Local output path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List files in workspace
    List {
        /// Path to list
        #[arg(default_value = "/")]
        path: String,
    },

    /// Delete a file from workspace
    Delete {
        /// File path in workspace
        file: String,
    },

    /// Get session information
    Session,

    /// Terminate session
    Terminate {
        /// Also delete all workspace files
        #[arg(long)]
        purge: bool,
    },

    /// Get server health status
    Health,
}

#[derive(Debug, Serialize)]
struct ExecuteRequest {
    language: String,
    code: String,
    timeout_seconds: u64,
    persistent: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExecuteResponse {
    execution_id: String,
    status: String,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    duration_ms: u64,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    active_sessions: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Execute {
            language,
            code,
            timeout,
            persistent,
            output,
        } => {
            // Read code from file if starts with @
            let actual_code = if code.starts_with('@') {
                let file_path = code.trim_start_matches('@');
                fs::read_to_string(file_path)
                    .with_context(|| format!("Failed to read code file: {}", file_path))?
            } else {
                code
            };

            let request = ExecuteRequest {
                language: language.clone(),
                code: actual_code,
                timeout_seconds: timeout,
                persistent,
            };

            let response = client
                .post(format!("{}/api/v1/execute", cli.api_url))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .json(&request)
                .send()
                .context("Failed to send request")?;

            if !response.status().is_success() {
                let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
                anyhow::bail!("Request failed: {}", error_text);
            }

            let result: ExecuteResponse = response.json().context("Failed to parse response")?;

            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{}", "=".repeat(60).bright_blue());
                println!("{} {}", "Execution ID:".bright_cyan(), result.execution_id);
                println!("{} {}", "Status:".bright_cyan(),
                    if result.status == "success" {
                        result.status.green()
                    } else {
                        result.status.red()
                    }
                );
                println!("{} {}ms", "Duration:".bright_cyan(), result.duration_ms);

                if let Some(code) = result.exit_code {
                    println!("{} {}", "Exit Code:".bright_cyan(), code);
                }

                if !result.stdout.is_empty() {
                    println!("\n{}", "STDOUT:".bright_green().bold());
                    println!("{}", result.stdout);
                }

                if !result.stderr.is_empty() {
                    println!("\n{}", "STDERR:".bright_red().bold());
                    println!("{}", result.stderr);
                }

                println!("{}", "=".repeat(60).bright_blue());
            }
        }

        Commands::Upload { file, path } => {
            let filename = file
                .file_name()
                .and_then(|n| n.to_str())
                .context("Invalid filename")?;

            let content = fs::read(&file)
                .with_context(|| format!("Failed to read file: {}", file.display()))?;

            let form = reqwest::blocking::multipart::Form::new()
                .text("path", path)
                .part("file", reqwest::blocking::multipart::Part::bytes(content)
                    .file_name(filename.to_string()));

            let response = client
                .post(format!("{}/api/v1/files", cli.api_url))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .multipart(form)
                .send()
                .context("Failed to upload file")?;

            if !response.status().is_success() {
                let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
                anyhow::bail!("Upload failed: {}", error_text);
            }

            let result: Value = response.json().context("Failed to parse response")?;
            println!("{} {}", "✓ Uploaded:".green(), result["path"].as_str().unwrap_or(filename));
            println!("{} {} bytes", "  Size:".cyan(), result["size_bytes"]);
        }

        Commands::Download { file, output } => {
            let response = client
                .get(format!("{}/api/v1/files/{}", cli.api_url, file))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .send()
                .context("Failed to download file")?;

            if !response.status().is_success() {
                anyhow::bail!("Download failed: {}", response.status());
            }

            let content = response.bytes().context("Failed to read file content")?;

            let output_path = output.unwrap_or_else(|| PathBuf::from(file.split('/').last().unwrap_or(&file)));

            fs::write(&output_path, &content)
                .with_context(|| format!("Failed to write file: {}", output_path.display()))?;

            println!("{} {} ({} bytes)", "✓ Downloaded:".green(), output_path.display(), content.len());
        }

        Commands::List { path } => {
            let response = client
                .get(format!("{}/api/v1/files", cli.api_url))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .query(&[("path", path)])
                .send()
                .context("Failed to list files")?;

            if !response.status().is_success() {
                anyhow::bail!("List failed: {}", response.status());
            }

            let result: Value = response.json().context("Failed to parse response")?;
            let files = result["files"].as_array().context("Invalid response")?;

            println!("{}", "Files in workspace:".bright_cyan().bold());
            for file in files {
                let name = file["name"].as_str().unwrap_or("?");
                let file_type = file["type"].as_str().unwrap_or("file");
                let size = file["size_bytes"].as_u64().unwrap_or(0);

                let icon = if file_type == "directory" { "📁" } else { "📄" };
                println!("  {} {} ({} bytes)", icon, name, size);
            }
        }

        Commands::Delete { file } => {
            let response = client
                .delete(format!("{}/api/v1/files/{}", cli.api_url, file))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .send()
                .context("Failed to delete file")?;

            if !response.status().is_success() {
                anyhow::bail!("Delete failed: {}", response.status());
            }

            println!("{} {}", "✓ Deleted:".green(), file);
        }

        Commands::Session => {
            let response = client
                .get(format!("{}/api/v1/session", cli.api_url))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .send()
                .context("Failed to get session")?;

            if !response.status().is_success() {
                anyhow::bail!("Session request failed: {}", response.status());
            }

            let session: Value = response.json().context("Failed to parse response")?;
            println!("{}", serde_json::to_string_pretty(&session)?);
        }

        Commands::Terminate { purge } => {
            let response = client
                .delete(format!("{}/api/v1/session", cli.api_url))
                .header("Authorization", format!("ApiKey {}", &cli.key))
                .query(&[("purge_data", purge.to_string())])
                .send()
                .context("Failed to terminate session")?;

            if !response.status().is_success() {
                anyhow::bail!("Termination failed: {}", response.status());
            }

            println!("{}", "✓ Session terminated".green());
            if purge {
                println!("  Workspace files purged");
            }
        }

        Commands::Health => {
            let response = client
                .get(format!("{}/health", cli.api_url))
                .send()
                .context("Failed to get health")?;

            if !response.status().is_success() {
                anyhow::bail!("Health check failed: {}", response.status());
            }

            let health: HealthResponse = response.json().context("Failed to parse response")?;

            println!("{}", "CAGE Orchestrator Status".bright_cyan().bold());
            println!("{}", "=".repeat(40).bright_blue());
            println!("{} {}", "Status:".cyan(),
                if health.status == "healthy" {
                    health.status.green()
                } else {
                    health.status.yellow()
                }
            );
            println!("{} {}", "Version:".cyan(), health.version);
            println!("{} {}s", "Uptime:".cyan(), health.uptime_seconds);
            println!("{} {}", "Active Sessions:".cyan(), health.active_sessions);
        }
    }

    Ok(())
}
