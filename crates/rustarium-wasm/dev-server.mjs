// Dev server: serves site/ static files + proxies /api/sbdb/search to JPL SBDB API.
// Usage: node dev-server.mjs
// Then open http://localhost:3000

import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const PORT = 3000;
const SITE_DIR = join(fileURLToPath(import.meta.url), '..', 'site');
const SBDB_API = 'https://ssd-api.jpl.nasa.gov/sbdb.api';

const MIME = {
    '.html': 'text/html',
    '.js':   'application/javascript',
    '.mjs':  'application/javascript',
    '.css':  'text/css',
    '.json': 'application/json',
    '.wasm': 'application/wasm',
    '.png':  'image/png',
    '.svg':  'image/svg+xml',
    '.ico':  'image/x-icon',
};

const server = createServer(async (req, res) => {
    const url = new URL(req.url, `http://localhost:${PORT}`);

    // Proxy: /api/sbdb/search?sstr=... → JPL SBDB API
    if (url.pathname === '/api/sbdb/search') {
        const sstr = url.searchParams.get('sstr');
        if (!sstr) {
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: "Missing 'sstr' parameter" }));
            return;
        }
        try {
            const sbdbUrl = `${SBDB_API}?sstr=${encodeURIComponent(sstr)}&phys-par=true`;
            const resp = await fetch(sbdbUrl);
            const body = await resp.text();
            res.writeHead(200, {
                'Content-Type': 'application/json',
                'Access-Control-Allow-Origin': '*',
            });
            res.end(body);
        } catch (err) {
            res.writeHead(502, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: `SBDB proxy error: ${err.message}` }));
        }
        return;
    }

    // Static file serving
    let filePath = join(SITE_DIR, url.pathname === '/' ? 'index.html' : url.pathname);
    try {
        const info = await stat(filePath);
        if (info.isDirectory()) filePath = join(filePath, 'index.html');
        const data = await readFile(filePath);
        const ext = extname(filePath);
        res.writeHead(200, { 'Content-Type': MIME[ext] || 'application/octet-stream' });
        res.end(data);
    } catch {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end('404 Not Found');
    }
});

server.listen(PORT, () => {
    console.log(`Rustarium dev server: http://localhost:${PORT}`);
    console.log(`SBDB proxy:           /api/sbdb/search?sstr=...`);
    console.log(`Static files:         ${SITE_DIR}`);
});
