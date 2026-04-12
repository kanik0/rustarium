# Rustarium

**Solar System Prediction Engine** — a Rust program that predicts positions of solar system objects, generates astronomical reports, and provides a 3D web visualization. All computation runs as WebAssembly, either server-side (Cloudflare Worker) or entirely client-side in the browser.

**Live demo**: https://rustarium.gateway0.workers.dev/

## Features

- **Planet positions** — heliocentric and geocentric coordinates for all 8 planets using VSOP87D analytical theory (~1 arcsecond accuracy)
- **Moon position** — ELP-2000/82 theory with planetary perturbations (~5 arcsecond accuracy)
- **Rise/set times** — sunrise, sunset, transit times for any body from any location on Earth
- **Eclipse prediction** — lunar and solar eclipses with contact times, magnitude, and local visibility
- **N-body simulation** — gravitational simulation with Dormand-Prince RK45 and Wisdom-Holman symplectic integrators
- **Custom objects** — add asteroids, comets, or spacecraft via orbital elements or state vectors
- **Track any small body** — search NASA JPL's Small-Body Database by name, designation, or SPK-ID and track asteroids, comets, and dwarf planets on the fly
- **3D visualization** — Three.js orrery with real orbital elements, inclinations, and eccentricities
- **CLI** — beautiful command-line interface with colored output, timezone support, and multiple output formats

## Quick Start

### CLI

```bash
cargo build -p rustarium-cli
cargo run -p rustarium-cli          # Show the sky right now
cargo run -p rustarium-cli -- position mars
cargo run -p rustarium-cli -- riseset --city roma --days 7
cargo run -p rustarium-cli -- moon --calendar
cargo run -p rustarium-cli -- eclipse lunar --year 2025
cargo run -p rustarium-cli -- ephemeris jupiter --days 30
cargo run -p rustarium-cli -- track Ceres --city roma  # Track an asteroid
```

### Web (Client-Side WASM)

```bash
# Build the WASM module
cd crates/rustarium-wasm
wasm-pack build --target web --out-dir site/pkg --release

# Serve locally (includes SBDB proxy for asteroid search)
node dev-server.mjs
# Open http://localhost:3000
```

The dev server serves static files and proxies `/api/sbdb/search` requests to NASA JPL's Small-Body Database API (required because the SBDB API does not support CORS).

### Deploy to Cloudflare Workers

```bash
cd crates/rustarium-web
npx wrangler deploy
```

The Worker serves the API endpoints and proxies SBDB requests via `/api/sbdb/search`.

## Architecture

```
rustarium/
  crates/
    rustarium-core/     # Computation library (WASM-safe, no I/O)
    rustarium-cli/      # Command-line interface
    rustarium-web/      # Cloudflare Worker (server-side API)
    rustarium-wasm/     # Client-side WASM bindings + static site
```

### rustarium-core

The core library is the heart of the project. It compiles to both native and `wasm32-unknown-unknown` with zero filesystem or network dependencies. All astronomical data is embedded as constants.

**Modules:**

| Module | Purpose |
|--------|---------|
| `time` | Julian Day conversions, delta-T, sidereal time |
| `coords` | Coordinate types (ecliptic, equatorial, horizontal) and transformations |
| `bodies` | Planet/Body enums, GM values, physical constants |
| `planet` | VSOP87D planetary positions with light-time and FK5 corrections |
| `moon` | ELP-2000/82 lunar position with planetary perturbations |
| `sun` | Solar position derived from Earth's VSOP87 |
| `nutation` | IAU 2000B 77-term nutation model |
| `precession` | Rigorous precession (Meeus ch. 21) |
| `aberration` | Annual aberration correction |
| `refraction` | Atmospheric refraction model |
| `observer` | Observer location, local sidereal time |
| `rise_set` | Rise/transit/set calculations |
| `eclipse` | Lunar and solar eclipse prediction |
| `nbody` | N-body simulation (RK45 + symplectic integrators) |
| `custom_body` | Keplerian propagation for asteroids, comets, dwarf planets |
| `sbdb` | NASA JPL Small-Body Database response parser |

### Data Sources

All data is embedded in the source code — no external files are downloaded at runtime.

| Data | Source | Epoch |
|------|--------|-------|
| Planet positions | VSOP87D analytical theory (via `vsop87` crate) | Continuous |
| Moon position | ELP-2000/82 (Meeus ch. 47 + planetary perturbations) | Continuous |
| N-body initial conditions | JPL Horizons API (DE441) | J2000.0 |
| Nutation | IAU 2000B (77 lunisolar terms) | Continuous |
| Asteroid state vectors | JPL Horizons API (DE441) | J2000.0 |
| Custom body elements | JPL Small-Body Database API (fetched on demand) | Osculating epoch |

### Precision

| Component | Accuracy |
|-----------|----------|
| Planet positions (VSOP87) | ~1 arcsecond |
| Moon position | ~5 arcseconds |
| N-body (10 years) | ~500 arcseconds vs VSOP87 |
| Rise/set times | ~1 minute |
| Lunar eclipse magnitude | ~0.007 |
| Solar eclipse detection | Correct type and date |
| Nutation | ~1 milliarcsecond (IAU 2000B) |

### Precision Improvements Implemented

1. **Post-Newtonian 1PN** — Einstein-Infeld-Hoffmann relativistic corrections (fixes Mercury precession)
2. **J2 oblateness** — Earth and Jupiter zonal harmonic perturbation
3. **IAU 2000B nutation** — 77-term model (replaces 13-term approximation)
4. **Asteroid perturbations** — Ceres, Pallas, Vesta included in N-body simulation
5. **Expanded Moon ELP** — Venus and Jupiter planetary perturbation corrections
6. **Topocentric eclipses** — lunar parallax correction for local solar eclipse visibility
7. **Light-time correction** — planet positions computed at retarded time
8. **FK5 correction** — VSOP87 dynamical ecliptic to FK5/J2000 frame
9. **Wisdom-Holman integrator** — symplectic integrator for long-term stability

## CLI Examples

```bash
# Overview of the sky (default: today, Rome)
rustarium

# Planet position with JSON output
rustarium position mars --json

# Rise/set times from different cities
rustarium riseset --city tokyo --days 14
rustarium riseset moon --city ny --days 7

# Moon phase calendar
rustarium moon --calendar --date 2025-01-01

# Eclipse search
rustarium eclipse lunar --year 2025 --range 3
rustarium eclipse solar --year 2026

# Ephemeris table
rustarium ephemeris jupiter --days 60 --step 5

# Track asteroids, comets, dwarf planets (fetches from JPL SBDB)
rustarium track Ceres --city roma
rustarium track 99942 --days 60 --step 5        # Apophis by SPK-ID
rustarium track 1P                               # Halley's Comet
rustarium track "2024 YR4" --json               # JSON output
```

The CLI supports city names in Italian (roma, milano, napoli, giove, marte, luna...) and displays times in local timezone with UT in parentheses.

## API Endpoints (Worker mode)

When deployed as a Cloudflare Worker (`rustarium-web`):

| Endpoint | Description |
|----------|-------------|
| `GET /api/sky?date=YYYY-MM-DD` | Full sky overview (planets, Moon, Sun) |
| `GET /api/position/:body?date=...` | Position of a specific body |
| `GET /api/moon?date=...` | Moon details (phase, distance, parallax) |
| `GET /api/riseset?body=sun&date=...&lat=41.9&lon=12.5` | Rise/set times |
| `GET /api/eclipse/lunar?year=2025&range=2` | Lunar eclipse search |
| `GET /api/eclipse/solar?year=2025&range=2` | Solar eclipse search |
| `GET /api/ephemeris/:body?date=...&days=30&step=1` | Multi-day ephemeris |
| `GET /api/sbdb/search?sstr=Ceres` | Search JPL Small-Body Database (proxy) |
| `POST /api/custom/position` | Compute position from orbital elements |

## Tracking Custom Objects

### CLI — search by name

The `track` command fetches orbital data from NASA JPL's Small-Body Database and displays position, rise/set times, and an ephemeris table:

```bash
rustarium track Ceres --city roma
```

### Web — add to the 3D orrery

Click the **"+ Add"** button in the bottom bar, search for any asteroid, comet, or dwarf planet, and click "Add to Orrery". The object appears in the 3D scene with its orbit, and its position updates in real time. Tracked objects persist across page reloads via localStorage.

### Rust library — N-body simulation

For high-precision work, add objects directly to the N-body integrator:

```rust
use rustarium_core::nbody::{NBodySystem, NBodyObject, OrbitalElements};
use rustarium_core::bodies::SUN_GM;

let elements = OrbitalElements::from_au_and_degrees(
    2.7691,  // semi-major axis (AU)
    0.0760,  // eccentricity
    10.59,   // inclination (deg)
    80.31,   // longitude of ascending node (deg)
    73.60,   // argument of perihelion (deg)
    77.37,   // mean anomaly at epoch (deg)
);
let state = elements.to_state_vector(SUN_GM);
let mut system = NBodySystem::solar_system();
system.add_body(NBodyObject::new("Ceres", 62.6284, state));
system.propagate_to(target_jd, Some(1.0));
```

### Rust library — Keplerian propagation

For quick position lookups without full N-body simulation:

```rust
use rustarium_core::custom_body::{CustomBody, SmallBodyType};
use rustarium_core::nbody::orbital_elements::OrbitalElements;
use rustarium_core::time::jd_from_date;

let body = CustomBody {
    name: "Ceres".into(),
    designation: Some("1".into()),
    body_type: SmallBodyType::DwarfPlanet,
    elements: OrbitalElements::from_au_and_degrees(2.766, 0.0796, 10.59, 80.31, 73.60, 130.0),
    epoch_jd: 2460600.5,
    gm: 62.6284,
    diameter_km: Some(939.4),
    abs_magnitude_h: Some(3.33),
};

let jd = jd_from_date(2026, 4, 12.0);
let pos = body.heliocentric_position(jd);     // ecliptic lon/lat/distance
let eq = body.apparent_equatorial(jd);         // RA/Dec
```

## License

Apache-2.0
