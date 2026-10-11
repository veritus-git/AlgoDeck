const vscode = require('vscode');
const http = require('http');

let server = null;
const PORT = 49152;

function activate(context) {
    server = http.createServer(async (req, res) => {
        try {
            const parsed = new URL(req.url, `http://127.0.0.1:${PORT}`);
            res.setHeader('Content-Type', 'application/json');

            if (parsed.pathname === '/ping') {
                res.writeHead(200);
                res.end(JSON.stringify({ status: 'ok', app: 'algodeck-vscode-bridge' }));
                return;
            }

            if (parsed.pathname === '/test') {
                try {
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
                        // Bezpośrednie uruchomienie przez wbudowany terminal VS Code
                        const pdir = parsed.searchParams.get('pdir');
                        const term = vscode.window.terminals.find(t => t.name === 'AlgoDeck') || vscode.window.createTerminal('AlgoDeck');
                        term.show(true);
                        if (pdir) {
                            term.sendText(`cd "${pdir}" && bash .algo/test.sh`);
                        } else {
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
                        // Bezpośrednie uruchomienie przez wbudowany terminal VS Code
                        const pdir = parsed.searchParams.get('pdir');
                        const term = vscode.window.terminals.find(t => t.name === 'AlgoDeck') || vscode.window.createTerminal('AlgoDeck');
                        term.show(true);
                        if (pdir) {
                            term.sendText(`cd "${pdir}" && bash .algo/run.sh`);
                        } else {
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
    });

    server.on('error', (err) => {
        // Ignoruj błąd jeśli port jest już zajęty przez inne okno VS Code
        console.log('[AlgoDeck Bridge] HTTP Server error:', err.message);
    });

    server.listen(PORT, '127.0.0.1', () => {
        console.log(`[AlgoDeck Bridge] Nasłuchiwanie na 127.0.0.1:${PORT}`);
    });

    context.subscriptions.push({
        dispose: () => {
            if (server) {
                try { server.close(); } catch (_) {}
            }
        }
    });
}

function deactivate() {
    if (server) {
        try { server.close(); } catch (_) {}
    }
}

module.exports = {
    activate,
    deactivate
};
