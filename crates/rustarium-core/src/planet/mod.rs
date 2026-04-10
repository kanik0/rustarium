mod vsop87_wrapper;

use crate::aberration::annual_aberration;
use crate::bodies::Planet;
use crate::coords::{heliocentric_to_geocentric, ecliptic_to_equatorial, EclipticCoords, EquatorialCoords};
use crate::nutation::{nutation, true_obliquity};
use crate::time::JulianDay;

/// Compute heliocentric ecliptic coordinates of a planet (referred to the ecliptic
/// and mean equinox of the date).
/// Uses VSOP87D theory via the `vsop87` crate.
pub fn heliocentric_position(planet: Planet, jd: JulianDay) -> EclipticCoords {
    vsop87_wrapper::heliocentric_ecliptic(planet, jd)
}

/// Compute geocentric ecliptic coordinates of a planet with light-time correction.
/// The planet's position is computed at the retarded time (jd - light_travel_time)
/// to account for the finite speed of light.
pub fn geocentric_position(planet: Planet, jd: JulianDay) -> EclipticCoords {
    let earth = heliocentric_position(Planet::Earth, jd);

    // First approximation without light-time
    let body0 = heliocentric_position(planet, jd);
    let geo0 = heliocentric_to_geocentric(&body0, &earth);

    // Light-time correction: recompute planet position at retarded time
    // light_time = distance_AU / speed_of_light_AU_per_day
    let light_time = geo0.distance / crate::bodies::SPEED_OF_LIGHT_AU_DAY;
    let body = heliocentric_position(planet, jd - light_time);
    let mut geo = heliocentric_to_geocentric(&body, &earth);

    // FK5 correction: VSOP87 coordinates are referred to the dynamical ecliptic,
    // which differs from the FK5/J2000 ecliptic by a small rotation.
    // Meeus chapter 32: ΔL = -1.397" - 0.00031" * T² (T in centuries from J2000)
    // This correction converts dynamical ecliptic longitude to FK5 longitude.
    let t = jd.century();
    let fk5_correction = (-1.397 - 0.00031 * t * t) / 3600.0;
    geo.longitude += fk5_correction.to_radians();

    geo
}

/// Compute apparent equatorial coordinates of a planet.
/// Includes: light-time correction, FK5 correction, nutation, annual aberration.
pub fn apparent_equatorial(planet: Planet, jd: JulianDay) -> EquatorialCoords {
    let geo_ecl = geocentric_position(planet, jd);

    // Apply nutation to ecliptic longitude
    let (delta_psi, _delta_eps) = nutation(jd);
    let nutated = EclipticCoords {
        longitude: geo_ecl.longitude + delta_psi,
        ..geo_ecl
    };

    // Convert to equatorial using true obliquity
    let obliquity = true_obliquity(jd);
    let eq = ecliptic_to_equatorial(&nutated, obliquity);

    // Apply annual aberration
    let sun_geo = crate::sun::geometric_longitude(jd);
    annual_aberration(&eq, sun_geo, obliquity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    #[test]
    fn mars_heliocentric_smoke() {
        let jd = jd_from_date(2024, 1, 1.0);
        let pos = heliocentric_position(Planet::Mars, jd);
        // Mars distance from Sun should be between 1.38 and 1.67 AU
        assert!(pos.distance > 1.3 && pos.distance < 1.7, "Mars distance: {}", pos.distance);
    }

    #[test]
    fn venus_geocentric_exists() {
        let jd = jd_from_date(2024, 6, 15.0);
        let pos = geocentric_position(Planet::Venus, jd);
        // Venus geocentric distance should be < 1.75 AU
        assert!(pos.distance < 1.75 && pos.distance > 0.26, "Venus distance: {}", pos.distance);
    }

    #[test]
    fn earth_geocentric_is_zero() {
        let jd = jd_from_date(2024, 3, 20.0);
        let pos = geocentric_position(Planet::Earth, jd);
        // Earth-Earth distance should be essentially zero
        assert!(pos.distance < 0.0001, "Earth geocentric distance: {}", pos.distance);
    }
}
