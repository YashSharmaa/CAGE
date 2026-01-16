# CAGE Python SDK

Official Python SDK for CAGE (Contained AI-Generated Code Execution).

## Installation

```bash
pip install cage-sdk
```

Or install from source:

```bash
cd sdk/python
pip install -e .
```

## Quick Start

### REST API Client

```python
from cage import CAGEClient

# Create client
client = CAGEClient(
    api_url="http://127.0.0.1:8080",
    api_key="dev_myuser"
)

# Execute Python code
result = client.execute("print('Hello from CAGE!')")
print(result['stdout'])  # Hello from CAGE!

# Execute JavaScript
result = client.execute(
    "console.log(process.version)",
    language="javascript"
)

# Persistent interpreter mode
client.execute("x = 42", persistent=True)
result = client.execute("print(x)", persistent=True)
print(result['stdout'])  # 42

# Upload file
client.upload_file("data.csv", target_path="/")

# Execute code that uses the file
result = client.execute("""
import pandas as pd
df = pd.read_csv('data.csv')
print(df.head())
""")

# Download result file
content = client.download_file("output.png", output_path="./result.png")

# List workspace files
files = client.list_files()
for file in files:
    print(f"{file['name']} - {file['size_bytes']} bytes")
```

### MCP WebSocket Client

```python
import asyncio
from cage import MCPClient

async def main():
    async with MCPClient(user_id="demo") as client:
        # List available tools
        tools = await client.list_tools()
        print(f"Available tools: {[t['name'] for t in tools]}")

        # Execute code
        result = await client.execute_code("print('Hello MCP!')")
        print(result)

        # Persistent mode
        await client.execute_code("x = 100", persistent=True)
        result = await client.execute_code("print(x)", persistent=True)

asyncio.run(main())
```

## API Reference

### CAGEClient

#### `execute(code, language='python', timeout_seconds=None, persistent=False, env=None)`

Execute code in sandbox.

**Parameters:**
- `code` (str): Code to execute
- `language` (str): `python`, `javascript`, `bash`, `r`, `julia`, `typescript`, `ruby`, `go`, `wasm`
- `timeout_seconds` (int, optional): Max execution time
- `persistent` (bool): Use persistent interpreter (Python only)
- `env` (dict, optional): Environment variables

**Returns:** Dict with `stdout`, `stderr`, `exit_code`, `duration_ms`, etc.

#### `upload_file(file_path, target_path='/')`

Upload file to workspace.

#### `download_file(file_path, output_path=None)`

Download file from workspace.

#### `list_files(path='/', recursive=False)`

List workspace files.

#### `execute_async(code, language='python', timeout_seconds=None)`

Submit async job, returns `job_id`.

#### `get_job_status(job_id)`

Get async job status and result.

### MCPClient

#### `execute_code(code, language='python', persistent=False, timeout_seconds=30)`

Execute via MCP WebSocket.

#### `list_files(path='/')`

List files via MCP.

#### `upload_file(filename, content)`

Upload file via MCP (bytes).

## Error Handling

```python
from cage import CAGEClient, ExecutionError, AuthenticationError

client = CAGEClient(api_key="my_key")

try:
    result = client.execute("print('test')")
except AuthenticationError:
    print("Invalid API key")
except ExecutionError as e:
    print(f"Execution failed: {e}")
```

## License

MIT License
