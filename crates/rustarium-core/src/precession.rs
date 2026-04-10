use crate::coords::EquatorialCoords;
use crate::time::JulianDay;

/// Precess equatorial coordinates from one epoch to another.
/// Uses the rigorous method from Meeus chapter 21.
pub fn precess(coords: &EquatorialCoords, from: JulianDay, to: JulianDay) -> EquatorialCoords {
    let t = (from.0 - 2451545.0) / 36525.0; // centuries from J2000 to start epoch
    let dt = (to.0 - from.0) / 36525.0; // centuries between epochs

    // Precession parameters in arcseconds (Meeus eq. 21.2 / 21.3)
    let zeta_a = (2306.2181 + 1.39656 * t - 0.000139 * t * t) * dt
        + (0.30188 - 0.000344 * t) * dt * dt
        + 0.017998 * dt * dt * dt;

    let z_a = (2306.2181 + 1.39656 * t - 0.000139 * t * t) * dt
        + (1.09468 + 0.000066 * t) * dt * dt
        + 0.018203 * dt * dt * dt;

    let theta_a = (2004.3109 - 0.85330 * t - 0.000217 * t * t) * dt
        - (0.42665 + 0.000217 * t) * dt * dt
        - 0.041833 * dt * dt * dt;

    let arcsec_to_rad = std::f64::consts::PI / (180.0 * 3600.0);
    let zeta = zeta_a * arcsec_to_rad;
    let z = z_a * arcsec_to_rad;
    let theta = theta_a * arcsec_to_rad;

    let ra0 = coords.ra + zeta;
    let cos_dec = coords.dec.cos();
    let sin_dec = coords.dec.sin();
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    let a = cos_dec * (ra0).sin();
    let b = cos_theta * cos_dec * (ra0).cos() - sin_theta * sin_dec;
    let c = sin_theta * cos_dec * (ra0).cos() + cos_theta * sin_dec;

    let ra = a.atan2(b) + z;
    let dec = c.asin();

    EquatorialCoords {
        ra: crate::time::normalize_radians(ra),
        dec,
        distance: coords.distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// Meeus example 21.b: θ Persei from J2000.0 to 2028-Nov-13.19
    /// Initial: α=41.054063°, δ=49.227750°
    /// Expected: α=41.547214°, δ=49.348483°
    #[test]
    fn precess_meeus_21b() {
        let from = JulianDay(2451545.0); // J2000.0
        let to = jd_from_date(2028, 11, 13.19);

        let coords = EquatorialCoords {
            ra: 41.054063_f64.to_radians(),
            dec: 49.227750_f64.to_radians(),
            distance: 1.0,
        };

        let result = precess(&coords, from, to);
        let ra_deg = result.ra.to_degrees();
        let dec_deg = result.dec.to_degrees();

        assert!(
            (ra_deg - 41.547214).abs() < 0.01,
            "RA: {:.6}° expected 41.547°",
            ra_deg
        );
        assert!(
            (dec_deg - 49.348483).abs() < 0.01,
            "Dec: {:.6}° expected 49.348°",
            dec_deg
        );
    }
}
