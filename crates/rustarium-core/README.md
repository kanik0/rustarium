# rustarium-core

Core computation library for the Rustarium solar system prediction engine.

Compiles to both native and `wasm32-unknown-unknown`. Zero filesystem or network dependencies — all astronomical data is embedded as constants.

## Usage

```rust
use rustarium_core::planet;
use rustarium_core::bodies::Planet;
use rustarium_core::time::jd_from_date;
use rustarium_core::coords::format_ra;

let jd = jd_from_date(2025, 6, 15.0);
let eq = planet::apparent_equatorial(Planet::Mars, jd);
println!("Mars RA: {}", format_ra(eq.ra));
```

## Verification

Run the examples to verify against known data:

```bash
cargo run --example verify_positions      # Compare with JPL Horizons
cargo run --example verify_nbody          # N-body vs VSOP87 cross-validation
cargo run --example verify_moon_riseset   # Moon position + rise/set times
cargo run --example verify_eclipses       # Eclipse predictions vs NASA catalog
cargo run --example custom_body           # Adding asteroids to the N-body simulation
cargo run --example track_asteroid        # Track asteroids/comets via SBDB data
```
