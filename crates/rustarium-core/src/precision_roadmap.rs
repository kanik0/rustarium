//! # Precision Roadmap
//!
//! This module documents planned improvements to increase prediction accuracy.
//! Each improvement is listed with its expected impact and implementation complexity.
//!
//! ## Current precision
//!
//! | Component | Accuracy | Source |
//! |-----------|----------|--------|
//! | Planet positions (analytical) | ~1" (arcsecond) | VSOP87D theory |
//! | Moon position | ~10" | ELP-2000/82 (Meeus truncation, 120 terms) |
//! | N-body simulation (10 years) | ~500" vs VSOP87 | Newtonian gravity, RK45 |
//! | Rise/set times | ~1 minute | Meeus ch.15 algorithm |
//! | Lunar eclipse magnitude | ±0.007 | Shadow geometry |
//! | Solar eclipse detection | Correct type/date | Geocentric approximation |
//!
//! ## Planned improvements (by priority)
//!
//! ### 1. N-body: Post-Newtonian relativistic corrections
//! - **Impact**: Fixes Mercury precession (~43"/century), improves all inner planets
//! - **Method**: Add 1PN (first post-Newtonian) acceleration term from GR
//! - **Expected gain**: N-body error reduced from ~500" to ~50" over 10 years
//! - **Complexity**: Low — one extra acceleration term per body pair
//!
//! ### 2. Moon: Full ELP-2000/82 theory
//! - **Impact**: Improve Moon position from ~10" to ~1" accuracy
//! - **Method**: Include all 37,862 terms instead of current 120-term Meeus truncation
//! - **Expected gain**: 10x improvement in lunar position
//! - **Complexity**: Medium — large coefficient tables but same algorithm
//!
//! ### 3. Solar eclipse: Topocentric computation
//! - **Impact**: Accurate local eclipse visibility, contact times, magnitude per observer
//! - **Method**: Besselian elements + topocentric parallax correction
//! - **Expected gain**: From "is it visible?" to "exact contact times ±1 second"
//! - **Complexity**: Medium — well-documented algorithms (Meeus ch. 54)
//!
//! ### 4. N-body: J2 oblateness perturbation
//! - **Impact**: Better satellite/close-orbit accuracy
//! - **Method**: Add Earth/Jupiter J2 zonal harmonic term
//! - **Expected gain**: Important for Moon and artificial satellites
//! - **Complexity**: Low — one additional force term
//!
//! ### 5. JPL DE440 ephemeris support (optional, desktop only)
//! - **Impact**: Sub-kilometer planet positions, ~1m Moon position
//! - **Method**: Parse SPK/BSP binary files via ANISE crate, feature-gated
//! - **Expected gain**: 100-1000x improvement over analytical theories
//! - **Complexity**: Medium — add optional dependency, binary file parsing
//! - **Note**: NOT WASM-compatible (17MB data file), desktop/CLI only
//!
//! ### 6. Higher-order nutation model
//! - **Impact**: Improve nutation from ~0.5" to ~0.001" accuracy
//! - **Method**: Replace 13-term series with full IAU 2000B (77 terms) or IAU 2000A
//! - **Expected gain**: Better apparent positions, especially for rise/set
//! - **Complexity**: Low — more terms in existing nutation function
//!
//! ### 7. N-body: Asteroid perturbations
//! - **Impact**: Better long-term Mars accuracy
//! - **Method**: Include Ceres, Pallas, Vesta as additional bodies
//! - **Expected gain**: Mars position improved over centuries
//! - **Complexity**: Low — just add 3 more bodies with initial conditions
//!
//! ### 8. Symplectic integrator
//! - **Impact**: Better energy conservation over very long integrations (>100 years)
//! - **Method**: Replace RK45 with Wisdom-Holman or similar symplectic integrator
//! - **Expected gain**: Stable orbits over millennia instead of centuries
//! - **Complexity**: High — fundamentally different integration approach
