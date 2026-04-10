use crate::coords::GeoLocation;
use crate::nutation::{nutation, true_obliquity};
use crate::time::{greenwich_apparent_sidereal_time, greenwich_mean_sidereal_time, JulianDay};

/// Compute the local sidereal time in radians.
/// Accounts for the observer's longitude.
pub fn local_mean_sidereal_time(jd_ut: JulianDay, observer: &GeoLocation) -> f64 {
    let gmst = greenwich_mean_sidereal_time(jd_ut);
    crate::time::normalize_radians(gmst + observer.lon)
}

/// Local apparent sidereal time (corrected for nutation).
pub fn local_apparent_sidereal_time(jd_ut: JulianDay, observer: &GeoLocation) -> f64 {
    let (dpsi, _) = nutation(jd_ut);
    let obliquity = true_obliquity(jd_ut);
    let gast = greenwich_apparent_sidereal_time(jd_ut, dpsi, obliquity);
    crate::time::normalize_radians(gast + observer.lon)
}

/// Local hour angle of a celestial object.
/// H = LST - RA (in radians).
pub fn hour_angle(local_sidereal_time: f64, right_ascension: f64) -> f64 {
    crate::time::normalize_radians(local_sidereal_time - right_ascension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::GeoLocation;
    use crate::time::jd_from_date;

    #[test]
    fn lst_greenwich_equals_gst() {
        let jd = jd_from_date(2024, 3, 20.0);
        let greenwich = GeoLocation::from_degrees(51.4772, 0.0, 0.0);
        let lst = local_mean_sidereal_time(jd, &greenwich);
        let gst = greenwich_mean_sidereal_time(jd);
        assert!((lst - gst).abs() < 1e-10);
    }

    #[test]
    fn lst_increases_eastward() {
        let jd = jd_from_date(2024, 3, 20.0);
        let london = GeoLocation::from_degrees(51.5, 0.0, 0.0);
        let rome = GeoLocation::from_degrees(41.9, 12.5, 0.0);
        let lst_london = local_mean_sidereal_time(jd, &london);
        let lst_rome = local_mean_sidereal_time(jd, &rome);
        // Rome is ~12.5° east, so its LST should be ~12.5° ahead
        let diff_deg = (lst_rome - lst_london).to_degrees();
        assert!(
            (diff_deg - 12.5).abs() < 0.1,
            "LST difference: {:.2}° expected ~12.5°",
            diff_deg
        );
    }
}
