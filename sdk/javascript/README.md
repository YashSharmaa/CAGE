# CAGE JavaScript/TypeScript SDK

Official JavaScript/TypeScript SDK for CAGE (Contained AI-Generated Code Execution).

## Installation

```bash
npm install @cage/sdk
```

## Quick Start

### TypeScript

```typescript
import { CAGEClient } from '@cage/sdk';

const client = new CAGEClient(
  'http://127.0.0.1:8080',
  'dev_myuser'
);

// Execute Python code
const result = await client.execute({
  code: "print('Hello from CAGE!')",
  language: 'python'
});

console.log(result.stdout);  // Hello from CAGE!

// Execute JavaScript
const jsResult = await client.execute({
  code: "console.log(process.version)",
  language: 'javascript'
});

// Persistent mode
await client.execute({
  code: "x = 42",
  persistent: true
});

const persistResult = await client.execute({
  code: "print(x)",
  persistent: true
});

console.log(persistResult.stdout);  // 42
```

### JavaScript (CommonJS)

```javascript
const { CAGEClient } = require('@cage/sdk');

const client = new CAGEClient();

client.execute({ code: "print('Hello')" })
  .then(result => console.log(result.stdout))
  .catch(error => console.error(error));
```

### MCP WebSocket Client

```typescript
import { MCPClient } from '@cage/sdk';

const client = new MCPClient('ws://127.0.0.1:8080/mcp', 'demo_user');

await client.connect();

const tools = await client.listTools();
console.log('Available tools:', tools);

const result = await client.executeCode("print('Hello MCP!')");
console.log(result);

await client.close();
```

## API Reference

### CAGEClient

#### `constructor(baseUrl?, apiKey?, timeout?)`

Create new client instance.

#### `execute(request: ExecuteRequest): Promise<ExecuteResponse>`

Execute code in sandbox.

**Request:**
```typescript
{
  code: string;
  language?: 'python' | 'javascript' | 'bash' | 'r' | 'julia' | 'typescript' | 'ruby' | 'go' | 'wasm';
  timeout_seconds?: number;
  persistent?: boolean;
  env?: Record<string, string>;
}
```

#### `uploadFile(file: File | Buffer, filename: string, targetPath?: string)`

Upload file to workspace.

#### `downloadFile(filePath: string): Promise<ArrayBuffer>`

Download file from workspace.

#### `listFiles(path?: string, recursive?: boolean): Promise<FileInfo[]>`

List workspace files.

#### `deleteFile(filePath: string): Promise<void>`

Delete file from workspace.

#### `getSession(): Promise<SessionInfo>`

Get session information.

#### `terminateSession(purgeData?: boolean): Promise<void>`

Terminate session.

#### `health(): Promise<HealthResponse>`

Get server health.

### MCPClient

#### `connect(): Promise<void>`

Connect to MCP WebSocket.

#### `executeCode(code, language?, persistent?, timeoutSeconds?): Promise<any>`

Execute code via MCP.

#### `listFiles(path?): Promise<any>`

List files via MCP.

#### `close(): Promise<void>`

Close WebSocket connection.

## Examples

### File Upload and Processing

```typescript
import fs from 'fs';
import { CAGEClient } from '@cage/sdk';

const client = new CAGEClient();

// Upload CSV
const fileBuffer = fs.readFileSync('data.csv');
await client.uploadFile(fileBuffer, 'data.csv');

// Process it
const result = await client.execute({
  code: `
import pandas as pd
df = pd.read_csv('data.csv')
print(df.describe())
  `
});

console.log(result.stdout);
```

### Async Execution

```typescript
// Submit long-running job
const jobId = await client.executeAsync({
  code: "import time; time.sleep(10); print('Done')"
});

// Poll for result
let status;
do {
  await new Promise(r => setTimeout(r, 1000));
  status = await client.getJobStatus(jobId);
} while (status.status === 'queued' || status.status === 'running');

console.log(status.result.stdout);
```

## License

MIT License
