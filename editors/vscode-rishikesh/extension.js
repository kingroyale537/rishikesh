// ==============================================================================
// Rishikesh Language Official VS Code Extension Engine
// Provides 1-Click Zero-Config Execution, Formatting, Linting & REPL
// ==============================================================================

const vscode = require('vscode');
const { exec, spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

/**
 * Resolves the path to the `rishi` executable.
 * Checks system PATH, ~/.rishi/bin, and ~/.cargo/bin.
 */
function findRishiBinary() {
    const homeDir = os.homedir();
    const candidatePaths = [
        path.join(homeDir, '.rishi', 'bin', os.platform() === 'win32' ? 'rishi.exe' : 'rishi'),
        path.join(homeDir, '.cargo', 'bin', os.platform() === 'win32' ? 'rishi.exe' : 'rishi'),
        os.platform() === 'win32' ? 'rishi.exe' : 'rishi'
    ];

    for (const p of candidatePaths) {
        if (fs.existsSync(p)) {
            return p;
        }
    }
    return 'rishi'; // Fallback to PATH resolution
}

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    console.log('⚡ Rishikesh Language extension is now active!');

    // 1. Run Active File Command
    const runCommand = vscode.commands.registerCommand('rishikesh.run', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active Rishikesh file to run.');
            return;
        }

        const document = editor.document;
        if (document.languageId !== 'rishikesh' && !document.fileName.endsWith('.rk') && !document.fileName.endsWith('.rishi')) {
            vscode.window.showWarningMessage('Current file is not a Rishikesh script (.rk, .rishi).');
            return;
        }

        // Auto-save before running
        if (document.isDirty) {
            await document.save();
        }

        const filePath = document.fileName;
        const rishiBin = findRishiBinary();

        // Check if terminal exists or create new one
        let terminal = vscode.window.terminals.find(t => t.name === 'Rishikesh Runner');
        if (!terminal) {
            terminal = vscode.window.createTerminal('Rishikesh Runner');
        }

        terminal.show(true);
        terminal.sendText(`"${rishiBin}" run "${filePath}"`);
    });

    // 2. Format Active Document
    const formatCommand = vscode.commands.registerCommand('rishikesh.format', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const rishiBin = findRishiBinary();

        exec(`"${rishiBin}" fmt "${filePath}"`, (err, stdout, stderr) => {
            if (err) {
                vscode.window.showErrorMessage(`Formatting failed: ${stderr || err.message}`);
            } else {
                vscode.window.showInformationMessage(`✓ Formatted ${path.basename(filePath)}`);
            }
        });
    });

    // 3. Lint Active Document
    const lintCommand = vscode.commands.registerCommand('rishikesh.lint', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const rishiBin = findRishiBinary();

        exec(`"${rishiBin}" lint "${filePath}"`, (err, stdout, stderr) => {
            if (err) {
                vscode.window.showWarningMessage(`Lint warnings in ${path.basename(filePath)}: ${stdout}`);
            } else {
                vscode.window.showInformationMessage(`✓ ${path.basename(filePath)} is clean! 0 lint errors.`);
            }
        });
    });

    // 4. Open Interactive REPL
    const replCommand = vscode.commands.registerCommand('rishikesh.repl', () => {
        const rishiBin = findRishiBinary();
        let terminal = vscode.window.createTerminal('Rishikesh REPL');
        terminal.show();
        terminal.sendText(`"${rishiBin}" repl`);
    });

    // 5. Run Profile Breakdown
    const profileCommand = vscode.commands.registerCommand('rishikesh.profile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const rishiBin = findRishiBinary();

        let terminal = vscode.window.createTerminal('Rishikesh Profiler');
        terminal.show();
        terminal.sendText(`"${rishiBin}" profile "${filePath}"`);
    });

    context.subscriptions.push(runCommand, formatCommand, lintCommand, replCommand, profileCommand);
}

function deactivate() {}

module.exports = {
    activate,
    deactivate
};
