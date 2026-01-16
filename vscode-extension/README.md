# CAGE Code Executor for VS Code

Execute code selections directly in CAGE sandbox from VS Code.

## Features

- **Execute Selection** - Run highlighted code in CAGE (`Cmd+Shift+E` / `Ctrl+Shift+E`)
- **Execute File** - Run entire file in CAGE
- **Upload File** - Upload current file to CAGE workspace
- **View Workspace** - Browse CAGE workspace files
- **Multi-Language** - Python, JavaScript, TypeScript, Bash, R, Julia, Ruby, Go

## Installation

1. Install from VSIX:
```bash
cd vscode-extension
npm install
npm run compile
vsce package
code --install-extension cage-code-executor-1.0.0.vsix
```

2. Or publish to marketplace:
```bash
vsce publish
```

## Configuration

Open VS Code settings and configure:

- **CAGE: API URL** - CAGE orchestrator URL (default: `http://127.0.0.1:8080`)
- **CAGE: API Key** - Your API key (default: `dev_vscode`)

Or use Command Palette: `CAGE: Configure Connection`

## Usage

### Execute Selection

1. Select code in editor
2. Press `Cmd+Shift+E` (Mac) or `Ctrl+Shift+E` (Windows/Linux)
3. View output in CAGE output panel

### Execute File

1. Open any supported file
2. Command Palette → `CAGE: Execute Current File`
3. View results

### Upload File

1. Open file to upload
2. Command Palette → `CAGE: Upload File to Workspace`
3. File uploaded to CAGE workspace

### View Workspace

1. Command Palette → `CAGE: View Workspace Files`
2. Select file to view/download

## Keyboard Shortcuts

| Command | Shortcut (Mac) | Shortcut (Win/Linux) |
|---------|---------------|---------------------|
| Execute Selection | `Cmd+Shift+E` | `Ctrl+Shift+E` |

## Requirements

- CAGE orchestrator running on http://127.0.0.1:8080 or configured URL
- Valid API key

## Extension Settings

This extension contributes the following settings:

* `cage.apiUrl`: CAGE orchestrator API URL
* `cage.apiKey`: API key for authentication

## Known Issues

- File downloads create temporary documents (not saved to disk)
- Large file uploads may timeout
- WebSocket streaming not yet supported

## Release Notes

### 1.0.0

Initial release:
- Execute code selections
- File upload/download
- Workspace browser
- 9 language support

## License

MIT
