use crate::bodies::{Planet, AU_KM, SUN_GM};
use crate::coords::{
    ecliptic_to_equatorial, heliocentric_to_geocentric, EclipticCoords, EquatorialCoords, Vec3,
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
    Spacecraft,
    Unknown,
}

impl SmallBodyType {
    pub fn name(self) -> &'static str {
        match self {
            SmallBodyType::Asteroid => "Asteroid",
            SmallBodyType::Comet => "Comet",
            SmallBodyType::DwarfPlanet => "Dwarf Planet",
            SmallBodyType::Tno => "TNO",
            SmallBodyType::Spacecraft => "Spacecraft",
            SmallBodyType::Unknown => "Unknown",
        }
    }
}

/// A point in an ephemeris table (heliocentric ecliptic, AU and AU/day).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemerisPoint {
    pub jd: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

/// How a custom body's position is computed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropagationMethod {
    /// Two-body Keplerian propagation from osculating elements at an epoch.
    Keplerian {
        elements: OrbitalElements,
        epoch_jd: f64,
    },
    /// Interpolated pre-computed ephemeris table (e.g. from JPL Horizons).
    /// Table must be sorted by JD.
    Ephemeris {
        table: Vec<EphemerisPoint>,
    },
}

/// A custom celestial body — asteroid, comet, dwarf planet, or spacecraft.
/// Position is computed via Keplerian propagation or ephemeris interpolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomBody {
    pub name: String,
    pub designation: Option<String>,
    pub body_type: SmallBodyType,
    pub propagation: PropagationMethod,
    /// Gravitational parameter in km^3/s^2 (0.0 if unknown/negligible)
    pub gm: f64,
    pub diameter_km: Option<f64>,
    /// Absolute magnitude H
    pub abs_magnitude_h: Option<f64>,
    /// JPL Horizons ID (e.g. "-31" for Voyager 1), if applicable
    pub horizons_id: Option<String>,
}

impl CustomBody {
    /// Reference epoch: for Keplerian, the osculating epoch; for Ephemeris, the midpoint.
    pub fn epoch_jd(&self) -> f64 {
        match &self.propagation {
            PropagationMethod::Keplerian { epoch_jd, .. } => *epoch_jd,
            PropagationMethod::Ephemeris { table } => {
                if table.is_empty() {
                    0.0
                } else {
                    (table.first().unwrap().jd + table.last().unwrap().jd) / 2.0
                }
            }
        }
    }

    /// Access orbital elements (only available for Keplerian bodies).
    pub fn elements(&self) -> Option<&OrbitalElements> {
        match &self.propagation {
            PropagationMethod::Keplerian { elements, .. } => Some(elements),
            PropagationMethod::Ephemeris { .. } => None,
        }
    }

    /// Access ephemeris table (only available for Ephemeris bodies).
    pub fn ephemeris_table(&self) -> Option<&[EphemerisPoint]> {
        match &self.propagation {
            PropagationMethod::Keplerian { .. } => None,
            PropagationMethod::Ephemeris { table } => Some(table),
        }
    }

    /// Heliocentric ecliptic position at the given Julian Day.
    /// Returns coordinates with distance in AU.
    pub fn heliocentric_position(&self, jd: JulianDay) -> EclipticCoords {
        match &self.propagation {
            PropagationMethod::Keplerian { elements, epoch_jd } => {
                keplerian_position(elements, *epoch_jd, jd)
            }
            PropagationMethod::Ephemeris { table } => ephemeris_position(table, jd),
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
    pub fn apparent_equatorial(&self, jd: JulianDay) -> EquatorialCoords {
        let geo = self.geocentric_position(jd);
        let obliquity = nutation::true_obliquity(jd);
        ecliptic_to_equatorial(&geo, obliquity)
    }

    /// Interpolated velocity at JD (AU/day), only for Ephemeris bodies.
    pub fn velocity_au_day(&self, jd: JulianDay) -> Option<(f64, f64, f64)> {
        match &self.propagation {
            PropagationMethod::Ephemeris { table } => {
                let (i, t) = find_interval(table, jd);
                if i >= table.len() - 1 {
                    return Some((table.last()?.vx, table.last()?.vy, table.last()?.vz));
                }
                let a = &table[i];
                let b = &table[i + 1];
                Some((
                    a.vx + t * (b.vx - a.vx),
                    a.vy + t * (b.vy - a.vy),
                    a.vz + t * (b.vz - a.vz),
                ))
            }
            PropagationMethod::Keplerian { .. } => None,
        }
    }
}

/// Keplerian two-body propagation.
fn keplerian_position(
    elements: &OrbitalElements,
    epoch_jd: f64,
    jd: JulianDay,
) -> EclipticCoords {
    let a_km = elements.semi_major_axis_km;
    let e = elements.eccentricity;

    let dt_seconds = (jd.0 - epoch_jd) * 86400.0;
    let a_abs = a_km.abs();
    let n = (SUN_GM / (a_abs * a_abs * a_abs)).sqrt();

    let mut m_new = elements.mean_anomaly_rad + n * dt_seconds;
    if e < 1.0 {
        m_new = m_new % (2.0 * std::f64::consts::PI);
        if m_new < 0.0 {
            m_new += 2.0 * std::f64::consts::PI;
        }
    }

    let propagated = OrbitalElements {
        mean_anomaly_rad: m_new,
        ..*elements
    };
    let state = propagated.to_state_vector(SUN_GM);
    let ecl = state.position.to_ecliptic();
    EclipticCoords {
        longitude: ecl.longitude,
        latitude: ecl.latitude,
        distance: ecl.distance / AU_KM,
    }
}

/// Interpolate position from an ephemeris table.
/// Table is in AU (heliocentric ecliptic). Returns EclipticCoords in AU.
fn ephemeris_position(table: &[EphemerisPoint], jd: JulianDay) -> EclipticCoords {
    if table.is_empty() {
        return EclipticCoords {
            longitude: 0.0,
            latitude: 0.0,
            distance: 0.0,
        };
    }
    if table.len() == 1 {
        let p = &table[0];
        return Vec3::new(p.x, p.y, p.z).to_ecliptic();
    }

    let (i, t) = find_interval(table, jd);

    if i >= table.len() - 1 {
        // Beyond end — extrapolate from last two points
        let a = &table[table.len() - 2];
        let b = &table[table.len() - 1];
        let dt = (jd.0 - b.jd) / (b.jd - a.jd);
        let x = b.x + dt * (b.x - a.x);
        let y = b.y + dt * (b.y - a.y);
        let z = b.z + dt * (b.z - a.z);
        return Vec3::new(x, y, z).to_ecliptic();
    }

    let a = &table[i];
    let b = &table[i + 1];
    let x = a.x + t * (b.x - a.x);
    let y = a.y + t * (b.y - a.y);
    let z = a.z + t * (b.z - a.z);
    Vec3::new(x, y, z).to_ecliptic()
}

/// Find the bracketing interval and interpolation parameter t in [0, 1].
/// Returns (index of lower bracket, t).
fn find_interval(table: &[EphemerisPoint], jd: JulianDay) -> (usize, f64) {
    if table.is_empty() {
        return (0, 0.0);
    }
    // Before start — clamp to first interval
    if jd.0 <= table[0].jd {
        return (0, 0.0);
    }
    // After end
    if jd.0 >= table[table.len() - 1].jd {
        return (table.len() - 1, 0.0);
    }
    // Binary search
    let idx = table.partition_point(|p| p.jd <= jd.0);
    let i = if idx > 0 { idx - 1 } else { 0 };
    let a_jd = table[i].jd;
    let b_jd = table[i + 1].jd;
    let t = if (b_jd - a_jd).abs() > 1e-10 {
        (jd.0 - a_jd) / (b_jd - a_jd)
    } else {
        0.0
    };
    (i, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    fn ceres_test_body() -> CustomBody {
        CustomBody {
            name: "Ceres".into(),
            designation: Some("1".into()),
            body_type: SmallBodyType::DwarfPlanet,
            propagation: PropagationMethod::Keplerian {
                elements: OrbitalElements::from_au_and_degrees(
                    2.7691651, 0.0760090, 10.5935, 80.3055, 73.5977, 77.372,
                ),
                epoch_jd: 2460200.5,
            },
            gm: 62.6284,
            diameter_km: Some(939.4),
            abs_magnitude_h: Some(3.33),
            horizons_id: None,
        }
    }

    #[test]
    fn ceres_heliocentric_distance_reasonable() {
        let ceres = ceres_test_body();
        let jd = jd_from_date(2026, 1, 1.0);
        let pos = ceres.heliocentric_position(jd);
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
        assert!(eq.ra >= 0.0 && eq.ra < 2.0 * std::f64::consts::PI);
        assert!(eq.dec >= -std::f64::consts::FRAC_PI_2 && eq.dec <= std::f64::consts::FRAC_PI_2);
    }

    #[test]
    fn mean_anomaly_propagation() {
        let ceres = ceres_test_body();
        let elements = ceres.elements().unwrap();
        let period_days = 2.0 * std::f64::consts::PI
            / (SUN_GM / (elements.semi_major_axis_km.powi(3))).sqrt()
            / 86400.0;
        let jd_epoch = JulianDay(ceres.epoch_jd());
        let jd_one_period = JulianDay(ceres.epoch_jd() + period_days);
        let pos_epoch = ceres.heliocentric_position(jd_epoch);
        let pos_period = ceres.heliocentric_position(jd_one_period);
        let lon_diff = (pos_epoch.longitude - pos_period.longitude).abs();
        assert!(
            lon_diff < 0.01,
            "After one period, longitude diff = {:.4} rad",
            lon_diff
        );
    }

    #[test]
    fn ephemeris_interpolation() {
        // Create a simple 3-point ephemeris table (straight line along x-axis)
        let table = vec![
            EphemerisPoint { jd: 2460000.0, x: 1.0, y: 0.0, z: 0.0, vx: 0.01, vy: 0.0, vz: 0.0 },
            EphemerisPoint { jd: 2460010.0, x: 2.0, y: 0.0, z: 0.0, vx: 0.01, vy: 0.0, vz: 0.0 },
            EphemerisPoint { jd: 2460020.0, x: 3.0, y: 0.0, z: 0.0, vx: 0.01, vy: 0.0, vz: 0.0 },
        ];
        let body = CustomBody {
            name: "Test".into(),
            designation: None,
            body_type: SmallBodyType::Spacecraft,
            propagation: PropagationMethod::Ephemeris { table },
            gm: 0.0,
            diameter_km: None,
            abs_magnitude_h: None,
            horizons_id: Some("-999".into()),
        };

        // Midpoint of first interval should give x=1.5
        let pos = body.heliocentric_position(JulianDay(2460005.0));
        assert!(
            (pos.distance - 1.5).abs() < 0.01,
            "Interpolated distance {:.4} should be ~1.5 AU",
            pos.distance
        );

        // Exact table point
        let pos2 = body.heliocentric_position(JulianDay(2460010.0));
        assert!(
            (pos2.distance - 2.0).abs() < 0.01,
            "Exact point distance {:.4} should be ~2.0 AU",
            pos2.distance
        );
    }

    #[test]
    fn ephemeris_velocity() {
        let table = vec![
            EphemerisPoint { jd: 2460000.0, x: 1.0, y: 0.0, z: 0.0, vx: 0.01, vy: 0.02, vz: 0.0 },
            EphemerisPoint { jd: 2460010.0, x: 2.0, y: 0.0, z: 0.0, vx: 0.03, vy: 0.04, vz: 0.0 },
        ];
        let body = CustomBody {
            name: "Test".into(),
            designation: None,
            body_type: SmallBodyType::Spacecraft,
            propagation: PropagationMethod::Ephemeris { table },
            gm: 0.0,
            diameter_km: None,
            abs_magnitude_h: None,
            horizons_id: None,
        };
        // Midpoint velocity should be interpolated
        let v = body.velocity_au_day(JulianDay(2460005.0)).unwrap();
        assert!((v.0 - 0.02).abs() < 1e-10);
        assert!((v.1 - 0.03).abs() < 1e-10);
    }
}
