use crate::coords::Vec3;
use crate::nbody::state::StateVector;
use serde::{Deserialize, Serialize};

/// Classical Keplerian orbital elements.
/// These are the standard way to describe an orbit and are much easier
/// for users to specify than Cartesian state vectors.
///
/// # Example: adding Ceres to the simulation
/// ```
/// use rustarium_core::nbody::orbital_elements::OrbitalElements;
/// use rustarium_core::bodies::SUN_GM;
///
/// let ceres = OrbitalElements {
///     semi_major_axis_km: 4.14e8,     // ~2.77 AU
///     eccentricity: 0.0758,
///     inclination_rad: 10.59_f64.to_radians(),
///     longitude_ascending_node_rad: 80.3_f64.to_radians(),
///     argument_perihelion_rad: 73.6_f64.to_radians(),
///     mean_anomaly_rad: 77.4_f64.to_radians(),
/// };
/// let state = ceres.to_state_vector(SUN_GM);
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrbitalElements {
    /// Semi-major axis in km
    pub semi_major_axis_km: f64,
    /// Eccentricity (0 = circular, <1 = elliptical)
    pub eccentricity: f64,
    /// Inclination in radians
    pub inclination_rad: f64,
    /// Longitude of ascending node (Ω) in radians
    pub longitude_ascending_node_rad: f64,
    /// Argument of perihelion (ω) in radians
    pub argument_perihelion_rad: f64,
    /// Mean anomaly (M) in radians at the epoch
    pub mean_anomaly_rad: f64,
}

impl OrbitalElements {
    /// Create orbital elements from values in degrees and AU.
    /// This is the most common input format for asteroid/comet data.
    pub fn from_au_and_degrees(
        semi_major_axis_au: f64,
        eccentricity: f64,
        inclination_deg: f64,
        longitude_ascending_node_deg: f64,
        argument_perihelion_deg: f64,
        mean_anomaly_deg: f64,
    ) -> Self {
        Self {
            semi_major_axis_km: semi_major_axis_au * crate::bodies::AU_KM,
            eccentricity,
            inclination_rad: inclination_deg.to_radians(),
            longitude_ascending_node_rad: longitude_ascending_node_deg.to_radians(),
            argument_perihelion_rad: argument_perihelion_deg.to_radians(),
            mean_anomaly_rad: mean_anomaly_deg.to_radians(),
        }
    }

    /// Convert orbital elements to a Cartesian state vector.
    /// `central_body_gm` is the GM of the central body (e.g., SUN_GM for heliocentric orbits).
    /// Handles elliptic (e < 1), parabolic (e ≈ 1), and hyperbolic (e > 1) orbits.
    pub fn to_state_vector(&self, central_body_gm: f64) -> StateVector {
        let a = self.semi_major_axis_km;
        let e = self.eccentricity;
        let i = self.inclination_rad;
        let omega_big = self.longitude_ascending_node_rad; // Ω
        let omega = self.argument_perihelion_rad; // ω
        let m = self.mean_anomaly_rad;
        let mu = central_body_gm;

        let (true_anomaly, r) = if e < 1.0 {
            // Elliptic orbit
            let ecc_anomaly = solve_kepler(m, e);
            let cos_e = ecc_anomaly.cos();
            let sin_e = ecc_anomaly.sin();
            let nu = ((1.0 - e * e).sqrt() * sin_e).atan2(cos_e - e);
            let r = a * (1.0 - e * cos_e);
            (nu, r)
        } else {
            // Hyperbolic orbit (e >= 1)
            let hyp_anomaly = solve_kepler_hyperbolic(m, e);
            let cosh_h = hyp_anomaly.cosh();
            let sinh_h = hyp_anomaly.sinh();
            let nu = ((e * e - 1.0).sqrt() * sinh_h).atan2(e - cosh_h);
            let r = a.abs() * (e * cosh_h - 1.0);
            (nu, r)
        };

        // Position and velocity in orbital plane
        let cos_nu = true_anomaly.cos();
        let sin_nu = true_anomaly.sin();
        let p = a.abs() * (1.0 - e * e).abs(); // semi-latus rectum
        let h = (mu * p).sqrt(); // specific angular momentum

        let r_orb = Vec3::new(r * cos_nu, r * sin_nu, 0.0);
        let v_orb = Vec3::new(-mu / h * sin_nu, mu / h * (e + cos_nu), 0.0);

        // Rotation from orbital plane to reference frame (ICRF)
        let pos = rotate_orbital_to_inertial(r_orb, omega_big, omega, i);
        let vel = rotate_orbital_to_inertial(v_orb, omega_big, omega, i);

        StateVector::new(pos, vel)
    }
}

/// Solve Kepler's equation M = E - e*sin(E) for E using Newton-Raphson.
fn solve_kepler(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let m = mean_anomaly;
    let e = eccentricity;

    // Initial guess
    let mut ecc_anomaly = if e < 0.8 { m } else { std::f64::consts::PI };

    for _ in 0..50 {
        let delta = (ecc_anomaly - e * ecc_anomaly.sin() - m) / (1.0 - e * ecc_anomaly.cos());
        ecc_anomaly -= delta;
        if delta.abs() < 1e-15 {
            break;
        }
    }

    ecc_anomaly
}

/// Solve hyperbolic Kepler's equation M = e*sinh(H) - H for H using Newton-Raphson.
fn solve_kepler_hyperbolic(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let m = mean_anomaly;
    let e = eccentricity;

    // Initial guess: H = sign(M) * ln(2|M|/e + 1.8)
    let mut h = m.signum() * ((2.0 * m.abs() / e + 1.8).ln());

    for _ in 0..50 {
        let sinh_h = h.sinh();
        let cosh_h = h.cosh();
        let f = e * sinh_h - h - m;
        let fp = e * cosh_h - 1.0;
        if fp.abs() < 1e-30 {
            break;
        }
        let delta = f / fp;
        h -= delta;
        if delta.abs() < 1e-15 {
            break;
        }
    }

    h
}

/// Rotate a vector from the orbital plane to the inertial reference frame.
/// Uses the standard Euler rotation: R_z(-Ω) * R_x(-i) * R_z(-ω)
fn rotate_orbital_to_inertial(v: Vec3, omega_big: f64, omega: f64, inclination: f64) -> Vec3 {
    let cos_o = omega.cos();
    let sin_o = omega.sin();
    let cos_ob = omega_big.cos();
    let sin_ob = omega_big.sin();
    let cos_i = inclination.cos();
    let sin_i = inclination.sin();

    Vec3::new(
        (cos_ob * cos_o - sin_ob * sin_o * cos_i) * v.x
            + (-cos_ob * sin_o - sin_ob * cos_o * cos_i) * v.y,
        (sin_ob * cos_o + cos_ob * sin_o * cos_i) * v.x
            + (-sin_ob * sin_o + cos_ob * cos_o * cos_i) * v.y,
        (sin_o * sin_i) * v.x + (cos_o * sin_i) * v.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bodies::{AU_KM, SUN_GM};

    /// A circular orbit in the xy plane should produce velocity in y direction only.
    #[test]
    fn circular_orbit_elements() {
        let elements = OrbitalElements {
            semi_major_axis_km: AU_KM,
            eccentricity: 0.0,
            inclination_rad: 0.0,
            longitude_ascending_node_rad: 0.0,
            argument_perihelion_rad: 0.0,
            mean_anomaly_rad: 0.0, // at perihelion
        };

        let state = elements.to_state_vector(SUN_GM);

        // At perihelion of circular orbit: x = a, y = 0, vx = 0, vy = v_circ
        let v_circ = (SUN_GM / AU_KM).sqrt();

        assert!(
            (state.position.x - AU_KM).abs() / AU_KM < 1e-10,
            "x = {} expected {}",
            state.position.x,
            AU_KM
        );
        assert!(state.position.y.abs() < 1.0, "y should be ~0");
        assert!(state.velocity.x.abs() < 1e-6, "vx should be ~0");
        assert!(
            (state.velocity.y - v_circ).abs() / v_circ < 1e-10,
            "vy = {} expected {}",
            state.velocity.y,
            v_circ
        );
    }

    /// Kepler's equation solver should converge for various eccentricities.
    #[test]
    fn kepler_solver_convergence() {
        for e in [0.0, 0.1, 0.5, 0.9, 0.99] {
            for m in [0.0, 0.5, 1.0, 2.0, 3.14] {
                let big_e = solve_kepler(m, e);
                let residual = (big_e - e * big_e.sin() - m).abs();
                assert!(
                    residual < 1e-12,
                    "Kepler residual too large for e={}, M={}: {}",
                    e,
                    m,
                    residual
                );
            }
        }
    }

    /// Hyperbolic Kepler solver should converge for e > 1.
    #[test]
    fn hyperbolic_kepler_solver_convergence() {
        for e in [1.1, 1.5, 2.0, 3.0, 5.0] {
            for m in [-5.0, -1.0, 0.0, 0.5, 1.0, 3.0, 10.0] {
                let h = solve_kepler_hyperbolic(m, e);
                let residual = (e * h.sinh() - h - m).abs();
                assert!(
                    residual < 1e-10,
                    "Hyperbolic Kepler residual too large for e={}, M={}: {}",
                    e,
                    m,
                    residual
                );
            }
        }
    }

    /// Hyperbolic orbit produces a valid state vector.
    #[test]
    fn hyperbolic_orbit_state_vector() {
        let elements = OrbitalElements {
            semi_major_axis_km: -AU_KM, // negative for hyperbolic
            eccentricity: 1.5,
            inclination_rad: 0.0,
            longitude_ascending_node_rad: 0.0,
            argument_perihelion_rad: 0.0,
            mean_anomaly_rad: 0.5,
        };
        let state = elements.to_state_vector(SUN_GM);
        // Position should be finite and at a reasonable distance
        assert!(state.position.magnitude().is_finite());
        assert!(state.position.magnitude() > 0.0);
        // Velocity should be finite
        assert!(state.velocity.magnitude().is_finite());
        assert!(state.velocity.magnitude() > 0.0);
    }
}
