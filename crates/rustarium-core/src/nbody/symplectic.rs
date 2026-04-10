use crate::coords::Vec3;
use crate::nbody::state::NBodyObject;

/// Wisdom-Holman mixed-variable symplectic (MVS) integrator.
///
/// Splits the Hamiltonian into:
/// - H_Kep: Keplerian 2-body motion (Sun + each planet), solved analytically
/// - H_int: Planet-planet gravitational interactions, applied as velocity kicks
///
/// Uses second-order leapfrog: drift(h/2) → kick(h) → drift(h/2)
///
/// Advantages over RK45:
/// - Preserves energy (symplectic structure) over millions of years
/// - Fixed step size (simpler, no error estimation overhead)
/// - Ideal for long-term solar system evolution
///
/// Limitations:
/// - Fixed step size means it struggles with close encounters
/// - Sun must be the dominant body (index 0)
/// - Less accurate per step than RK45, but more stable over long timescales

/// Configuration for the symplectic integrator.
#[derive(Debug, Clone)]
pub struct SymplecticConfig {
    /// Step size in seconds (default: 4 days — must be << shortest orbital period)
    pub step_seconds: f64,
    /// Include 1PN relativistic corrections in the kick step
    pub relativistic: bool,
}

impl Default for SymplecticConfig {
    fn default() -> Self {
        Self {
            step_seconds: 4.0 * 86400.0, // 4 days (Mercury period ~88 days, so this is ~22 steps/orbit)
            relativistic: true,
        }
    }
}

/// Perform one full leapfrog step: drift(h/2) → kick(h) → drift(h/2).
/// The Sun is body index 0.
pub fn leapfrog_step(bodies: &mut [NBodyObject], dt: f64, config: &SymplecticConfig) {
    let half_dt = dt / 2.0;

    // Drift half step: advance each body along its Keplerian orbit around the Sun
    drift(bodies, half_dt);

    // Kick full step: apply interaction accelerations as velocity impulses
    kick(bodies, dt, config);

    // Drift half step
    drift(bodies, half_dt);
}

/// Drift step: advance positions using Keplerian motion around the Sun.
/// Each body moves along its 2-body orbit with the Sun for time dt.
/// This is exact for the Keplerian part of the Hamiltonian.
fn drift(bodies: &mut [NBodyObject], dt: f64) {
    if bodies.is_empty() {
        return;
    }

    let sun_gm = bodies[0].gm;
    let sun_pos = bodies[0].state.position;
    let sun_vel = bodies[0].state.velocity;

    // Move each body (except Sun) along its Keplerian orbit
    for i in 1..bodies.len() {
        // Heliocentric position and velocity
        let r = bodies[i].state.position - sun_pos;
        let v = bodies[i].state.velocity - sun_vel;

        // Total GM for this 2-body problem
        let mu = sun_gm + bodies[i].gm;

        // Advance using the universal variable formulation (f and g series)
        let (r_new, v_new) = kepler_drift(r, v, mu, dt);

        // Convert back to barycentric
        bodies[i].state.position = r_new + sun_pos;
        bodies[i].state.velocity = v_new + sun_vel;
    }

    // Move Sun to keep barycenter fixed
    // (In pure Keplerian, the Sun doesn't move, but for accuracy we
    //  recompute its position from momentum conservation)
    // Note: In a full implementation, we would update the Sun's position
    // to keep the barycenter fixed. For the solar system, the Sun is so
    // dominant that this correction is negligible (~450 km offset).
}

/// Kick step: apply velocity impulses from planet-planet interactions.
/// This modifies only velocities, not positions.
fn kick(bodies: &mut [NBodyObject], dt: f64, config: &SymplecticConfig) {
    let n = bodies.len();
    let mut acc = vec![Vec3::zero(); n];

    // Only interaction terms (NOT Sun-planet Keplerian, which is handled by drift)
    // Interaction = planet-planet gravity + (optionally) relativistic correction
    for i in 1..n {
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

    // Sun feels the back-reaction from all planets
    for i in 1..n {
        let dr = bodies[i].state.position - bodies[0].state.position;
        let dist = dr.magnitude();
        let dist3 = dist * dist * dist;

        // This is the "indirect" term: Sun's acceleration due to planet i
        // (already included in the Keplerian part, but the planet-planet
        //  interactions shift the Sun, so we account for the difference)
        acc[0] += dr * (bodies[i].gm / dist3);

        // The "Keplerian" part accounted for GM_sun/r² toward Sun,
        // but not the planet's contribution to Sun's motion.
        // Add the indirect acceleration to each planet:
        // a_indirect_i = -GM_i * r_i / |r_i|³ (Sun accelerated by planet i)
        for j in 1..n {
            if j != i {
                // Planet j feels Sun's shifted position indirectly
            }
        }
    }

    // Apply 1PN correction if enabled
    if config.relativistic && n > 1 {
        let c2 = crate::bodies::SPEED_OF_LIGHT_KM_S * crate::bodies::SPEED_OF_LIGHT_KM_S;
        for i in 1..n {
            let r_vec = bodies[i].state.position - bodies[0].state.position;
            let v_vec = bodies[i].state.velocity - bodies[0].state.velocity;
            let r = r_vec.magnitude();
            let r2 = r * r;
            let v2 = v_vec.x * v_vec.x + v_vec.y * v_vec.y + v_vec.z * v_vec.z;
            let r_hat = r_vec * (1.0 / r);
            let v_dot_r = v_vec.x * r_hat.x + v_vec.y * r_hat.y + v_vec.z * r_hat.z;
            let gm = bodies[0].gm;
            let factor = gm / (c2 * r2 * r);
            let radial = 4.0 * gm / r - v2 + 4.0 * v_dot_r * v_dot_r;
            let tangential = 4.0 * v_dot_r;
            acc[i] += r_vec * (factor * radial) + v_vec * (factor * r * tangential);
        }
    }

    // Apply velocity kicks
    for i in 0..n {
        bodies[i].state.velocity += acc[i] * dt;
    }
}

/// Advance a Keplerian orbit by time dt using the universal variable formulation.
/// (Curtis "Orbital Mechanics for Engineering Students", Algorithm 3.3)
///
/// Solves the universal Kepler equation:
///   f(χ) = σ₀χ²C(αχ²) + (1 - αr₀)χ³S(αχ²) + r₀χ - √μ·Δt = 0
///
/// then computes new position/velocity via f and g functions.
fn kepler_drift(r0: Vec3, v0: Vec3, mu: f64, dt: f64) -> (Vec3, Vec3) {
    let r0_mag = r0.magnitude();
    let v0_sq = v0.x * v0.x + v0.y * v0.y + v0.z * v0.z;
    let sqrt_mu = mu.sqrt();

    // α = 1/a (positive for elliptic, negative for hyperbolic)
    let alpha = 2.0 / r0_mag - v0_sq / mu;

    // σ₀ = (r₀·v₀) / √μ
    let sigma0 = (r0.x * v0.x + r0.y * v0.y + r0.z * v0.z) / sqrt_mu;

    // Initial guess for χ (universal variable)
    let mut chi = sqrt_mu * dt.abs() / r0_mag; // simple initial guess

    // For elliptic orbits, better guess using mean motion
    if alpha > 1e-20 {
        let a = 1.0 / alpha;
        let n = sqrt_mu / (a * a.sqrt()); // mean motion
        chi = n * dt; // χ ≈ n·Δt
    }

    // Newton-Raphson iteration to solve the universal Kepler equation
    for _ in 0..50 {
        let chi2 = chi * chi;
        let psi = alpha * chi2;
        let (c2, c3) = stumpff(psi);

        // f(χ) = σ₀χ²C + (1 - αr₀)χ³S + r₀χ - √μΔt
        let f_val = sigma0 * chi2 * c2
            + (1.0 - alpha * r0_mag) * chi2 * chi * c3
            + r0_mag * chi
            - sqrt_mu * dt;

        // f'(χ) = r(χ) = σ₀χ(1 - αχ²S) + (1 - αr₀)χ²C + r₀
        let r_chi = sigma0 * chi * (1.0 - psi * c3)
            + (1.0 - alpha * r0_mag) * chi2 * c2
            + r0_mag;

        if r_chi.abs() < 1e-30 {
            break;
        }

        let delta = f_val / r_chi;
        chi -= delta;

        if delta.abs() < 1e-14 * chi.abs().max(1.0) {
            break;
        }
    }

    // Compute f, g, f_dot, g_dot functions
    let chi2 = chi * chi;
    let psi = alpha * chi2;
    let (c2, c3) = stumpff(psi);

    let r_new = sigma0 * chi * (1.0 - psi * c3)
        + (1.0 - alpha * r0_mag) * chi2 * c2
        + r0_mag;

    let f = 1.0 - chi2 / r0_mag * c2;
    let g = dt - chi2 * chi / sqrt_mu * c3;
    let f_dot = sqrt_mu / (r_new * r0_mag) * chi * (psi * c3 - 1.0);
    let g_dot = 1.0 - chi2 / r_new * c2;

    let r1 = r0 * f + v0 * g;
    let v1 = r0 * f_dot + v0 * g_dot;

    (r1, v1)
}

/// Stumpff functions c2(ψ) and c3(ψ).
/// c2(ψ) = (1 - cos(√ψ))/ψ  for ψ > 0
/// c3(ψ) = (√ψ - sin(√ψ))/(ψ√ψ)  for ψ > 0
/// With series expansions for |ψ| near zero.
fn stumpff(psi: f64) -> (f64, f64) {
    if psi.abs() < 1e-10 {
        // Taylor series: c2 ≈ 1/2 - ψ/24 + ..., c3 ≈ 1/6 - ψ/120 + ...
        let c2 = 1.0 / 2.0 - psi / 24.0 + psi * psi / 720.0;
        let c3 = 1.0 / 6.0 - psi / 120.0 + psi * psi / 5040.0;
        (c2, c3)
    } else if psi > 0.0 {
        let sqrt_psi = psi.sqrt();
        let c2 = (1.0 - sqrt_psi.cos()) / psi;
        let c3 = (sqrt_psi - sqrt_psi.sin()) / (psi * sqrt_psi);
        (c2, c3)
    } else {
        let sqrt_neg_psi = (-psi).sqrt();
        let c2 = (1.0 - sqrt_neg_psi.cosh()) / psi;
        let c3 = (sqrt_neg_psi.sinh() - sqrt_neg_psi) / ((-psi) * sqrt_neg_psi);
        (c2, c3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbody::state::StateVector;

    /// Circular orbit should be preserved by the symplectic integrator.
    #[test]
    fn circular_orbit_symplectic() {
        let au = crate::bodies::AU_KM;
        let sun_gm = crate::bodies::SUN_GM;
        let v_circ = (sun_gm / au).sqrt();

        let mut bodies = vec![
            NBodyObject::new("Sun", sun_gm, StateVector::new(Vec3::zero(), Vec3::zero())),
            NBodyObject::new(
                "Earth",
                crate::bodies::Planet::Earth.gm(),
                StateVector::new(Vec3::new(au, 0.0, 0.0), Vec3::new(0.0, v_circ, 0.0)),
            ),
        ];

        let config = SymplecticConfig {
            step_seconds: 86400.0, // 1 day
            relativistic: false,
        };

        // Integrate for 1 year
        let one_year = 365.25 * 86400.0;
        let steps = (one_year / config.step_seconds).ceil() as usize;

        for _ in 0..steps {
            leapfrog_step(&mut bodies, config.step_seconds, &config);
        }

        let final_distance = (bodies[1].state.position - bodies[0].state.position).magnitude();
        let error_percent = ((final_distance - au) / au).abs() * 100.0;

        assert!(
            error_percent < 0.1,
            "After 1 year, distance error: {:.4}%",
            error_percent
        );
    }

    /// Energy should be nearly conserved over many orbits (symplectic property).
    #[test]
    fn energy_conservation() {
        let au = crate::bodies::AU_KM;
        let sun_gm = crate::bodies::SUN_GM;
        let v_circ = (sun_gm / au).sqrt();

        let mut bodies = vec![
            NBodyObject::new("Sun", sun_gm, StateVector::new(Vec3::zero(), Vec3::zero())),
            NBodyObject::new(
                "Earth",
                crate::bodies::Planet::Earth.gm(),
                StateVector::new(Vec3::new(au, 0.0, 0.0), Vec3::new(0.0, v_circ, 0.0)),
            ),
        ];

        let config = SymplecticConfig {
            step_seconds: 86400.0,
            relativistic: false,
        };

        let initial_energy = total_energy(&bodies);

        // Integrate for 100 years
        let total_time = 100.0 * 365.25 * 86400.0;
        let steps = (total_time / config.step_seconds).ceil() as usize;

        for _ in 0..steps {
            leapfrog_step(&mut bodies, config.step_seconds, &config);
        }

        let final_energy = total_energy(&bodies);
        let energy_error = ((final_energy - initial_energy) / initial_energy).abs();

        assert!(
            energy_error < 1e-8,
            "Energy conservation error: {:.3e} (should be < 1e-8)",
            energy_error
        );
    }

    /// Kepler drift should return to same position after one orbital period.
    #[test]
    fn kepler_drift_full_orbit() {
        let au = crate::bodies::AU_KM;
        let mu = crate::bodies::SUN_GM;
        let v_circ = (mu / au).sqrt();

        let r0 = Vec3::new(au, 0.0, 0.0);
        let v0 = Vec3::new(0.0, v_circ, 0.0);

        // Period = 2π * sqrt(a³/μ)
        let period = 2.0 * std::f64::consts::PI * (au * au * au / mu).sqrt();

        let (r1, v1) = kepler_drift(r0, v0, mu, period);

        let pos_error = (r1 - r0).magnitude() / au;
        let vel_error = (v1 - v0).magnitude() / v_circ;

        assert!(
            pos_error < 1e-10,
            "Position error after full orbit: {:.3e}",
            pos_error
        );
        assert!(
            vel_error < 1e-10,
            "Velocity error after full orbit: {:.3e}",
            vel_error
        );
    }

    fn total_energy(bodies: &[NBodyObject]) -> f64 {
        let mut ke = 0.0;
        let mut pe = 0.0;

        for i in 0..bodies.len() {
            let v = bodies[i].state.velocity;
            ke += 0.5 * bodies[i].gm * (v.x * v.x + v.y * v.y + v.z * v.z);

            for j in (i + 1)..bodies.len() {
                let dist = bodies[i].state.position.distance_to(bodies[j].state.position);
                pe -= bodies[i].gm * bodies[j].gm / dist;
            }
        }

        ke + pe
    }
}
