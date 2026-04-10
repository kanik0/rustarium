# Rustarium

**Solar System Prediction Engine** — a Rust program that predicts positions of solar system objects, generates astronomical reports, and provides a 3D web visualization. All computation runs as WebAssembly, either server-side (Cloudflare Worker) or entirely client-side in the browser.

## Features

- **Planet positions** — heliocentric and geocentric coordinates for all 8 planets using VSOP87D analytical theory (~1 arcsecond accuracy)
- **Moon position** — ELP-2000/82 theory with planetary perturbations (~5 arcsecond accuracy)
- **Rise/set times** — sunrise, sunset, transit times for any body from any location on Earth
- **Eclipse prediction** — lunar and solar eclipses with contact times, magnitude, and local visibility
- **N-body simulation** — gravitational simulation with Dormand-Prince RK45 and Wisdom-Holman symplectic integrators
- **Custom objects** — add asteroids, comets, or spacecraft via orbital elements or state vectors
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
```

### Web (Client-Side WASM)

```bash
# Build the WASM module
cd crates/rustarium-wasm
wasm-pack build --target web --release

# Copy to site directory
cp pkg/rustarium_wasm.js site/pkg/
cp pkg/rustarium_wasm_bg.wasm site/pkg/

# Serve locally
python3 -m http.server 8080 --directory site
# Open http://localhost:8080
```

### Deploy to Cloudflare Pages

```bash
cd crates/rustarium-wasm
npx wrangler pages deploy site/ --project-name rustarium
```

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

### Data Sources

All data is embedded in the source code — no external files are downloaded at runtime.

| Data | Source | Epoch |
|------|--------|-------|
| Planet positions | VSOP87D analytical theory (via `vsop87` crate) | Continuous |
| Moon position | ELP-2000/82 (Meeus ch. 47 + planetary perturbations) | Continuous |
| N-body initial conditions | JPL Horizons API (DE441) | J2000.0 |
| Nutation | IAU 2000B (77 lunisolar terms) | Continuous |
| Asteroid state vectors | JPL Horizons API (DE441) | J2000.0 |

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

## Adding Custom Objects

```rust
use rustarium_core::nbody::{NBodySystem, NBodyObject, OrbitalElements};
use rustarium_core::bodies::SUN_GM;

// From orbital elements (easiest — data from JPL SBDB)
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

## License

MIT OR Apache-2.0
