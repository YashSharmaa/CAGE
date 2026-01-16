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
export declare class CAGEClient {
    private baseUrl;
    private apiKey;
    private timeout;
    /**
     * Create CAGE REST API client
     *
     * @param baseUrl - Base URL of CAGE orchestrator (default: http://127.0.0.1:8080)
     * @param apiKey - API key for authentication (default: dev_user)
     * @param timeout - Request timeout in milliseconds (default: 60000)
     */
    constructor(baseUrl?: string, apiKey?: string, timeout?: number);
    /**
     * Execute code in sandbox
     */
    execute(request: ExecuteRequest): Promise<ExecuteResponse>;
    /**
     * Execute code asynchronously (returns job ID immediately)
     */
    executeAsync(request: ExecuteRequest): Promise<string>;
    /**
     * Get async job status
     */
    getJobStatus(jobId: string): Promise<any>;
    /**
     * Upload file to workspace
     */
    uploadFile(file: File | Buffer, filename: string, targetPath?: string): Promise<any>;
    /**
     * Download file from workspace
     */
    downloadFile(filePath: string): Promise<ArrayBuffer>;
    /**
     * List files in workspace
     */
    listFiles(path?: string, recursive?: boolean): Promise<FileInfo[]>;
    /**
     * Delete file from workspace
     */
    deleteFile(filePath: string): Promise<void>;
    /**
     * Get session information
     */
    getSession(): Promise<SessionInfo>;
    /**
     * Terminate session
     */
    terminateSession(purgeData?: boolean): Promise<void>;
    /**
     * Get server health
     */
    health(): Promise<HealthResponse>;
}
/**
 * MCP WebSocket Client
 */
export declare class MCPClient {
    private url;
    private userId;
    private ws;
    private msgId;
    constructor(url?: string, userId?: string);
    connect(): Promise<void>;
    executeCode(code: string, language?: string, persistent?: boolean, timeoutSeconds?: number): Promise<any>;
    listFiles(path?: string): Promise<any>;
    private sendRequest;
    private callTool;
    close(): Promise<void>;
}
export default CAGEClient;
