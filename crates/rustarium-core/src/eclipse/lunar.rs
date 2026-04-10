use crate::bodies::AU_KM;
use crate::moon;
use crate::sun;
use crate::time::JulianDay;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Type of lunar eclipse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LunarEclipseType {
    Penumbral,
    Partial,
    Total,
}

/// A predicted lunar eclipse with timing and magnitude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LunarEclipse {
    pub eclipse_type: LunarEclipseType,
    /// JD of greatest eclipse (UT)
    pub greatest_eclipse: JulianDay,
    /// Umbral magnitude (>= 1.0 for total, 0..1 for partial, <0 for penumbral only)
    pub umbral_magnitude: f64,
    /// Penumbral magnitude (> 0 means some penumbral contact)
    pub penumbral_magnitude: f64,
    /// First penumbral contact (P1)
    pub p1: JulianDay,
    /// First umbral contact (U1) — None for penumbral-only eclipses
    pub u1: Option<JulianDay>,
    /// Totality begins (U2) — None unless total
    pub u2: Option<JulianDay>,
    /// Totality ends (U3) — None unless total
    pub u3: Option<JulianDay>,
    /// Last umbral contact (U4) — None for penumbral-only eclipses
    pub u4: Option<JulianDay>,
    /// Last penumbral contact (P4)
    pub p4: JulianDay,
}

/// Search for lunar eclipses in a date range.
/// Returns all lunar eclipses (penumbral, partial, total) found between start and end.
pub fn search(start: JulianDay, end: JulianDay) -> Vec<LunarEclipse> {
    let mut eclipses = Vec::new();

    // Iterate through approximate Full Moon dates using the synodic month
    // Synodic month ≈ 29.530588853 days
    let synodic = 29.530588853;

    // Find the first Full Moon near start using Meeus ch. 49 approximation
    let mut jd = find_nearest_full_moon(start);
    if jd.0 < start.0 {
        jd = JulianDay(jd.0 + synodic);
    }

    while jd.0 <= end.0 {
        // Refine Full Moon time
        let full_moon = refine_full_moon(jd);

        // Check if eclipse is possible: Moon's ecliptic latitude must be small
        let moon_ecl = moon::geocentric_ecliptic(full_moon);
        let moon_lat_deg = moon_ecl.latitude.to_degrees().abs();

        // Eclipse is possible if |β| < ~1.9° (generous threshold for penumbral)
        if moon_lat_deg < 1.9 {
            if let Some(eclipse) = compute_lunar_eclipse(full_moon) {
                eclipses.push(eclipse);
            }
        }

        jd = JulianDay(jd.0 + synodic);
    }

    eclipses
}

/// Find approximate time of nearest Full Moon to a given JD.
/// Uses the lunation number approach (Meeus ch. 49).
fn find_nearest_full_moon(jd: JulianDay) -> JulianDay {
    // Approximate: Full Moon of known epoch + N * synodic month
    // Known Full Moon: 2000-Jan-21 04:40 UT = JD 2451564.694
    let known_full = 2451564.694;
    let synodic = 29.530588853;
    let n = ((jd.0 - known_full) / synodic).round();
    JulianDay(known_full + n * synodic)
}

/// Refine Full Moon time by finding the exact moment of opposition
/// (Sun-Earth-Moon alignment in ecliptic longitude).
fn refine_full_moon(approx: JulianDay) -> JulianDay {
    let mut jd = approx;

    for _ in 0..10 {
        let moon_ecl = moon::geocentric_ecliptic(jd);
        let sun_ecl = sun::geocentric_ecliptic(jd);

        // How far is Moon-Sun elongation from 180° (opposition)?
        let mut delta = moon_ecl.longitude - sun_ecl.longitude - PI;
        // Wrap delta to [-π, π]
        while delta > PI {
            delta -= 2.0 * PI;
        }
        while delta < -PI {
            delta += 2.0 * PI;
        }

        if delta.abs() < 1e-8 {
            break;
        }

        // Moon moves ~12.2° per day relative to Sun
        let correction = delta / (12.2_f64.to_radians());
        jd = JulianDay(jd.0 - correction);
    }

    jd
}

/// Compute lunar eclipse geometry at a given Full Moon time.
/// Returns None if no eclipse occurs.
fn compute_lunar_eclipse(full_moon: JulianDay) -> Option<LunarEclipse> {
    let moon_ecl = moon::geocentric_ecliptic(full_moon);
    let sun_ecl = sun::geocentric_ecliptic(full_moon);

    let sun_dist_km = sun_ecl.distance * AU_KM;

    // Earth shadow geometry at the Moon's distance
    // Sun angular semi-diameter (radians)
    let sun_sd = sun::angular_semidiameter(sun_ecl.distance);
    // Moon angular semi-diameter (radians)
    let moon_sd = moon::angular_semidiameter(full_moon);
    // Moon horizontal parallax (radians)
    let moon_par = moon::horizontal_parallax(full_moon);

    // Earth's shadow cone radii at the Moon's distance (Meeus ch. 54)
    // Penumbral radius: f1 = 1.02 * (moon_par + sun_sd + π_sun)
    // Umbral radius:    f2 = 1.02 * (moon_par + sun_sd - π_sun) ... no
    // Actually using Danjon's enlargement factor:
    // Penumbral shadow angular radius (Danjon):
    let pi_sun = (crate::bodies::EARTH_RADIUS_KM / sun_dist_km).asin();

    // Penumbral radius at Moon distance (angular, radians)
    let f1 = 1.02 * (moon_par + pi_sun + sun_sd);
    // Umbral radius at Moon distance (angular, radians)
    let f2 = 1.02 * (moon_par + pi_sun - sun_sd);

    // Moon's distance from the shadow axis = |ecliptic latitude| in radians
    // (approximately, since at opposition the axis is close to the ecliptic)
    let gamma = moon_ecl.latitude.abs();

    // Penumbral magnitude: (f1 + moon_sd - gamma) / (2 * moon_sd)
    let pen_mag = (f1 + moon_sd - gamma) / (2.0 * moon_sd);
    if pen_mag <= 0.0 {
        return None; // No eclipse
    }

    // Umbral magnitude: (f2 + moon_sd - gamma) / (2 * moon_sd)
    let umb_mag = (f2 + moon_sd - gamma) / (2.0 * moon_sd);

    let eclipse_type = if umb_mag >= 1.0 {
        LunarEclipseType::Total
    } else if umb_mag > 0.0 {
        LunarEclipseType::Partial
    } else {
        LunarEclipseType::Penumbral
    };

    // Contact times
    // Moon's angular speed relative to shadow ≈ 0.55"/s ≈ 2.67e-6 rad/s
    let moon_speed = 0.55 / 3600.0 * PI / 180.0; // radians per second
    let moon_speed_day = moon_speed * 86400.0; // radians per day

    // P1/P4: penumbral contacts
    // Distance from axis at contact = f1 + moon_sd (entry) or f1 + moon_sd (exit)
    // Time offset = sqrt((f1+s)² - γ²) / speed ... but γ might change
    // Simplified: use the chord formula
    let pen_half = half_duration(f1, moon_sd, gamma, moon_speed_day);
    let p1 = JulianDay(full_moon.0 - pen_half);
    let p4 = JulianDay(full_moon.0 + pen_half);

    let (u1, u4) = if umb_mag > 0.0 {
        let umb_half = half_duration(f2, moon_sd, gamma, moon_speed_day);
        (
            Some(JulianDay(full_moon.0 - umb_half)),
            Some(JulianDay(full_moon.0 + umb_half)),
        )
    } else {
        (None, None)
    };

    let (u2, u3) = if umb_mag >= 1.0 {
        // Totality: Moon fully inside umbra
        // Entry when leading edge enters = f2 - moon_sd from axis
        let tot_half = half_duration_inner(f2, moon_sd, gamma, moon_speed_day);
        (
            Some(JulianDay(full_moon.0 - tot_half)),
            Some(JulianDay(full_moon.0 + tot_half)),
        )
    } else {
        (None, None)
    };

    Some(LunarEclipse {
        eclipse_type,
        greatest_eclipse: full_moon,
        umbral_magnitude: umb_mag,
        penumbral_magnitude: pen_mag,
        p1,
        u1,
        u2,
        u3,
        u4,
        p4,
    })
}

/// Half-duration for shadow entry/exit (outer contact).
/// The Moon's leading edge touches the shadow edge.
fn half_duration(shadow_radius: f64, moon_sd: f64, gamma: f64, speed: f64) -> f64 {
    let r = shadow_radius + moon_sd;
    if r * r < gamma * gamma {
        return 0.0;
    }
    (r * r - gamma * gamma).sqrt() / speed
}

/// Half-duration for inner contact (totality).
/// The Moon's trailing edge enters the shadow.
fn half_duration_inner(shadow_radius: f64, moon_sd: f64, gamma: f64, speed: f64) -> f64 {
    let r = shadow_radius - moon_sd;
    if r <= 0.0 || r * r < gamma * gamma {
        return 0.0;
    }
    (r * r - gamma * gamma).sqrt() / speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// Search for lunar eclipses in 2024 and verify against known eclipses:
    /// - 2024-Mar-25: Penumbral
    /// - 2024-Sep-18: Partial
    #[test]
    fn find_2024_lunar_eclipses() {
        let start = jd_from_date(2024, 1, 1.0);
        let end = jd_from_date(2024, 12, 31.0);
        let eclipses = search(start, end);

        assert!(
            eclipses.len() >= 2,
            "Expected at least 2 lunar eclipses in 2024, found {}",
            eclipses.len()
        );

        // Check that at least one is in March and one in September
        let (y1, m1, _) = crate::time::date_from_jd(eclipses[0].greatest_eclipse);
        let (y2, m2, _) = crate::time::date_from_jd(eclipses[1].greatest_eclipse);

        assert_eq!(y1, 2024);
        assert_eq!(y2, 2024);
        assert!(
            (m1 == 3 && m2 == 9) || (m1 == 9 && m2 == 3),
            "Expected eclipses in March and September, got months {} and {}",
            m1,
            m2
        );
    }

    /// The September 2024 eclipse should be partial (not penumbral, not total).
    #[test]
    fn sep_2024_partial() {
        let start = jd_from_date(2024, 9, 1.0);
        let end = jd_from_date(2024, 9, 30.0);
        let eclipses = search(start, end);

        assert_eq!(eclipses.len(), 1, "Expected 1 eclipse in Sep 2024");
        assert_eq!(
            eclipses[0].eclipse_type,
            LunarEclipseType::Partial,
            "Sep 2024 should be partial, got {:?}",
            eclipses[0].eclipse_type
        );
    }

    /// Verify that the known total lunar eclipse of 2025-Mar-14 is found.
    #[test]
    fn mar_2025_total() {
        let start = jd_from_date(2025, 3, 1.0);
        let end = jd_from_date(2025, 3, 31.0);
        let eclipses = search(start, end);

        assert_eq!(eclipses.len(), 1, "Expected 1 eclipse in Mar 2025");
        assert_eq!(
            eclipses[0].eclipse_type,
            LunarEclipseType::Total,
            "Mar 2025 should be total, got {:?}",
            eclipses[0].eclipse_type
        );
    }
}
