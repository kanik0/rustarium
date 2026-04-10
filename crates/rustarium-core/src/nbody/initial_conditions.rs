use crate::coords::Vec3;
use crate::nbody::state::{NBodyObject, StateVector};

/// Solar System state vectors at J2000.0 epoch (JD 2451545.0, 2000-Jan-01 12:00:00 TDB).
/// Frame: Ecliptic J2000, origin: Solar System Barycenter.
/// Units: km, km/s.
/// Source: JPL Horizons API (DE441), fetched 2026-04-10.
///
/// Query parameters used:
///   EPHEM_TYPE=VECTORS, CENTER=500@0, REF_PLANE=ECLIPTIC, VEC_TABLE=2
///   Body COMMAND codes: Sun=10, Mercury=1, Venus=2, Earth-Moon=3,
///   Mars=4, Jupiter=5, Saturn=6, Uranus=7, Neptune=8
///
/// These are the barycenter positions for planet systems (planet + satellites),
/// which is appropriate for solar system n-body integration.

pub fn sun() -> NBodyObject {
    NBodyObject::new(
        "Sun",
        crate::bodies::SUN_GM,
        StateVector::new(
            Vec3::new(
                -1.067706805380953e+06,
                -4.182752718194473e+05,
                 3.086181725476820e+04,
            ),
            Vec3::new(
                 9.312571926520472e-03,
                -1.282475570794162e-02,
                -1.633507186350417e-04,
            ),
        ),
    )
}

pub fn mercury() -> NBodyObject {
    NBodyObject::new(
        "Mercury",
        crate::bodies::Planet::Mercury.gm(),
        StateVector::new(
            Vec3::new(
                -2.052943316123468e+07,
                -6.733155053534345e+07,
                -3.648992526494771e+06,
            ),
            Vec3::new(
                 3.700430442920571e+01,
                -1.117724068132644e+01,
                -4.307791469376854e+00,
            ),
        ),
    )
}

pub fn venus() -> NBodyObject {
    NBodyObject::new(
        "Venus",
        crate::bodies::Planet::Venus.gm(),
        StateVector::new(
            Vec3::new(
                -1.085242008575715e+08,
                -5.303290247691983e+06,
                 6.166496116973171e+06,
            ),
            Vec3::new(
                 1.391218601189967e+00,
                -3.515311993215464e+01,
                -5.602056890007159e-01,
            ),
        ),
    )
}

pub fn earth() -> NBodyObject {
    // Earth-Moon Barycenter
    NBodyObject::new(
        "Earth",
        crate::bodies::EARTH_MOON_GM,
        StateVector::new(
            Vec3::new(
                -2.757028369509406e+07,
                 1.442756803561715e+08,
                 3.069138406456262e+04,
            ),
            Vec3::new(
                -2.977712821605761e+01,
                -5.491001578052183e+00,
                -1.213773110433358e-04,
            ),
        ),
    )
}

pub fn mars() -> NBodyObject {
    NBodyObject::new(
        "Mars",
        crate::bodies::Planet::Mars.gm(),
        StateVector::new(
            Vec3::new(
                 2.069804338364514e+08,
                -2.425327900043258e+06,
                -5.125427142018680e+06,
            ),
            Vec3::new(
                 1.171985008531777e+00,
                 2.628323978397636e+01,
                 5.221336559764609e-01,
            ),
        ),
    )
}

pub fn jupiter() -> NBodyObject {
    NBodyObject::new(
        "Jupiter",
        crate::bodies::Planet::Jupiter.gm(),
        StateVector::new(
            Vec3::new(
                 5.974998767925479e+08,
                 4.391864532202049e+08,
                -1.519599883576381e+07,
            ),
            Vec3::new(
                -7.900525116640771e+00,
                 1.114330834163639e+01,
                 1.306993263541205e-01,
            ),
        ),
    )
}

pub fn saturn() -> NBodyObject {
    NBodyObject::new(
        "Saturn",
        crate::bodies::Planet::Saturn.gm(),
        StateVector::new(
            Vec3::new(
                 9.573174174143425e+08,
                 9.824381819394076e+08,
                -5.518218567454088e+07,
            ),
            Vec3::new(
                -7.422709426014511e+00,
                 6.723088956951871e+00,
                 1.780864069576586e-01,
            ),
        ),
    )
}

pub fn uranus() -> NBodyObject {
    NBodyObject::new(
        "Uranus",
        crate::bodies::Planet::Uranus.gm(),
        StateVector::new(
            Vec3::new(
                 2.157907312953845e+09,
                -2.055043522509252e+09,
                -3.559462760241723e+07,
            ),
            Vec3::new(
                 4.646336807878125e+00,
                 4.614832825625936e+00,
                -4.305510952280978e-02,
            ),
        ),
    )
}

pub fn neptune() -> NBodyObject {
    NBodyObject::new(
        "Neptune",
        crate::bodies::Planet::Neptune.gm(),
        StateVector::new(
            Vec3::new(
                 2.513978721723721e+09,
                -3.739132788548089e+09,
                 1.906313375764561e+07,
            ),
            Vec3::new(
                 4.475214621751308e+00,
                 3.063802317434378e+00,
                -1.662267093013879e-01,
            ),
        ),
    )
}

// --- Major asteroids (for perturbation accuracy) ---
// Source: JPL Horizons API (DE441), fetched 2026-04-10.

pub fn ceres() -> NBodyObject {
    // GM source: Konopliv et al. (2011), Dawn mission
    NBodyObject::new(
        "Ceres",
        62.6284, // km³/s²
        StateVector::new(
            Vec3::new(
                -3.570100537503446e+08,
                 1.185847361125704e+08,
                 6.929549148058444e+07,
            ),
            Vec3::new(
                -6.196624728846403e+00,
                -1.834193841538127e+01,
                 5.778898523931160e-01,
            ),
        ),
    )
}

pub fn pallas() -> NBodyObject {
    NBodyObject::new(
        "Pallas",
        13.1, // km³/s² (estimated)
        StateVector::new(
            Vec3::new(
                -1.269002269677963e+08,
                 2.469776216783289e+08,
                -1.606207199263422e+08,
            ),
            Vec3::new(
                -2.031372264197489e+01,
                -7.149845768774706e+00,
                 6.609604438320030e+00,
            ),
        ),
    )
}

pub fn vesta() -> NBodyObject {
    // GM source: Russell et al. (2012), Dawn mission
    NBodyObject::new(
        "Vesta",
        17.3, // km³/s²
        StateVector::new(
            Vec3::new(
                -2.035604403464110e+08,
                -2.507159574584280e+08,
                 3.217971529677551e+07,
            ),
            Vec3::new(
                 1.667424655301909e+01,
                -1.283034254277192e+01,
                -1.637611423496558e+00,
            ),
        ),
    )
}

/// Create a complete solar system with all major bodies at J2000.0.
/// Includes Sun, 8 planets, and 3 major asteroids (12 bodies total).
pub fn solar_system_j2000() -> Vec<NBodyObject> {
    vec![
        sun(),
        mercury(),
        venus(),
        earth(),
        mars(),
        jupiter(),
        saturn(),
        uranus(),
        neptune(),
        ceres(),
        pallas(),
        vesta(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_system_has_twelve_bodies() {
        let ss = solar_system_j2000();
        assert_eq!(ss.len(), 12); // Sun + 8 planets + 3 asteroids
    }

    #[test]
    fn earth_at_roughly_1au() {
        let earth = earth();
        let au = crate::bodies::AU_KM;
        let distance = earth.state.position.magnitude();
        let au_ratio = distance / au;
        assert!(
            (au_ratio - 1.0).abs() < 0.02,
            "Earth distance = {:.4} AU, expected ~1.0 AU",
            au_ratio
        );
    }

    #[test]
    fn jupiter_at_roughly_5au() {
        let jup = jupiter();
        let au = crate::bodies::AU_KM;
        let distance = jup.state.position.magnitude();
        let au_ratio = distance / au;
        assert!(
            (au_ratio - 5.2).abs() < 0.5,
            "Jupiter distance = {:.2} AU, expected ~5.2 AU",
            au_ratio
        );
    }
}
