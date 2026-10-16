const vscode = require('vscode');
const http = require('http');
const fs = require('fs');
const path = require('path');

let server = null;
let sockServer = null;
let activeSockPath = null;
let lastActionTime = 0;
let lastActionType = '';

const BASE_PORT = 49152;

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
            res.writeHead(502, { 'Content-Type': 'application/json', 'Connection': 'close' });
            res.end(JSON.stringify({ status: 'bad_gateway', error: err.message }));
            resolve();
        });
        req.pipe(client);
    });
}

function getActiveProblemDir() {
    if (vscode.window.activeTextEditor) {
        return path.dirname(vscode.window.activeTextEditor.document.uri.fsPath);
    }
    if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
        return vscode.workspace.workspaceFolders[0].uri.fsPath;
    }
    return null;
}

async function openProblemFile(filePath, pdir) {
    try {
        let target = filePath;
        if (!target && pdir) {
            const prob = path.basename(pdir);
            const cand = path.join(pdir, `${prob}.cpp`);
            if (fs.existsSync(cand)) {
                target = cand;
            }
        }
        if (target && fs.existsSync(target)) {
            const doc = await vscode.workspace.openTextDocument(target);
            await vscode.window.showTextDocument(doc, { preview: false, preserveFocus: false });
            return true;
        }
    } catch (e) {
        console.error('[AlgoDeck Bridge] openProblemFile error:', e);
    }
    return false;
}

function handleKill() {
    try {
        for (const execution of vscode.tasks.taskExecutions) {
            if (execution.task && execution.task.name && execution.task.name.includes('AlgoDeck')) {
                execution.terminate();
            }
        }
        for (const term of vscode.window.terminals) {
            if (term.name.includes('AlgoDeck') || term.name.includes('ODPAL') || term.name.includes('TESTUJ')) {
                term.sendText('\x03', false);
            }
        }
        return true;
    } catch (e) {
        console.error('[AlgoDeck Bridge] handleKill error:', e);
        return false;
    }
}

function runInAlgoTerminal(name, scriptName, pdir) {
    const now = Date.now();
    if (lastActionType === name && (now - lastActionTime) < 250) {
        return true;
    }
    lastActionType = name;
    lastActionTime = now;

    const isWin = process.platform === 'win32';
    const actualScript = isWin ? scriptName.replace(/\.sh$/, '.cmd') : scriptName;
    const cmd = isWin
        ? (pdir ? `cd /d "${pdir}" && .algo\\${actualScript}` : `.algo\\${actualScript}`)
        : (pdir ? `cd "${pdir}" && bash .algo/${actualScript}` : `bash .algo/${actualScript}`);

    let term = vscode.window.terminals.find(t => t.name === name || t.name === 'AlgoDeck');
    const isNew = !term;
    if (isNew) {
        term = vscode.window.createTerminal({
            name: name,
            cwd: pdir || undefined
        });
    }
    term.show(false);

    if (isNew) {
        setTimeout(() => {
            term.sendText(cmd, true);
        }, 300);
    } else {
        term.sendText('\x03', false);
        setTimeout(() => {
            term.sendText(cmd, true);
        }, 120);
    }
    return true;
}

function writeActivePointers(sockPath, port) {
    const home = process.env.HOME || '';
    const cfgDir = path.join(home, '.config', 'algodeck');
    try { fs.mkdirSync(cfgDir, { recursive: true }); } catch (_) {}

    const sockLocations = [
        process.env.XDG_RUNTIME_DIR ? path.join(process.env.XDG_RUNTIME_DIR, 'algodeck_vscode_active.sock') : null,
        '/tmp/algodeck_vscode_active.sock',
        path.join(cfgDir, 'algodeck_vscode_active.sock')
    ];

    if (sockPath) {
        for (const loc of sockLocations) {
            if (loc) {
                try { fs.writeFileSync(loc, sockPath); } catch (_) {}
            }
        }
    }

    if (port) {
        const portLocations = [
            process.env.XDG_RUNTIME_DIR ? path.join(process.env.XDG_RUNTIME_DIR, 'algodeck_bridge_port.txt') : null,
            '/tmp/algodeck_bridge_port.txt',
            path.join(cfgDir, 'bridge_port.txt')
        ];
        for (const loc of portLocations) {
            if (loc) {
                try { fs.writeFileSync(loc, String(port)); } catch (_) {}
            }
        }
    }
}

function getRequestHandler() {
    return async (req, res) => {
        try {
            const parsed = new URL(req.url, `http://127.0.0.1:${BASE_PORT}`);
            res.setHeader('Content-Type', 'application/json');
            res.setHeader('Connection', 'close');

            if (parsed.pathname === '/ping') {
                const qProb = parsed.searchParams.get('problem_id');
                if (qProb) {
                    const qLower = qProb.toLowerCase();
                    let isOpen = false;

                    if (vscode.workspace.workspaceFolders) {
                        for (const f of vscode.workspace.workspaceFolders) {
                            const fName = f.name.toLowerCase();
                            const fPath = f.uri.fsPath.toLowerCase();
                            if (fName === qLower || fPath.endsWith(qLower) || fPath.includes('/' + qLower) || fName.includes('algodeck')) {
                                isOpen = true;
                                break;
                            }
                            try {
                                if (fs.existsSync(path.join(f.uri.fsPath, qProb)) || fs.existsSync(path.join(f.uri.fsPath, qLower))) {
                                    isOpen = true;
                                    break;
                                }
                            } catch (_) {}
                        }
                    }

                    if (!isOpen && vscode.window.activeTextEditor) {
                        const curPath = vscode.window.activeTextEditor.document.uri.fsPath.toLowerCase();
                        if (curPath.includes(qLower)) {
                            isOpen = true;
                        }
                    }

                    if (!isOpen) {
                        for (const doc of vscode.workspace.textDocuments) {
                            if (doc.uri.fsPath.toLowerCase().includes(qLower)) {
                                isOpen = true;
                                break;
                            }
                        }
                    }

                    if (!isOpen && vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
                        isOpen = true;
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
                handleKill();
                res.writeHead(200);
                res.end(JSON.stringify({ status: 'ok', action: 'kill' }));
                return;
            }

            const pdir = parsed.searchParams.get('pdir');
            const file = parsed.searchParams.get('file');

            if (parsed.pathname === '/open') {
                await openProblemFile(file, pdir);
                res.writeHead(200);
                res.end(JSON.stringify({ status: 'ok', action: 'open' }));
                return;
            }

            if (process.platform !== 'win32' && pdir) {
                const targetProb = path.basename(pdir).toLowerCase();
                const myWs = (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0)
                    ? vscode.workspace.workspaceFolders[0].name.toLowerCase()
                    : '';
                const targetSock = `/tmp/algodeck_vscode_${targetProb}.sock`;
                if (myWs !== targetProb && myWs !== 'algodeckworkspace' && fs.existsSync(targetSock) && targetSock !== activeSockPath) {
                    try {
                        await forwardToSocket(targetSock, req, res);
                        return;
                    } catch (_) {}
                }
            }

            if (parsed.pathname === '/test') {
                try {
                    handleKill();
                    let executed = false;
                    const targetDir = pdir || getActiveProblemDir();

                    if (targetDir) {
                        try {
                            runInAlgoTerminal('AlgoDeck: TESTUJ', 'test.sh', targetDir);
                            executed = true;
                        } catch (termErr) {
                            console.error('[AlgoDeck Bridge] runInAlgoTerminal error:', termErr);
                        }
                    }

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
                    handleKill();
                    let executed = false;
                    const targetDir = pdir || getActiveProblemDir();

                    if (targetDir) {
                        try {
                            runInAlgoTerminal('AlgoDeck: ODPAL', 'run.sh', targetDir);
                            executed = true;
                        } catch (termErr) {
                            console.error('[AlgoDeck Bridge] runInAlgoTerminal error:', termErr);
                        }
                    }

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

function tryBindHttpServer(handler, portOffset = 0) {
    if (server) return;
    if (portOffset > 5) return;
    const port = BASE_PORT + portOffset;
    const s = http.createServer(handler);
    s.on('error', (err) => {
        if (err.code === 'EADDRINUSE') {
            tryBindHttpServer(handler, portOffset + 1);
        }
    });
    s.listen(port, '127.0.0.1', () => {
        server = s;
        console.log(`[AlgoDeck Bridge] Nasłuchiwanie na 127.0.0.1:${port}`);
        writeActivePointers(activeSockPath, port);
    });
}

function activate(context) {
    const handler = getRequestHandler();

    // 1. Zarejestruj bezpośrednie komendy VS Code dostępne z CLI i palety komend
    context.subscriptions.push(
        vscode.commands.registerCommand('algodeck.test', (args) => {
            const pdir = (args && args.pdir) || getActiveProblemDir();
            runInAlgoTerminal('AlgoDeck: TESTUJ', 'test.sh', pdir);
        }),
        vscode.commands.registerCommand('algodeck.run', (args) => {
            const pdir = (args && args.pdir) || getActiveProblemDir();
            runInAlgoTerminal('AlgoDeck: ODPAL', 'run.sh', pdir);
        }),
        vscode.commands.registerCommand('algodeck.kill', () => {
            handleKill();
        }),
        vscode.commands.registerCommand('algodeck.open', (args) => {
            const file = args && args.file;
            const pdir = args && args.pdir;
            openProblemFile(file, pdir);
        })
    );

    // 2. Serwer HTTP na porcie 49152+ (zapisuje port do bridge_port.txt)
    tryBindHttpServer(handler);

    // 3. Dedykowany UNIX domain socket w XDG_RUNTIME_DIR oraz /tmp (macOS i Linux)
    if (process.platform !== 'win32') {
        const folders = vscode.workspace.workspaceFolders;
        const wsName = (folders && folders.length > 0) ? folders[0].name.toLowerCase() : 'default';

        const socketCandidates = [
            process.env.XDG_RUNTIME_DIR ? path.join(process.env.XDG_RUNTIME_DIR, `algodeck_vscode_${wsName}.sock`) : null,
            `/tmp/algodeck_vscode_${wsName}.sock`,
            path.join(process.env.HOME || '', '.config', 'algodeck', `algodeck_vscode_${wsName}.sock`)
        ].filter(Boolean);

        for (const candidate of socketCandidates) {
            try {
                if (fs.existsSync(candidate)) {
                    try { fs.unlinkSync(candidate); } catch (_) {}
                }
                const s = http.createServer(handler);
                s.on('error', (err) => {
                    console.log('[AlgoDeck Bridge] UNIX socket error:', err.message);
                });
                s.listen(candidate, () => {
                    sockServer = s;
                    activeSockPath = candidate;
                    console.log(`[AlgoDeck Bridge] Nasłuchiwanie na gnieździe UNIX: ${candidate}`);
                    writeActivePointers(activeSockPath, server ? server.address().port : BASE_PORT);
                });
                break;
            } catch (e) {
                console.warn('[AlgoDeck Bridge] Nie udało się otworzyć socketu:', candidate, e);
            }
        }
    }

    // 4. File Watcher IPC dla .algo/trigger.json oraz algodeck_trigger.json (niezawodny, działa niezależnie od sieci, portów i SELinux)
    try {
        const handleTrigger = (uri) => {
            try {
                if (!fs.existsSync(uri.fsPath)) return;
                const raw = fs.readFileSync(uri.fsPath, 'utf8');
                const data = JSON.parse(raw);
                if (!data || !data.action) return;

                const pdir = data.pdir || path.dirname(path.dirname(uri.fsPath));
                const ackPath = path.join(path.dirname(uri.fsPath), 'trigger_ack.json');

                if (data.action === 'run') {
                    runInAlgoTerminal('AlgoDeck: ODPAL', 'run.sh', pdir);
                } else if (data.action === 'test') {
                    runInAlgoTerminal('AlgoDeck: TESTUJ', 'test.sh', pdir);
                } else if (data.action === 'kill') {
                    handleKill();
                } else if (data.action === 'open') {
                    openProblemFile(data.file, pdir);
                }

                try { fs.writeFileSync(ackPath, JSON.stringify({ ack: true, time: Date.now() })); } catch (_) {}
                try { fs.unlinkSync(uri.fsPath); } catch (_) {}
            } catch (e) {
                console.error('[AlgoDeck Bridge] trigger file error:', e);
            }
        };

        const watcher1 = vscode.workspace.createFileSystemWatcher('**/.algo/trigger.json');
        watcher1.onDidChange(handleTrigger);
        watcher1.onDidCreate(handleTrigger);
        context.subscriptions.push(watcher1);

        const watcher2 = vscode.workspace.createFileSystemWatcher('**/algodeck_trigger.json');
        watcher2.onDidChange(handleTrigger);
        watcher2.onDidCreate(handleTrigger);
        context.subscriptions.push(watcher2);

        // Dodatkowy szybki poller (co 80ms) jako fallback w razie limitów inotify w systemie Linux
        const pollInterval = setInterval(() => {
            try {
                if (vscode.workspace.workspaceFolders) {
                    for (const f of vscode.workspace.workspaceFolders) {
                        const cand1 = path.join(f.uri.fsPath, '.algo', 'trigger.json');
                        const cand2 = path.join(f.uri.fsPath, 'algodeck_trigger.json');
                        if (fs.existsSync(cand1)) handleTrigger({ fsPath: cand1 });
                        if (fs.existsSync(cand2)) handleTrigger({ fsPath: cand2 });
                    }
                }
                if (vscode.window.activeTextEditor) {
                    const dir = path.dirname(vscode.window.activeTextEditor.document.uri.fsPath);
                    const cand1 = path.join(dir, '.algo', 'trigger.json');
                    const cand2 = path.join(dir, 'algodeck_trigger.json');
                    if (fs.existsSync(cand1)) handleTrigger({ fsPath: cand1 });
                    if (fs.existsSync(cand2)) handleTrigger({ fsPath: cand2 });
                }
            } catch (_) {}
        }, 80);

        context.subscriptions.push({ dispose: () => clearInterval(pollInterval) });
    } catch (e) {
        console.error('[AlgoDeck Bridge] Failed to create triggerWatcher:', e);
    }

    // 5. Obsługa przełączania fokusu okien VS Code
    vscode.window.onDidChangeWindowState((e) => {
        if (e.focused) {
            tryBindHttpServer(handler);
            if (activeSockPath) {
                writeActivePointers(activeSockPath, server ? server.address().port : BASE_PORT);
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
            if (activeSockPath && fs.existsSync(activeSockPath)) {
                try { fs.unlinkSync(activeSockPath); } catch (_) {}
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
    if (activeSockPath && fs.existsSync(activeSockPath)) {
        try { fs.unlinkSync(activeSockPath); } catch (_) {}
    }
}

module.exports = {
    activate,
    deactivate
};
