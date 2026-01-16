"use strict";
/**
 * CAGE JavaScript/TypeScript SDK
 *
 * Provides REST API and MCP WebSocket clients for CAGE orchestrator
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.MCPClient = exports.CAGEClient = void 0;
class CAGEClient {
    /**
     * Create CAGE REST API client
     *
     * @param baseUrl - Base URL of CAGE orchestrator (default: http://127.0.0.1:8080)
     * @param apiKey - API key for authentication (default: dev_user)
     * @param timeout - Request timeout in milliseconds (default: 60000)
     */
    constructor(baseUrl = 'http://127.0.0.1:8080', apiKey = 'dev_user', timeout = 60000) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.apiKey = apiKey;
        this.timeout = timeout;
    }
    /**
     * Execute code in sandbox
     */
    async execute(request) {
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
        return (await response.json());
    }
    /**
     * Execute code asynchronously (returns job ID immediately)
     */
    async executeAsync(request) {
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
        const result = (await response.json());
        return result.job_id;
    }
    /**
     * Get async job status
     */
    async getJobStatus(jobId) {
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
    async uploadFile(file, filename, targetPath = '/') {
        const formData = new FormData();
        if (file instanceof Buffer) {
            formData.append('file', new Blob([file]), filename);
        }
        else {
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
    async downloadFile(filePath) {
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
    async listFiles(path = '/', recursive = false) {
        const params = new URLSearchParams({ path });
        if (recursive)
            params.set('recursive', 'true');
        const response = await fetch(`${this.baseUrl}/api/v1/files?${params}`, {
            headers: { 'Authorization': `ApiKey ${this.apiKey}` },
        });
        if (!response.ok) {
            throw new Error(`List files failed: ${await response.text()}`);
        }
        const result = (await response.json());
        return result.files;
    }
    /**
     * Delete file from workspace
     */
    async deleteFile(filePath) {
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
    async getSession() {
        const response = await fetch(`${this.baseUrl}/api/v1/session`, {
            headers: { 'Authorization': `ApiKey ${this.apiKey}` },
        });
        if (!response.ok) {
            throw new Error(`Get session failed: ${await response.text()}`);
        }
        return (await response.json());
    }
    /**
     * Terminate session
     */
    async terminateSession(purgeData = false) {
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
    async health() {
        const response = await fetch(`${this.baseUrl}/health`);
        if (!response.ok) {
            throw new Error(`Health check failed: ${await response.text()}`);
        }
        return (await response.json());
    }
}
exports.CAGEClient = CAGEClient;
/**
 * MCP WebSocket Client
 */
class MCPClient {
    constructor(url = 'ws://127.0.0.1:8080/mcp', userId = 'default') {
        this.msgId = 0;
        this.url = url;
        this.userId = userId;
    }
    async connect() {
        const WebSocket = (await Promise.resolve().then(() => __importStar(require('ws')))).default;
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
    async executeCode(code, language = 'python', persistent = false, timeoutSeconds = 30) {
        return await this.callTool('execute_code', {
            code,
            language,
            persistent,
            timeout_seconds: timeoutSeconds,
        });
    }
    async listFiles(path = '/') {
        return await this.callTool('list_files', { path });
    }
    async sendRequest(method, params) {
        this.msgId++;
        const request = {
            jsonrpc: '2.0',
            id: this.msgId,
            method,
            ...(params && { params }),
        };
        this.ws.send(JSON.stringify(request));
        return new Promise((resolve, reject) => {
            this.ws.once('message', (data) => {
                try {
                    resolve(JSON.parse(data));
                }
                catch (e) {
                    reject(e);
                }
            });
        });
    }
    async callTool(toolName, arguments_) {
        return await this.sendRequest('tools/call', {
            name: toolName,
            arguments: arguments_,
        });
    }
    async close() {
        if (this.ws) {
            this.ws.close();
        }
    }
}
exports.MCPClient = MCPClient;
exports.default = CAGEClient;
