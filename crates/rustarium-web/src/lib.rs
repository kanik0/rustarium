use rustarium_core::bodies::{Body, Planet};
use rustarium_core::coords::{format_dec, format_ra, GeoLocation};
use rustarium_core::eclipse::{lunar, solar};
use rustarium_core::moon;
use rustarium_core::planet;
use rustarium_core::rise_set::{self, EventType};
use rustarium_core::sbdb::SbdbResponse;
use rustarium_core::sun;
use rustarium_core::time::{date_from_jd, jd_from_date, JulianDay};
use serde_json::{json, Value};
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .get("/", |_, _| {
            Response::ok(
                r#"{"name":"rustarium","version":"0.1.0","description":"Solar System Prediction Engine","endpoints":["/api/sky","/api/position/:body","/api/moon","/api/riseset","/api/eclipse/lunar","/api/eclipse/solar","/api/ephemeris/:body"]}"#,
            )
        })
        .get_async("/api/sky", handle_sky)
        .get_async("/api/position/:body", handle_position)
        .get_async("/api/moon", handle_moon)
        .get_async("/api/riseset", handle_riseset)
        .get_async("/api/eclipse/lunar", handle_lunar_eclipses)
        .get_async("/api/eclipse/solar", handle_solar_eclipses)
        .get_async("/api/ephemeris/:body", handle_ephemeris)
        .get_async("/api/sbdb/search", handle_sbdb_search)
        .get_async("/api/horizons/vectors", handle_horizons_vectors)
        .post_async("/api/custom/position", handle_custom_position)
        .run(req, _env)
        .await
}

fn parse_date_param(url: &Url) -> JulianDay {
    url.query_pairs()
        .find(|(k, _)| k == "date")
        .and_then(|(_, v)| {
            let parts: Vec<&str> = v.split('-').collect();
            if parts.len() == 3 {
                let y = parts[0].parse::<i32>().ok()?;
                let m = parts[1].parse::<u32>().ok()?;
                let d = parts[2].parse::<f64>().ok()?;
                Some(jd_from_date(y, m, d))
            } else {
                None
            }
        })
        .unwrap_or(jd_from_date(2026, 1, 1.0)) // fallback
}

fn parse_location_params(url: &Url) -> GeoLocation {
    let lat = url
        .query_pairs()
        .find(|(k, _)| k == "lat")
        .and_then(|(_, v)| v.parse::<f64>().ok())
        .unwrap_or(41.9028);
    let lon = url
        .query_pairs()
        .find(|(k, _)| k == "lon")
        .and_then(|(_, v)| v.parse::<f64>().ok())
        .unwrap_or(12.4964);
    GeoLocation::from_degrees(lat, lon, 0.0)
}

fn parse_body_param(name: &str) -> Option<Body> {
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

fn json_response(value: Value) -> Result<Response> {
    Response::from_json(&value)
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

// --- Handlers ---

async fn handle_sky(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let jd = parse_date_param(&url);

    // Heliocentric positions for the 3D orrery view
    let mut planets = Vec::new();
    for p in Planet::ALL {
        let helio = planet::heliocentric_position(p, jd);
        let mut entry = json!({
            "body": p.name(),
            "helio_lon_deg": helio.longitude.to_degrees(),
            "helio_lat_deg": helio.latitude.to_degrees(),
            "helio_distance_au": helio.distance,
        });
        // Add geocentric data for non-Earth planets
        if p != Planet::Earth {
            let eq = planet::apparent_equatorial(p, jd);
            let geo = planet::geocentric_position(p, jd);
            entry.as_object_mut().unwrap().insert("ra_deg".into(), json!(eq.ra.to_degrees()));
            entry.as_object_mut().unwrap().insert("dec_deg".into(), json!(eq.dec.to_degrees()));
            entry.as_object_mut().unwrap().insert("ra_hms".into(), json!(format_ra(eq.ra)));
            entry.as_object_mut().unwrap().insert("dec_dms".into(), json!(format_dec(eq.dec)));
            entry.as_object_mut().unwrap().insert("distance_au".into(), json!(geo.distance));
        }
        planets.push(entry);
    }

    let illum = moon::illuminated_fraction(jd);
    let moon_ecl = moon::geocentric_ecliptic(jd);
    let earth_helio = planet::heliocentric_position(Planet::Earth, jd);

    json_response(json!({
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
    }))
}

async fn handle_position(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let jd = parse_date_param(&url);

    let body_name = ctx.param("body").unwrap();
    let body = match parse_body_param(body_name) {
        Some(b) => b,
        None => return Response::error(format!("Unknown body: {}", body_name), 400),
    };

    json_response(json!({
        "date": jd_to_iso(jd),
        "julian_day": jd.0,
        "position": body_position_json(body, jd),
    }))
}

async fn handle_moon(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let jd = parse_date_param(&url);

    let ecl = moon::geocentric_ecliptic(jd);
    let eq = moon::apparent_equatorial(jd);
    let illum = moon::illuminated_fraction(jd);

    json_response(json!({
        "date": jd_to_iso(jd),
        "ra_hms": format_ra(eq.ra),
        "dec_dms": format_dec(eq.dec),
        "distance_km": ecl.distance,
        "illumination": illum,
        "phase": phase_name(illum),
        "angular_diameter_arcmin": moon::angular_semidiameter(jd).to_degrees() * 120.0,
        "horizontal_parallax_arcmin": moon::horizontal_parallax(jd).to_degrees() * 60.0,
    }))
}

async fn handle_riseset(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let jd = parse_date_param(&url);
    let loc = parse_location_params(&url);

    let body_name = url
        .query_pairs()
        .find(|(k, _)| k == "body")
        .map(|(_, v)| v.to_string())
        .unwrap_or("sun".into());

    let body = match parse_body_param(&body_name) {
        Some(b) => b,
        None => return Response::error(format!("Unknown body: {}", body_name), 400),
    };

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
                    let event_type = match e.event {
                        EventType::Rise => "rise",
                        EventType::Transit => "transit",
                        EventType::Set => "set",
                    };
                    json!({
                        "event": event_type,
                        "time_ut": jd_to_iso(e.jd),
                        "julian_day": e.jd.0,
                        "azimuth_deg": e.azimuth_deg,
                        "altitude_deg": e.altitude_deg,
                    })
                })
                .collect();

            json_response(json!({
                "date": jd_to_iso(jd_0h),
                "body": body.name(),
                "observer": { "lat": loc.lat.to_degrees(), "lon": loc.lon.to_degrees() },
                "events": ev_json,
            }))
        }
        Err(_) => json_response(json!({
            "date": jd_to_iso(jd_0h),
            "body": body.name(),
            "events": [],
            "note": "Body does not rise/set at this location on this date",
        })),
    }
}

async fn handle_lunar_eclipses(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let year = url
        .query_pairs()
        .find(|(k, _)| k == "year")
        .and_then(|(_, v)| v.parse::<i32>().ok())
        .unwrap_or(2025);
    let range = url
        .query_pairs()
        .find(|(k, _)| k == "range")
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .unwrap_or(1);

    let start = jd_from_date(year, 1, 1.0);
    let end = jd_from_date(year + range as i32, 1, 1.0);

    let eclipses = lunar::search(start, end);
    let results: Vec<Value> = eclipses
        .iter()
        .map(|e| {
            let type_str = match e.eclipse_type {
                lunar::LunarEclipseType::Total => "total",
                lunar::LunarEclipseType::Partial => "partial",
                lunar::LunarEclipseType::Penumbral => "penumbral",
            };
            json!({
                "type": type_str,
                "greatest_eclipse": jd_to_iso(e.greatest_eclipse),
                "umbral_magnitude": e.umbral_magnitude,
                "penumbral_magnitude": e.penumbral_magnitude,
                "p1": jd_to_iso(e.p1),
                "p4": jd_to_iso(e.p4),
                "u1": e.u1.map(|j| jd_to_iso(j)),
                "u4": e.u4.map(|j| jd_to_iso(j)),
            })
        })
        .collect();

    json_response(json!({
        "year_start": year,
        "year_end": year + range as i32 - 1,
        "count": results.len(),
        "eclipses": results,
    }))
}

async fn handle_solar_eclipses(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let year = url
        .query_pairs()
        .find(|(k, _)| k == "year")
        .and_then(|(_, v)| v.parse::<i32>().ok())
        .unwrap_or(2025);
    let range = url
        .query_pairs()
        .find(|(k, _)| k == "range")
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .unwrap_or(1);

    let start = jd_from_date(year, 1, 1.0);
    let end = jd_from_date(year + range as i32, 1, 1.0);

    let eclipses = solar::search(start, end);
    let results: Vec<Value> = eclipses
        .iter()
        .map(|e| {
            let type_str = match e.eclipse_type {
                solar::SolarEclipseType::Total => "total",
                solar::SolarEclipseType::Annular => "annular",
                solar::SolarEclipseType::Partial => "partial",
                solar::SolarEclipseType::Hybrid => "hybrid",
            };
            json!({
                "type": type_str,
                "greatest_eclipse": jd_to_iso(e.greatest_eclipse),
                "gamma": e.gamma,
                "magnitude": e.magnitude,
            })
        })
        .collect();

    json_response(json!({
        "year_start": year,
        "year_end": year + range as i32 - 1,
        "count": results.len(),
        "eclipses": results,
    }))
}

async fn handle_ephemeris(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let jd = parse_date_param(&url);

    let body_name = ctx.param("body").unwrap();
    let body = match parse_body_param(body_name) {
        Some(b) => b,
        None => return Response::error(format!("Unknown body: {}", body_name), 400),
    };

    let days = url
        .query_pairs()
        .find(|(k, _)| k == "days")
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .unwrap_or(30)
        .min(365);
    let step = url
        .query_pairs()
        .find(|(k, _)| k == "step")
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);

    let mut entries = Vec::new();
    let mut i = 0u32;
    while i < days {
        let day_jd = jd + i as f64;
        entries.push(json!({
            "date": jd_to_iso(day_jd),
            "position": body_position_json(body, day_jd),
        }));
        i += step;
    }

    json_response(json!({
        "body": body.name(),
        "start": jd_to_iso(jd),
        "days": days,
        "step": step,
        "entries": entries,
    }))
}

async fn handle_sbdb_search(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let query = url
        .query_pairs()
        .find(|(k, _)| k == "sstr")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    if query.is_empty() {
        return Response::error("Missing 'sstr' parameter", 400);
    }

    let sbdb_url = format!(
        "https://ssd-api.jpl.nasa.gov/sbdb.api?sstr={}&phys-par=true",
        urlencoding::encode(&query)
    );

    let sbdb_req = Request::new(&sbdb_url, Method::Get)?;
    let mut resp = Fetch::Request(sbdb_req).send().await?;
    let body = resp.text().await?;

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    Ok(Response::ok(body)?.with_headers(headers))
}

async fn handle_horizons_vectors(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let id = url
        .query_pairs()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    if id.is_empty() {
        return Response::error("Missing 'id' parameter", 400);
    }

    let start = url
        .query_pairs()
        .find(|(k, _)| k == "start")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "2025-01-01".into());
    let stop = url
        .query_pairs()
        .find(|(k, _)| k == "stop")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "2027-01-01".into());
    let step = url
        .query_pairs()
        .find(|(k, _)| k == "step")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "5d".into());

    let horizons_url = format!(
        "https://ssd.jpl.nasa.gov/api/horizons.api?format=json\
        &COMMAND='{}'\
        &OBJ_DATA='YES'&MAKE_EPHEM='YES'&EPHEM_TYPE='VECTORS'\
        &CENTER='500@10'&START_TIME='{}'\
        &STOP_TIME='{}'&STEP_SIZE='{}'\
        &REF_PLANE='ECLIPTIC'&REF_SYSTEM='ICRF'&OUT_UNITS='AU-D'",
        urlencoding::encode(&id),
        urlencoding::encode(&start),
        urlencoding::encode(&stop),
        urlencoding::encode(&step),
    );

    let h_req = Request::new(&horizons_url, Method::Get)?;
    let mut resp = Fetch::Request(h_req).send().await?;
    let body = resp.text().await?;

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    Ok(Response::ok(body)?.with_headers(headers))
}

async fn handle_custom_position(mut req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let body_text = req.text().await?;
    let input: Value = serde_json::from_str(&body_text)
        .map_err(|e| Error::RustError(format!("Invalid JSON: {}", e)))?;

    // Parse SBDB data to create a CustomBody
    let sbdb_json = input
        .get("sbdb_data")
        .ok_or_else(|| Error::RustError("missing sbdb_data".into()))?;
    let resp: SbdbResponse = serde_json::from_value(sbdb_json.clone())
        .map_err(|e| Error::RustError(format!("Invalid SBDB data: {}", e)))?;
    let custom_body = resp
        .to_custom_body()
        .map_err(|e| Error::RustError(e))?;

    // Parse date
    let date_str = input
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap_or("2026-01-01");
    let jd = {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() == 3 {
            let y = parts[0].parse::<i32>().unwrap_or(2026);
            let m = parts[1].parse::<u32>().unwrap_or(1);
            let d = parts[2].parse::<f64>().unwrap_or(1.0);
            jd_from_date(y, m, d)
        } else {
            jd_from_date(2026, 1, 1.0)
        }
    };

    let eq = custom_body.apparent_equatorial(jd);
    let geo = custom_body.geocentric_position(jd);
    let helio = custom_body.heliocentric_position(jd);

    json_response(json!({
        "date": jd_to_iso(jd),
        "julian_day": jd.0,
        "position": {
            "body": custom_body.name,
            "body_type": custom_body.body_type.name(),
            "ra_hms": format_ra(eq.ra),
            "dec_dms": format_dec(eq.dec),
            "ra_deg": eq.ra.to_degrees(),
            "dec_deg": eq.dec.to_degrees(),
            "distance_au": geo.distance,
            "distance_from_sun_au": helio.distance,
            "diameter_km": custom_body.diameter_km,
        },
    }))
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
