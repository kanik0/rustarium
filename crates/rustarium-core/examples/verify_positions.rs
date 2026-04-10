//! Manual verification: planet positions vs JPL Horizons.
//!
//! Run with: cargo run -p rustarium-core --example verify_positions
//!
//! Compare the output against JPL Horizons:
//!   https://ssd.jpl.nasa.gov/horizons/app.html
//!
//! Settings for Horizons comparison:
//!   Ephemeris Type: Observer Table
//!   Target Body: [select planet]
//!   Observer Location: Geocentric (code 500)
//!   Time: [use dates from output]
//!   Table Settings: check "1. Astrometric RA & DEC" and "20. Range & range-rate"

use rustarium_core::bodies::Planet;
use rustarium_core::coords::{format_dec, format_ra};
use rustarium_core::planet;
use rustarium_core::sun;
use rustarium_core::time::{jd_from_date, jd_from_datetime};

fn main() {
    println!("=== Rustarium Phase 1 Verification ===");
    println!("Compare these values against JPL Horizons (https://ssd.jpl.nasa.gov/horizons/)");
    println!();

    // -- Test 1: Known date from Meeus --
    println!("--- Test 1: Venus 1992-Dec-20.0 TDT (Meeus example 25.b) ---");
    let jd = jd_from_date(1992, 12, 20.0);
    let helio = planet::heliocentric_position(Planet::Venus, jd);
    println!("  Heliocentric ecliptic:");
    println!("    L = {:.5}°  (Meeus: 26.11428°)", helio.longitude.to_degrees());
    println!("    B = {:.5}°  (Meeus: -2.62070°)", helio.latitude.to_degrees());
    println!("    R = {:.6} AU (Meeus: 0.724603)", helio.distance);
    println!();

    // -- Test 2: Sun position --
    println!("--- Test 2: Sun position 1992-Oct-13.0 TDT (Meeus example 25.a) ---");
    let jd = jd_from_date(1992, 10, 13.0);
    let sun_ecl = sun::apparent_ecliptic(jd);
    let sun_eq = sun::apparent_equatorial(jd);
    println!("  Apparent ecliptic longitude: {:.3}° (Meeus: 199.907°)", sun_ecl.longitude.to_degrees());
    println!("  Apparent RA:  {} ", format_ra(sun_eq.ra));
    println!("  Apparent Dec: {}", format_dec(sun_eq.dec));
    println!("  Distance:     {:.6} AU", sun_ecl.distance);
    println!();

    // -- Test 3: Current-era planet positions (verify against Horizons) --
    println!("--- Test 3: Planet positions on 2024-Jan-01 0h TT ---");
    println!("           (compare with JPL Horizons, geocentric, J2000)");
    let jd = jd_from_datetime(2024, 1, 1, 0, 0, 0.0);
    println!("  JD = {:.1}", jd.0);
    println!();

    for planet in Planet::ALL {
        if planet == Planet::Earth {
            continue; // Earth geocentric = (0,0,0)
        }
        let eq = planet::apparent_equatorial(planet, jd);
        let geo = planet::geocentric_position(planet, jd);
        println!(
            "  {:8}  RA: {}  Dec: {}  Dist: {:.4} AU",
            planet.name(),
            format_ra(eq.ra),
            format_dec(eq.dec),
            geo.distance
        );
    }
    println!();

    // -- Test 4: Sun at solstice/equinox --
    println!("--- Test 4: Sun at 2024 equinoxes/solstices ---");
    for (label, y, m, d) in [
        ("Spring equinox ~Mar 20", 2024, 3, 20.0),
        ("Summer solstice ~Jun 20", 2024, 6, 20.0),
        ("Autumn equinox ~Sep 22", 2024, 9, 22.0),
        ("Winter solstice ~Dec 21", 2024, 12, 21.0),
    ] {
        let jd = jd_from_date(y, m, d);
        let sun_ecl = sun::apparent_ecliptic(jd);
        println!(
            "  {}: Sun longitude = {:.2}°",
            label,
            sun_ecl.longitude.to_degrees()
        );
    }
    println!("  (Expected: ~0°, ~90°, ~180°, ~270° at exact moments)");
    println!();

    // -- Test 5: All planet heliocentric distances --
    println!("--- Test 5: Heliocentric distances 2024-Jul-04 ---");
    println!("           (compare with known orbital radii in AU)");
    let jd = jd_from_date(2024, 7, 4.0);
    let expected = [
        ("Mercury", 0.31, 0.47),
        ("Venus", 0.718, 0.728),
        ("Earth", 0.98, 1.02),
        ("Mars", 1.38, 1.67),
        ("Jupiter", 4.95, 5.46),
        ("Saturn", 9.02, 10.05),
        ("Uranus", 18.3, 20.1),
        ("Neptune", 29.8, 30.3),
    ];
    for (planet, (name, min_au, max_au)) in Planet::ALL.iter().zip(expected.iter()) {
        let pos = planet::heliocentric_position(*planet, jd);
        let ok = pos.distance >= *min_au && pos.distance <= *max_au;
        println!(
            "  {:8}: {:.4} AU  [{:.2}-{:.2}] {}",
            name,
            pos.distance,
            min_au,
            max_au,
            if ok { "OK" } else { "FAIL" }
        );
    }
}
