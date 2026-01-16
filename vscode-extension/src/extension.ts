/**
 * CAGE VS Code Extension
 *
 * Executes code selections in CAGE sandbox and displays results
 */

import * as vscode from 'vscode';
import axios, { AxiosInstance } from 'axios';

let outputChannel: vscode.OutputChannel;
let cageClient: CAGEClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('CAGE extension activated');

    outputChannel = vscode.window.createOutputChannel('CAGE');
    cageClient = new CAGEClient();

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('cage.executeSelection', executeSelection),
        vscode.commands.registerCommand('cage.executeFile', executeFile),
        vscode.commands.registerCommand('cage.uploadFile', uploadFile),
        vscode.commands.registerCommand('cage.viewWorkspace', viewWorkspace),
        vscode.commands.registerCommand('cage.configure', configure)
    );
}

export function deactivate() {
    outputChannel.dispose();
}

/**
 * Execute selected code in CAGE
 */
async function executeSelection() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const selection = editor.selection;
    const code = editor.document.getText(selection.isEmpty ? undefined : selection);

    if (!code.trim()) {
        vscode.window.showErrorMessage('No code selected');
        return;
    }

    // Detect language
    const language = detectLanguage(editor.document.languageId);

    try {
        vscode.window.showInformationMessage(`Executing ${language} code in CAGE...`);
        outputChannel.show();
        outputChannel.appendLine(`\n${'='.repeat(60)}`);
        outputChannel.appendLine(`Executing ${language} code...`);
        outputChannel.appendLine(`Code length: ${code.length} characters`);
        outputChannel.appendLine(`${'='.repeat(60)}\n`);

        const result = await cageClient.execute(code, language);

        if (result.status === 'success') {
            outputChannel.appendLine('✅ Execution successful\n');
            outputChannel.appendLine(`Duration: ${result.duration_ms}ms`);
            outputChannel.appendLine(`Exit Code: ${result.exit_code}\n`);

            if (result.stdout) {
                outputChannel.appendLine('STDOUT:');
                outputChannel.appendLine(result.stdout);
            }

            if (result.files_created && result.files_created.length > 0) {
                outputChannel.appendLine(`\nFiles created: ${result.files_created.join(', ')}`);
            }

            vscode.window.showInformationMessage('✅ Code executed successfully');
        } else {
            outputChannel.appendLine(`❌ Execution failed: ${result.status}\n`);

            if (result.stderr) {
                outputChannel.appendLine('STDERR:');
                outputChannel.appendLine(result.stderr);
            }

            vscode.window.showErrorMessage(`Execution failed: ${result.status}`);
        }
    } catch (error) {
        outputChannel.appendLine(`\n❌ Error: ${error}`);
        vscode.window.showErrorMessage(`CAGE error: ${error}`);
    }
}

/**
 * Execute entire current file
 */
async function executeFile() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const code = editor.document.getText();
    const language = detectLanguage(editor.document.languageId);

    try {
        vscode.window.showInformationMessage(`Executing file in CAGE...`);
        outputChannel.show();
        outputChannel.appendLine(`\nExecuting ${editor.document.fileName}...`);

        const result = await cageClient.execute(code, language);

        if (result.status === 'success') {
            outputChannel.appendLine('✅ Success\n');
            outputChannel.appendLine(result.stdout);
            vscode.window.showInformationMessage('✅ File executed successfully');
        } else {
            outputChannel.appendLine(`❌ Failed\n${result.stderr}`);
            vscode.window.showErrorMessage(`Execution failed`);
        }
    } catch (error) {
        vscode.window.showErrorMessage(`CAGE error: ${error}`);
    }
}

/**
 * Upload current file to CAGE workspace
 */
async function uploadFile() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    try {
        await editor.document.save();
        const content = Buffer.from(editor.document.getText());
        const filename = vscode.workspace.asRelativePath(editor.document.fileName);

        vscode.window.showInformationMessage(`Uploading ${filename} to CAGE...`);

        await cageClient.uploadFile(filename, content);

        outputChannel.appendLine(`✅ Uploaded: ${filename}`);
        vscode.window.showInformationMessage(`✅ File uploaded successfully`);
    } catch (error) {
        vscode.window.showErrorMessage(`Upload failed: ${error}`);
    }
}

/**
 * View workspace files
 */
async function viewWorkspace() {
    try {
        const files = await cageClient.listFiles();

        const items: vscode.QuickPickItem[] = files.map(f => ({
            label: f.name,
            description: `${f.size_bytes} bytes`,
            detail: f.type === 'directory' ? '📁 Directory' : '📄 File'
        }));

        const selected = await vscode.window.showQuickPick(items, {
            placeHolder: 'CAGE Workspace Files'
        });

        if (selected) {
            // Download and open file
            try {
                const content = await cageClient.downloadFile(selected.label);
                const doc = await vscode.workspace.openTextDocument({
                    content: content.toString(),
                    language: detectLanguageFromFilename(selected.label)
                });
                await vscode.window.showTextDocument(doc);
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to open file: ${error}`);
            }
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Failed to list files: ${error}`);
    }
}

/**
 * Configure CAGE connection
 */
async function configure() {
    const config = vscode.workspace.getConfiguration('cage');

    const apiUrl = await vscode.window.showInputBox({
        prompt: 'CAGE API URL',
        value: config.get('apiUrl', 'http://127.0.0.1:8080'),
        placeHolder: 'http://127.0.0.1:8080'
    });

    const apiKey = await vscode.window.showInputBox({
        prompt: 'API Key',
        value: config.get('apiKey', ''),
        placeHolder: 'dev_your_username',
        password: true
    });

    if (apiUrl) {
        await config.update('apiUrl', apiUrl, vscode.ConfigurationTarget.Global);
    }

    if (apiKey) {
        await config.update('apiKey', apiKey, vscode.ConfigurationTarget.Global);
    }

    cageClient = new CAGEClient();
    vscode.window.showInformationMessage('✅ CAGE configuration updated');
}

/**
 * CAGE API Client
 */
class CAGEClient {
    private axios: AxiosInstance;
    private apiUrl: string;
    private apiKey: string;

    constructor() {
        const config = vscode.workspace.getConfiguration('cage');
        this.apiUrl = config.get('apiUrl', 'http://127.0.0.1:8080');
        this.apiKey = config.get('apiKey', 'dev_vscode');

        this.axios = axios.create({
            baseURL: this.apiUrl,
            headers: {
                'Authorization': `ApiKey ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            timeout: 60000
        });
    }

    async execute(code: string, language: string = 'python') {
        const response = await this.axios.post('/api/v1/execute', {
            code,
            language,
            timeout_seconds: 60
        });
        return response.data;
    }

    async uploadFile(filename: string, content: Buffer) {
        const base64Content = content.toString('base64');
        const response = await this.axios.post('/api/v1/files', {
            filename,
            content: base64Content,
            path: '/'
        });
        return response.data;
    }

    async listFiles() {
        const response = await this.axios.get('/api/v1/files');
        return response.data.files;
    }

    async downloadFile(filename: string): Promise<Buffer> {
        const response = await this.axios.get(`/api/v1/files/${filename}`, {
            responseType: 'arraybuffer'
        });
        return Buffer.from(response.data);
    }
}

/**
 * Map VS Code language ID to CAGE language
 */
function detectLanguage(vscodeLanguageId: string): string {
    const mapping: Record<string, string> = {
        'python': 'python',
        'javascript': 'javascript',
        'typescript': 'typescript',
        'shellscript': 'bash',
        'bash': 'bash',
        'sh': 'bash',
        'r': 'r',
        'julia': 'julia',
        'ruby': 'ruby',
        'go': 'go'
    };

    return mapping[vscodeLanguageId] || 'python';
}

/**
 * Detect language from filename
 */
function detectLanguageFromFilename(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();
    const mapping: Record<string, string> = {
        'py': 'python',
        'js': 'javascript',
        'ts': 'typescript',
        'sh': 'shellscript',
        'r': 'r',
        'jl': 'julia',
        'rb': 'ruby',
        'go': 'go'
    };

    return mapping[ext || ''] || 'plaintext';
}
