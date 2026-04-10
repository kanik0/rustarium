use crate::bodies::Body;
use crate::coords::{equatorial_to_horizontal, GeoLocation, EquatorialCoords};
use crate::observer;
use crate::refraction::standard_altitude_for_rise_set;
use crate::time::{greenwich_mean_sidereal_time, normalize_radians, JulianDay};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Type of rise/set event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Rise,
    Transit,
    Set,
}

/// A rise, transit, or set event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiseSetEvent {
    pub event: EventType,
    /// Julian Day (UT) of the event
    pub jd: JulianDay,
    /// Azimuth in degrees (for rise/set, None for transit)
    pub azimuth_deg: Option<f64>,
    /// Altitude in degrees (for transit, None for rise/set)
    pub altitude_deg: Option<f64>,
}

/// Error type for rise/set calculations.
#[derive(Debug, Clone)]
pub enum RiseSetError {
    /// Body is always above the horizon (circumpolar)
    AlwaysAbove,
    /// Body is always below the horizon
    AlwaysBelow,
    /// Computation did not converge
    NoConvergence,
}

/// Compute rise, transit, and set times for a celestial body.
///
/// Uses the algorithm from Meeus chapter 15 with iterative refinement.
///
/// `jd_0h_ut`: Julian Day at 0h UT on the date of interest.
/// `observer`: Geographic location.
/// `body`: Which body to compute for.
///
/// The `equatorial_at` function provides the body's geocentric equatorial
/// coordinates at a given JD. This abstraction allows using either VSOP87
/// (planets/Sun) or ELP (Moon).
pub fn rise_transit_set(
    jd_0h_ut: JulianDay,
    observer: &GeoLocation,
    body: Body,
    equatorial_at: impl Fn(JulianDay) -> EquatorialCoords,
) -> Result<Vec<RiseSetEvent>, RiseSetError> {
    // Get equatorial positions at JD-1, JD, JD+1 for interpolation
    let eq_prev = equatorial_at(jd_0h_ut - 1.0);
    let eq_curr = equatorial_at(jd_0h_ut);
    let eq_next = equatorial_at(jd_0h_ut + 1.0);

    // Standard altitude h0
    let moon_par = if body == Body::Moon {
        Some(crate::moon::horizontal_parallax(jd_0h_ut))
    } else {
        None
    };
    let h0 = standard_altitude_for_rise_set(body, moon_par);

    // Sidereal time at Greenwich at 0h UT
    let theta0 = greenwich_mean_sidereal_time(jd_0h_ut);

    // Approximate hour angle at rise/set
    let cos_h0 = (h0.sin() - observer.lat.sin() * eq_curr.dec.sin())
        / (observer.lat.cos() * eq_curr.dec.cos());

    if cos_h0 < -1.0 {
        return Err(RiseSetError::AlwaysAbove);
    }
    if cos_h0 > 1.0 {
        return Err(RiseSetError::AlwaysBelow);
    }

    let h_rise_set = cos_h0.acos(); // hour angle at rise/set

    // Approximate times as fraction of day
    // Transit: m0 = (RA - lon - theta0) / (2*pi)
    let m_transit = normalize_m((eq_curr.ra - observer.lon - theta0) / (2.0 * PI));
    let m_rise = normalize_m(m_transit - h_rise_set / (2.0 * PI));
    let m_set = normalize_m(m_transit + h_rise_set / (2.0 * PI));

    let mut events = Vec::new();

    // Refine each event iteratively
    for (event_type, m_init) in [
        (EventType::Rise, m_rise),
        (EventType::Transit, m_transit),
        (EventType::Set, m_set),
    ] {
        let mut m = m_init;

        for _ in 0..5 {
            // Interpolated RA and Dec at time m
            let ra = interpolate_angle(eq_prev.ra, eq_curr.ra, eq_next.ra, m);
            let dec = interpolate(eq_prev.dec, eq_curr.dec, eq_next.dec, m);

            // Local hour angle at time m
            let theta = theta0 + 6.300388092591991 * m; // 360.985647° per day in radians
            let h = normalize_radians(theta + observer.lon - ra);
            let h = if h > PI { h - 2.0 * PI } else { h };

            if event_type == EventType::Transit {
                // Correction: dm = -H / (2*pi)
                let dm = -h / (2.0 * PI);
                m += dm;
                if dm.abs() < 1e-8 {
                    break;
                }
            } else {
                // Altitude at this time
                let alt = (observer.lat.sin() * dec.sin()
                    + observer.lat.cos() * dec.cos() * h.cos())
                .asin();

                let dm = (alt - h0) / (2.0 * PI * dec.cos() * observer.lat.cos() * h.sin());
                m += dm;
                if dm.abs() < 1e-8 {
                    break;
                }
            }
        }

        // Only include events that fall within the day [0, 1)
        if m >= -0.01 && m <= 1.01 {
            let jd = jd_0h_ut + m;

            let (azimuth_deg, altitude_deg) = if event_type == EventType::Transit {
                // Compute transit altitude
                let eq = equatorial_at(jd);
                let lst = observer::local_mean_sidereal_time(jd, observer);
                let ha = observer::hour_angle(lst, eq.ra);
                let hz = equatorial_to_horizontal(&eq, ha, observer.lat);
                (None, Some(hz.altitude.to_degrees()))
            } else {
                // Compute azimuth at rise/set
                let eq = equatorial_at(jd);
                let lst = observer::local_mean_sidereal_time(jd, observer);
                let ha = observer::hour_angle(lst, eq.ra);
                let hz = equatorial_to_horizontal(&eq, ha, observer.lat);
                (Some(hz.azimuth.to_degrees()), None)
            };

            events.push(RiseSetEvent {
                event: event_type,
                jd: JulianDay(jd.0),
                azimuth_deg,
                altitude_deg,
            });
        }
    }

    if events.is_empty() {
        return Err(RiseSetError::NoConvergence);
    }

    Ok(events)
}

/// Normalize m to [0, 1)
fn normalize_m(mut m: f64) -> f64 {
    while m < 0.0 {
        m += 1.0;
    }
    while m >= 1.0 {
        m -= 1.0;
    }
    m
}

/// Linear interpolation
fn interpolate(y_prev: f64, y_curr: f64, y_next: f64, n: f64) -> f64 {
    let a = y_curr - y_prev;
    let b = y_next - y_curr;
    let c = b - a;
    y_curr + n / 2.0 * (a + b + n * c)
}

/// Interpolation for angles (handles wrap-around at 2π)
fn interpolate_angle(y_prev: f64, y_curr: f64, y_next: f64, n: f64) -> f64 {
    // Adjust for discontinuity at 0/2π
    let mut a = y_curr - y_prev;
    let mut b = y_next - y_curr;

    if a.abs() > PI {
        a -= a.signum() * 2.0 * PI;
    }
    if b.abs() > PI {
        b -= b.signum() * 2.0 * PI;
    }

    let c = b - a;
    normalize_radians(y_curr + n / 2.0 * (a + b + n * c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bodies::Planet;
    use crate::coords::GeoLocation;
    use crate::time::jd_from_date;

    /// Test rise/set of the Sun from Rome on 2024 spring equinox.
    /// Sun should rise ~E and set ~W, roughly 12h apart.
    #[test]
    fn sun_rise_set_equinox() {
        let jd = jd_from_date(2024, 3, 20.0); // ~equinox
        let rome = GeoLocation::from_degrees(41.9, 12.5, 0.0);

        let events = rise_transit_set(jd, &rome, Body::Sun, |jd| {
            crate::sun::apparent_equatorial(jd)
        });

        let events = events.expect("Sun should rise and set at equinox in Rome");
        assert!(events.len() >= 3, "Expected rise, transit, set");

        let rise = events.iter().find(|e| e.event == EventType::Rise).unwrap();
        let set = events.iter().find(|e| e.event == EventType::Set).unwrap();
        let transit = events.iter().find(|e| e.event == EventType::Transit).unwrap();

        // Day length at equinox should be ~12 hours
        let day_length = (set.jd.0 - rise.jd.0) * 24.0;
        assert!(
            (day_length - 12.0).abs() < 0.5,
            "Day length at equinox: {:.2}h expected ~12h",
            day_length
        );

        // Rise azimuth should be near East (~90°)
        if let Some(az) = rise.azimuth_deg {
            assert!(
                (az - 90.0).abs() < 10.0,
                "Rise azimuth: {:.1}° expected ~90°",
                az
            );
        }

        // Transit altitude at Rome (~41.9°N) at equinox ≈ 90° - 41.9° ≈ 48.1°
        if let Some(alt) = transit.altitude_deg {
            assert!(
                (alt - 48.1).abs() < 2.0,
                "Transit altitude: {:.1}° expected ~48.1°",
                alt
            );
        }
    }

    #[test]
    fn mars_rise_set_basic() {
        let jd = jd_from_date(2024, 6, 15.0);
        let rome = GeoLocation::from_degrees(41.9, 12.5, 0.0);

        let result = rise_transit_set(jd, &rome, Body::Planet(Planet::Mars), |jd| {
            crate::planet::apparent_equatorial(Planet::Mars, jd)
        });

        // Mars should generally be visible from Rome
        assert!(result.is_ok(), "Mars should have rise/set events");
    }
}
