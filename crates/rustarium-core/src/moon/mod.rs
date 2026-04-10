mod coefficients;

use crate::coords::{ecliptic_to_equatorial, EclipticCoords, EquatorialCoords};
use crate::nutation::{nutation, true_obliquity};
use crate::time::{normalize_degrees, normalize_radians, JulianDay};

/// Mean distance of the Moon in km.
pub const MEAN_DISTANCE_KM: f64 = 385000.56;

/// Geocentric ecliptic position of the Moon.
/// Returns longitude, latitude in radians, distance in km.
/// Based on ELP-2000/82 as simplified by Meeus, chapter 47.
/// Accuracy: ~10" in longitude, ~4" in latitude.
pub fn geocentric_ecliptic(jd: JulianDay) -> EclipticCoords {
    let t = jd.century();

    // Fundamental arguments (Meeus 47.1-47.4) in degrees
    // L' = Moon's mean longitude
    let lp = normalize_degrees(
        218.3164477 + 481267.88123421 * t - 0.0015786 * t * t + t * t * t / 538841.0
            - t.powi(4) / 65194000.0,
    );
    // D = mean elongation of Moon
    let d = normalize_degrees(
        297.8501921 + 445267.1114034 * t - 0.0018819 * t * t + t * t * t / 545868.0
            - t.powi(4) / 113065000.0,
    );
    // M = Sun's mean anomaly
    let m = normalize_degrees(
        357.5291092 + 35999.0502909 * t - 0.0001536 * t * t + t * t * t / 24490000.0,
    );
    // M' = Moon's mean anomaly
    let mp = normalize_degrees(
        134.9633964 + 477198.8675055 * t + 0.0087414 * t * t + t * t * t / 69699.0
            - t.powi(4) / 14712000.0,
    );
    // F = Moon's argument of latitude
    let f = normalize_degrees(
        93.2720950 + 483202.0175233 * t - 0.0036539 * t * t - t * t * t / 3526000.0
            + t.powi(4) / 863310000.0,
    );

    // Additional correction arguments
    let a1 = normalize_degrees(119.75 + 131.849 * t);
    let a2 = normalize_degrees(53.09 + 479264.290 * t);
    let a3 = normalize_degrees(313.45 + 481266.484 * t);

    // Convert to radians for trig
    let d_r = d.to_radians();
    let m_r = m.to_radians();
    let mp_r = mp.to_radians();
    let f_r = f.to_radians();
    let a1_r = a1.to_radians();
    let a2_r = a2.to_radians();
    let a3_r = a3.to_radians();

    // Eccentricity correction factor
    let e = 1.0 - 0.002516 * t - 0.0000074 * t * t;
    let e2 = e * e;

    // Sum longitude and distance terms
    let mut sum_l = 0.0_f64;
    let mut sum_r = 0.0_f64;

    for &(cd, cm, cmp, cf, sl, sr) in &coefficients::LONGITUDE_DISTANCE_TERMS {
        let arg = cd as f64 * d_r + cm as f64 * m_r + cmp as f64 * mp_r + cf as f64 * f_r;

        // Apply eccentricity correction based on M coefficient
        let e_factor = match cm.abs() {
            1 => e,
            2 => e2,
            _ => 1.0,
        };

        sum_l += sl * e_factor * arg.sin();
        sum_r += sr * e_factor * arg.cos();
    }

    // Sum latitude terms
    let mut sum_b = 0.0_f64;

    for &(cd, cm, cmp, cf, sb) in &coefficients::LATITUDE_TERMS {
        let arg = cd as f64 * d_r + cm as f64 * m_r + cmp as f64 * mp_r + cf as f64 * f_r;

        let e_factor = match cm.abs() {
            1 => e,
            2 => e2,
            _ => 1.0,
        };

        sum_b += sb * e_factor * arg.sin();
    }

    // Additional corrections (Meeus p. 338)
    sum_l += 3958.0 * a1_r.sin() + 1962.0 * (lp.to_radians() - f_r).sin() + 318.0 * a2_r.sin();

    sum_b += -2235.0 * lp.to_radians().sin()
        + 382.0 * a3_r.sin()
        + 175.0 * (a1_r - f_r).sin()
        + 175.0 * (a1_r + f_r).sin()
        + 127.0 * (lp.to_radians() - mp_r).sin()
        - 115.0 * (lp.to_radians() + mp_r).sin();

    // Planetary perturbation corrections (Venus and Jupiter)
    // These improve accuracy by ~1-2 arcseconds.
    // Mean longitudes of Venus and Jupiter
    let venus_lon = normalize_degrees(181.979801 + 58517.8156748 * t).to_radians();
    let jupiter_lon = normalize_degrees(34.351519 + 4452.67114 * t).to_radians();

    // Venus perturbations to longitude
    sum_l += 271.0 * (2.0 * venus_lon).sin()
        + 196.0 * (2.0 * venus_lon - 2.0 * d_r).sin()
        + 167.0 * (2.0 * venus_lon - d_r - mp_r).sin()
        + 103.0 * (2.0 * venus_lon - 2.0 * d_r + mp_r).sin()
        + 78.0 * (2.0 * venus_lon - d_r).sin()
        - 54.0 * (venus_lon - 3.0 * d_r).sin();

    // Jupiter perturbations to longitude
    sum_l += 40.0 * (jupiter_lon - d_r).sin()
        + 32.0 * (jupiter_lon - 2.0 * d_r).sin()
        - 27.0 * (jupiter_lon).sin()
        + 21.0 * (2.0 * jupiter_lon - d_r).sin();

    // Venus perturbations to latitude
    sum_b += 21.0 * (venus_lon).sin()
        - 18.0 * (venus_lon - 2.0 * d_r + f_r).sin()
        + 15.0 * (venus_lon - 2.0 * d_r - f_r).sin();

    // Convert sums to actual values
    let longitude = lp + sum_l / 1_000_000.0; // degrees
    let latitude = sum_b / 1_000_000.0; // degrees
    let distance = 385000.56 + sum_r / 1000.0; // km

    EclipticCoords {
        longitude: normalize_radians(longitude.to_radians()),
        latitude: latitude.to_radians(),
        distance,
    }
}

/// Apparent geocentric equatorial coordinates of the Moon.
/// Corrected for nutation.
pub fn apparent_equatorial(jd: JulianDay) -> EquatorialCoords {
    let ecl = geocentric_ecliptic(jd);

    let (delta_psi, _) = nutation(jd);
    let nutated = EclipticCoords {
        longitude: ecl.longitude + delta_psi,
        ..ecl
    };

    let obliquity = true_obliquity(jd);
    ecliptic_to_equatorial(&nutated, obliquity)
}

/// Equatorial horizontal parallax of the Moon in radians.
/// π = arcsin(6378.14 / distance)
pub fn horizontal_parallax(jd: JulianDay) -> f64 {
    let ecl = geocentric_ecliptic(jd);
    (crate::bodies::EARTH_RADIUS_KM / ecl.distance).asin()
}

/// Angular semi-diameter of the Moon in radians.
/// ≈ 0.2725 * horizontal_parallax (a good approximation)
pub fn angular_semidiameter(jd: JulianDay) -> f64 {
    0.2725 * horizontal_parallax(jd)
}

/// Moon illuminated fraction (phase).
/// Returns a value from 0.0 (new moon) to 1.0 (full moon).
pub fn illuminated_fraction(jd: JulianDay) -> f64 {
    let moon_ecl = geocentric_ecliptic(jd);
    let sun_ecl = crate::sun::geocentric_ecliptic(jd);

    // Geocentric elongation of the Moon from the Sun
    let cos_psi = moon_ecl.latitude.cos()
        * (moon_ecl.longitude - sun_ecl.longitude).cos();
    let psi = cos_psi.acos();

    // Phase angle i
    let sun_dist_km = sun_ecl.distance * crate::bodies::AU_KM;
    let i = (sun_dist_km * psi.sin())
        .atan2(moon_ecl.distance - sun_dist_km * cos_psi);

    (1.0 + i.cos()) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// Meeus example 47.a: Moon position on 1992 April 12 0h TDT
    /// Expected: λ=133.162655°, β=-3.229126°, Δ=368409.7 km
    #[test]
    fn moon_position_meeus_47a() {
        let jd = jd_from_date(1992, 4, 12.0);
        let pos = geocentric_ecliptic(jd);

        let lon_deg = pos.longitude.to_degrees();
        let lat_deg = pos.latitude.to_degrees();

        assert!(
            (lon_deg - 133.162655).abs() < 0.01,
            "Moon λ = {:.6}° expected 133.163°",
            lon_deg
        );
        assert!(
            (lat_deg - (-3.229126)).abs() < 0.01,
            "Moon β = {:.6}° expected -3.229°",
            lat_deg
        );
        assert!(
            (pos.distance - 368409.7).abs() < 50.0,
            "Moon Δ = {:.1} km expected 368409.7 km",
            pos.distance
        );
    }

    #[test]
    fn moon_distance_reasonable() {
        // Moon distance should be ~356,000 - ~407,000 km
        for month in 1..=12 {
            let jd = jd_from_date(2024, month, 15.0);
            let pos = geocentric_ecliptic(jd);
            assert!(
                pos.distance > 350000.0 && pos.distance < 410000.0,
                "Moon distance {:.0} km out of range at month {}",
                pos.distance,
                month
            );
        }
    }

    #[test]
    fn illuminated_fraction_range() {
        // Check multiple dates - fraction should always be 0..1
        for day in (0..365).step_by(3) {
            let jd = jd_from_date(2024, 1, 1.0) + day as f64;
            let frac = illuminated_fraction(jd);
            assert!(
                (0.0..=1.0).contains(&frac),
                "Illuminated fraction {:.4} out of range at day {}",
                frac,
                day
            );
        }
    }
}
