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
const HORIZONS_API = 'https://ssd.jpl.nasa.gov/api/horizons.api';

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

    // Proxy: /api/horizons/vectors?id=... → JPL Horizons API
    if (url.pathname === '/api/horizons/vectors') {
        const id = url.searchParams.get('id');
        if (!id) {
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: "Missing 'id' parameter" }));
            return;
        }
        // Default: 2 years centered on today
        const now = new Date();
        const y = now.getFullYear();
        const startDate = url.searchParams.get('start') || `${y - 1}-01-01`;
        const stopDate = url.searchParams.get('stop') || `${y + 1}-01-01`;
        const step = url.searchParams.get('step') || '5d';
        try {
            const hUrl = `${HORIZONS_API}?format=json` +
                `&COMMAND='${encodeURIComponent(id)}'` +
                `&OBJ_DATA='YES'&MAKE_EPHEM='YES'&EPHEM_TYPE='VECTORS'` +
                `&CENTER='500@10'&START_TIME='${startDate}'&STOP_TIME='${stopDate}'` +
                `&STEP_SIZE='${step}'&REF_PLANE='ECLIPTIC'&REF_SYSTEM='ICRF'&OUT_UNITS='AU-D'`;
            const resp = await fetch(hUrl);
            const body = await resp.text();
            res.writeHead(200, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' });
            res.end(body);
        } catch (err) {
            res.writeHead(502, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: `Horizons proxy error: ${err.message}` }));
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
    console.log(`Horizons proxy:       /api/horizons/vectors?id=...`);
    console.log(`Static files:         ${SITE_DIR}`);
});
