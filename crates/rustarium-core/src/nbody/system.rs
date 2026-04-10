use crate::bodies::AU_KM;
use crate::coords::Vec3;
use crate::nbody::initial_conditions;
use crate::nbody::integrator::{self, IntegratorConfig};
use crate::nbody::state::NBodyObject;
use crate::time::{JulianDay, J2000};
use serde::{Deserialize, Serialize};

/// A complete N-body simulation system.
#[derive(Debug, Clone)]
pub struct NBodySystem {
    /// All bodies in the simulation
    pub bodies: Vec<NBodyObject>,
    /// Current simulation time as Julian Day (TDB)
    pub current_jd: JulianDay,
    /// Integrator configuration
    pub config: IntegratorConfig,
    /// Current adaptive step size (seconds)
    step_size: f64,
}

/// Snapshot of the system state at a given time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub jd: JulianDay,
    pub bodies: Vec<BodySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodySnapshot {
    pub name: String,
    pub position_km: Vec3,
    pub velocity_km_s: Vec3,
    pub distance_au: f64,
}

impl NBodySystem {
    /// Create a solar system simulation initialized at J2000.0.
    pub fn solar_system() -> Self {
        Self {
            bodies: initial_conditions::solar_system_j2000(),
            current_jd: J2000,
            config: IntegratorConfig::default(),
            step_size: 86400.0,
        }
    }

    /// Create a system with custom bodies and start time.
    pub fn new(bodies: Vec<NBodyObject>, start_jd: JulianDay) -> Self {
        Self {
            bodies,
            current_jd: start_jd,
            config: IntegratorConfig::default(),
            step_size: 86400.0,
        }
    }

    /// Add a custom body to the simulation.
    pub fn add_body(&mut self, body: NBodyObject) {
        self.bodies.push(body);
    }

    /// Propagate the system forward or backward to the target Julian Day.
    /// Returns snapshots at each output step.
    pub fn propagate_to(
        &mut self,
        target_jd: JulianDay,
        output_step_days: Option<f64>,
    ) -> Vec<Snapshot> {
        let mut snapshots = Vec::new();
        let total_seconds = (target_jd.0 - self.current_jd.0) * 86400.0;
        let direction = total_seconds.signum();

        let output_step_seconds = output_step_days.unwrap_or(1.0) * 86400.0;
        let mut next_output = output_step_seconds;
        let mut elapsed: f64 = 0.0;

        // Save initial state
        snapshots.push(self.snapshot());

        while elapsed.abs() < total_seconds.abs() {
            let remaining = total_seconds - elapsed;
            let step = (direction * self.step_size).min(remaining.abs()) * direction;

            self.step_size = integrator::step(&mut self.bodies, step, &self.config);
            elapsed += step;
            self.current_jd = JulianDay(self.current_jd.0 + step / 86400.0);

            // Output at regular intervals
            if elapsed.abs() >= next_output.abs() {
                snapshots.push(self.snapshot());
                next_output += output_step_seconds * direction;
            }
        }

        // Always include final state
        if let Some(last) = snapshots.last() {
            if (last.jd.0 - self.current_jd.0).abs() > 1e-10 {
                snapshots.push(self.snapshot());
            }
        }

        snapshots
    }

    /// Get a snapshot of the current system state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            jd: self.current_jd,
            bodies: self
                .bodies
                .iter()
                .map(|b| BodySnapshot {
                    name: b.name.clone(),
                    position_km: b.state.position,
                    velocity_km_s: b.state.velocity,
                    distance_au: b.state.position.magnitude() / AU_KM,
                })
                .collect(),
        }
    }

    /// Get the position of a body by name.
    pub fn body_position(&self, name: &str) -> Option<Vec3> {
        self.bodies
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.state.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_system_creates() {
        let sys = NBodySystem::solar_system();
        assert_eq!(sys.bodies.len(), 12); // Sun + 8 planets + 3 asteroids
        assert_eq!(sys.current_jd, J2000);
    }

    #[test]
    fn propagate_one_day() {
        let mut sys = NBodySystem::solar_system();
        let target = J2000 + 1.0;
        let snaps = sys.propagate_to(target, Some(1.0));

        assert!(snaps.len() >= 2, "Should have at least start and end snapshots");
        assert!((sys.current_jd.0 - target.0).abs() < 0.01);
    }

    #[test]
    fn body_position_lookup() {
        let sys = NBodySystem::solar_system();
        let earth = sys.body_position("Earth");
        assert!(earth.is_some());
        assert!(sys.body_position("Pluto").is_none());
    }
}
