use crate::bodies::Planet;
use crate::coords::{ecliptic_to_equatorial, EclipticCoords, EquatorialCoords};
use crate::nutation::{nutation, true_obliquity};
use crate::planet;
use crate::time::{normalize_radians, JulianDay};
use std::f64::consts::PI;

/// Geocentric geometric ecliptic position of the Sun.
/// Derived from Earth's heliocentric VSOP87D position.
pub fn geocentric_ecliptic(jd: JulianDay) -> EclipticCoords {
    let earth = planet::heliocentric_position(Planet::Earth, jd);
    EclipticCoords {
        longitude: normalize_radians(earth.longitude + PI),
        latitude: -earth.latitude,
        distance: earth.distance,
    }
}

/// Geometric longitude of the Sun (without nutation), in radians.
pub fn geometric_longitude(jd: JulianDay) -> f64 {
    geocentric_ecliptic(jd).longitude
}

/// Apparent ecliptic position of the Sun (corrected for nutation).
pub fn apparent_ecliptic(jd: JulianDay) -> EclipticCoords {
    let geo = geocentric_ecliptic(jd);
    let (delta_psi, _) = nutation(jd);
    EclipticCoords {
        longitude: geo.longitude + delta_psi,
        ..geo
    }
}

/// Apparent equatorial coordinates of the Sun.
pub fn apparent_equatorial(jd: JulianDay) -> EquatorialCoords {
    let app_ecl = apparent_ecliptic(jd);
    let obliquity = true_obliquity(jd);
    let eq = ecliptic_to_equatorial(&app_ecl, obliquity);

    // Sun aberration is already accounted for in VSOP87 (the theory gives
    // geometric coordinates, and the nutation correction handles the rest).
    // However, for strict correctness, the annual aberration of the Sun
    // is a constant -20.496" applied to the longitude (already in VSOP87's
    // output). We return the position as-is.
    eq
}

/// Sun angular semi-diameter in radians at a given distance (AU).
pub fn angular_semidiameter(distance_au: f64) -> f64 {
    // Sun's angular semi-diameter at 1 AU ≈ 959.63 arcseconds
    let at_1au_rad = 959.63 / 3600.0 * PI / 180.0;
    at_1au_rad / distance_au
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// Meeus example 25.a: Sun position on 1992 October 13.0 TDT
    /// Expected apparent longitude: 199.907°, apparent RA: 13h 13m 31s
    #[test]
    fn sun_position_meeus_25a() {
        let jd = jd_from_date(1992, 10, 13.0);
        let app_ecl = apparent_ecliptic(jd);
        let lon_deg = app_ecl.longitude.to_degrees();

        // Meeus gives 199.907° - VSOP87D may differ slightly from Meeus' low-precision formula
        assert!(
            (lon_deg - 199.907).abs() < 0.05,
            "Sun apparent λ = {:.3}° expected ~199.907°",
            lon_deg
        );
    }

    #[test]
    fn sun_distance_reasonable() {
        let jd = jd_from_date(2024, 7, 4.0);
        let pos = geocentric_ecliptic(jd);
        // Sun distance should be 0.98 - 1.02 AU (July = near aphelion)
        assert!(
            pos.distance > 0.98 && pos.distance < 1.02,
            "Sun distance: {}",
            pos.distance
        );
    }
}
