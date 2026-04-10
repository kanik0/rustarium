use rustarium_core::bodies::{Body, Planet};
use rustarium_core::coords::{format_dec, format_ra, GeoLocation};
use rustarium_core::eclipse::{lunar, solar};
use rustarium_core::moon;
use rustarium_core::planet;
use rustarium_core::rise_set::{self, EventType};
use rustarium_core::sun;
use rustarium_core::time::{date_from_jd, jd_from_date, JulianDay};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

fn parse_date(date_str: &str) -> JulianDay {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        let y = parts[0].parse::<i32>().unwrap_or(2026);
        let m = parts[1].parse::<u32>().unwrap_or(1);
        let d = parts[2].parse::<f64>().unwrap_or(1.0);
        return jd_from_date(y, m, d);
    }
    jd_from_date(2026, 1, 1.0)
}

fn jd_to_iso(jd: JulianDay) -> String {
    let (y, m, d) = date_from_jd(jd);
    let day = d as u32;
    let frac = d - d.floor();
    let h = (frac * 24.0).floor() as u32;
    let min = ((frac * 24.0 - h as f64) * 60.0).floor() as u32;
    let sec = ((frac * 24.0 - h as f64) * 60.0 - min as f64) * 60.0;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:04.1}Z",
        y, m, day, h, min, sec
    )
}

fn parse_body(name: &str) -> Option<Body> {
    Some(match name.to_lowercase().as_str() {
        "sun" => Body::Sun,
        "moon" => Body::Moon,
        "mercury" => Body::Planet(Planet::Mercury),
        "venus" => Body::Planet(Planet::Venus),
        "earth" => Body::Planet(Planet::Earth),
        "mars" => Body::Planet(Planet::Mars),
        "jupiter" => Body::Planet(Planet::Jupiter),
        "saturn" => Body::Planet(Planet::Saturn),
        "uranus" => Body::Planet(Planet::Uranus),
        "neptune" => Body::Planet(Planet::Neptune),
        _ => return None,
    })
}

fn body_position_json(body: Body, jd: JulianDay) -> Value {
    match body {
        Body::Sun => {
            let eq = sun::apparent_equatorial(jd);
            let ecl = sun::apparent_ecliptic(jd);
            json!({
                "body": "Sun",
                "ra_hms": format_ra(eq.ra),
                "dec_dms": format_dec(eq.dec),
                "ra_deg": eq.ra.to_degrees(),
                "dec_deg": eq.dec.to_degrees(),
                "distance_au": ecl.distance,
                "ecliptic_lon_deg": ecl.longitude.to_degrees(),
            })
        }
        Body::Moon => {
            let eq = moon::apparent_equatorial(jd);
            let ecl = moon::geocentric_ecliptic(jd);
            let illum = moon::illuminated_fraction(jd);
            json!({
                "body": "Moon",
                "ra_hms": format_ra(eq.ra),
                "dec_dms": format_dec(eq.dec),
                "ra_deg": eq.ra.to_degrees(),
                "dec_deg": eq.dec.to_degrees(),
                "distance_km": ecl.distance,
                "illumination": illum,
            })
        }
        Body::Planet(p) => {
            let eq = planet::apparent_equatorial(p, jd);
            let geo = planet::geocentric_position(p, jd);
            let helio = planet::heliocentric_position(p, jd);
            json!({
                "body": p.name(),
                "ra_hms": format_ra(eq.ra),
                "dec_dms": format_dec(eq.dec),
                "ra_deg": eq.ra.to_degrees(),
                "dec_deg": eq.dec.to_degrees(),
                "distance_au": geo.distance,
                "distance_from_sun_au": helio.distance,
            })
        }
    }
}

fn phase_name(illumination: f64) -> &'static str {
    match (illumination * 100.0) as u32 {
        0..=3 => "New Moon",
        4..=22 => "Waxing Crescent",
        23..=27 => "First Quarter",
        28..=47 => "Waxing Gibbous",
        48..=52 => "Full Moon",
        53..=72 => "Waning Gibbous",
        73..=77 => "Last Quarter",
        78..=96 => "Waning Crescent",
        _ => "New Moon",
    }
}

// ===== Public WASM API =====

#[wasm_bindgen]
pub fn sky(date_str: &str) -> String {
    let jd = parse_date(date_str);

    let mut planets = Vec::new();
    for p in Planet::ALL {
        let helio = planet::heliocentric_position(p, jd);
        let mut entry = json!({
            "body": p.name(),
            "helio_lon_deg": helio.longitude.to_degrees(),
            "helio_lat_deg": helio.latitude.to_degrees(),
            "helio_distance_au": helio.distance,
        });
        if p != Planet::Earth {
            let eq = planet::apparent_equatorial(p, jd);
            let geo = planet::geocentric_position(p, jd);
            let obj = entry.as_object_mut().unwrap();
            obj.insert("ra_deg".into(), json!(eq.ra.to_degrees()));
            obj.insert("dec_deg".into(), json!(eq.dec.to_degrees()));
            obj.insert("ra_hms".into(), json!(format_ra(eq.ra)));
            obj.insert("dec_dms".into(), json!(format_dec(eq.dec)));
            obj.insert("distance_au".into(), json!(geo.distance));
        }
        planets.push(entry);
    }

    let illum = moon::illuminated_fraction(jd);
    let moon_ecl = moon::geocentric_ecliptic(jd);
    let earth_helio = planet::heliocentric_position(Planet::Earth, jd);

    let result = json!({
        "date": jd_to_iso(jd),
        "julian_day": jd.0,
        "sun": body_position_json(Body::Sun, jd),
        "moon": {
            "position": body_position_json(Body::Moon, jd),
            "illumination": illum,
            "phase_name": phase_name(illum),
            "geocentric_lon_deg": moon_ecl.longitude.to_degrees(),
            "geocentric_lat_deg": moon_ecl.latitude.to_degrees(),
            "earth_helio_lon_deg": earth_helio.longitude.to_degrees(),
        },
        "planets": planets,
    });

    serde_json::to_string(&result).unwrap_or_default()
}

#[wasm_bindgen]
pub fn position(body_name: &str, date_str: &str) -> String {
    let jd = parse_date(date_str);
    let body = match parse_body(body_name) {
        Some(b) => b,
        None => return json!({"error": "unknown body"}).to_string(),
    };
    let result = json!({
        "date": jd_to_iso(jd),
        "julian_day": jd.0,
        "position": body_position_json(body, jd),
    });
    serde_json::to_string(&result).unwrap_or_default()
}

#[wasm_bindgen]
pub fn moon_info(date_str: &str) -> String {
    let jd = parse_date(date_str);
    let ecl = moon::geocentric_ecliptic(jd);
    let eq = moon::apparent_equatorial(jd);
    let illum = moon::illuminated_fraction(jd);
    let result = json!({
        "date": jd_to_iso(jd),
        "ra_hms": format_ra(eq.ra),
        "dec_dms": format_dec(eq.dec),
        "distance_km": ecl.distance,
        "illumination": illum,
        "phase": phase_name(illum),
    });
    serde_json::to_string(&result).unwrap_or_default()
}

#[wasm_bindgen]
pub fn riseset(body_name: &str, date_str: &str, lat: f64, lon: f64) -> String {
    let jd = parse_date(date_str);
    let body = match parse_body(body_name) {
        Some(b) => b,
        None => return json!({"error": "unknown body"}).to_string(),
    };
    let loc = GeoLocation::from_degrees(lat, lon, 0.0);
    let jd_0h = JulianDay((jd.0 - 0.5).floor() + 0.5);

    let eq_fn = |jd: JulianDay| match body {
        Body::Sun => sun::apparent_equatorial(jd),
        Body::Moon => moon::apparent_equatorial(jd),
        Body::Planet(p) => planet::apparent_equatorial(p, jd),
    };

    match rise_set::rise_transit_set(jd_0h, &loc, body, eq_fn) {
        Ok(events) => {
            let ev_json: Vec<Value> = events
                .iter()
                .map(|e| {
                    json!({
                        "event": match e.event {
                            EventType::Rise => "rise",
                            EventType::Transit => "transit",
                            EventType::Set => "set",
                        },
                        "time_ut": jd_to_iso(e.jd),
                        "julian_day": e.jd.0,
                        "azimuth_deg": e.azimuth_deg,
                        "altitude_deg": e.altitude_deg,
                    })
                })
                .collect();
            let result = json!({
                "date": jd_to_iso(jd_0h),
                "body": body.name(),
                "events": ev_json,
            });
            serde_json::to_string(&result).unwrap_or_default()
        }
        Err(_) => {
            let result = json!({
                "date": jd_to_iso(jd_0h),
                "body": body.name(),
                "events": [],
            });
            serde_json::to_string(&result).unwrap_or_default()
        }
    }
}

#[wasm_bindgen]
pub fn lunar_eclipses(year: i32, range: u32) -> String {
    let start = jd_from_date(year, 1, 1.0);
    let end = jd_from_date(year + range as i32, 1, 1.0);
    let eclipses = lunar::search(start, end);
    let results: Vec<Value> = eclipses
        .iter()
        .map(|e| {
            json!({
                "type": match e.eclipse_type {
                    lunar::LunarEclipseType::Total => "total",
                    lunar::LunarEclipseType::Partial => "partial",
                    lunar::LunarEclipseType::Penumbral => "penumbral",
                },
                "greatest_eclipse": jd_to_iso(e.greatest_eclipse),
                "umbral_magnitude": e.umbral_magnitude,
                "penumbral_magnitude": e.penumbral_magnitude,
                "p1": jd_to_iso(e.p1),
                "p4": jd_to_iso(e.p4),
            })
        })
        .collect();
    let result = json!({
        "year_start": year,
        "year_end": year + range as i32 - 1,
        "count": results.len(),
        "eclipses": results,
    });
    serde_json::to_string(&result).unwrap_or_default()
}

#[wasm_bindgen]
pub fn solar_eclipses(year: i32, range: u32) -> String {
    let start = jd_from_date(year, 1, 1.0);
    let end = jd_from_date(year + range as i32, 1, 1.0);
    let eclipses = solar::search(start, end);
    let results: Vec<Value> = eclipses
        .iter()
        .map(|e| {
            json!({
                "type": match e.eclipse_type {
                    solar::SolarEclipseType::Total => "total",
                    solar::SolarEclipseType::Annular => "annular",
                    solar::SolarEclipseType::Partial => "partial",
                    solar::SolarEclipseType::Hybrid => "hybrid",
                },
                "greatest_eclipse": jd_to_iso(e.greatest_eclipse),
                "gamma": e.gamma,
                "magnitude": e.magnitude,
            })
        })
        .collect();
    let result = json!({
        "year_start": year,
        "year_end": year + range as i32 - 1,
        "count": results.len(),
        "eclipses": results,
    });
    serde_json::to_string(&result).unwrap_or_default()
}
