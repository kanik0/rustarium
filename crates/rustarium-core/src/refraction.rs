/// Atmospheric refraction correction in radians.
/// Given the true (geometric) altitude, returns the refraction to ADD
/// to get the apparent altitude.
/// Uses Meeus formula 16.3 (Bennett's formula for true altitude).
///
/// Input: true_altitude in radians.
/// Returns: refraction angle in radians (always positive).
pub fn refraction(true_altitude: f64) -> f64 {
    let alt_deg = true_altitude.to_degrees();

    if alt_deg < -1.0 {
        return 0.0; // Below horizon, no meaningful refraction
    }

    // Meeus eq. 16.3: R in arcminutes, h in degrees
    // R = 1 / tan(h + 7.31/(h + 4.4))
    let arg_deg = alt_deg + 7.31 / (alt_deg + 4.4);
    let r_arcmin = 1.0 / arg_deg.to_radians().tan();

    // Convert arcminutes to radians
    (r_arcmin / 60.0).to_radians().max(0.0)
}

/// Standard altitude h0 (geometric altitude of center of body at rise/set).
/// This accounts for atmospheric refraction and the body's apparent size.
///
/// For rise/set purposes, a body "rises" when its center is at altitude h0.
pub fn standard_altitude_for_rise_set(body: crate::bodies::Body, moon_parallax: Option<f64>) -> f64 {
    match body {
        crate::bodies::Body::Sun => {
            // Sun: refraction (34') + semi-diameter (16') = -50' = -0.8333°
            -0.8333_f64.to_radians()
        }
        crate::bodies::Body::Moon => {
            // Moon: 0.7275 * parallax - 34' refraction
            // Parallax varies ~0.9° - 1.0°, so h0 ≈ +0.125°
            let par = moon_parallax.unwrap_or(0.9507_f64.to_radians());
            0.7275 * par - (34.0 / 60.0_f64).to_radians()
        }
        crate::bodies::Body::Planet(_) => {
            // Stars and planets: -34' = -0.5667° (refraction only)
            -0.5667_f64.to_radians()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refraction_at_horizon() {
        // At the horizon (0°), refraction ≈ 34 arcminutes
        let r = refraction(0.0);
        let r_arcmin = r.to_degrees() * 60.0;
        assert!(
            (r_arcmin - 34.5).abs() < 1.0,
            "Refraction at horizon: {:.1}' expected ~34.5'",
            r_arcmin
        );
    }

    #[test]
    fn refraction_at_zenith() {
        // At zenith (90°), refraction ≈ 0
        let r = refraction(std::f64::consts::FRAC_PI_2);
        let r_arcmin = r.to_degrees() * 60.0;
        assert!(r_arcmin < 0.1, "Refraction at zenith: {:.4}' expected ~0", r_arcmin);
    }
}
