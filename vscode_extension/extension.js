const vscode = require('vscode');
const http = require('http');
const fs = require('fs');
const path = require('path');

let server = null;
let sockServer = null;
let sockPath = null;
const PORT = 49152;

function getRequestHandler() {
    return async (req, res) => {
        try {
            const parsed = new URL(req.url, `http://127.0.0.1:${PORT}`);
            res.setHeader('Content-Type', 'application/json');

            if (parsed.pathname === '/ping') {
                res.writeHead(200);
                res.end(JSON.stringify({ status: 'ok', app: 'algodeck-vscode-bridge' }));
                return;
            }

            if (parsed.pathname === '/kill') {
                try {
                    // 1. Wyślij SIGINT (Ctrl+C) do terminali AlgoDeck
                    for (const term of vscode.window.terminals) {
                        if (term.name.includes('AlgoDeck') || term.name.includes('ODPAL') || term.name.includes('TESTUJ')) {
                            term.sendText('\x03', false);
                        }
                    }
                    // 2. Anuluj aktywne zadania VS Code
                    for (const execution of vscode.tasks.taskExecutions) {
                        if (execution.task && execution.task.name && execution.task.name.includes('AlgoDeck')) {
                            execution.terminate();
                        }
                    }
                    res.writeHead(200);
                    res.end(JSON.stringify({ status: 'ok', action: 'kill' }));
                } catch (err) {
                    res.writeHead(500);
                    res.end(JSON.stringify({ status: 'error', error: String(err) }));
                }
                return;
            }

            if (parsed.pathname === '/test') {
                try {
                    const pdir = parsed.searchParams.get('pdir');
                    if (pdir) {
                        const term = vscode.window.terminals.find(t => t.name === 'AlgoDeck' || t.name === 'AlgoDeck: TESTUJ')
                            || vscode.window.createTerminal('AlgoDeck: TESTUJ');
                        term.show(true);
                        term.sendText('\x03', false);
                        term.sendText(`cd "${pdir}" && bash .algo/test.sh`);
                    } else {
                        let executed = false;
                        try {
                            const allTasks = await vscode.tasks.fetchTasks();
                            const match = allTasks.find(t => t.name === 'AlgoDeck: TESTUJ' || (t.name && t.name.includes('TESTUJ')));
                            if (match) {
                                await vscode.tasks.executeTask(match);
                                executed = true;
                            }
                        } catch (e) {
                            console.error('[AlgoDeck Bridge] fetchTasks error:', e);
                        }

                        if (!executed) {
                            await vscode.commands.executeCommand('workbench.action.tasks.test');
                        }
                    }

                    res.writeHead(200);
                    res.end(JSON.stringify({ status: 'ok', action: 'test' }));
                } catch (err) {
                    res.writeHead(500);
                    res.end(JSON.stringify({ status: 'error', error: String(err) }));
                }
                return;
            }

            if (parsed.pathname === '/run') {
                try {
                    const pdir = parsed.searchParams.get('pdir');
                    if (pdir) {
                        const term = vscode.window.terminals.find(t => t.name === 'AlgoDeck' || t.name === 'AlgoDeck: ODPAL')
                            || vscode.window.createTerminal('AlgoDeck: ODPAL');
                        term.show(true);
                        term.sendText('\x03', false);
                        term.sendText(`cd "${pdir}" && bash .algo/run.sh`);
                    } else {
                        let executed = false;
                        try {
                            const allTasks = await vscode.tasks.fetchTasks();
                            const match = allTasks.find(t => t.name === 'AlgoDeck: ODPAL' || (t.name && t.name.includes('ODPAL')));
                            if (match) {
                                await vscode.tasks.executeTask(match);
                                executed = true;
                            }
                        } catch (e) {
                            console.error('[AlgoDeck Bridge] fetchTasks error:', e);
                        }

                        if (!executed) {
                            await vscode.commands.executeCommand('workbench.action.tasks.build');
                        }
                    }

                    res.writeHead(200);
                    res.end(JSON.stringify({ status: 'ok', action: 'run' }));
                } catch (err) {
                    res.writeHead(500);
                    res.end(JSON.stringify({ status: 'error', error: String(err) }));
                }
                return;
            }

            res.writeHead(404);
            res.end(JSON.stringify({ status: 'not_found' }));
        } catch (globalErr) {
            try {
                res.writeHead(500);
                res.end(JSON.stringify({ status: 'exception', error: String(globalErr) }));
            } catch (_) {}
        }
    };
}

function activate(context) {
    const handler = getRequestHandler();

    // 1. Serwer HTTP na stałym porcie (wieloplatformowy: macOS, Linux, Windows)
    server = http.createServer(handler);
    server.on('error', (err) => {
        console.log('[AlgoDeck Bridge] HTTP port zajęty (inne okno VS Code):', err.message);
    });
    server.listen(PORT, '127.0.0.1', () => {
        console.log(`[AlgoDeck Bridge] Nasłuchiwanie na 127.0.0.1:${PORT}`);
    });

    // 2. Dedykowany UNIX domain socket dla każdego okna/workspace (macOS i Linux)
    if (process.platform !== 'win32') {
        const folders = vscode.workspace.workspaceFolders;
        if (folders && folders.length > 0) {
            const wsName = folders[0].name.toLowerCase();
            sockPath = `/tmp/algodeck_vscode_${wsName}.sock`;
        } else {
            sockPath = `/tmp/algodeck_vscode_default.sock`;
        }

        if (sockPath) {
            if (fs.existsSync(sockPath)) {
                try { fs.unlinkSync(sockPath); } catch (_) {}
            }
            sockServer = http.createServer(handler);
            sockServer.on('error', (err) => {
                console.log('[AlgoDeck Bridge] UNIX socket error:', err.message);
            });
            sockServer.listen(sockPath, () => {
                console.log(`[AlgoDeck Bridge] Nasłuchiwanie na gnieździe UNIX: ${sockPath}`);
            });
        }
    }

    context.subscriptions.push({
        dispose: () => {
            if (server) {
                try { server.close(); } catch (_) {}
            }
            if (sockServer) {
                try { sockServer.close(); } catch (_) {}
            }
            if (sockPath && fs.existsSync(sockPath)) {
                try { fs.unlinkSync(sockPath); } catch (_) {}
            }
        }
    });
}

function deactivate() {
    if (server) {
        try { server.close(); } catch (_) {}
    }
    if (sockServer) {
        try { sockServer.close(); } catch (_) {}
    }
    if (sockPath && fs.existsSync(sockPath)) {
        try { fs.unlinkSync(sockPath); } catch (_) {}
    }
}

module.exports = {
    activate,
    deactivate
};
