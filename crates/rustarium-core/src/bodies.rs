use serde::{Deserialize, Serialize};

/// Planets of the solar system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Planet {
    Mercury,
    Venus,
    Earth,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

impl Planet {
    pub const ALL: [Planet; 8] = [
        Planet::Mercury,
        Planet::Venus,
        Planet::Earth,
        Planet::Mars,
        Planet::Jupiter,
        Planet::Saturn,
        Planet::Uranus,
        Planet::Neptune,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Planet::Mercury => "Mercury",
            Planet::Venus => "Venus",
            Planet::Earth => "Earth",
            Planet::Mars => "Mars",
            Planet::Jupiter => "Jupiter",
            Planet::Saturn => "Saturn",
            Planet::Uranus => "Uranus",
            Planet::Neptune => "Neptune",
        }
    }

    /// Gravitational parameter GM (km³/s²) for the planet system (planet + satellites).
    /// Source: JPL DE440 header.
    pub fn gm(self) -> f64 {
        match self {
            Planet::Mercury => 22031.868551,
            Planet::Venus => 324858.592000,
            Planet::Earth => 398600.435507,
            Planet::Mars => 42828.375816,
            Planet::Jupiter => 126712764.100000,
            Planet::Saturn => 37940584.841800,
            Planet::Uranus => 5794556.400000,
            Planet::Neptune => 6836527.100580,
        }
    }
}

/// Any celestial body the system can track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Sun,
    Moon,
    Planet(Planet),
}

impl Body {
    pub fn name(self) -> &'static str {
        match self {
            Body::Sun => "Sun",
            Body::Moon => "Moon",
            Body::Planet(p) => p.name(),
        }
    }

    /// Gravitational parameter GM (km³/s²).
    pub fn gm(self) -> f64 {
        match self {
            Body::Sun => SUN_GM,
            Body::Moon => MOON_GM,
            Body::Planet(p) => p.gm(),
        }
    }
}

// --- Physical constants ---

/// Sun GM (km³/s²) - DE440
pub const SUN_GM: f64 = 132712440041.279419;

/// Moon GM (km³/s²) - DE440
pub const MOON_GM: f64 = 4902.800118;

/// Earth-Moon barycenter GM (km³/s²)
pub const EARTH_MOON_GM: f64 = 403503.235502;

/// Astronomical Unit in km (IAU 2012)
pub const AU_KM: f64 = 149597870.700;

/// Speed of light in km/s
pub const SPEED_OF_LIGHT_KM_S: f64 = 299792.458;

/// Speed of light in AU/day
pub const SPEED_OF_LIGHT_AU_DAY: f64 = 173.144632674;

/// Earth equatorial radius in km (IERS 2010)
pub const EARTH_RADIUS_KM: f64 = 6378.1366;

/// Earth flattening (IERS 2010)
pub const EARTH_FLATTENING: f64 = 1.0 / 298.25642;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_all_has_eight() {
        assert_eq!(Planet::ALL.len(), 8);
    }

    #[test]
    fn sun_gm_is_positive() {
        assert!(SUN_GM > 0.0);
        assert!(SUN_GM > 1e11);
    }

    #[test]
    fn body_name_matches_planet() {
        assert_eq!(Body::Planet(Planet::Mars).name(), "Mars");
        assert_eq!(Body::Sun.name(), "Sun");
    }
}
