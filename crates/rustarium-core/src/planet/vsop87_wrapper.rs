use crate::bodies::Planet;
use crate::coords::EclipticCoords;
use crate::time::JulianDay;

/// Compute heliocentric ecliptic coordinates using VSOP87D.
/// VSOP87D returns: longitude, latitude (radians), radius (AU)
/// referred to the ecliptic and mean equinox of the date.
pub fn heliocentric_ecliptic(planet: Planet, jd: JulianDay) -> EclipticCoords {
    let jde = jd.0; // VSOP87 uses JDE (Julian Ephemeris Day ≈ JD TT)

    let sc = match planet {
        Planet::Mercury => vsop87::vsop87d::mercury(jde),
        Planet::Venus => vsop87::vsop87d::venus(jde),
        Planet::Earth => vsop87::vsop87d::earth(jde),
        Planet::Mars => vsop87::vsop87d::mars(jde),
        Planet::Jupiter => vsop87::vsop87d::jupiter(jde),
        Planet::Saturn => vsop87::vsop87d::saturn(jde),
        Planet::Uranus => vsop87::vsop87d::uranus(jde),
        Planet::Neptune => vsop87::vsop87d::neptune(jde),
    };

    EclipticCoords {
        longitude: crate::time::normalize_radians(sc.longitude()),
        latitude: sc.latitude(),
        distance: sc.distance(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// Meeus example 25.b: Venus on 1992 December 20.0 TDT
    /// Expected heliocentric L=26.11428°, B=-2.62070°, R=0.724603 AU
    #[test]
    fn venus_meeus_25b() {
        let jd = jd_from_date(1992, 12, 20.0);
        let pos = heliocentric_ecliptic(Planet::Venus, jd);

        let lon_deg = pos.longitude.to_degrees();
        let lat_deg = pos.latitude.to_degrees();

        assert!(
            (lon_deg - 26.11428).abs() < 0.01,
            "Venus L={:.5}° expected 26.114°",
            lon_deg
        );
        assert!(
            (lat_deg - (-2.62070)).abs() < 0.01,
            "Venus B={:.5}° expected -2.621°",
            lat_deg
        );
        assert!(
            (pos.distance - 0.724603).abs() < 0.0001,
            "Venus R={:.6} expected 0.7246",
            pos.distance
        );
    }
}
