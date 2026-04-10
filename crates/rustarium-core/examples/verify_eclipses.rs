//! Manual verification: eclipse predictions.
//!
//! Run with: cargo run -p rustarium-core --example verify_eclipses
//!
//! Compare against:
//!   - NASA Eclipse catalog: https://eclipse.gsfc.nasa.gov/
//!   - timeanddate.com eclipse list: https://www.timeanddate.com/eclipse/
//!
//! Lunar eclipses 2024-2026 (NASA):
//!   2024-Mar-25  Penumbral
//!   2024-Sep-18  Partial     (umb mag 0.085)
//!   2025-Mar-14  Total       (umb mag 1.178)
//!   2025-Sep-07  Total       (umb mag 1.362)
//!   2026-Mar-03  Total       (umb mag 1.151)
//!
//! Solar eclipses 2024-2026 (NASA):
//!   2024-Apr-08  Total
//!   2024-Oct-02  Annular
//!   2025-Mar-29  Partial
//!   2025-Sep-21  Partial
//!   2026-Feb-17  Annular

use rustarium_core::coords::GeoLocation;
use rustarium_core::eclipse::{lunar, solar};
use rustarium_core::time::{date_from_jd, jd_from_date};

fn format_jd(jd: rustarium_core::time::JulianDay) -> String {
    let (y, m, d) = date_from_jd(jd);
    let day = d as u32;
    let frac = d - d.floor();
    let h = (frac * 24.0).floor() as u32;
    let min = ((frac * 24.0 - h as f64) * 60.0).floor() as u32;
    format!("{:4}-{:02}-{:02} {:02}:{:02} UT", y, m, day, h, min)
}

fn main() {
    println!("=== Rustarium Phase 4 Verification: Eclipse Predictions ===\n");

    // --- Lunar eclipses 2024-2026 ---
    println!("--- Lunar eclipses 2024-2026 ---");
    println!("  (Compare with https://eclipse.gsfc.nasa.gov/LEcat5/LE2021-2030.html)\n");

    let start = jd_from_date(2024, 1, 1.0);
    let end = jd_from_date(2026, 12, 31.0);
    let lunar_eclipses = lunar::search(start, end);

    println!(
        "  {:>18}  {:>10}  {:>8}  {:>8}  {:>18}  {:>18}",
        "Greatest Eclipse", "Type", "Umb Mag", "Pen Mag", "P1 (start)", "P4 (end)"
    );
    for e in &lunar_eclipses {
        let type_str = match e.eclipse_type {
            lunar::LunarEclipseType::Penumbral => "Penumbral",
            lunar::LunarEclipseType::Partial => "Partial",
            lunar::LunarEclipseType::Total => "Total",
        };
        println!(
            "  {:>18}  {:>10}  {:>8.3}  {:>8.3}  {:>18}  {:>18}",
            format_jd(e.greatest_eclipse),
            type_str,
            e.umbral_magnitude,
            e.penumbral_magnitude,
            format_jd(e.p1),
            format_jd(e.p4),
        );

        // Print contact times for non-penumbral eclipses
        if let (Some(u1), Some(u4)) = (e.u1, e.u4) {
            println!(
                "  {:>18}  {:>10}  {:>18}  {:>18}",
                "", "Umbral:", format_jd(u1), format_jd(u4)
            );
        }
        if let (Some(u2), Some(u3)) = (e.u2, e.u3) {
            println!(
                "  {:>18}  {:>10}  {:>18}  {:>18}",
                "", "Totality:", format_jd(u2), format_jd(u3)
            );
        }
    }

    println!("\n  Found {} lunar eclipses (NASA catalog: 5 in 2024-2026)\n", lunar_eclipses.len());

    // --- Solar eclipses 2024-2026 ---
    println!("--- Solar eclipses 2024-2026 ---");
    println!("  (Compare with https://eclipse.gsfc.nasa.gov/SEcat5/SE2021-2030.html)\n");

    let solar_eclipses = solar::search(start, end);

    println!(
        "  {:>18}  {:>10}  {:>8}  {:>8}",
        "Greatest Eclipse", "Type", "Gamma", "Mag"
    );
    for e in &solar_eclipses {
        let type_str = match e.eclipse_type {
            solar::SolarEclipseType::Partial => "Partial",
            solar::SolarEclipseType::Annular => "Annular",
            solar::SolarEclipseType::Total => "Total",
            solar::SolarEclipseType::Hybrid => "Hybrid",
        };
        println!(
            "  {:>18}  {:>10}  {:>8.4}  {:>8.3}",
            format_jd(e.greatest_eclipse),
            type_str,
            e.gamma,
            e.magnitude,
        );
    }

    println!("\n  Found {} solar eclipses (NASA catalog: 5 in 2024-2026)\n", solar_eclipses.len());

    // --- Local visibility check ---
    println!("--- Local visibility: Solar eclipses from specific cities ---\n");

    let cities = [
        ("Rome, Italy", GeoLocation::from_degrees(41.90, 12.50, 21.0)),
        ("Dallas, TX", GeoLocation::from_degrees(32.78, -96.80, 130.0)),
        ("Madrid, Spain", GeoLocation::from_degrees(40.42, -3.70, 650.0)),
        ("Tokyo, Japan", GeoLocation::from_degrees(35.68, 139.69, 40.0)),
    ];

    for eclipse in &solar_eclipses {
        let (y, m, d) = date_from_jd(eclipse.greatest_eclipse);
        println!(
            "  Eclipse {:4}-{:02}-{:02} ({:?}):",
            y, m, d as u32, eclipse.eclipse_type
        );

        for (name, loc) in &cities {
            match solar::local_circumstances(eclipse, loc) {
                Some(local) => {
                    println!(
                        "    {:16} Visible! mag={:.3} obscur={:.1}% C1={} max={} C4={}",
                        name,
                        local.magnitude,
                        local.obscuration * 100.0,
                        format_jd(local.c1),
                        format_jd(local.maximum),
                        format_jd(local.c4),
                    );
                }
                None => {
                    println!("    {:16} Not visible", name);
                }
            }
        }
        println!();
    }
}
