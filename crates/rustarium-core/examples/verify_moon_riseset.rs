//! Manual verification: Moon position and rise/set times.
//!
//! Run with: cargo run -p rustarium-core --example verify_moon_riseset
//!
//! Compare Moon positions against:
//!   - JPL Horizons: https://ssd.jpl.nasa.gov/horizons/
//!   - Stellarium (free planetarium software)
//!
//! Compare rise/set times against:
//!   - timeanddate.com (search for "Sun rise set [city]" or "Moon rise set [city]")
//!   - USNO: https://aa.usno.navy.mil/data/RS_OneDay
//!
//! NOTE: Rise/set times are in UT (UTC). Add timezone offset for local time.
//!       Rome (CET) = UT + 1h, (CEST summer) = UT + 2h

use rustarium_core::bodies::{Body, Planet};
use rustarium_core::coords::{format_dec, format_ra, GeoLocation};
use rustarium_core::moon;
use rustarium_core::planet;
use rustarium_core::rise_set::{self, EventType};
use rustarium_core::sun;
use rustarium_core::time::{date_from_jd, jd_from_date};

fn format_jd_time(jd: rustarium_core::time::JulianDay) -> String {
    let (_, _, day) = date_from_jd(jd);
    let frac = day - day.floor();
    let hours = frac * 24.0;
    let h = hours.floor() as u32;
    let m = ((hours - h as f64) * 60.0).floor() as u32;
    let s = ((hours - h as f64) * 60.0 - m as f64) * 60.0;
    format!("{:02}:{:02}:{:04.1} UT", h, m, s)
}

fn main() {
    println!("=== Rustarium Phase 3 Verification: Moon + Rise/Set ===\n");

    // --- Moon position test ---
    println!("--- Test 1: Moon position 1992-Apr-12 (Meeus example 47.a) ---");
    let jd = jd_from_date(1992, 4, 12.0);
    let moon_ecl = moon::geocentric_ecliptic(jd);
    let moon_eq = moon::apparent_equatorial(jd);
    println!("  Geocentric ecliptic:");
    println!("    λ = {:.4}°  (Meeus: 133.1627°)", moon_ecl.longitude.to_degrees());
    println!("    β = {:.4}°  (Meeus: -3.2291°)", moon_ecl.latitude.to_degrees());
    println!("    Δ = {:.1} km (Meeus: 368409.7 km)", moon_ecl.distance);
    println!("  Equatorial: RA = {}, Dec = {}", format_ra(moon_eq.ra), format_dec(moon_eq.dec));
    println!();

    // --- Moon for a current-era date ---
    println!("--- Test 2: Moon position 2024-Jan-01 (compare with Horizons/Stellarium) ---");
    let jd = jd_from_date(2024, 1, 1.0);
    let moon_eq = moon::apparent_equatorial(jd);
    let illum = moon::illuminated_fraction(jd);
    let par = moon::horizontal_parallax(jd);
    let semi = moon::angular_semidiameter(jd);
    println!("  RA:  {}", format_ra(moon_eq.ra));
    println!("  Dec: {}", format_dec(moon_eq.dec));
    println!("  Distance: {:.0} km", moon::geocentric_ecliptic(jd).distance);
    println!("  Illuminated fraction: {:.1}%", illum * 100.0);
    println!("  Horizontal parallax: {:.2}'", par.to_degrees() * 60.0);
    println!("  Angular semi-diameter: {:.2}'", semi.to_degrees() * 60.0);
    println!();

    // --- Moon phases through January 2024 ---
    println!("--- Test 3: Moon illumination through January 2024 ---");
    println!("  (Compare with timeanddate.com Moon phase calendar)");
    for day in (1..=31).step_by(2) {
        let jd = jd_from_date(2024, 1, day as f64);
        let frac = moon::illuminated_fraction(jd);
        let bar_len = (frac * 20.0).round() as usize;
        let bar: String = "█".repeat(bar_len) + &"░".repeat(20 - bar_len);
        println!("  Jan {:2}: {:.0}% {}", day, frac * 100.0, bar);
    }
    println!("  (New Moon ~Jan 11, Full Moon ~Jan 25)");
    println!();

    // --- Rise/Set times ---
    let rome = GeoLocation::from_degrees(41.9028, 12.4964, 21.0);
    println!("--- Test 4: Sun rise/set from Rome, 2024-Jun-21 (summer solstice) ---");
    println!("  (Compare with timeanddate.com/sun/italy/rome)");

    let jd = jd_from_date(2024, 6, 21.0);
    match rise_set::rise_transit_set(jd, &rome, Body::Sun, |jd| sun::apparent_equatorial(jd)) {
        Ok(events) => {
            for ev in &events {
                let time_str = format_jd_time(ev.jd);
                match ev.event {
                    EventType::Rise => println!(
                        "  Rise:    {} (az {:.1}°)",
                        time_str,
                        ev.azimuth_deg.unwrap_or(0.0)
                    ),
                    EventType::Transit => println!(
                        "  Transit: {} (alt {:.1}°)",
                        time_str,
                        ev.altitude_deg.unwrap_or(0.0)
                    ),
                    EventType::Set => println!(
                        "  Set:     {} (az {:.1}°)",
                        time_str,
                        ev.azimuth_deg.unwrap_or(0.0)
                    ),
                }
            }
            // Expected for Rome Jun 21: rise ~03:35 UT, transit ~11:13 UT, set ~18:50 UT
            println!("  Expected (approx): Rise ~03:35, Transit ~11:13, Set ~18:50 UT");
        }
        Err(e) => println!("  Error: {:?}", e),
    }
    println!();

    // --- Planet rise/set ---
    println!("--- Test 5: Planet rise/set from Rome, 2024-Jun-15 ---");
    println!("  (Compare with timeanddate.com/astronomy/italy/rome)\n");

    let jd = jd_from_date(2024, 6, 15.0);
    for planet in [Planet::Venus, Planet::Mars, Planet::Jupiter, Planet::Saturn] {
        print!("  {:8}: ", planet.name());
        match rise_set::rise_transit_set(jd, &rome, Body::Planet(planet), |jd| {
            planet::apparent_equatorial(planet, jd)
        }) {
            Ok(events) => {
                let parts: Vec<String> = events
                    .iter()
                    .map(|ev| {
                        let t = format_jd_time(ev.jd);
                        match ev.event {
                            EventType::Rise => format!("Rise {}", t),
                            EventType::Transit => format!("Transit {}", t),
                            EventType::Set => format!("Set {}", t),
                        }
                    })
                    .collect();
                println!("{}", parts.join("  |  "));
            }
            Err(e) => println!("{:?}", e),
        }
    }
    println!();

    // --- Moon rise/set ---
    println!("--- Test 6: Moon rise/set from Rome, 2024-Jun-15 ---");
    let jd = jd_from_date(2024, 6, 15.0);
    match rise_set::rise_transit_set(jd, &rome, Body::Moon, |jd| moon::apparent_equatorial(jd)) {
        Ok(events) => {
            for ev in &events {
                let t = format_jd_time(ev.jd);
                match ev.event {
                    EventType::Rise => println!("  Rise:    {}", t),
                    EventType::Transit => println!("  Transit: {}", t),
                    EventType::Set => println!("  Set:     {}", t),
                }
            }
        }
        Err(e) => println!("  Error: {:?}", e),
    }
    println!("  (Compare with timeanddate.com/moon/italy/rome)");
    println!();

    // --- Multi-day sunrise table ---
    println!("--- Test 7: Sunrise times Rome, June 2024 (first week) ---");
    println!("  {:>6}  {:>14}  {:>14}  {:>14}", "Date", "Sunrise UT", "Transit UT", "Sunset UT");
    for day in 1..=7 {
        let jd = jd_from_date(2024, 6, day as f64);
        if let Ok(events) =
            rise_set::rise_transit_set(jd, &rome, Body::Sun, |jd| sun::apparent_equatorial(jd))
        {
            let rise = events.iter().find(|e| e.event == EventType::Rise).map(|e| format_jd_time(e.jd)).unwrap_or_default();
            let transit = events.iter().find(|e| e.event == EventType::Transit).map(|e| format_jd_time(e.jd)).unwrap_or_default();
            let set = events.iter().find(|e| e.event == EventType::Set).map(|e| format_jd_time(e.jd)).unwrap_or_default();
            println!("  Jun {:2}  {:>14}  {:>14}  {:>14}", day, rise, transit, set);
        }
    }
    println!("  (Add +2h for CEST local time)");
}
