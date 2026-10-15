const vscode = require('vscode');
const http = require('http');
const fs = require('fs');
const path = require('path');

let server = null;
let sockServer = null;
let sockPath = null;
const PORT = 49152;

function forwardToSocket(targetSock, req, res) {
    return new Promise((resolve) => {
        const client = http.request({
            socketPath: targetSock,
            path: req.url,
            method: req.method,
            headers: req.headers
        }, (remoteRes) => {
            res.writeHead(remoteRes.statusCode, remoteRes.headers);
            remoteRes.pipe(res);
            remoteRes.on('end', resolve);
        });
        client.on('error', (err) => {
            res.writeHead(502);
            res.end(JSON.stringify({ status: 'bad_gateway', error: err.message }));
            resolve();
        });
        req.pipe(client);
    });
}

function createAlgoTask(name, scriptName, pdir) {
    const wsFolder = (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0)
        ? vscode.workspace.workspaceFolders[0]
        : vscode.TaskScope.Global;

    const cmd = pdir ? `cd "${pdir}" && bash .algo/${scriptName}` : `bash .algo/${scriptName}`;
    const execution = new vscode.ShellExecution(cmd);
    const task = new vscode.Task(
        { type: 'shell', task: name },
        wsFolder,
        name,
        'AlgoDeck',
        execution
    );
    task.presentationOptions = {
        reveal: vscode.TaskRevealKind.Always,
        panel: vscode.TaskPanelKind.Shared,
        focus: true,
        clear: true
    };
    return task;
}

function getRequestHandler() {
    return async (req, res) => {
        try {
            const parsed = new URL(req.url, `http://127.0.0.1:${PORT}`);
            res.setHeader('Content-Type', 'application/json');

            if (parsed.pathname === '/ping') {
                const qProb = parsed.searchParams.get('problem_id');
                if (qProb) {
                    const qLower = qProb.toLowerCase();
                    let isOpen = false;

                    // 1. Sprawdź foldery otwarte w workspace
                    if (vscode.workspace.workspaceFolders) {
                        for (const f of vscode.workspace.workspaceFolders) {
                            const fName = f.name.toLowerCase();
                            const fPath = f.uri.fsPath.toLowerCase();
                            if (fName === qLower || fPath.endsWith(qLower) || fPath.includes('/' + qLower)) {
                                isOpen = true;
                                break;
                            }
                        }
                    }

                    // 2. Sprawdź aktywny edytor
                    if (!isOpen && vscode.window.activeTextEditor) {
                        const curPath = vscode.window.activeTextEditor.document.uri.fsPath.toLowerCase();
                        if (curPath.includes(qLower)) {
                            isOpen = true;
                        }
                    }

                    // 3. Sprawdź otwarte dokumenty
                    if (!isOpen) {
                        for (const doc of vscode.workspace.textDocuments) {
                            if (doc.uri.fsPath.toLowerCase().includes(qLower)) {
                                isOpen = true;
                                break;
                            }
                        }
                    }

                    if (isOpen) {
                        res.writeHead(200);
                        res.end(JSON.stringify({ status: 'ok', problem_id: qProb, open: true }));
                    } else {
                        res.writeHead(404);
                        res.end(JSON.stringify({ status: 'not_open', problem_id: qProb, open: false }));
                    }
                    return;
                }

                res.writeHead(200);
                res.end(JSON.stringify({ status: 'ok', app: 'algodeck-vscode-bridge' }));
                return;
            }

            if (parsed.pathname === '/kill') {
                try {
                    // 1. Anuluj aktywne zadania VS Code
                    for (const execution of vscode.tasks.taskExecutions) {
                        if (execution.task && execution.task.name && execution.task.name.includes('AlgoDeck')) {
                            execution.terminate();
                        }
                    }
                    // 2. Wyślij SIGINT do terminali AlgoDeck
                    for (const term of vscode.window.terminals) {
                        if (term.name.includes('AlgoDeck') || term.name.includes('ODPAL') || term.name.includes('TESTUJ')) {
                            term.sendText('\x03', false);
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

            // Sprawdź przekierowanie do innego okna w trybie separate_windows
            const pdir = parsed.searchParams.get('pdir');
            if (process.platform !== 'win32' && pdir) {
                const targetProb = path.basename(pdir).toLowerCase();
                const myWs = (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0)
                    ? vscode.workspace.workspaceFolders[0].name.toLowerCase()
                    : '';
                const targetSock = `/tmp/algodeck_vscode_${targetProb}.sock`;
                if (myWs !== targetProb && myWs !== 'algodeckworkspace' && fs.existsSync(targetSock) && targetSock !== sockPath) {
                    try {
                        await forwardToSocket(targetSock, req, res);
                        return;
                    } catch (_) {}
                }
            }

            if (parsed.pathname === '/test') {
                try {
                    // Przerwij poprzednie wykonanie jeśli nadal trwa
                    for (const execution of vscode.tasks.taskExecutions) {
                        if (execution.task && execution.task.name && execution.task.name.includes('AlgoDeck')) {
                            execution.terminate();
                        }
                    }

                    let executed = false;

                    // 1. Wykonanie jako natywne zadanie VS Code Task
                    if (pdir) {
                        try {
                            const task = createAlgoTask('AlgoDeck: TESTUJ', 'test.sh', pdir);
                            await vscode.tasks.executeTask(task);
                            executed = true;
                        } catch (taskErr) {
                            console.error('[AlgoDeck Bridge] executeTask error:', taskErr);
                        }
                    }

                    // 2. Fallback: wyszukaj zadanie w workspace (tasks.json)
                    if (!executed) {
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
                    }

                    // 3. Fallback: workbench test action
                    if (!executed) {
                        await vscode.commands.executeCommand('workbench.action.tasks.test');
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
                    // Przerwij poprzednie wykonanie jeśli nadal trwa
                    for (const execution of vscode.tasks.taskExecutions) {
                        if (execution.task && execution.task.name && execution.task.name.includes('AlgoDeck')) {
                            execution.terminate();
                        }
                    }

                    let executed = false;

                    // 1. Wykonanie jako natywne zadanie VS Code Task
                    if (pdir) {
                        try {
                            const task = createAlgoTask('AlgoDeck: ODPAL', 'run.sh', pdir);
                            await vscode.tasks.executeTask(task);
                            executed = true;
                        } catch (taskErr) {
                            console.error('[AlgoDeck Bridge] executeTask error:', taskErr);
                        }
                    }

                    // 2. Fallback: wyszukaj zadanie w workspace (tasks.json)
                    if (!executed) {
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
                    }

                    // 3. Fallback: workbench build action
                    if (!executed) {
                        await vscode.commands.executeCommand('workbench.action.tasks.build');
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

function tryBindHttpServer(handler) {
    if (server) return;
    const s = http.createServer(handler);
    s.on('error', (err) => {
        if (err.code === 'EADDRINUSE') {
            server = null;
        }
    });
    s.listen(PORT, '127.0.0.1', () => {
        server = s;
        console.log(`[AlgoDeck Bridge] Nasłuchiwanie na 127.0.0.1:${PORT}`);
    });
}

function activate(context) {
    const handler = getRequestHandler();

    // 1. Serwer HTTP na stałym porcie (wieloplatformowy: macOS, Linux, Windows)
    tryBindHttpServer(handler);

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

    // 3. Obsługa przełączania fokusu okien VS Code
    vscode.window.onDidChangeWindowState((e) => {
        if (e.focused) {
            tryBindHttpServer(handler);
            if (sockPath) {
                try {
                    fs.writeFileSync('/tmp/algodeck_vscode_active.sock', sockPath);
                } catch (_) {}
            }
        }
    });

    context.subscriptions.push({
        dispose: () => {
            if (server) {
                try { server.close(); } catch (_) {}
                server = null;
            }
            if (sockServer) {
                try { sockServer.close(); } catch (_) {}
                sockServer = null;
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
        server = null;
    }
    if (sockServer) {
        try { sockServer.close(); } catch (_) {}
        sockServer = null;
    }
    if (sockPath && fs.existsSync(sockPath)) {
        try { fs.unlinkSync(sockPath); } catch (_) {}
    }
}

module.exports = {
    activate,
    deactivate
};
