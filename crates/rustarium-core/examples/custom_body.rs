//! Demonstrates adding a custom body (asteroid Ceres) to the N-body simulation.
//!
//! Run with: cargo run -p rustarium-core --example custom_body
//!
//! This shows how to:
//! 1. Create a body from Keplerian orbital elements (the easy way)
//! 2. Create a body from a Cartesian state vector (the precise way)
//! 3. Run a simulation with the custom body
//!
//! You can find orbital elements for any asteroid at:
//!   https://ssd.jpl.nasa.gov/tools/sbdb_lookup.html

use rustarium_core::bodies::{AU_KM, SUN_GM};
use rustarium_core::coords::Vec3;
use rustarium_core::nbody::{NBodyObject, NBodySystem, OrbitalElements, StateVector};
use rustarium_core::time::jd_from_date;

fn main() {
    println!("=== Adding Custom Bodies to Rustarium ===\n");

    // --- Method 1: From Keplerian orbital elements ---
    // Data for Ceres from JPL Small-Body Database
    // Epoch: J2000.0 (approximate)
    println!("Method 1: Create from orbital elements (easiest)");
    println!("  Data source: JPL SBDB (https://ssd.jpl.nasa.gov/tools/sbdb_lookup.html)\n");

    let ceres_elements = OrbitalElements::from_au_and_degrees(
        2.7691651,   // semi-major axis (AU)
        0.0760090,   // eccentricity
        10.5935,     // inclination (°)
        80.3055,     // longitude of ascending node (°)
        73.5977,     // argument of perihelion (°)
        77.372,      // mean anomaly at epoch (°)
    );

    // GM of Ceres: 62.6284 km³/s² (IAU 2015)
    let ceres_gm = 62.6284;
    let ceres_state = ceres_elements.to_state_vector(SUN_GM);

    let ceres = NBodyObject::new("Ceres", ceres_gm, ceres_state);
    println!("  Ceres initial state:");
    println!("    Position: ({:.0}, {:.0}, {:.0}) km", ceres.state.position.x, ceres.state.position.y, ceres.state.position.z);
    println!("    Velocity: ({:.4}, {:.4}, {:.4}) km/s", ceres.state.velocity.x, ceres.state.velocity.y, ceres.state.velocity.z);
    println!("    Distance: {:.4} AU", ceres.state.position.magnitude() / AU_KM);
    println!();

    // --- Method 2: From Cartesian state vector ---
    println!("Method 2: Create from Cartesian state vector (precise)");
    println!("  Use JPL Horizons to get exact vectors at your epoch.\n");

    let halley = NBodyObject::new(
        "Halley",
        0.0, // negligible mass
        StateVector::new(
            // Example position (not real Halley data)
            Vec3::new(-3.5e9, 2.1e9, -1.2e8),
            Vec3::new(-1.5, -3.2, 0.8),
        ),
    );
    println!("  Halley initial distance: {:.4} AU\n", halley.state.position.magnitude() / AU_KM);

    // --- Run simulation with custom body ---
    println!("=== Simulating solar system + Ceres for 1 year ===\n");

    let mut system = NBodySystem::solar_system();
    system.add_body(NBodyObject::new("Ceres", ceres_gm, ceres_state));

    println!("  Bodies in simulation: {}", system.bodies.len());
    for b in &system.bodies {
        println!("    - {} (GM={:.4})", b.name, b.gm);
    }
    println!();

    // Propagate to 2001-Jan-01 (1 year from J2000.0)
    let target = jd_from_date(2001, 1, 1.0);
    let snaps = system.propagate_to(target, Some(30.0));

    println!("  {:>12}  {:>12}  {:>12}", "Date", "Ceres (AU)", "Earth (AU)");
    for snap in &snaps {
        let (y, m, d) = rustarium_core::time::date_from_jd(snap.jd);
        let ceres = snap.bodies.iter().find(|b| b.name == "Ceres").unwrap();
        let earth = snap.bodies.iter().find(|b| b.name == "Earth").unwrap();
        println!(
            "  {:4}-{:02}-{:02}    {:>10.4}    {:>10.4}",
            y, m, d as u32,
            ceres.distance_au,
            earth.distance_au
        );
    }

    println!();
    println!("=== How to add your own object ===");
    println!();
    println!("1. Find orbital elements at https://ssd.jpl.nasa.gov/tools/sbdb_lookup.html");
    println!("2. Use OrbitalElements::from_au_and_degrees(a, e, i, Omega, omega, M)");
    println!("3. Convert: let state = elements.to_state_vector(SUN_GM);");
    println!("4. Create: let body = NBodyObject::new(\"Name\", gm, state);");
    println!("5. Add:    system.add_body(body);");
}
