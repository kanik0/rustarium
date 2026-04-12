// Client-side WASM API — calls Rust functions directly, no server needed.

import init, {
    sky, position, moon_info, riseset, lunar_eclipses, solar_eclipses,
    add_custom_body, remove_custom_body, list_custom_bodies,
    custom_position, custom_riseset,
    spacecraft_catalog, spacecraft_search, add_spacecraft
} from '../pkg/rustarium_wasm.js';

let wasmReady = false;

export async function initWasm() {
    await init();
    wasmReady = true;
}

function formatDate(date) {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
}

export async function fetchSky(date) {
    if (!wasmReady) await initWasm();
    const json = sky(formatDate(date));
    return JSON.parse(json);
}

export async function fetchPosition(body, date) {
    if (!wasmReady) await initWasm();
    const json = position(body, formatDate(date));
    return JSON.parse(json);
}

export async function fetchMoon(date) {
    if (!wasmReady) await initWasm();
    const json = moon_info(formatDate(date));
    return JSON.parse(json);
}

export async function fetchRiseSet(body, date, lat, lon) {
    if (!wasmReady) await initWasm();
    const json = riseset(body, formatDate(date), lat, lon);
    return JSON.parse(json);
}

export async function fetchLunarEclipses(year, range = 2) {
    if (!wasmReady) await initWasm();
    const json = lunar_eclipses(year, range);
    return JSON.parse(json);
}

export async function fetchSolarEclipses(year, range = 2) {
    if (!wasmReady) await initWasm();
    const json = solar_eclipses(year, range);
    return JSON.parse(json);
}

// ===== Custom Bodies =====

export async function searchSBDB(query) {
    // Always proxy through same-origin to avoid CORS issues.
    // Works on Cloudflare Worker (/api/sbdb/search) and local dev server (proxy).
    const url = `/api/sbdb/search?sstr=${encodeURIComponent(query)}`;
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 12000);
    try {
        const resp = await fetch(url, { signal: controller.signal });
        if (!resp.ok) throw new Error(`SBDB search error: ${resp.status}`);
        return await resp.json();
    } finally {
        clearTimeout(timeout);
    }
}

export async function addCustomBody(sbdbJson) {
    if (!wasmReady) await initWasm();
    const json = add_custom_body(JSON.stringify(sbdbJson));
    return JSON.parse(json);
}

export async function removeCustomBody(name) {
    if (!wasmReady) await initWasm();
    const json = remove_custom_body(name);
    return JSON.parse(json);
}

export async function fetchCustomBodies() {
    if (!wasmReady) await initWasm();
    const json = list_custom_bodies();
    return JSON.parse(json);
}

export async function fetchCustomPosition(name, date) {
    if (!wasmReady) await initWasm();
    const json = custom_position(name, formatDate(date));
    return JSON.parse(json);
}

export async function fetchCustomRiseSet(name, date, lat, lon) {
    if (!wasmReady) await initWasm();
    const json = custom_riseset(name, formatDate(date), lat, lon);
    return JSON.parse(json);
}

// ===== Spacecraft =====

export async function getSpacecraftCatalog() {
    if (!wasmReady) await initWasm();
    return JSON.parse(spacecraft_catalog());
}

export async function searchSpacecraftCatalog(query) {
    if (!wasmReady) await initWasm();
    return JSON.parse(spacecraft_search(query));
}

export async function fetchHorizons(horizonsId) {
    const url = `/api/horizons/vectors?id=${encodeURIComponent(horizonsId)}`;
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 20000);
    try {
        const resp = await fetch(url, { signal: controller.signal });
        if (!resp.ok) throw new Error(`Horizons error: ${resp.status}`);
        return await resp.json();
    } finally {
        clearTimeout(timeout);
    }
}

export async function addSpacecraft(name, horizonsId, horizonsResponse) {
    if (!wasmReady) await initWasm();
    const payload = { name, horizons_id: horizonsId, horizons_response: horizonsResponse };
    const json = add_spacecraft(JSON.stringify(payload));
    return JSON.parse(json);
}
