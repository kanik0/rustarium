import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import * as api from './api.js';

const PLANET_DATA = {
    sun:     { color: 0xFFD700, size: 0.12, orbit: 0,     emissive: 0xFFAA00, label: 'Sun' },
    mercury: { color: 0xB0B0B0, size: 0.022, orbit: 0.387, emissive: 0,       label: 'Mercury' },
    venus:   { color: 0xE8D44D, size: 0.032, orbit: 0.723, emissive: 0,       label: 'Venus' },
    earth:   { color: 0x4488FF, size: 0.034, orbit: 1.0,   emissive: 0x112244, label: 'Earth' },
    mars:    { color: 0xE05040, size: 0.026, orbit: 1.524, emissive: 0,       label: 'Mars' },
    jupiter: { color: 0xD4A574, size: 0.065, orbit: 5.203, emissive: 0,       label: 'Jupiter' },
    saturn:  { color: 0xC8A84E, size: 0.055, orbit: 9.537, emissive: 0,       label: 'Saturn' },
    uranus:  { color: 0x72C4C8, size: 0.042, orbit: 19.19, emissive: 0,       label: 'Uranus' },
    neptune: { color: 0x4060E0, size: 0.040, orbit: 30.07, emissive: 0,       label: 'Neptune' },
    moon:    { color: 0xDDDDDD, size: 0.012, orbit: 0,     emissive: 0,       label: 'Moon' },
};

function scaleDistance(au) {
    if (au <= 0) return 0;
    return (Math.log10(au * 10 + 1)) * 2.0;
}

// --- State ---
let scene, camera, renderer, controls;
let planetMeshes = {};
let orbitLines = {};
let moonOrbitLine = null;
let currentDate = new Date();
let playing = false;
let speedDaysPerSec = 1;
let selectedBody = null;
let observerLat = 41.9028;
let observerLon = 12.4964;

// Smooth interpolation: store current and target positions
let targetPositions = {};
let currentPositions = {};

function init() {
    const canvas = document.getElementById('solar-system');
    renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    renderer.setSize(window.innerWidth, window.innerHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0x030308);

    scene = new THREE.Scene();
    camera = new THREE.PerspectiveCamera(55, window.innerWidth / window.innerHeight, 0.01, 500);
    camera.position.set(0, 10, 14);

    controls = new OrbitControls(camera, canvas);
    controls.enableDamping = true;
    controls.dampingFactor = 0.06;
    controls.minDistance = 0.5;
    controls.maxDistance = 80;

    scene.add(new THREE.AmbientLight(0x334466, 0.5));
    const sunLight = new THREE.PointLight(0xFFEECC, 3, 120);
    scene.add(sunLight);

    createStarfield();
    createPlanets();
    createOrbits();
    createMoonOrbit();
    setupUI();
    updateDate(new Date());

    window.addEventListener('resize', onResize);
    canvas.addEventListener('click', onCanvasClick);
    animate();
}

function createStarfield() {
    const geo = new THREE.BufferGeometry();
    const n = 4000;
    const pos = new Float32Array(n * 3);
    const col = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) {
        const r = 60 + Math.random() * 140;
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.acos(2 * Math.random() - 1);
        pos[i*3] = r * Math.sin(phi) * Math.cos(theta);
        pos[i*3+1] = r * Math.sin(phi) * Math.sin(theta);
        pos[i*3+2] = r * Math.cos(phi);
        const b = 0.3 + Math.random() * 0.7;
        col[i*3] = b * (0.9 + Math.random() * 0.2);
        col[i*3+1] = b;
        col[i*3+2] = b * (0.9 + Math.random() * 0.3);
    }
    geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    geo.setAttribute('color', new THREE.BufferAttribute(col, 3));
    scene.add(new THREE.Points(geo, new THREE.PointsMaterial({
        size: 0.12, vertexColors: true, transparent: true, opacity: 0.9
    })));
}

function createPlanets() {
    for (const [name, data] of Object.entries(PLANET_DATA)) {
        const mat = new THREE.MeshStandardMaterial({
            color: data.color,
            emissive: data.emissive || 0x000000,
            emissiveIntensity: name === 'sun' ? 2.0 : (data.emissive ? 0.4 : 0),
            roughness: 0.6,
        });
        const mesh = new THREE.Mesh(new THREE.SphereGeometry(data.size, 20, 14), mat);
        mesh.userData = { bodyName: name };

        if (name === 'sun') {
            mesh.add(new THREE.Mesh(
                new THREE.SphereGeometry(0.2, 20, 14),
                new THREE.MeshBasicMaterial({ color: 0xFFDD66, transparent: true, opacity: 0.12 })
            ));
            mesh.add(new THREE.Mesh(
                new THREE.SphereGeometry(0.35, 16, 10),
                new THREE.MeshBasicMaterial({ color: 0xFFBB33, transparent: true, opacity: 0.04 })
            ));
        }
        if (name === 'saturn') {
            const ring = new THREE.Mesh(
                new THREE.RingGeometry(data.size * 1.4, data.size * 2.2, 32),
                new THREE.MeshBasicMaterial({ color: 0xC8A84E, side: THREE.DoubleSide, transparent: true, opacity: 0.4 })
            );
            ring.rotation.x = Math.PI / 2.2;
            mesh.add(ring);
        }

        scene.add(mesh);
        planetMeshes[name] = mesh;
        currentPositions[name] = new THREE.Vector3();
        targetPositions[name] = new THREE.Vector3();
    }
}

// Orbital elements for each planet (J2000 ecliptic).
// a = semi-major axis (AU), e = eccentricity, inc = inclination (deg),
// node = longitude of ascending node (deg), peri = longitude of perihelion (deg)
const ORBIT_ELEMENTS = {
    mercury: { a: 0.387, e: 0.2056, inc: 7.005, node: 48.331,  peri: 77.456 },
    venus:   { a: 0.723, e: 0.0068, inc: 3.395, node: 76.680,  peri: 131.564 },
    earth:   { a: 1.000, e: 0.0167, inc: 0.000, node: 0.0,     peri: 102.937 },
    mars:    { a: 1.524, e: 0.0934, inc: 1.850, node: 49.558,  peri: 336.060 },
    jupiter: { a: 5.203, e: 0.0489, inc: 1.303, node: 100.464, peri: 14.331 },
    saturn:  { a: 9.537, e: 0.0565, inc: 2.489, node: 113.666, peri: 93.057 },
    uranus:  { a: 19.19, e: 0.0463, inc: 0.773, node: 74.006,  peri: 173.005 },
    neptune: { a: 30.07, e: 0.0095, inc: 1.770, node: 131.784, peri: 48.124 },
};

function createOrbits() {
    for (const name of ['mercury','venus','earth','mars','jupiter','saturn','uranus','neptune']) {
        const el = ORBIT_ELEMENTS[name];
        const incRad = el.inc * Math.PI / 180 * 3.0; // amplified 3x to match planet positions
        const nodeRad = el.node * Math.PI / 180;
        // Argument of perihelion = longitude of perihelion - longitude of ascending node
        const omegaRad = (el.peri - el.node) * Math.PI / 180;

        const pts = [];
        for (let i = 0; i <= 256; i++) {
            // True anomaly from 0 to 2π
            const nu = (i / 256) * Math.PI * 2;
            // Orbital radius from the focus: r = a(1-e²) / (1 + e*cos(nu))
            const r_au = el.a * (1 - el.e * el.e) / (1 + el.e * Math.cos(nu));
            const r = scaleDistance(r_au);

            // Position in orbital plane (perihelion along x)
            const angle = nu + omegaRad;
            const x0 = r * Math.cos(angle);
            const z0 = r * Math.sin(angle);

            // Rotate: node rotation + inclination tilt
            const cosN = Math.cos(-nodeRad), sinN = Math.sin(-nodeRad);
            const x1 = x0 * cosN - z0 * sinN;
            const z1 = x0 * sinN + z0 * cosN;
            const y2 = z1 * Math.sin(incRad);
            const z2 = z1 * Math.cos(incRad);
            const cosN2 = Math.cos(nodeRad), sinN2 = Math.sin(nodeRad);
            const x3 = x1 * cosN2 - z2 * sinN2;
            const z3 = x1 * sinN2 + z2 * cosN2;

            pts.push(new THREE.Vector3(x3, y2, z3));
        }
        const line = new THREE.Line(
            new THREE.BufferGeometry().setFromPoints(pts),
            new THREE.LineBasicMaterial({ color: PLANET_DATA[name].color, transparent: true, opacity: 0.25 })
        );
        scene.add(line);
        orbitLines[name] = line;
    }
}

function createMoonOrbit() {
    // Tilted ellipse around Earth (e=0.0549, inc=5.15° amplified 3x)
    const pts = [];
    const a = 0.12; // visual semi-major axis
    const e = 0.0549;
    const incRad = 5.15 * Math.PI / 180 * 3.0;
    for (let i = 0; i <= 64; i++) {
        const nu = (i / 64) * Math.PI * 2;
        const r = a * (1 - e * e) / (1 + e * Math.cos(nu));
        const x = r * Math.cos(nu);
        const z0 = r * Math.sin(nu);
        const y = z0 * Math.sin(incRad);
        const z = z0 * Math.cos(incRad);
        pts.push(new THREE.Vector3(x, y, z));
    }
    moonOrbitLine = new THREE.Line(
        new THREE.BufferGeometry().setFromPoints(pts),
        new THREE.LineBasicMaterial({ color: 0x888888, transparent: true, opacity: 0.2 })
    );
    scene.add(moonOrbitLine);
}

function highlightOrbit(bodyName) {
    for (const [name, line] of Object.entries(orbitLines)) {
        line.material.opacity = name === bodyName ? 0.7 : 0.2;
    }
    // Moon orbit highlight
    if (moonOrbitLine) {
        moonOrbitLine.material.opacity = bodyName === 'moon' ? 0.6 : 0.2;
    }
}

// --- Smooth interpolation ---
function setTargetPositions(data) {
    if (!data) return;
    targetPositions['sun'] = new THREE.Vector3(0, 0, 0);

    if (data.planets) {
        for (const p of data.planets) {
            const name = p.body.toLowerCase();
            if (!targetPositions[name]) continue;
            const lon = (p.helio_lon_deg || 0) * Math.PI / 180;
            const lat = (p.helio_lat_deg || 0) * Math.PI / 180;
            const dist = p.helio_distance_au || PLANET_DATA[name]?.orbit || 1;
            const s = scaleDistance(dist);
            const cosLat = Math.cos(lat);
            // Y = vertical offset from ecliptic plane (amplified 3x for visibility)
            targetPositions[name].set(
                s * cosLat * Math.cos(lon),
                s * Math.sin(lat) * 3.0,
                -s * cosLat * Math.sin(lon)
            );
        }
    }

    // Moon relative to Earth (with ecliptic latitude for orbital inclination)
    if (data.moon && targetPositions['earth'] && targetPositions['moon']) {
        const ep = targetPositions['earth'];
        const earthLon = (data.moon.earth_helio_lon_deg || 0) * Math.PI / 180;
        const moonGeoLon = (data.moon.geocentric_lon_deg || 0) * Math.PI / 180;
        const moonGeoLat = (data.moon.geocentric_lat_deg || 0) * Math.PI / 180;
        const moonLon = earthLon + moonGeoLon;
        const offset = 0.12;
        targetPositions['moon'].set(
            ep.x + offset * Math.cos(moonLon),
            ep.y + offset * Math.sin(moonGeoLat) * 3.0,
            ep.z - offset * Math.sin(moonLon)
        );
    }

    if (data.moon) {
        const illum = data.moon.illumination || 0;
        updateMoonWidget(illum, data.moon.phase_name || 'Moon');
    }
}

function interpolatePositions(alpha) {
    const t = Math.min(alpha, 1.0);
    for (const [name, mesh] of Object.entries(planetMeshes)) {
        const target = targetPositions[name];
        if (!target) continue;
        mesh.position.lerp(target, t);
    }
    // Move Moon orbit ring to Earth's position
    if (moonOrbitLine && planetMeshes['earth']) {
        moonOrbitLine.position.copy(planetMeshes['earth'].position);
    }
}

function updateMoonWidget(illumination, phaseName) {
    const icons = ['\u{1F311}','\u{1F312}','\u{1F313}','\u{1F314}','\u{1F315}','\u{1F316}','\u{1F317}','\u{1F318}'];
    const idx = Math.round(illumination * 7) % 8;
    document.getElementById('moon-icon').textContent = icons[idx];
    document.getElementById('moon-phase').textContent = phaseName;
    document.getElementById('moon-illum').textContent = `${(illumination * 100).toFixed(0)}%`;
}

// --- UI ---
function setupUI() {
    const picker = document.getElementById('date-picker');
    picker.value = fmtDate(currentDate);
    picker.addEventListener('change', e => {
        const d = new Date(e.target.value + 'T12:00:00');
        if (!isNaN(d.getTime())) updateDate(d);
    });
    document.getElementById('btn-today').addEventListener('click', () => updateDate(new Date()));

    document.getElementById('btn-play').addEventListener('click', () => {
        playing = !playing;
        document.getElementById('btn-play').textContent = playing ? '\u23F8' : '\u25B6';
    });

    document.getElementById('speed-select').addEventListener('change', e => {
        speedDaysPerSec = parseFloat(e.target.value);
    });

    document.querySelectorAll('.planet-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            selectBody(btn.dataset.body);
        });
    });
    document.getElementById('close-panel').addEventListener('click', () => {
        document.getElementById('info-panel').classList.add('hidden');
        selectedBody = null;
        highlightOrbit(null);
        document.querySelectorAll('.planet-btn').forEach(b => b.classList.remove('active'));
    });

    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const tab = btn.dataset.tab;
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.toggle('active', b.dataset.tab === tab));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.toggle('hidden', c.id !== `tab-${tab}`));
        });
    });

    // Location widget
    const citySelect = document.getElementById('city-select');
    citySelect.addEventListener('change', () => {
        const val = citySelect.value;
        if (val === 'custom') {
            document.getElementById('custom-coords').classList.remove('hidden');
        } else {
            document.getElementById('custom-coords').classList.add('hidden');
            const [lat, lon] = val.split(',').map(Number);
            setObserver(lat, lon);
        }
    });
    document.getElementById('obs-lat').addEventListener('change', updateCustomCoords);
    document.getElementById('obs-lon').addEventListener('change', updateCustomCoords);

    loadEclipses();
}

function updateCustomCoords() {
    const lat = parseFloat(document.getElementById('obs-lat').value) || 0;
    const lon = parseFloat(document.getElementById('obs-lon').value) || 0;
    setObserver(lat, lon);
}

function setObserver(lat, lon) {
    observerLat = lat;
    observerLon = lon;
    const ns = lat >= 0 ? 'N' : 'S';
    const ew = lon >= 0 ? 'E' : 'W';
    document.getElementById('obs-label').textContent = `${Math.abs(lat).toFixed(2)}\u00B0${ns}, ${Math.abs(lon).toFixed(2)}\u00B0${ew}`;
    // Refresh rise/set if panel is open
    if (selectedBody) selectBody(selectedBody);
}

async function updateDate(date) {
    currentDate = date;
    document.getElementById('date-picker').value = fmtDate(date);
    try {
        const data = await api.fetchSky(date);
        setTargetPositions(data);
        // On manual date change, snap immediately
        interpolatePositions(1.0);
    } catch (err) { console.error('API error:', err); }
    // Refresh info panel if a body is selected
    if (selectedBody) refreshPanel(selectedBody);
}

async function selectBody(bodyName) {
    selectedBody = bodyName;
    document.querySelectorAll('.planet-btn').forEach(b => {
        b.classList.toggle('active', b.dataset.body === bodyName);
    });
    highlightOrbit(bodyName);

    const mesh = planetMeshes[bodyName];
    if (mesh) controls.target.copy(mesh.position);

    const panel = document.getElementById('info-panel');
    panel.classList.remove('hidden');
    document.getElementById('panel-title').textContent = PLANET_DATA[bodyName]?.label || bodyName;

    document.querySelectorAll('.tab-btn').forEach(b => b.classList.toggle('active', b.dataset.tab === 'position'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.toggle('hidden', c.id !== 'tab-position'));

    await refreshPanel(bodyName);
}

async function refreshPanel(bodyName) {
    try {
        const data = await api.fetchPosition(bodyName, currentDate);
        const pos = data.position || {};
        const el = document.getElementById('tab-position');
        el.textContent = '';
        addSection(el, 'Equatorial');
        addRow(el, 'RA', pos.ra_hms || '-');
        addRow(el, 'Dec', pos.dec_dms || '-');
        if (pos.distance_au) addRow(el, 'Distance', `${pos.distance_au.toFixed(4)} AU`);
        else if (pos.distance_km) addRow(el, 'Distance', `${Math.round(pos.distance_km).toLocaleString()} km`);
        if (pos.distance_from_sun_au) addRow(el, 'From Sun', `${pos.distance_from_sun_au.toFixed(4)} AU`);
        if (pos.illumination !== undefined) {
            addSection(el, 'Phase');
            addRow(el, 'Illumination', `${(pos.illumination * 100).toFixed(1)}%`);
        }
    } catch (_) {}

    try {
        const rs = await api.fetchRiseSet(bodyName, currentDate, observerLat, observerLon);
        const el = document.getElementById('tab-riseset');
        el.textContent = '';
        addSection(el, `${Math.abs(observerLat).toFixed(1)}\u00B0${observerLat>=0?'N':'S'}, ${Math.abs(observerLon).toFixed(1)}\u00B0${observerLon>=0?'E':'W'}`);
        if (rs.events && rs.events.length > 0) {
            for (const ev of rs.events) {
                const time = ev.time_ut ? ev.time_ut.split('T')[1]?.substring(0, 5) : '-';
                const icon = ev.event === 'rise' ? '\u2191' : ev.event === 'set' ? '\u2193' : '\u25C9';
                addRow(el, `${icon} ${cap(ev.event)}`, `${time} UT`);
                if (ev.azimuth_deg != null) addRow(el, '  Azimuth', `${ev.azimuth_deg.toFixed(1)}\u00B0`);
                if (ev.altitude_deg != null) addRow(el, '  Altitude', `${ev.altitude_deg.toFixed(1)}\u00B0`);
            }
        } else {
            const p = document.createElement('p');
            p.textContent = 'Not visible today.';
            p.style.cssText = 'color:var(--text-dim);font-size:12px';
            el.appendChild(p);
        }
    } catch (_) {}
}

async function loadEclipses() {
    try {
        const year = currentDate.getFullYear();
        const [lunar, solar] = await Promise.all([
            api.fetchLunarEclipses(year, 2),
            api.fetchSolarEclipses(year, 2),
        ]);
        const list = document.getElementById('eclipse-list');
        list.textContent = '';
        const all = [
            ...(lunar.eclipses || []).map(e => ({ ...e, kind: 'Lunar' })),
            ...(solar.eclipses || []).map(e => ({ ...e, kind: 'Solar' })),
        ].sort((a, b) => (a.greatest_eclipse || '').localeCompare(b.greatest_eclipse || ''));

        for (const e of all.slice(0, 6)) {
            const date = (e.greatest_eclipse || '').split('T')[0];
            const item = document.createElement('div');
            item.className = 'eclipse-item';
            const badge = document.createElement('span');
            badge.className = `eclipse-badge ${e.type}`;
            badge.textContent = `${e.kind} ${cap(e.type || '')}`;
            const dateEl = document.createElement('span');
            dateEl.className = 'eclipse-date';
            dateEl.textContent = date;
            item.appendChild(badge);
            item.appendChild(dateEl);
            list.appendChild(item);
        }
        if (all.length > 0) document.getElementById('eclipse-widget').classList.remove('hidden');
    } catch (_) {}
}

// --- DOM helpers ---
function addSection(parent, title) {
    const el = document.createElement('div');
    el.className = 'info-section';
    el.textContent = title;
    parent.appendChild(el);
}
function addRow(parent, label, value) {
    const row = document.createElement('div');
    row.className = 'info-row';
    const l = document.createElement('span'); l.className = 'label'; l.textContent = label;
    const v = document.createElement('span'); v.className = 'value'; v.textContent = value;
    row.appendChild(l); row.appendChild(v);
    parent.appendChild(row);
}

// --- Raycasting ---
const raycaster = new THREE.Raycaster();
const mouse = new THREE.Vector2();

function onCanvasClick(event) {
    mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
    mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;
    raycaster.setFromCamera(mouse, camera);
    const intersects = raycaster.intersectObjects(Object.values(planetMeshes), true);
    if (intersects.length > 0) {
        let obj = intersects[0].object;
        while (obj.parent && !obj.userData.bodyName) obj = obj.parent;
        if (obj.userData.bodyName) selectBody(obj.userData.bodyName);
    }
}

// --- Animation with smooth interpolation ---
let lastTime = 0;
let lastFetchTime = 0;
let timeSinceLastFetch = 0;
const FETCH_INTERVAL = 500; // ms between API calls

function animate(time = 0) {
    requestAnimationFrame(animate);
    const dt = time - lastTime;
    lastTime = time;

    if (playing && dt > 0 && dt < 200) {
        currentDate = new Date(currentDate.getTime() + (dt / 1000) * speedDaysPerSec * 86400000);
        document.getElementById('date-picker').value = fmtDate(currentDate);

        timeSinceLastFetch += dt;

        // Fetch new positions and refresh panel periodically
        if (timeSinceLastFetch >= FETCH_INTERVAL) {
            timeSinceLastFetch = 0;
            api.fetchSky(currentDate).then(data => setTargetPositions(data)).catch(() => {});
            if (selectedBody) refreshPanel(selectedBody);
            // Pulse the date picker to signal update
            const picker = document.getElementById('date-picker');
            picker.classList.remove('pulse');
            // Force reflow so re-adding the class restarts the animation
            void picker.offsetWidth;
            picker.classList.add('pulse');
        }

        // Smooth interpolation between fetches: lerp toward targets each frame
        // Use a smooth factor that gives fluid motion
        const lerpFactor = 1.0 - Math.exp(-dt * 0.008); // exponential smoothing
        interpolatePositions(lerpFactor);
    }

    for (const mesh of Object.values(planetMeshes)) mesh.rotation.y += 0.001;
    controls.update();
    renderer.render(scene, camera);
}

function onResize() {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
}

function fmtDate(d) { return d.toISOString().split('T')[0]; }
function cap(s) { return s.charAt(0).toUpperCase() + s.slice(1); }

// Initialize WASM module, then start the app
api.initWasm().then(() => {
    console.log('WASM loaded - all computation runs locally');
    init();
}).catch(err => {
    console.error('WASM init failed:', err);
    init(); // start anyway, API calls will fail gracefully
});
