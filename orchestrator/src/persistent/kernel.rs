//! Jupyter kernel manager implementation

use std::process::Stdio;
use anyhow::Result;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, info};
use uuid::Uuid;

use super::KernelInfo;

/// Start a Jupyter kernel in a container
pub async fn start_jupyter_kernel(
    container_id: &str,
    user_id: &str,
    podman_path: &str,
) -> Result<KernelInfo> {
    let kernel_id = Uuid::new_v4();
    let base_port = 50000 + (user_id.bytes().sum::<u8>() as u16 * 10);

    info!(
        user_id = %user_id,
        container_id = %container_id,
        "Starting Jupyter kernel"
    );

    // Create kernel connection file
    let connection_file = format!("/tmp/kernel-{}.json", kernel_id);
    let connection_content = format!(
        r#"{{
  "shell_port": {},
  "iopub_port": {},
  "stdin_port": {},
  "control_port": {},
  "hb_port": {},
  "ip": "127.0.0.1",
  "key": "{}",
  "transport": "tcp",
  "signature_scheme": "hmac-sha256",
  "kernel_name": "python3"
}}"#,
        base_port + 1,
        base_port + 2,
        base_port + 3,
        base_port + 4,
        base_port,
        Uuid::new_v4()
    );

    // Write connection file to container
    let write_cmd = format!(
        "echo '{}' > {}",
        connection_content.replace('\'', "'\\''"),
        connection_file
    );

    let output = Command::new(podman_path)
        .args(["exec", container_id, "sh", "-c", &write_cmd])
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("Failed to write kernel connection file");
    }

    // Start Jupyter kernel
    let start_cmd = format!(
        "nohup python -m ipykernel_launcher -f {} > /tmp/kernel.log 2>&1 &",
        connection_file
    );

    let output = Command::new(podman_path)
        .args(["exec", "-d", container_id, "sh", "-c", &start_cmd])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to start kernel: {}", stderr);
    }

    debug!("Jupyter kernel started in container");

    // Give kernel time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(KernelInfo {
        kernel_id,
        user_id: user_id.to_string(),
        container_id: container_id.to_string(),
        kernel_port: base_port,
        shell_port: base_port + 1,
        iopub_port: base_port + 2,
        stdin_port: base_port + 3,
        control_port: base_port + 4,
        key: Uuid::new_v4().to_string(),
    })
}

/// Execute code in a running Jupyter kernel
pub async fn execute_in_kernel(
    kernel_info: &KernelInfo,
    code: &str,
    podman_path: &str,
) -> Result<(String, String)> {
    debug!(
        kernel_id = %kernel_info.kernel_id,
        "Executing code in persistent kernel"
    );

    // For full Jupyter integration, would use ZeroMQ to send execute_request
    // For now, use a simpler approach: python -c with shared namespace file

    // Create a namespace file that persists state
    let namespace_file = format!("/tmp/namespace_{}.py", kernel_info.kernel_id.simple());

    // Wrap code to save/load namespace
    let wrapped_code = format!(
        r#"
import sys
import pickle
import os

namespace_file = '{}'

# Load existing namespace if available
if os.path.exists(namespace_file):
    try:
        with open(namespace_file, 'rb') as f:
            saved_ns = pickle.load(f)
        globals().update(saved_ns)
    except:
        pass

# Execute user code
try:
    exec('''{}''', globals())
except Exception as e:
    print(f'Error: {{e}}', file=sys.stderr)
    raise

# Save namespace (excluding builtins and modules)
save_ns = {{k: v for k, v in globals().items()
           if not k.startswith('_') and
           not isinstance(v, type(sys)) and
           k not in ['sys', 'pickle', 'os']}}

with open(namespace_file, 'wb') as f:
    pickle.dump(save_ns, f)
"#,
        namespace_file,
        code.replace('\\', "\\\\").replace('\'', "\\'")
    );

    // Execute wrapped code
    let temp_file = format!("/tmp/exec_{}.py", Uuid::new_v4().simple());
    let write_cmd = format!(
        "cat > {} << 'EOFPYTHON'\n{}\nEOFPYTHON",
        temp_file, wrapped_code
    );

    let _ = Command::new(podman_path)
        .args(["exec", &kernel_info.container_id, "sh", "-c", &write_cmd])
        .output()
        .await?;

    // Run the code
    let mut child = Command::new(podman_path)
        .args(["exec", &kernel_info.container_id, "python", "-u", &temp_file])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout_handle = child.stdout.take().unwrap();
    let stderr_handle = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout_handle).lines();
    let mut stderr_reader = BufReader::new(stderr_handle).lines();

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    // Read output
    while let Ok(Some(line)) = stdout_reader.next_line().await {
        stdout_lines.push(line);
    }

    while let Ok(Some(line)) = stderr_reader.next_line().await {
        stderr_lines.push(line);
    }

    let _ = child.wait().await?;

    Ok((stdout_lines.join("\n"), stderr_lines.join("\n")))
}
