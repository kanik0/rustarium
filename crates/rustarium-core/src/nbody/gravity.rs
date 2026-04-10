use crate::bodies::SPEED_OF_LIGHT_KM_S;
use crate::coords::Vec3;
use crate::nbody::state::NBodyObject;

/// J2 zonal harmonic coefficient for oblate bodies.
/// Only Earth and Jupiter have significant J2 effects.
const EARTH_J2: f64 = 1.08263e-3;
const JUPITER_J2: f64 = 1.4736e-2;
const EARTH_EQUATORIAL_RADIUS: f64 = 6378.137; // km
const JUPITER_EQUATORIAL_RADIUS: f64 = 71492.0; // km

/// Compute gravitational accelerations for ALL bodies simultaneously.
/// Includes:
/// - Newtonian pairwise gravity
/// - Post-Newtonian 1PN relativistic corrections (Einstein-Infeld-Hoffmann)
/// - J2 oblateness perturbation (Earth, Jupiter)
pub fn all_accelerations(bodies: &[NBodyObject]) -> Vec<Vec3> {
    let n = bodies.len();
    let mut acc = vec![Vec3::zero(); n];

    // Newtonian pairwise gravity (exploiting Newton's third law)
    for i in 0..n {
        for j in (i + 1)..n {
            let dr = bodies[j].state.position - bodies[i].state.position;
            let dist = dr.magnitude();
            let dist3 = dist * dist * dist;

            let a_ij = dr * (bodies[j].gm / dist3);
            let a_ji = dr * (-bodies[i].gm / dist3);

            acc[i] += a_ij;
            acc[j] += a_ji;
        }
    }

    // 1PN relativistic corrections (Sun as central body, index 0)
    if n > 1 {
        let sun_idx = 0;
        for i in 1..n {
            let correction = post_newtonian_1pn(bodies, sun_idx, i);
            acc[i] += correction;
        }
    }

    // J2 oblateness perturbation
    for i in 0..n {
        let (j2, radius) = match bodies[i].name.as_str() {
            "Earth" => (EARTH_J2, EARTH_EQUATORIAL_RADIUS),
            "Jupiter" => (JUPITER_J2, JUPITER_EQUATORIAL_RADIUS),
            _ => continue,
        };
        for j in 0..n {
            if j == i {
                continue;
            }
            let j2_acc = j2_acceleration(&bodies[i], &bodies[j], j2, radius);
            acc[j] += j2_acc;
        }
    }

    acc
}

/// Post-Newtonian 1PN correction to the acceleration of body `planet` due to `sun`.
/// Based on the Einstein-Infeld-Hoffmann equations (simplified for Sun-planet pair).
///
/// This correction accounts for:
/// - Mercury's perihelion precession (43"/century)
/// - General relativistic effects on all inner planets
///
/// Formula (Soffel et al., IAU 2000):
/// a_1PN = (GM_S / (c²r³)) * {
///   r_vec * [4*GM_S/r - v² + 4*(v·r_hat)²]
///   + v_vec * [4*(v·r_hat)]
/// }
///
/// where r_vec = planet - sun, r = |r_vec|, v = planet velocity relative to sun
fn post_newtonian_1pn(bodies: &[NBodyObject], sun: usize, planet: usize) -> Vec3 {
    let c2 = SPEED_OF_LIGHT_KM_S * SPEED_OF_LIGHT_KM_S;

    let r_vec = bodies[planet].state.position - bodies[sun].state.position;
    let v_vec = bodies[planet].state.velocity - bodies[sun].state.velocity;

    let r = r_vec.magnitude();
    let r2 = r * r;
    let v2 = v_vec.x * v_vec.x + v_vec.y * v_vec.y + v_vec.z * v_vec.z;

    let r_hat = r_vec * (1.0 / r);
    let v_dot_r = v_vec.x * r_hat.x + v_vec.y * r_hat.y + v_vec.z * r_hat.z;

    let gm = bodies[sun].gm;
    let factor = gm / (c2 * r2 * r);

    // Radial term: r_vec * [4*GM/r - v² + 4*(v·r_hat)²]
    let radial_coeff = 4.0 * gm / r - v2 + 4.0 * v_dot_r * v_dot_r;

    // Tangential term: v_vec * [4*(v·r_hat)]
    let tangential_coeff = 4.0 * v_dot_r;

    r_vec * (factor * radial_coeff) + v_vec * (factor * r * tangential_coeff)
}

/// J2 oblateness perturbation.
/// Acceleration on `target` due to the oblateness of `oblate_body`.
///
/// a_J2 = (3/2) * J2 * GM * R² / r⁵ * {
///   r_vec * (5*(z/r)² - 1) - 2*z*z_hat * r²
/// }
///
/// where R = equatorial radius, z = component along spin axis (ecliptic Z approximation)
fn j2_acceleration(
    oblate_body: &NBodyObject,
    target: &NBodyObject,
    j2: f64,
    equatorial_radius: f64,
) -> Vec3 {
    let dr = target.state.position - oblate_body.state.position;
    let r = dr.magnitude();

    // Skip if too far (J2 effect is negligible beyond ~10 body radii)
    if r > equatorial_radius * 1000.0 {
        return Vec3::zero();
    }

    let r2 = r * r;
    let r5 = r2 * r2 * r;
    let z = dr.z; // approximate spin axis as ecliptic Z
    let z_over_r_sq = (z / r) * (z / r);

    let coeff = 1.5 * j2 * oblate_body.gm * equatorial_radius * equatorial_radius / r5;

    Vec3::new(
        coeff * dr.x * (5.0 * z_over_r_sq - 1.0),
        coeff * dr.y * (5.0 * z_over_r_sq - 1.0),
        coeff * dr.z * (5.0 * z_over_r_sq - 3.0),
    )
}

/// Compute Newtonian-only acceleration on body `i` (used for testing).
pub fn gravitational_acceleration(bodies: &[NBodyObject], i: usize) -> Vec3 {
    let ri = bodies[i].state.position;
    let mut acc = Vec3::zero();

    for (j, body_j) in bodies.iter().enumerate() {
        if j == i {
            continue;
        }
        let rj = body_j.state.position;
        let dr = rj - ri;
        let dist = dr.magnitude();
        let dist3 = dist * dist * dist;
        acc += dr * (body_j.gm / dist3);
    }

    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbody::state::StateVector;

    #[test]
    fn two_body_acceleration() {
        let au_km = crate::bodies::AU_KM;
        let bodies = vec![
            NBodyObject::new("Sun", crate::bodies::SUN_GM, StateVector::new(
                Vec3::zero(),
                Vec3::zero(),
            )),
            NBodyObject::new("Earth", crate::bodies::Planet::Earth.gm(), StateVector::new(
                Vec3::new(au_km, 0.0, 0.0),
                Vec3::zero(),
            )),
        ];

        let acc = gravitational_acceleration(&bodies, 1);
        assert!(acc.x < 0.0, "Acceleration should point toward Sun");

        let expected = crate::bodies::SUN_GM / (au_km * au_km);
        assert!(
            (acc.x.abs() - expected).abs() / expected < 1e-6,
            "acc={:.6e} expected={:.6e}",
            acc.x.abs(),
            expected
        );
    }

    #[test]
    fn relativistic_correction_nonzero() {
        let au_km = crate::bodies::AU_KM;
        let v_circ = (crate::bodies::SUN_GM / au_km).sqrt();

        let bodies = vec![
            NBodyObject::new("Sun", crate::bodies::SUN_GM, StateVector::new(
                Vec3::zero(),
                Vec3::zero(),
            )),
            NBodyObject::new("Mercury", crate::bodies::Planet::Mercury.gm(), StateVector::new(
                Vec3::new(0.387 * au_km, 0.0, 0.0),
                Vec3::new(0.0, 47.87, 0.0), // Mercury orbital velocity ~47.87 km/s
            )),
        ];

        let correction = post_newtonian_1pn(&bodies, 0, 1);
        // 1PN correction should be small but nonzero
        assert!(
            correction.magnitude() > 0.0,
            "1PN correction should be nonzero"
        );
        // Should be ~v²/c² ≈ 2.5e-8 times Newtonian acceleration
        let newtonian = gravitational_acceleration(&bodies, 1).magnitude();
        let ratio = correction.magnitude() / newtonian;
        assert!(
            ratio > 1e-9 && ratio < 1e-6,
            "1PN ratio={:.3e}, expected ~1e-8",
            ratio
        );
    }

    #[test]
    fn j2_correction_nonzero_near_earth() {
        let earth = NBodyObject::new("Earth", crate::bodies::Planet::Earth.gm(), StateVector::new(
            Vec3::zero(),
            Vec3::zero(),
        ));
        // Satellite at 7000 km altitude
        let satellite = NBodyObject::new("Sat", 0.0, StateVector::new(
            Vec3::new(7000.0 + EARTH_EQUATORIAL_RADIUS, 0.0, 0.0),
            Vec3::zero(),
        ));

        let j2_acc = j2_acceleration(&earth, &satellite, EARTH_J2, EARTH_EQUATORIAL_RADIUS);
        assert!(j2_acc.magnitude() > 0.0, "J2 should be nonzero near Earth");

        // J2 should be ~J2 * (R/r)² ≈ 1e-3 * (6378/13378)² ≈ 2.3e-4 times Newtonian
        let newtonian = crate::bodies::Planet::Earth.gm() / ((7000.0 + EARTH_EQUATORIAL_RADIUS).powi(2));
        let ratio = j2_acc.magnitude() / newtonian;
        assert!(
            ratio > 1e-5 && ratio < 1e-2,
            "J2 ratio={:.3e}",
            ratio
        );
    }

    #[test]
    fn j2_negligible_far_away() {
        let earth = NBodyObject::new("Earth", crate::bodies::Planet::Earth.gm(), StateVector::new(
            Vec3::zero(),
            Vec3::zero(),
        ));
        // Mars distance
        let mars = NBodyObject::new("Mars", crate::bodies::Planet::Mars.gm(), StateVector::new(
            Vec3::new(crate::bodies::AU_KM * 1.5, 0.0, 0.0),
            Vec3::zero(),
        ));

        let j2_acc = j2_acceleration(&earth, &mars, EARTH_J2, EARTH_EQUATORIAL_RADIUS);
        assert!(j2_acc.magnitude() == 0.0, "J2 should be zero at planetary distances");
    }
}
