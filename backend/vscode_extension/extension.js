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
                    await vscode.commands.executeCommand('workbench.action.tasks.runTask', 'AlgoDeck: TESTUJ');
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
                    await vscode.commands.executeCommand('workbench.action.tasks.runTask', 'AlgoDeck: ODPAL');
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
