use serde::{Deserialize, Serialize};

/// Ecliptic coordinates (heliocentric or geocentric, depending on context).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EclipticCoords {
    /// Longitude in radians
    pub longitude: f64,
    /// Latitude in radians
    pub latitude: f64,
    /// Distance in AU (planets/Sun) or km (Moon)
    pub distance: f64,
}

/// Equatorial coordinates (geocentric or topocentric).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EquatorialCoords {
    /// Right ascension in radians [0, 2π)
    pub ra: f64,
    /// Declination in radians [-π/2, π/2]
    pub dec: f64,
    /// Distance (AU or km, same unit as input)
    pub distance: f64,
}

/// Horizontal (topocentric) coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HorizontalCoords {
    /// Azimuth in radians, measured from North through East [0, 2π)
    pub azimuth: f64,
    /// Altitude in radians above the horizon [-π/2, π/2]
    pub altitude: f64,
}

/// Cartesian coordinates in 3D (used for n-body, frame conversions).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Geographic position on Earth.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude in radians, positive North
    pub lat: f64,
    /// Longitude in radians, positive East
    pub lon: f64,
    /// Altitude above sea level in meters
    pub alt_m: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn magnitude(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance_to(self, other: Vec3) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn to_ecliptic(self) -> EclipticCoords {
        let distance = self.magnitude();
        let longitude = self.y.atan2(self.x);
        let latitude = (self.z / distance).asin();
        EclipticCoords {
            longitude: crate::time::normalize_radians(longitude),
            latitude,
            distance,
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, scalar: f64) -> Vec3 {
        Vec3::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, scalar: f64) -> Vec3 {
        Vec3::new(self.x / scalar, self.y / scalar, self.z / scalar)
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Vec3) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl GeoLocation {
    /// Create a new geographic location from degrees and meters.
    pub fn from_degrees(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Self {
        Self {
            lat: lat_deg.to_radians(),
            lon: lon_deg.to_radians(),
            alt_m,
        }
    }
}

impl EclipticCoords {
    /// Convert to Cartesian (rectangular ecliptic) coordinates.
    pub fn to_rectangular(self) -> Vec3 {
        let cos_lat = self.latitude.cos();
        Vec3 {
            x: self.distance * cos_lat * self.longitude.cos(),
            y: self.distance * cos_lat * self.longitude.sin(),
            z: self.distance * self.latitude.sin(),
        }
    }
}

// --- Coordinate transformations ---

/// Ecliptic → Equatorial.
/// `obliquity` is the obliquity of the ecliptic in radians.
pub fn ecliptic_to_equatorial(ecl: &EclipticCoords, obliquity: f64) -> EquatorialCoords {
    let sin_lon = ecl.longitude.sin();
    let cos_lon = ecl.longitude.cos();
    let sin_lat = ecl.latitude.sin();
    let cos_lat = ecl.latitude.cos();
    let sin_obl = obliquity.sin();
    let cos_obl = obliquity.cos();

    let ra = (sin_lon * cos_obl - sin_lat / cos_lat * sin_obl).atan2(cos_lon);
    let dec = (sin_lat * cos_obl + cos_lat * sin_obl * sin_lon).asin();

    EquatorialCoords {
        ra: crate::time::normalize_radians(ra),
        dec,
        distance: ecl.distance,
    }
}

/// Equatorial → Ecliptic.
pub fn equatorial_to_ecliptic(eq: &EquatorialCoords, obliquity: f64) -> EclipticCoords {
    let sin_ra = eq.ra.sin();
    let cos_ra = eq.ra.cos();
    let sin_dec = eq.dec.sin();
    let cos_dec = eq.dec.cos();
    let sin_obl = obliquity.sin();
    let cos_obl = obliquity.cos();

    let lon = (sin_ra * cos_obl + sin_dec / cos_dec * sin_obl).atan2(cos_ra);
    let lat = (sin_dec * cos_obl - cos_dec * sin_obl * sin_ra).asin();

    EclipticCoords {
        longitude: crate::time::normalize_radians(lon),
        latitude: lat,
        distance: eq.distance,
    }
}

/// Equatorial → Horizontal.
/// `hour_angle` is the local hour angle in radians.
/// `observer_lat` is the observer's latitude in radians.
pub fn equatorial_to_horizontal(
    eq: &EquatorialCoords,
    hour_angle: f64,
    observer_lat: f64,
) -> HorizontalCoords {
    let sin_h = hour_angle.sin();
    let cos_h = hour_angle.cos();
    let sin_dec = eq.dec.sin();
    let cos_dec = eq.dec.cos();
    let sin_lat = observer_lat.sin();
    let cos_lat = observer_lat.cos();

    let azimuth = sin_h.atan2(cos_h * sin_lat - sin_dec / cos_dec * cos_lat);
    let altitude = (sin_lat * sin_dec + cos_lat * cos_dec * cos_h).asin();

    HorizontalCoords {
        azimuth: crate::time::normalize_radians(azimuth + std::f64::consts::PI),
        altitude,
    }
}

/// Convert heliocentric ecliptic coordinates to geocentric ecliptic coordinates.
/// `body_helio`: heliocentric position of the body
/// `earth_helio`: heliocentric position of Earth
pub fn heliocentric_to_geocentric(
    body_helio: &EclipticCoords,
    earth_helio: &EclipticCoords,
) -> EclipticCoords {
    let body_rect = body_helio.to_rectangular();
    let earth_rect = earth_helio.to_rectangular();
    let geo_rect = body_rect - earth_rect;
    geo_rect.to_ecliptic()
}

/// Format right ascension (radians) as "HHh MMm SS.Ss"
pub fn format_ra(ra: f64) -> String {
    let hours_total = ra.to_degrees() / 15.0;
    let h = hours_total.floor() as u32;
    let m_total = (hours_total - h as f64) * 60.0;
    let m = m_total.floor() as u32;
    let s = (m_total - m as f64) * 60.0;
    format!("{:02}h {:02}m {:05.2}s", h, m, s)
}

/// Format declination (radians) as "+DD° MM' SS.S\""
pub fn format_dec(dec: f64) -> String {
    let sign = if dec < 0.0 { '-' } else { '+' };
    let deg_total = dec.to_degrees().abs();
    let d = deg_total.floor() as u32;
    let m_total = (deg_total - d as f64) * 60.0;
    let m = m_total.floor() as u32;
    let s = (m_total - m as f64) * 60.0;
    format!("{}{}° {:02}' {:04.1}\"", sign, d, m, s)
}

/// Format angle in degrees as "DDD° MM' SS.S\""
pub fn format_dms(angle_rad: f64) -> String {
    let deg_total = angle_rad.to_degrees();
    let sign = if deg_total < 0.0 { '-' } else { '+' };
    let deg_total = deg_total.abs();
    let d = deg_total.floor() as u32;
    let m_total = (deg_total - d as f64) * 60.0;
    let m = m_total.floor() as u32;
    let s = (m_total - m as f64) * 60.0;
    format!("{}{}° {:02}' {:04.1}\"", sign, d, m, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const EPSILON: f64 = 1e-6;
    const ARCSEC: f64 = PI / (180.0 * 3600.0);

    /// Meeus example 13.a: λ=113.21563°, β=6.68417°, ε=23.4392911°
    /// Expected: α=116.32894°, δ=28.02617°
    #[test]
    fn ecliptic_to_equatorial_meeus_13a() {
        let ecl = EclipticCoords {
            longitude: 113.215630_f64.to_radians(),
            latitude: 6.684170_f64.to_radians(),
            distance: 1.0,
        };
        let obliquity = 23.4392911_f64.to_radians();
        let eq = ecliptic_to_equatorial(&ecl, obliquity);

        let ra_deg = eq.ra.to_degrees();
        let dec_deg = eq.dec.to_degrees();

        assert!(
            (ra_deg - 116.32894).abs() < 0.001,
            "RA: {} expected: 116.329",
            ra_deg
        );
        assert!(
            (dec_deg - 28.02617).abs() < 0.001,
            "Dec: {} expected: 28.026",
            dec_deg
        );
    }

    /// Round-trip ecliptic → equatorial → ecliptic
    #[test]
    fn ecliptic_equatorial_roundtrip() {
        let original = EclipticCoords {
            longitude: 1.5,
            latitude: 0.3,
            distance: 2.5,
        };
        let obliquity = 23.44_f64.to_radians();
        let eq = ecliptic_to_equatorial(&original, obliquity);
        let recovered = equatorial_to_ecliptic(&eq, obliquity);

        assert!((original.longitude - recovered.longitude).abs() < EPSILON);
        assert!((original.latitude - recovered.latitude).abs() < EPSILON);
        assert!((original.distance - recovered.distance).abs() < EPSILON);
    }

    #[test]
    fn vec3_magnitude() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!((v.magnitude() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn rectangular_spherical_roundtrip() {
        let original = EclipticCoords {
            longitude: 1.2,
            latitude: -0.3,
            distance: 5.0,
        };
        let rect = original.to_rectangular();
        let recovered = rect.to_ecliptic();

        assert!((original.longitude - recovered.longitude).abs() < EPSILON);
        assert!((original.latitude - recovered.latitude).abs() < EPSILON);
        assert!((original.distance - recovered.distance).abs() < EPSILON);
    }

    #[test]
    fn format_ra_test() {
        // 12h 30m 00.00s = 187.5°
        let ra = 187.5_f64.to_radians();
        let s = format_ra(ra);
        assert!(s.starts_with("12h 30m"));
    }
}
