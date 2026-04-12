use crate::bodies::{Planet, AU_KM, SUN_GM};
use crate::coords::{
    ecliptic_to_equatorial, heliocentric_to_geocentric, EclipticCoords, EquatorialCoords,
};
use crate::nbody::orbital_elements::OrbitalElements;
use crate::nutation;
use crate::planet;
use crate::time::JulianDay;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmallBodyType {
    Asteroid,
    Comet,
    DwarfPlanet,
    #[serde(rename = "TNO")]
    Tno,
    Unknown,
}

impl SmallBodyType {
    pub fn name(self) -> &'static str {
        match self {
            SmallBodyType::Asteroid => "Asteroid",
            SmallBodyType::Comet => "Comet",
            SmallBodyType::DwarfPlanet => "Dwarf Planet",
            SmallBodyType::Tno => "TNO",
            SmallBodyType::Unknown => "Unknown",
        }
    }
}

/// A custom celestial body defined by osculating orbital elements at a given epoch.
/// Positions are computed via Keplerian (two-body) propagation from the epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomBody {
    pub name: String,
    pub designation: Option<String>,
    pub body_type: SmallBodyType,
    pub elements: OrbitalElements,
    /// Julian Day of the osculating epoch (from SBDB)
    pub epoch_jd: f64,
    /// Gravitational parameter in km^3/s^2 (0.0 if unknown/negligible)
    pub gm: f64,
    pub diameter_km: Option<f64>,
    /// Absolute magnitude H
    pub abs_magnitude_h: Option<f64>,
}

impl CustomBody {
    /// Heliocentric ecliptic position at the given Julian Day.
    /// Propagates mean anomaly from the osculating epoch using mean motion.
    /// Returns coordinates with distance in AU.
    pub fn heliocentric_position(&self, jd: JulianDay) -> EclipticCoords {
        let propagated = self.propagate_elements_to(jd);
        let state = propagated.to_state_vector(SUN_GM);
        let ecl = state.position.to_ecliptic();
        // to_ecliptic returns distance in km (same units as state vector)
        EclipticCoords {
            longitude: ecl.longitude,
            latitude: ecl.latitude,
            distance: ecl.distance / AU_KM,
        }
    }

    /// Geocentric ecliptic position (heliocentric - Earth heliocentric).
    /// Returns distance in AU.
    pub fn geocentric_position(&self, jd: JulianDay) -> EclipticCoords {
        let helio = self.heliocentric_position(jd);
        let earth_helio = planet::heliocentric_position(Planet::Earth, jd);
        heliocentric_to_geocentric(&helio, &earth_helio)
    }

    /// Apparent geocentric equatorial coordinates (RA/Dec).
    /// Applies nutation + obliquity but not aberration (negligible for asteroids).
    pub fn apparent_equatorial(&self, jd: JulianDay) -> EquatorialCoords {
        let geo = self.geocentric_position(jd);
        let obliquity = nutation::true_obliquity(jd);
        ecliptic_to_equatorial(&geo, obliquity)
    }

    /// Propagate the mean anomaly from self.epoch_jd to the target JD.
    /// Returns a new OrbitalElements with the updated mean anomaly.
    fn propagate_elements_to(&self, jd: JulianDay) -> OrbitalElements {
        let a_km = self.elements.semi_major_axis_km;
        let e = self.elements.eccentricity;

        // Elapsed time in seconds
        let dt_days = jd.0 - self.epoch_jd;
        let dt_seconds = dt_days * 86400.0;

        // Mean motion: n = sqrt(GM / |a|^3) rad/s
        let a_abs = a_km.abs();
        let n = (SUN_GM / (a_abs * a_abs * a_abs)).sqrt();

        // Propagate mean anomaly
        let mut m_new = self.elements.mean_anomaly_rad + n * dt_seconds;

        // Normalize for elliptic orbits
        if e < 1.0 {
            m_new = m_new % (2.0 * std::f64::consts::PI);
            if m_new < 0.0 {
                m_new += 2.0 * std::f64::consts::PI;
            }
        }

        OrbitalElements {
            mean_anomaly_rad: m_new,
            ..self.elements
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    fn ceres_test_body() -> CustomBody {
        // Ceres orbital elements at epoch JD 2460200.5 (2023-Sep-13)
        CustomBody {
            name: "Ceres".into(),
            designation: Some("1".into()),
            body_type: SmallBodyType::DwarfPlanet,
            elements: OrbitalElements::from_au_and_degrees(
                2.7691651, // a (AU)
                0.0760090, // e
                10.5935,   // i
                80.3055,   // Ω
                73.5977,   // ω
                77.372,    // M
            ),
            epoch_jd: 2460200.5,
            gm: 62.6284,
            diameter_km: Some(939.4),
            abs_magnitude_h: Some(3.33),
        }
    }

    #[test]
    fn ceres_heliocentric_distance_reasonable() {
        let ceres = ceres_test_body();
        let jd = jd_from_date(2026, 1, 1.0);
        let pos = ceres.heliocentric_position(jd);
        // Ceres orbits at ~2.5-3.0 AU
        assert!(
            pos.distance > 2.0 && pos.distance < 3.5,
            "Ceres distance {:.2} AU should be 2.0-3.5",
            pos.distance
        );
    }

    #[test]
    fn ceres_geocentric_position_reasonable() {
        let ceres = ceres_test_body();
        let jd = jd_from_date(2026, 1, 1.0);
        let geo = ceres.geocentric_position(jd);
        // Geocentric distance should be roughly 1.5-4.0 AU
        assert!(
            geo.distance > 1.0 && geo.distance < 5.0,
            "Ceres geocentric distance {:.2} AU",
            geo.distance
        );
    }

    #[test]
    fn ceres_equatorial_coords_valid() {
        let ceres = ceres_test_body();
        let jd = jd_from_date(2026, 1, 1.0);
        let eq = ceres.apparent_equatorial(jd);
        // RA should be [0, 2π), Dec should be [-π/2, π/2]
        assert!(eq.ra >= 0.0 && eq.ra < 2.0 * std::f64::consts::PI);
        assert!(eq.dec >= -std::f64::consts::FRAC_PI_2 && eq.dec <= std::f64::consts::FRAC_PI_2);
    }

    #[test]
    fn mean_anomaly_propagation() {
        let ceres = ceres_test_body();
        // Ceres orbital period ≈ 4.6 years ≈ 1681 days
        // After one full period, mean anomaly should wrap back to ~same value
        let period_days = 2.0 * std::f64::consts::PI
            / (SUN_GM / (ceres.elements.semi_major_axis_km.powi(3))).sqrt()
            / 86400.0;
        let jd_epoch = JulianDay(ceres.epoch_jd);
        let jd_one_period = JulianDay(ceres.epoch_jd + period_days);

        let pos_epoch = ceres.heliocentric_position(jd_epoch);
        let pos_period = ceres.heliocentric_position(jd_one_period);

        // Positions should be very close after one full orbit
        let lon_diff = (pos_epoch.longitude - pos_period.longitude).abs();
        assert!(
            lon_diff < 0.01, // < 0.01 radians ≈ 0.6°
            "After one period, longitude diff = {:.4} rad",
            lon_diff
        );
    }
}
