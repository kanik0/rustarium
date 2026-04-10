//! Manual verification: N-body simulation vs VSOP87 analytical positions.
//!
//! Run with: cargo run -p rustarium-core --example verify_nbody
//!
//! This propagates the solar system from J2000.0 forward by a given number
//! of years and compares the N-body result against VSOP87 analytical positions.
//! Agreement within arcminutes over years validates the integrator.

use rustarium_core::bodies::{Planet, AU_KM};
use rustarium_core::coords::Vec3;
use rustarium_core::nbody::NBodySystem;
use rustarium_core::planet;
use rustarium_core::time::{jd_from_date, J2000};
use std::f64::consts::PI;

fn main() {
    println!("=== Rustarium Phase 2 Verification: N-body vs VSOP87 ===");
    println!();

    // Propagate solar system from J2000 to 2010-Jan-01
    let target_jd = jd_from_date(2010, 1, 1.0);
    let years = (target_jd.0 - J2000.0) / 365.25;

    println!("Propagating solar system {:.1} years (J2000 -> 2010-Jan-01)...", years);
    let mut system = NBodySystem::solar_system();
    system.propagate_to(target_jd, None);

    println!("Done. Comparing positions:\n");
    println!(
        "  {:8}  {:>12}  {:>12}  {:>12}  {:>10}",
        "Planet", "NBody (AU)", "VSOP87 (AU)", "Error (km)", "Error (\")"
    );
    println!("  {}", "-".repeat(65));

    let planets_to_check = [
        Planet::Mercury,
        Planet::Venus,
        Planet::Earth,
        Planet::Mars,
        Planet::Jupiter,
        Planet::Saturn,
    ];

    for planet in planets_to_check {
        // N-body position (heliocentric, in km from SSB — need to subtract Sun)
        let nbody_pos = system.body_position(planet.name()).unwrap();
        let sun_pos = system.body_position("Sun").unwrap();
        let nbody_helio = nbody_pos - sun_pos;
        let nbody_dist = nbody_helio.magnitude() / AU_KM;

        // VSOP87 position (heliocentric ecliptic)
        let vsop = planet::heliocentric_position(planet, target_jd);
        let vsop_rect = vsop.to_rectangular(); // in AU
        let vsop_km = Vec3::new(vsop_rect.x * AU_KM, vsop_rect.y * AU_KM, vsop_rect.z * AU_KM);

        // Error
        let error_km = nbody_helio.distance_to(vsop_km);
        let vsop_dist = vsop.distance;

        // Angular error as seen from Sun
        let error_rad = error_km / (vsop_dist * AU_KM);
        let error_arcsec = error_rad * 180.0 * 3600.0 / PI;

        println!(
            "  {:8}  {:>12.6}  {:>12.6}  {:>10.0}  {:>10.1}",
            planet.name(),
            nbody_dist,
            vsop_dist,
            error_km,
            error_arcsec
        );
    }

    println!();
    println!("Notes:");
    println!("  - Errors are expected (different models: N-body Newtonian vs analytical VSOP87)");
    println!("  - Errors < 1000\" (arcminutes range) over 10 years = good");
    println!("  - Mercury may show larger errors (no relativistic corrections in N-body)");
    println!();

    // Now show a 2-year simulation with snapshots
    println!("=== N-body snapshot: 2024-2025, monthly output ===\n");
    let mut sys2 = NBodySystem::solar_system();
    let start = jd_from_date(2024, 1, 1.0);
    let end = jd_from_date(2025, 1, 1.0);

    // First propagate to 2024
    sys2.propagate_to(start, None);

    // Then get monthly snapshots
    let snaps = sys2.propagate_to(end, Some(30.0));

    println!("  {:>12}  {:>10}  {:>10}  {:>10}", "Date", "Mars (AU)", "Jupiter", "Saturn");
    for snap in &snaps {
        let (y, m, d) = rustarium_core::time::date_from_jd(snap.jd);
        let mars = snap.bodies.iter().find(|b| b.name == "Mars").unwrap();
        let jupiter = snap.bodies.iter().find(|b| b.name == "Jupiter").unwrap();
        let saturn = snap.bodies.iter().find(|b| b.name == "Saturn").unwrap();
        println!(
            "  {:4}-{:02}-{:02}    {:>10.4}  {:>10.4}  {:>10.4}",
            y,
            m,
            d as u32,
            mars.distance_au,
            jupiter.distance_au,
            saturn.distance_au
        );
    }
}
