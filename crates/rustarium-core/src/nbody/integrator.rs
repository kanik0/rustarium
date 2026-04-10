use crate::coords::Vec3;
use crate::nbody::gravity::all_accelerations;
use crate::nbody::state::NBodyObject;

/// Configuration for the Dormand-Prince RK45 adaptive integrator.
#[derive(Debug, Clone)]
pub struct IntegratorConfig {
    /// Initial step size in seconds (default: 86400 = 1 day)
    pub initial_step: f64,
    /// Minimum step size in seconds (default: 60 = 1 minute)
    pub min_step: f64,
    /// Maximum step size in seconds (default: 864000 = 10 days)
    pub max_step: f64,
    /// Position error tolerance in km (default: 1.0)
    pub tolerance: f64,
    /// Safety factor for step size control (default: 0.9)
    pub safety: f64,
}

impl Default for IntegratorConfig {
    fn default() -> Self {
        Self {
            initial_step: 86400.0,
            min_step: 60.0,
            max_step: 864000.0,
            tolerance: 1.0,
            safety: 0.9,
        }
    }
}

/// Dormand-Prince RK45 Butcher tableau coefficients.
/// b[i][j] = RK matrix coefficients
const B: [[f64; 6]; 6] = [
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
    [
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
        0.0,
        0.0,
    ],
    [
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
        0.0,
    ],
    [
        35.0 / 384.0,
        0.0,
        500.0 / 1113.0,
        125.0 / 192.0,
        -2187.0 / 6784.0,
        11.0 / 84.0,
    ],
];

/// 5th-order weights
const C5: [f64; 7] = [
    35.0 / 384.0,
    0.0,
    500.0 / 1113.0,
    125.0 / 192.0,
    -2187.0 / 6784.0,
    11.0 / 84.0,
    0.0,
];

/// 4th-order weights (for error estimation)
const C4: [f64; 7] = [
    5179.0 / 57600.0,
    0.0,
    7571.0 / 16695.0,
    393.0 / 640.0,
    -92097.0 / 339200.0,
    187.0 / 2100.0,
    1.0 / 40.0,
];

/// Perform one adaptive Dormand-Prince RK45 step.
/// Returns the new state of all bodies and the step size used.
/// If the error is too large, the step is rejected and retried with a smaller step.
pub fn step(
    bodies: &mut [NBodyObject],
    dt: f64,
    config: &IntegratorConfig,
) -> f64 {
    let n = bodies.len();
    let mut h = dt;

    loop {
        // Compute k1..k7 stages
        let mut k_pos: Vec<[Vec3; 7]> = vec![[Vec3::zero(); 7]; n];
        let mut k_vel: Vec<[Vec3; 7]> = vec![[Vec3::zero(); 7]; n];

        // k1: acceleration at current state
        let acc = all_accelerations(bodies);
        for i in 0..n {
            k_pos[i][0] = bodies[i].state.velocity;
            k_vel[i][0] = acc[i];
        }

        // k2..k7
        for stage in 1..7 {
            // Create temporary state at this stage
            let mut temp_bodies = bodies.to_vec();
            for i in 0..n {
                let mut dp = Vec3::zero();
                let mut dv = Vec3::zero();
                for j in 0..stage {
                    dp += k_pos[i][j] * B[stage - 1][j];
                    dv += k_vel[i][j] * B[stage - 1][j];
                }
                temp_bodies[i].state.position = bodies[i].state.position + dp * h;
                temp_bodies[i].state.velocity = bodies[i].state.velocity + dv * h;
            }

            let acc = all_accelerations(&temp_bodies);
            for i in 0..n {
                k_pos[i][stage] = temp_bodies[i].state.velocity;
                k_vel[i][stage] = acc[i];
            }
        }

        // Compute 5th-order solution and error estimate
        let mut max_error = 0.0_f64;
        let mut new_positions = vec![Vec3::zero(); n];
        let mut new_velocities = vec![Vec3::zero(); n];

        for i in 0..n {
            let mut dp5 = Vec3::zero();
            let mut dv5 = Vec3::zero();
            let mut dp4 = Vec3::zero();
            let mut dv4 = Vec3::zero();

            for j in 0..7 {
                dp5 += k_pos[i][j] * C5[j];
                dv5 += k_vel[i][j] * C5[j];
                dp4 += k_pos[i][j] * C4[j];
                dv4 += k_vel[i][j] * C4[j];
            }

            new_positions[i] = bodies[i].state.position + dp5 * h;
            new_velocities[i] = bodies[i].state.velocity + dv5 * h;

            // Error estimate: difference between 4th and 5th order
            let err_pos = (dp5 - dp4) * h;
            let err = err_pos.magnitude();
            max_error = max_error.max(err);
        }

        // Step size control
        if max_error <= config.tolerance || h <= config.min_step {
            // Accept step
            for i in 0..n {
                bodies[i].state.position = new_positions[i];
                bodies[i].state.velocity = new_velocities[i];
            }

            // Compute optimal step size for next step
            if max_error > 0.0 {
                let ratio = config.tolerance / max_error;
                let new_h = h * config.safety * ratio.powf(0.2);
                return new_h.clamp(config.min_step, config.max_step);
            }
            return (h * 2.0).min(config.max_step);
        }

        // Reject step: reduce step size and retry
        let ratio = config.tolerance / max_error;
        h = (h * config.safety * ratio.powf(0.25)).max(config.min_step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::Vec3;
    use crate::nbody::state::{NBodyObject, StateVector};

    /// Test circular orbit: Earth around Sun should stay at ~1 AU
    #[test]
    fn circular_orbit_stability() {
        let au = crate::bodies::AU_KM;
        let sun_gm = crate::bodies::SUN_GM;

        // Circular orbit velocity: v = sqrt(GM/r)
        let v_circ = (sun_gm / au).sqrt();

        let mut bodies = vec![
            NBodyObject::new("Sun", sun_gm, StateVector::new(Vec3::zero(), Vec3::zero())),
            NBodyObject::new(
                "Earth",
                crate::bodies::Planet::Earth.gm(),
                StateVector::new(Vec3::new(au, 0.0, 0.0), Vec3::new(0.0, v_circ, 0.0)),
            ),
        ];

        let config = IntegratorConfig::default();
        let mut t = 0.0;
        let one_year = 365.25 * 86400.0; // seconds
        let mut h = config.initial_step;

        // Integrate for 1 year
        while t < one_year {
            let remaining = one_year - t;
            let step_size = h.min(remaining);
            h = step(&mut bodies, step_size, &config);
            t += step_size;
        }

        // Check that Earth is still at approximately 1 AU
        let final_distance = bodies[1].state.position.magnitude();
        let error_percent = ((final_distance - au) / au).abs() * 100.0;

        assert!(
            error_percent < 0.01,
            "After 1 year, Earth distance error: {:.4}% (distance: {:.2} km, expected: {:.2} km)",
            error_percent,
            final_distance,
            au
        );
    }
}
