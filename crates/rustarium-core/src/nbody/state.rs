use crate::coords::Vec3;
use serde::{Deserialize, Serialize};

/// State vector of a single body: position and velocity in ICRF/J2000 frame.
/// Units: km and km/s (standard JPL units for state vectors).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StateVector {
    /// Position (km) in ICRF/J2000 frame, Solar System Barycenter origin
    pub position: Vec3,
    /// Velocity (km/s) in ICRF/J2000 frame
    pub velocity: Vec3,
}

impl StateVector {
    pub fn new(pos: Vec3, vel: Vec3) -> Self {
        Self {
            position: pos,
            velocity: vel,
        }
    }
}

/// A body in the N-body simulation with its physical properties and current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NBodyObject {
    /// Human-readable name
    pub name: String,
    /// Gravitational parameter GM (km³/s²)
    pub gm: f64,
    /// Current state vector
    pub state: StateVector,
}

impl NBodyObject {
    pub fn new(name: impl Into<String>, gm: f64, state: StateVector) -> Self {
        Self {
            name: name.into(),
            gm,
            state,
        }
    }
}
