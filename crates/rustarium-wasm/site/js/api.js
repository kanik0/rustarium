// Client-side WASM API — calls Rust functions directly, no server needed.

import init, { sky, position, moon_info, riseset, lunar_eclipses, solar_eclipses } from '../pkg/rustarium_wasm.js';

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
