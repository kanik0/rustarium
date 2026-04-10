use crate::coords::EquatorialCoords;

/// Constant of aberration in radians (~20.4898 arcseconds).
const KAPPA: f64 = 20.49552 / 3600.0 * std::f64::consts::PI / 180.0;

/// Apply annual aberration correction to equatorial coordinates.
/// `sun_longitude` is the geometric longitude of the Sun in radians.
/// `obliquity` is the obliquity of the ecliptic in radians.
/// Returns corrected equatorial coordinates.
///
/// Meeus chapter 23.
pub fn annual_aberration(
    eq: &EquatorialCoords,
    sun_longitude: f64,
    obliquity: f64,
) -> EquatorialCoords {
    let cos_ra = eq.ra.cos();
    let sin_ra = eq.ra.sin();
    let cos_dec = eq.dec.cos();
    let sin_dec = eq.dec.sin();
    let cos_sun = sun_longitude.cos();
    let sin_sun = sun_longitude.sin();
    let cos_obl = obliquity.cos();
    let sin_obl = obliquity.sin();

    // Correction to RA (Meeus eq. 23.3)
    let delta_ra = -KAPPA * (cos_ra * cos_sun * cos_obl + sin_ra * sin_sun) / cos_dec;

    // Correction to Dec (Meeus eq. 23.3)
    let delta_dec = -KAPPA
        * (cos_sun * cos_obl * (sin_obl / cos_obl * cos_dec - sin_ra * sin_dec)
            + cos_ra * sin_dec * sin_sun);

    EquatorialCoords {
        ra: crate::time::normalize_radians(eq.ra + delta_ra),
        dec: eq.dec + delta_dec,
        distance: eq.distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aberration_magnitude() {
        // Aberration correction should be on the order of 20 arcseconds
        let eq = EquatorialCoords {
            ra: 1.5,
            dec: 0.5,
            distance: 1.0,
        };
        let result = annual_aberration(&eq, 4.5, 23.44_f64.to_radians());
        let delta_ra = (result.ra - eq.ra).abs();
        let delta_dec = (result.dec - eq.dec).abs();

        let max_aberration = 25.0 / 3600.0 * std::f64::consts::PI / 180.0;
        assert!(delta_ra < max_aberration, "RA aberration too large");
        assert!(delta_dec < max_aberration, "Dec aberration too large");
    }
}
