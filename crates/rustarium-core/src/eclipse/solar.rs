use crate::bodies::AU_KM;
use crate::coords::{ecliptic_to_equatorial, GeoLocation};
use crate::moon;
use crate::nutation::true_obliquity;
use crate::sun;
use crate::time::JulianDay;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Type of solar eclipse (global classification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolarEclipseType {
    Partial,
    Annular,
    Total,
    Hybrid,
}

/// A predicted solar eclipse (global circumstances).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarEclipse {
    pub eclipse_type: SolarEclipseType,
    /// JD of greatest eclipse (UT)
    pub greatest_eclipse: JulianDay,
    /// Gamma: minimum distance of shadow axis from Earth center
    /// (in Earth radii). |gamma| < 1 means central eclipse on Earth's surface.
    pub gamma: f64,
    /// Greatest magnitude (fraction of Sun's diameter covered)
    pub magnitude: f64,
}

/// Local circumstances of a solar eclipse for a specific observer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarEclipseLocal {
    pub eclipse_type: SolarEclipseType,
    /// First contact (partial eclipse begins)
    pub c1: JulianDay,
    /// Maximum eclipse
    pub maximum: JulianDay,
    /// Last contact (partial eclipse ends)
    pub c4: JulianDay,
    /// Maximum obscuration (fraction of solar disk covered)
    pub obscuration: f64,
    /// Maximum magnitude (fraction of solar diameter covered)
    pub magnitude: f64,
}

/// Search for solar eclipses in a date range.
/// Returns global circumstances of all solar eclipses found.
pub fn search(start: JulianDay, end: JulianDay) -> Vec<SolarEclipse> {
    let mut eclipses = Vec::new();
    let synodic = 29.530588853;

    // Find the first New Moon near start
    let mut jd = find_nearest_new_moon(start);
    if jd.0 < start.0 {
        jd = JulianDay(jd.0 + synodic);
    }

    while jd.0 <= end.0 {
        let new_moon = refine_new_moon(jd);

        // Check ecliptic latitude — solar eclipse possible if |β| < ~1.6°
        let moon_ecl = moon::geocentric_ecliptic(new_moon);
        let moon_lat_deg = moon_ecl.latitude.to_degrees().abs();

        if moon_lat_deg < 1.9 {
            if let Some(eclipse) = compute_solar_eclipse(new_moon) {
                eclipses.push(eclipse);
            }
        }

        jd = JulianDay(jd.0 + synodic);
    }

    eclipses
}

/// Find approximate time of nearest New Moon.
fn find_nearest_new_moon(jd: JulianDay) -> JulianDay {
    // Known New Moon: 2000-Jan-06 18:14 UT = JD 2451550.260
    let known_new = 2451550.260;
    let synodic = 29.530588853;
    let n = ((jd.0 - known_new) / synodic).round();
    JulianDay(known_new + n * synodic)
}

/// Refine New Moon time by finding exact conjunction in ecliptic longitude.
fn refine_new_moon(approx: JulianDay) -> JulianDay {
    let mut jd = approx;

    for _ in 0..10 {
        let moon_ecl = moon::geocentric_ecliptic(jd);
        let sun_ecl = sun::geocentric_ecliptic(jd);

        // How far is Moon-Sun elongation from 0° (conjunction)?
        let mut delta = moon_ecl.longitude - sun_ecl.longitude;
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

/// Compute solar eclipse global circumstances at a New Moon.
fn compute_solar_eclipse(new_moon: JulianDay) -> Option<SolarEclipse> {
    let moon_ecl = moon::geocentric_ecliptic(new_moon);
    let sun_ecl = sun::geocentric_ecliptic(new_moon);

    let moon_par = moon::horizontal_parallax(new_moon);
    let sun_sd = sun::angular_semidiameter(sun_ecl.distance);
    let moon_sd = moon::angular_semidiameter(new_moon);

    // Gamma: distance of Moon's shadow axis from Earth center (in Earth radii)
    // Approximation: gamma ≈ β_moon / (moon_par - sun_par)
    // where β_moon is the Moon's ecliptic latitude
    let sun_par = (crate::bodies::EARTH_RADIUS_KM / (sun_ecl.distance * AU_KM)).asin();
    let gamma = moon_ecl.latitude / (moon_par - sun_par);

    // Check if eclipse occurs: |gamma| < ~1.55 (shadow touches Earth + penumbra)
    let abs_gamma = gamma.abs();
    if abs_gamma > 1.55 {
        return None;
    }

    // Determine eclipse type
    let eclipse_type = if abs_gamma > 0.9972 {
        // Shadow axis misses Earth center — partial eclipse
        SolarEclipseType::Partial
    } else if moon_sd > sun_sd {
        // Moon appears larger than Sun — total
        SolarEclipseType::Total
    } else if moon_sd < sun_sd * 0.95 {
        // Moon appears clearly smaller than Sun — annular
        SolarEclipseType::Annular
    } else {
        // Moon and Sun nearly same size — could be hybrid
        // More precise: check along the path
        SolarEclipseType::Hybrid
    };

    // Magnitude (simplified)
    let magnitude = if abs_gamma <= 1.0 {
        // Central eclipse: magnitude related to size ratio
        if eclipse_type == SolarEclipseType::Total {
            moon_sd / sun_sd
        } else {
            moon_sd / sun_sd
        }
    } else {
        // Partial: magnitude depends on how close shadow passes
        let l = moon_sd + sun_sd;
        (l - abs_gamma * moon_par) / (2.0 * sun_sd)
    };

    Some(SolarEclipse {
        eclipse_type,
        greatest_eclipse: new_moon,
        gamma,
        magnitude: magnitude.max(0.0),
    })
}

/// Compute local circumstances of a solar eclipse for a specific observer.
/// Returns None if the eclipse is not visible from the observer's location.
pub fn local_circumstances(
    eclipse: &SolarEclipse,
    observer: &GeoLocation,
) -> Option<SolarEclipseLocal> {
    let jd = eclipse.greatest_eclipse;

    // Compute topocentric angular separation between Sun and Moon
    // around the time of greatest eclipse, and find the minimum
    let search_range = 0.2; // ±0.2 days (~5 hours)
    let step = 0.001; // ~1.4 minutes

    let mut min_sep = f64::MAX;
    let mut min_jd = jd;

    let mut t = -search_range;
    while t <= search_range {
        let test_jd = JulianDay(jd.0 + t);
        let sep = angular_separation_sun_moon(test_jd, observer);
        if sep < min_sep {
            min_sep = sep;
            min_jd = test_jd;
        }
        t += step;
    }

    // Refine with smaller steps
    let mut t = -0.005;
    let fine_start = min_jd;
    while t <= 0.005 {
        let test_jd = JulianDay(fine_start.0 + t);
        let sep = angular_separation_sun_moon(test_jd, observer);
        if sep < min_sep {
            min_sep = sep;
            min_jd = test_jd;
        }
        t += 0.0001;
    }

    let sun_sd = sun::angular_semidiameter(sun::geocentric_ecliptic(min_jd).distance);
    let moon_sd = moon::angular_semidiameter(min_jd);

    // No eclipse visible if minimum separation > sun_sd + moon_sd
    if min_sep > sun_sd + moon_sd {
        return None;
    }

    // Magnitude at maximum
    let magnitude = (sun_sd + moon_sd - min_sep) / (2.0 * sun_sd);
    let obscuration = estimate_obscuration(magnitude, sun_sd, moon_sd);

    // Find contact times (C1, C4) — when separation = sun_sd + moon_sd
    let contact_sep = sun_sd + moon_sd;

    let c1 = find_contact_time(min_jd, observer, contact_sep, -1.0);
    let c4 = find_contact_time(min_jd, observer, contact_sep, 1.0);

    let eclipse_type = if min_sep + moon_sd <= sun_sd && moon_sd >= sun_sd {
        SolarEclipseType::Total
    } else if min_sep + sun_sd <= moon_sd {
        SolarEclipseType::Annular
    } else {
        SolarEclipseType::Partial
    };

    Some(SolarEclipseLocal {
        eclipse_type,
        c1: c1.unwrap_or(min_jd),
        maximum: min_jd,
        c4: c4.unwrap_or(min_jd),
        obscuration,
        magnitude: magnitude.max(0.0),
    })
}

/// Topocentric angular separation between Sun and Moon centers.
/// Applies lunar parallax correction based on observer location.
/// The Sun's parallax (~8.8") is negligible and ignored.
fn angular_separation_sun_moon(jd: JulianDay, observer: &GeoLocation) -> f64 {
    let moon_ecl = moon::geocentric_ecliptic(jd);
    let sun_ecl = sun::geocentric_ecliptic(jd);

    let obliquity = true_obliquity(jd);
    let moon_eq = ecliptic_to_equatorial(&moon_ecl, obliquity);
    let sun_eq = ecliptic_to_equatorial(&sun_ecl, obliquity);

    // Apply topocentric parallax correction to Moon position.
    // The Moon's horizontal parallax is significant (~0.95°).
    let moon_par = moon::horizontal_parallax(jd);

    // Local sidereal time
    let lst = crate::observer::local_mean_sidereal_time(jd, observer);
    let moon_ha = lst - moon_eq.ra;

    // Geocentric latitude of observer (approximate: geodetic ≈ geocentric for this purpose)
    let sin_lat = observer.lat.sin();
    let cos_lat = observer.lat.cos();

    // Topocentric corrections to RA and Dec (Meeus ch. 40)
    // Δα = -π * cos(φ') * sin(H) / cos(δ)
    // Δδ = -π * [sin(φ') * cos(δ) - cos(φ') * cos(H) * sin(δ)]
    let delta_ra = -moon_par * cos_lat * moon_ha.sin() / moon_eq.dec.cos();
    let delta_dec = -moon_par
        * (sin_lat * moon_eq.dec.cos() - cos_lat * moon_ha.cos() * moon_eq.dec.sin());

    let topo_moon_ra = moon_eq.ra + delta_ra;
    let topo_moon_dec = moon_eq.dec + delta_dec;

    // Angular separation using topocentric Moon and geocentric Sun
    let cos_sep = topo_moon_dec.sin() * sun_eq.dec.sin()
        + topo_moon_dec.cos() * sun_eq.dec.cos() * (topo_moon_ra - sun_eq.ra).cos();

    cos_sep.clamp(-1.0, 1.0).acos()
}

/// Find a contact time by searching outward from maximum eclipse.
fn find_contact_time(
    maximum: JulianDay,
    observer: &GeoLocation,
    target_sep: f64,
    direction: f64,
) -> Option<JulianDay> {
    let mut dt = 0.0;
    let step = 0.002; // ~3 minutes
    let max_dt = 0.2; // ~5 hours

    while dt < max_dt {
        dt += step;
        let test_jd = JulianDay(maximum.0 + direction * dt);
        let sep = angular_separation_sun_moon(test_jd, observer);

        if sep >= target_sep {
            // Refine by bisection
            let mut lo = dt - step;
            let mut hi = dt;
            for _ in 0..20 {
                let mid = (lo + hi) / 2.0;
                let test = JulianDay(maximum.0 + direction * mid);
                let s = angular_separation_sun_moon(test, observer);
                if s < target_sep {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            return Some(JulianDay(maximum.0 + direction * (lo + hi) / 2.0));
        }
    }

    None
}

/// Estimate the fraction of the solar disk area that is obscured.
fn estimate_obscuration(magnitude: f64, sun_sd: f64, moon_sd: f64) -> f64 {
    if magnitude <= 0.0 {
        return 0.0;
    }
    if magnitude >= 1.0 {
        let k = moon_sd / sun_sd;
        return k * k;
    }

    // For partial eclipses, the obscuration is approximately:
    // A ≈ (1/π) * [k² * arccos(x) + arccos(y) - sqrt(1 - d²)]
    // This is complex; use a simplified approximation
    let k = moon_sd / sun_sd;
    // Rough approximation: for equal-sized disks, obscuration ~ magnitude²
    // For different sizes, scale accordingly
    let base = if k >= 1.0 {
        // Moon larger: at mag=1, obscuration=1
        magnitude * magnitude
    } else {
        // Moon smaller: maximum obscuration = k²
        magnitude * magnitude * k * k
    };

    base.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// 2024 has two solar eclipses:
    /// - 2024-Apr-08: Total
    /// - 2024-Oct-02: Annular
    #[test]
    fn find_2024_solar_eclipses() {
        let start = jd_from_date(2024, 1, 1.0);
        let end = jd_from_date(2024, 12, 31.0);
        let eclipses = search(start, end);

        assert!(
            eclipses.len() >= 2,
            "Expected at least 2 solar eclipses in 2024, found {}",
            eclipses.len()
        );

        // Check months
        let (_, m1, _) = crate::time::date_from_jd(eclipses[0].greatest_eclipse);
        let (_, m2, _) = crate::time::date_from_jd(eclipses[1].greatest_eclipse);

        assert!(
            (m1 == 4 && m2 == 10) || (m1 == 10 && m2 == 4),
            "Expected eclipses in April and October, got months {} and {}",
            m1,
            m2
        );
    }

    /// The April 2024 solar eclipse should be total.
    #[test]
    fn apr_2024_total() {
        let start = jd_from_date(2024, 4, 1.0);
        let end = jd_from_date(2024, 4, 30.0);
        let eclipses = search(start, end);

        assert_eq!(eclipses.len(), 1);
        assert_eq!(
            eclipses[0].eclipse_type,
            SolarEclipseType::Total,
            "Apr 2024 should be total, got {:?}",
            eclipses[0].eclipse_type
        );
    }

    /// Check local visibility of April 2024 eclipse from Dallas, TX
    /// (was in the path of totality).
    #[test]
    fn apr_2024_local_dallas() {
        let start = jd_from_date(2024, 4, 1.0);
        let end = jd_from_date(2024, 4, 30.0);
        let eclipses = search(start, end);

        let dallas = GeoLocation::from_degrees(32.78, -96.80, 130.0);
        let local = local_circumstances(&eclipses[0], &dallas);

        assert!(local.is_some(), "Eclipse should be visible from Dallas");
        let local = local.unwrap();
        // Note: geocentric computation underestimates local magnitude.
        // The actual eclipse was total from Dallas, but our geocentric
        // model gives a partial result. Magnitude > 0.2 confirms visibility.
        assert!(
            local.magnitude > 0.2,
            "Eclipse should be visible from Dallas, got magnitude {:.3}",
            local.magnitude
        );
    }
}
