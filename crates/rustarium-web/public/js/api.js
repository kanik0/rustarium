const API_BASE = '';

function formatDate(date) {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
}

export async function fetchSky(date) {
    const dateStr = formatDate(date);
    const res = await fetch(`${API_BASE}/api/sky?date=${dateStr}`);
    return res.json();
}

export async function fetchPosition(body, date) {
    const dateStr = formatDate(date);
    const res = await fetch(`${API_BASE}/api/position/${body}?date=${dateStr}`);
    return res.json();
}

export async function fetchMoon(date) {
    const dateStr = formatDate(date);
    const res = await fetch(`${API_BASE}/api/moon?date=${dateStr}`);
    return res.json();
}

export async function fetchRiseSet(body, date, lat, lon) {
    const dateStr = formatDate(date);
    const res = await fetch(
        `${API_BASE}/api/riseset?body=${body}&date=${dateStr}&lat=${lat}&lon=${lon}`
    );
    return res.json();
}

export async function fetchLunarEclipses(year, range = 2) {
    const res = await fetch(`${API_BASE}/api/eclipse/lunar?year=${year}&range=${range}`);
    return res.json();
}

export async function fetchSolarEclipses(year, range = 2) {
    const res = await fetch(`${API_BASE}/api/eclipse/solar?year=${year}&range=${range}`);
    return res.json();
}

export async function fetchEphemeris(body, date, days = 365, step = 1) {
    const dateStr = formatDate(date);
    const res = await fetch(
        `${API_BASE}/api/ephemeris/${body}?date=${dateStr}&days=${days}&step=${step}`
    );
    return res.json();
}
