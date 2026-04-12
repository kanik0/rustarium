//! Test the new custom body tracking feature end-to-end.
//!
//! Simulates the full workflow: SBDB JSON parsing -> Keplerian propagation -> positions + rise/set.
//!
//! Run with: cargo run -p rustarium-core --example track_asteroid

use rustarium_core::coords::{format_dec, format_ra, GeoLocation};
use rustarium_core::rise_set;
use rustarium_core::sbdb::SbdbResponse;
use rustarium_core::time::{date_from_jd, jd_from_date, JulianDay};

fn main() {
    println!("=== Custom Body Tracking Test ===\n");

    // 1. Parse real SBDB response for Ceres
    test_body("Ceres (Dwarf Planet)", CERES_SBDB);

    // 2. Parse a comet
    test_body("Halley's Comet", HALLEY_SBDB);

    // 3. Test a near-Earth asteroid
    test_body("Apophis", APOPHIS_SBDB);
}

fn test_body(title: &str, sbdb_json: &str) {
    println!("--- {} ---\n", title);

    let resp: SbdbResponse = serde_json::from_str(sbdb_json).expect("parse SBDB JSON");
    let body = resp.to_custom_body().expect("convert to CustomBody");

    println!("  Name:        {}", body.name);
    println!("  Type:        {}", body.body_type.name());
    println!("  Designation: {}", body.designation.as_deref().unwrap_or("-"));
    println!("  Epoch JD:    {:.1}", body.epoch_jd());
    if let Some(el) = body.elements() {
        println!(
            "  a = {:.4} AU, e = {:.6}, i = {:.2}°",
            el.semi_major_axis_km / rustarium_core::bodies::AU_KM,
            el.eccentricity,
            el.inclination_rad.to_degrees()
        );
    }
    if let Some(d) = body.diameter_km {
        println!("  Diameter:    {:.1} km", d);
    }
    println!();

    // Compute positions for a few dates
    let dates = [
        (2025, 1, 1.0),
        (2025, 7, 1.0),
        (2026, 1, 1.0),
        (2026, 4, 12.0), // today
    ];

    println!(
        "  {:>12}  {:>8}  {:>8}  {:>14}  {:>14}  {:>8}",
        "Date", "Dist AU", "Sun AU", "RA", "Dec", "Geo AU"
    );
    for (y, m, d) in dates {
        let jd = jd_from_date(y, m, d);
        let helio = body.heliocentric_position(jd);
        let eq = body.apparent_equatorial(jd);
        let geo = body.geocentric_position(jd);

        println!(
            "  {:4}-{:02}-{:02}  {:>8.4}  {:>8.4}  {:>14}  {:>14}  {:>8.4}",
            y,
            m,
            d as u32,
            helio.distance,
            helio.distance,
            format_ra(eq.ra),
            format_dec(eq.dec),
            geo.distance
        );
    }
    println!();

    // Rise/set for today from Rome
    let jd = jd_from_date(2026, 4, 12.0);
    let jd_0h = JulianDay((jd.0 - 0.5).floor() + 0.5);
    let rome = GeoLocation::from_degrees(41.9028, 12.4964, 0.0);
    let h0 = (-0.5667_f64).to_radians();

    let eq_fn = |jd: JulianDay| body.apparent_equatorial(jd);
    match rise_set::rise_transit_set_custom(jd_0h, &rome, h0, eq_fn) {
        Ok(events) => {
            println!("  Rise/Set from Rome (2026-04-12):");
            for ev in &events {
                let (y, m, d) = date_from_jd(ev.jd);
                let frac = d - d.floor();
                let h = (frac * 24.0).floor() as u32;
                let min = ((frac * 24.0 - h as f64) * 60.0).floor() as u32;
                let event_name = match ev.event {
                    rise_set::EventType::Rise => "Rise   ",
                    rise_set::EventType::Transit => "Transit",
                    rise_set::EventType::Set => "Set    ",
                };
                print!("    {} {:02}:{:02} UT", event_name, h, min);
                if let Some(az) = ev.azimuth_deg {
                    print!("  az={:.1}°", az);
                }
                if let Some(alt) = ev.altitude_deg {
                    print!("  alt={:.1}°", alt);
                }
                println!();
            }
        }
        Err(e) => println!("  Rise/Set: {:?}", e),
    }
    println!();
}

// Real SBDB-format JSON for test bodies

const CERES_SBDB: &str = r#"{
    "object": {"fullname": "1 Ceres", "des": "1", "name": "Ceres", "kind": "an", "spkid": "2000001"},
    "orbit": {
        "epoch": "2460600.5",
        "elements": [
            {"name": "e", "value": "0.07600902910070946"},
            {"name": "a", "value": "2.766044736305795"},
            {"name": "i", "value": "10.59351035990559"},
            {"name": "om", "value": "80.30554898681753"},
            {"name": "w", "value": "73.59764315927306"},
            {"name": "ma", "value": "130.036"}
        ]
    },
    "phys_par": [
        {"name": "diameter", "value": "939.4"},
        {"name": "GM", "value": "62.6284"},
        {"name": "H", "value": "3.33"}
    ]
}"#;

const HALLEY_SBDB: &str = r#"{
    "object": {"fullname": "1P/Halley", "des": "1P", "name": "Halley", "kind": "cn"},
    "orbit": {
        "epoch": "2449400.5",
        "elements": [
            {"name": "e", "value": "0.9671429085"},
            {"name": "a", "value": "17.83414429"},
            {"name": "i", "value": "162.2626906"},
            {"name": "om", "value": "58.42008098"},
            {"name": "w", "value": "111.3324851"},
            {"name": "ma", "value": "38.3842644"}
        ]
    },
    "phys_par": [{"name": "diameter", "value": "11"}]
}"#;

const APOPHIS_SBDB: &str = r#"{
    "object": {"fullname": "99942 Apophis (2004 MN4)", "des": "99942", "name": "Apophis", "kind": "an"},
    "orbit": {
        "epoch": "2460600.5",
        "elements": [
            {"name": "e", "value": "0.1914"},
            {"name": "a", "value": "0.9224"},
            {"name": "i", "value": "3.339"},
            {"name": "om", "value": "204.446"},
            {"name": "w", "value": "126.393"},
            {"name": "ma", "value": "215.538"}
        ]
    },
    "phys_par": [{"name": "diameter", "value": "0.370"}, {"name": "H", "value": "19.7"}]
}"#;
