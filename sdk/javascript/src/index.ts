/**
 * CAGE JavaScript/TypeScript SDK
 *
 * Provides REST API and MCP WebSocket clients for CAGE orchestrator
 */

export interface ExecuteRequest {
  code: string;
  language?: 'python' | 'javascript' | 'bash' | 'r' | 'julia' | 'typescript' | 'ruby' | 'go' | 'wasm';
  timeout_seconds?: number;
  persistent?: boolean;
  env?: Record<string, string>;
}

export interface ExecuteResponse {
  execution_id: string;
  status: 'success' | 'error' | 'timeout' | 'killed';
  stdout: string;
  stderr: string;
  exit_code?: number;
  duration_ms: number;
  files_created?: string[];
  resource_usage?: ResourceUsage;
}

export interface ResourceUsage {
  cpu_percent: number;
  memory_mb: number;
  disk_mb: number;
  pids: number;
}

export interface HealthResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version: string;
  uptime_seconds: number;
  active_sessions: number;
  podman_version?: string;
}

export interface SessionInfo {
  session_id: string;
  user_id: string;
  container_id?: string;
  status: string;
  created_at: string;
  last_activity: string;
}

export interface FileInfo {
  name: string;
  path: string;
  type: 'file' | 'directory';
  size_bytes: number;
  modified_at: string;
}

export class CAGEClient {
  private baseUrl: string;
  private apiKey: string;
  private timeout: number;

  /**
   * Create CAGE REST API client
   *
   * @param baseUrl - Base URL of CAGE orchestrator (default: http://127.0.0.1:8080)
   * @param apiKey - API key for authentication (default: dev_user)
   * @param timeout - Request timeout in milliseconds (default: 60000)
   */
  constructor(
    baseUrl: string = 'http://127.0.0.1:8080',
    apiKey: string = 'dev_user',
    timeout: number = 60000
  ) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.apiKey = apiKey;
    this.timeout = timeout;
  }

  /**
   * Execute code in sandbox
   */
  async execute(request: ExecuteRequest): Promise<ExecuteResponse> {
    const response = await fetch(`${this.baseUrl}/api/v1/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `ApiKey ${this.apiKey}`,
      },
      body: JSON.stringify({
        language: request.language || 'python',
        code: request.code,
        timeout_seconds: request.timeout_seconds || 30,
        persistent: request.persistent || false,
        env: request.env || {},
      }),
      signal: AbortSignal.timeout(this.timeout),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Execution failed: ${error}`);
    }

    return (await response.json()) as ExecuteResponse;
  }

  /**
   * Execute code asynchronously (returns job ID immediately)
   */
  async executeAsync(request: ExecuteRequest): Promise<string> {
    const response = await fetch(`${this.baseUrl}/api/v1/execute/async`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `ApiKey ${this.apiKey}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Async execution failed: ${await response.text()}`);
    }

    const result = (await response.json()) as { job_id: string };
    return result.job_id;
  }

  /**
   * Get async job status
   */
  async getJobStatus(jobId: string): Promise<any> {
    const response = await fetch(`${this.baseUrl}/api/v1/jobs/${jobId}`, {
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
    });

    if (!response.ok) {
      throw new Error(`Failed to get job status: ${await response.text()}`);
    }

    return await response.json();
  }

  /**
   * Upload file to workspace
   */
  async uploadFile(file: File | Buffer, filename: string, targetPath: string = '/'): Promise<any> {
    const formData = new FormData();

    if (file instanceof Buffer) {
      formData.append('file', new Blob([file]), filename);
    } else {
      formData.append('file', file);
    }

    formData.append('path', targetPath);

    const response = await fetch(`${this.baseUrl}/api/v1/files`, {
      method: 'POST',
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
      body: formData,
    });

    if (!response.ok) {
      throw new Error(`Upload failed: ${await response.text()}`);
    }

    return await response.json();
  }

  /**
   * Download file from workspace
   */
  async downloadFile(filePath: string): Promise<ArrayBuffer> {
    const response = await fetch(`${this.baseUrl}/api/v1/files/${filePath}`, {
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
    });

    if (!response.ok) {
      throw new Error(`Download failed: ${await response.text()}`);
    }

    return await response.arrayBuffer();
  }

  /**
   * List files in workspace
   */
  async listFiles(path: string = '/', recursive: boolean = false): Promise<FileInfo[]> {
    const params = new URLSearchParams({ path });
    if (recursive) params.set('recursive', 'true');

    const response = await fetch(`${this.baseUrl}/api/v1/files?${params}`, {
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
    });

    if (!response.ok) {
      throw new Error(`List files failed: ${await response.text()}`);
    }

    const result = (await response.json()) as { files: FileInfo[] };
    return result.files;
  }

  /**
   * Delete file from workspace
   */
  async deleteFile(filePath: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/v1/files/${filePath}`, {
      method: 'DELETE',
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
    });

    if (!response.ok) {
      throw new Error(`Delete failed: ${await response.text()}`);
    }
  }

  /**
   * Get session information
   */
  async getSession(): Promise<SessionInfo> {
    const response = await fetch(`${this.baseUrl}/api/v1/session`, {
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
    });

    if (!response.ok) {
      throw new Error(`Get session failed: ${await response.text()}`);
    }

    return (await response.json()) as SessionInfo;
  }

  /**
   * Terminate session
   */
  async terminateSession(purgeData: boolean = false): Promise<void> {
    const params = new URLSearchParams({ purge_data: purgeData.toString() });

    const response = await fetch(`${this.baseUrl}/api/v1/session?${params}`, {
      method: 'DELETE',
      headers: { 'Authorization': `ApiKey ${this.apiKey}` },
    });

    if (!response.ok) {
      throw new Error(`Terminate failed: ${await response.text()}`);
    }
  }

  /**
   * Get server health
   */
  async health(): Promise<HealthResponse> {
    const response = await fetch(`${this.baseUrl}/health`);

    if (!response.ok) {
      throw new Error(`Health check failed: ${await response.text()}`);
    }

    return (await response.json()) as HealthResponse;
  }
}

/**
 * MCP WebSocket Client
 */
export class MCPClient {
  private url: string;
  private userId: string;
  private ws: any; // WebSocket
  private msgId: number = 0;

  constructor(url: string = 'ws://127.0.0.1:8080/mcp', userId: string = 'default') {
    this.url = url;
    this.userId = userId;
  }

  async connect(): Promise<void> {
    const WebSocket = (await import('ws')).default;
    this.ws = new WebSocket(this.url);

    await new Promise((resolve, reject) => {
      this.ws.on('open', resolve);
      this.ws.on('error', reject);
    });

    // Initialize
    const initResp = await this.sendRequest('initialize', {
      user_id: this.userId,
      clientInfo: { name: 'CAGE JS SDK', version: '1.0.0' },
    });

    if (initResp.error) {
      throw new Error(`Initialize failed: ${initResp.error.message}`);
    }
  }

  async executeCode(
    code: string,
    language: string = 'python',
    persistent: boolean = false,
    timeoutSeconds: number = 30
  ): Promise<any> {
    return await this.callTool('execute_code', {
      code,
      language,
      persistent,
      timeout_seconds: timeoutSeconds,
    });
  }

  async listFiles(path: string = '/'): Promise<any> {
    return await this.callTool('list_files', { path });
  }

  private async sendRequest(method: string, params?: any): Promise<any> {
    this.msgId++;
    const request = {
      jsonrpc: '2.0',
      id: this.msgId,
      method,
      ...(params && { params }),
    };

    this.ws.send(JSON.stringify(request));

    return new Promise((resolve, reject) => {
      this.ws.once('message', (data: string) => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          reject(e);
        }
      });
    });
  }

  private async callTool(toolName: string, arguments_: any): Promise<any> {
    return await this.sendRequest('tools/call', {
      name: toolName,
      arguments: arguments_,
    });
  }

  async close(): Promise<void> {
    if (this.ws) {
      this.ws.close();
    }
  }
}

export default CAGEClient;
