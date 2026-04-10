use crate::format::*;
use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Attribute, Cell, Color, ContentArrangement, Table};
use owo_colors::OwoColorize;
use rustarium_core::bodies::{Body, Planet};
use rustarium_core::coords::{format_dec, format_ra};
use rustarium_core::eclipse::{lunar, solar};
use rustarium_core::moon;
use rustarium_core::planet;
use rustarium_core::rise_set::{self, EventType};
use rustarium_core::sun;
use rustarium_core::time::{date_from_jd, jd_from_date, JulianDay};

#[derive(Parser)]
#[command(
    name = "rustarium",
    about = "✦ Solar System Prediction Engine",
    version,
    after_help = "Examples:\n  rustarium sky\n  rustarium position mars\n  rustarium riseset --city roma\n  rustarium moon\n  rustarium eclipse lunar --year 2025\n  rustarium ephemeris jupiter --days 30"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show the sky right now — planet positions, Moon phase, Sun
    Sky {
        /// Date (YYYY-MM-DD). Default: today
        #[arg(short, long)]
        date: Option<String>,
        /// City name (roma, london, paris, ny, tokyo...)
        #[arg(short, long)]
        city: Option<String>,
        /// Latitude in degrees
        #[arg(long)]
        lat: Option<f64>,
        /// Longitude in degrees
        #[arg(long)]
        lon: Option<f64>,
    },
    /// Position of a celestial body
    Position {
        /// Body name (sun, moon, mercury, venus, mars, jupiter, saturn, uranus, neptune)
        body: String,
        /// Date (YYYY-MM-DD). Default: today
        #[arg(short, long)]
        date: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Rise, transit, set times
    Riseset {
        /// Body name. Default: sun
        body: Option<String>,
        /// Date (YYYY-MM-DD). Default: today
        #[arg(short, long)]
        date: Option<String>,
        /// Number of days to show
        #[arg(long, default_value = "7")]
        days: u32,
        /// City name
        #[arg(short, long)]
        city: Option<String>,
        #[arg(long)]
        lat: Option<f64>,
        #[arg(long)]
        lon: Option<f64>,
    },
    /// Moon phase and position
    Moon {
        /// Date (YYYY-MM-DD). Default: today
        #[arg(short, long)]
        date: Option<String>,
        /// Show phase calendar for the month
        #[arg(long)]
        calendar: bool,
    },
    /// Search for eclipses
    Eclipse {
        #[command(subcommand)]
        kind: EclipseKind,
    },
    /// Multi-day ephemeris table
    Ephemeris {
        /// Body name
        body: String,
        /// Start date (YYYY-MM-DD). Default: today
        #[arg(short = 's', long)]
        start: Option<String>,
        /// Number of days
        #[arg(long, default_value = "30")]
        days: u32,
        /// Step in days
        #[arg(long, default_value = "1")]
        step: u32,
    },
}

#[derive(Subcommand)]
pub enum EclipseKind {
    /// Search for lunar eclipses
    Lunar {
        /// Year to search. Default: current year
        #[arg(long)]
        year: Option<i32>,
        /// Search range in years
        #[arg(long, default_value = "1")]
        range: u32,
    },
    /// Search for solar eclipses
    Solar {
        /// Year to search. Default: current year
        #[arg(long)]
        year: Option<i32>,
        /// Search range in years
        #[arg(long, default_value = "1")]
        range: u32,
        /// City for local visibility
        #[arg(short, long)]
        city: Option<String>,
        #[arg(long)]
        lat: Option<f64>,
        #[arg(long)]
        lon: Option<f64>,
    },
}

pub fn run(cli: Cli) {
    match cli.command {
        None => cmd_sky(None, None, None, None),
        Some(Command::Sky { date, city, lat, lon }) => cmd_sky(date, city, lat, lon),
        Some(Command::Position { body, date, json }) => cmd_position(body, date, json),
        Some(Command::Riseset { body, date, days, city, lat, lon }) => {
            cmd_riseset(body, date, days, city, lat, lon)
        }
        Some(Command::Moon { date, calendar }) => cmd_moon(date, calendar),
        Some(Command::Eclipse { kind }) => cmd_eclipse(kind),
        Some(Command::Ephemeris { body, start, days, step }) => {
            cmd_ephemeris(body, start, days, step)
        }
    }
}

fn cmd_sky(date: Option<String>, city: Option<String>, lat: Option<f64>, lon: Option<f64>) {
    let jd = parse_date_or_today(&date);
    let (loc, tz, loc_label) = resolve_location(&lat, &lon, &city);
    let (y, m, d) = date_from_jd(jd);

    banner();

    let tz_name = tz.name_at(jd);
    let date_str = format!("{:4}-{:02}-{:02}", y, m, d as u32);
    section(&format!("Sky on {} — {} ({})", date_str, loc_label, tz_name));

    // Sun
    let sun_eq = sun::apparent_equatorial(jd);
    let sun_ecl = sun::apparent_ecliptic(jd);
    println!(
        "    {}  RA {}  Dec {}  {:.4} AU",
        body_colored("Sun"),
        format_ra(sun_eq.ra).bright_white(),
        format_dec(sun_eq.dec).bright_white(),
        sun_ecl.distance
    );

    // Moon
    let moon_eq = moon::apparent_equatorial(jd);
    let illum = moon::illuminated_fraction(jd);
    println!(
        "    {}  RA {}  Dec {}  {} {:.0}%",
        body_colored("Moon"),
        format_ra(moon_eq.ra).bright_white(),
        format_dec(moon_eq.dec).bright_white(),
        moon_phase_icon(illum),
        illum * 100.0,
    );

    // Planets
    section("Planets");
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Planet").add_attribute(Attribute::Bold),
            Cell::new("RA").add_attribute(Attribute::Bold),
            Cell::new("Dec").add_attribute(Attribute::Bold),
            Cell::new("Dist (AU)").add_attribute(Attribute::Bold),
            Cell::new("Elong").add_attribute(Attribute::Bold),
        ]);

    for p in Planet::ALL {
        if p == Planet::Earth {
            continue;
        }
        let eq = planet::apparent_equatorial(p, jd);
        let geo = planet::geocentric_position(p, jd);

        // Elongation from Sun
        let elong = angular_elongation(jd, Body::Planet(p));

        let color = planet_color(p);
        table.add_row(vec![
            Cell::new(p.name()).fg(color),
            Cell::new(format_ra(eq.ra)),
            Cell::new(format_dec(eq.dec)),
            Cell::new(format!("{:.4}", geo.distance)),
            Cell::new(format!("{:.1}°", elong.to_degrees())),
        ]);
    }
    println!("    {}", table.to_string().replace('\n', "\n    "));

    // Rise/set for Sun
    section("Sun today");
    if let Ok(events) = rise_set::rise_transit_set(
        JulianDay((jd.0 - 0.5).floor() + 0.5),
        &loc,
        Body::Sun,
        |jd| sun::apparent_equatorial(jd),
    ) {
        for ev in &events {
            let icon = match ev.event {
                EventType::Rise => "↑",
                EventType::Transit => "◉",
                EventType::Set => "↓",
            };
            let label = match ev.event {
                EventType::Rise => "Rise   ",
                EventType::Transit => "Transit",
                EventType::Set => "Set    ",
            };
            let extra = match ev.event {
                EventType::Rise | EventType::Set => {
                    format!("  az {:.1}°", ev.azimuth_deg.unwrap_or(0.0))
                }
                EventType::Transit => {
                    format!("  alt {:.1}°", ev.altitude_deg.unwrap_or(0.0))
                }
            };
            println!(
                "    {} {} {}{}",
                icon.yellow(),
                label,
                jd_to_local_time_str(ev.jd, &tz).bright_white(),
                extra.dimmed()
            );
        }
    }

    println!();
}

fn cmd_position(body_name: String, date: Option<String>, json: bool) {
    let body = parse_body(&body_name);
    let jd = parse_date_or_today(&date);
    let (y, m, d) = date_from_jd(jd);

    if json {
        print_position_json(body, jd);
        return;
    }

    header(&format!("{} — {:4}-{:02}-{:02}", body.name(), y, m, d as u32));

    match body {
        Body::Sun => {
            let eq = sun::apparent_equatorial(jd);
            let ecl = sun::apparent_ecliptic(jd);
            section("Equatorial (apparent)");
            kv_colored("Right Ascension", &format_ra(eq.ra));
            kv_colored("Declination", &format_dec(eq.dec));
            section("Ecliptic (apparent)");
            kv("Longitude", &format!("{:.4}°", ecl.longitude.to_degrees()));
            kv("Latitude", &format!("{:.4}°", ecl.latitude.to_degrees()));
            section("Distance");
            kv_colored("Distance", &format!("{:.6} AU", ecl.distance));
            kv(
                "Angular diameter",
                &format!(
                    "{:.2}'",
                    sun::angular_semidiameter(ecl.distance).to_degrees() * 120.0
                ),
            );
        }
        Body::Moon => {
            let eq = moon::apparent_equatorial(jd);
            let ecl = moon::geocentric_ecliptic(jd);
            let illum = moon::illuminated_fraction(jd);
            section("Equatorial (apparent)");
            kv_colored("Right Ascension", &format_ra(eq.ra));
            kv_colored("Declination", &format_dec(eq.dec));
            section("Ecliptic");
            kv("Longitude", &format!("{:.4}°", ecl.longitude.to_degrees()));
            kv("Latitude", &format!("{:.4}°", ecl.latitude.to_degrees()));
            section("Physical");
            kv_colored("Distance", &format!("{:.0} km", ecl.distance));
            kv(
                "Phase",
                &format!(
                    "{} {:.1}%",
                    moon_phase_icon(illum),
                    illum * 100.0
                ),
            );
            kv(
                "Angular diameter",
                &format!(
                    "{:.2}'",
                    moon::angular_semidiameter(jd).to_degrees() * 120.0
                ),
            );
            kv(
                "Horizontal parallax",
                &format!(
                    "{:.2}'",
                    moon::horizontal_parallax(jd).to_degrees() * 60.0
                ),
            );
        }
        Body::Planet(p) => {
            let eq = planet::apparent_equatorial(p, jd);
            let geo = planet::geocentric_position(p, jd);
            let helio = planet::heliocentric_position(p, jd);
            section("Equatorial (apparent, geocentric)");
            kv_colored("Right Ascension", &format_ra(eq.ra));
            kv_colored("Declination", &format_dec(eq.dec));
            section("Distance");
            kv_colored("From Earth", &format!("{:.6} AU", geo.distance));
            kv("From Sun", &format!("{:.6} AU", helio.distance));
            section("Geometry");
            let elong = angular_elongation(jd, body);
            kv("Elongation", &format!("{:.1}°", elong.to_degrees()));
        }
    }

    println!();
}

fn cmd_moon(date: Option<String>, calendar: bool) {
    let jd = parse_date_or_today(&date);
    let (y, m, _) = date_from_jd(jd);

    if calendar {
        header(&format!("Moon Phases — {:4}-{:02}", y, m));
        println!();

        let days_in_month = match m {
            2 => {
                if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };

        for day in 1..=days_in_month {
            let day_jd = jd_from_date(y, m, day as f64);
            let frac = moon::illuminated_fraction(day_jd);
            let bar_width = 20;
            let filled = (frac * bar_width as f64).round() as usize;
            let bar_filled = "█".repeat(filled);
            let bar_empty = "░".repeat(bar_width - filled);

            let phase_str = format!(
                "    {:02}  {}  {}{} {:>5.1}%",
                day,
                moon_phase_icon(frac),
                bar_filled.bright_white(),
                bar_empty.dimmed(),
                frac * 100.0
            );
            println!("{}", phase_str);
        }
        println!();
    } else {
        cmd_position("moon".into(), date, false);
    }
}

fn cmd_riseset(
    body_name: Option<String>,
    date: Option<String>,
    days: u32,
    city: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
) {
    let body = parse_body(&body_name.unwrap_or("sun".into()));
    let start_jd = parse_date_or_today(&date);
    let (loc, tz, loc_label) = resolve_location(&lat, &lon, &city);

    let tz_name = tz.name_at(start_jd);
    header(&format!(
        "{} Rise/Set — {} ({} days, {})",
        body.name(),
        loc_label,
        days,
        tz_name,
    ));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Date").add_attribute(Attribute::Bold),
            Cell::new(format!("↑ Rise {}", tz_name)).add_attribute(Attribute::Bold),
            Cell::new(format!("◉ Transit")).add_attribute(Attribute::Bold),
            Cell::new(format!("↓ Set")).add_attribute(Attribute::Bold),
            Cell::new("UT Rise").add_attribute(Attribute::Bold),
            Cell::new("UT Set").add_attribute(Attribute::Bold),
            Cell::new("Day length").add_attribute(Attribute::Bold),
        ]);

    for i in 0..days {
        let day_jd = JulianDay(((start_jd.0 - 0.5).floor() + 0.5) + i as f64);
        let (dy, dm, dd) = date_from_jd(day_jd);

        let eq_fn = make_eq_fn(body);

        match rise_set::rise_transit_set(day_jd, &loc, body, eq_fn) {
            Ok(events) => {
                let rise_ev = events.iter().find(|e| e.event == EventType::Rise);
                let transit_ev = events.iter().find(|e| e.event == EventType::Transit);
                let set_ev = events.iter().find(|e| e.event == EventType::Set);

                let rise_local = rise_ev.map(|e| jd_to_local_time_short(e.jd, &tz)).unwrap_or("--:--".into());
                let transit_local = transit_ev.map(|e| jd_to_local_time_short(e.jd, &tz)).unwrap_or("--:--".into());
                let set_local = set_ev.map(|e| jd_to_local_time_short(e.jd, &tz)).unwrap_or("--:--".into());
                let rise_ut = rise_ev.map(|e| jd_to_time_str(e.jd)).unwrap_or("--:--".into());
                let set_ut = set_ev.map(|e| jd_to_time_str(e.jd)).unwrap_or("--:--".into());

                let day_len = match (rise_ev, set_ev) {
                    (Some(r), Some(s)) => {
                        let hours = (s.jd.0 - r.jd.0) * 24.0;
                        let h = hours.floor() as u32;
                        let m = ((hours - h as f64) * 60.0).round() as u32;
                        format!("{}h {:02}m", h, m)
                    }
                    _ => "—".into(),
                };

                table.add_row(vec![
                    Cell::new(format!("{:4}-{:02}-{:02}", dy, dm, dd as u32)),
                    Cell::new(rise_local),
                    Cell::new(transit_local),
                    Cell::new(set_local),
                    Cell::new(rise_ut).fg(Color::DarkGrey),
                    Cell::new(set_ut).fg(Color::DarkGrey),
                    Cell::new(day_len),
                ]);
            }
            Err(_) => {
                table.add_row(vec![
                    Cell::new(format!("{:4}-{:02}-{:02}", dy, dm, dd as u32)),
                    Cell::new("—"), Cell::new("—"), Cell::new("—"),
                    Cell::new("—"), Cell::new("—"), Cell::new("—"),
                ]);
            }
        }
    }

    println!();
    println!("    {}", table.to_string().replace('\n', "\n    "));
    println!();
}

fn cmd_eclipse(kind: EclipseKind) {
    match kind {
        EclipseKind::Lunar { year, range } => {
            let y = year.unwrap_or_else(|| {
                let (y, _, _) = date_from_jd(today_jd());
                y
            });
            let start = jd_from_date(y, 1, 1.0);
            let end = jd_from_date(y + range as i32, 1, 1.0);

            header(&format!("Lunar Eclipses {}-{}", y, y + range as i32 - 1));

            let eclipses = lunar::search(start, end);
            if eclipses.is_empty() {
                println!("\n    {}", "No lunar eclipses found in this period.".dimmed());
            } else {
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec![
                        Cell::new("Date").add_attribute(Attribute::Bold),
                        Cell::new("Type").add_attribute(Attribute::Bold),
                        Cell::new("Magnitude").add_attribute(Attribute::Bold),
                        Cell::new("Penumbral start").add_attribute(Attribute::Bold),
                        Cell::new("Greatest").add_attribute(Attribute::Bold),
                        Cell::new("Penumbral end").add_attribute(Attribute::Bold),
                    ]);

                for e in &eclipses {
                    let (ey, em, ed) = date_from_jd(e.greatest_eclipse);
                    let type_color = match e.eclipse_type {
                        lunar::LunarEclipseType::Total => Color::Red,
                        lunar::LunarEclipseType::Partial => Color::Yellow,
                        lunar::LunarEclipseType::Penumbral => Color::DarkGrey,
                    };
                    let type_str = match e.eclipse_type {
                        lunar::LunarEclipseType::Total => "Total",
                        lunar::LunarEclipseType::Partial => "Partial",
                        lunar::LunarEclipseType::Penumbral => "Penumbral",
                    };

                    table.add_row(vec![
                        Cell::new(format!("{:4}-{:02}-{:02}", ey, em, ed as u32)),
                        Cell::new(type_str).fg(type_color),
                        Cell::new(format!("{:.3}", e.umbral_magnitude)),
                        Cell::new(jd_to_time_str(e.p1)),
                        Cell::new(jd_to_time_str(e.greatest_eclipse))
                            .add_attribute(Attribute::Bold),
                        Cell::new(jd_to_time_str(e.p4)),
                    ]);
                }

                println!();
                println!("    {}", table.to_string().replace('\n', "\n    "));
                println!(
                    "    {} lunar eclipse(s) found. Times in UT.",
                    eclipses.len()
                );
            }
            println!();
        }
        EclipseKind::Solar {
            year,
            range,
            city: _,
            lat: _,
            lon: _,
        } => {
            let y = year.unwrap_or_else(|| {
                let (y, _, _) = date_from_jd(today_jd());
                y
            });
            let start = jd_from_date(y, 1, 1.0);
            let end = jd_from_date(y + range as i32, 1, 1.0);

            header(&format!("Solar Eclipses {}-{}", y, y + range as i32 - 1));

            let eclipses = solar::search(start, end);
            if eclipses.is_empty() {
                println!("\n    {}", "No solar eclipses found in this period.".dimmed());
            } else {
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec![
                        Cell::new("Date").add_attribute(Attribute::Bold),
                        Cell::new("Type").add_attribute(Attribute::Bold),
                        Cell::new("Gamma").add_attribute(Attribute::Bold),
                        Cell::new("Magnitude").add_attribute(Attribute::Bold),
                        Cell::new("Greatest").add_attribute(Attribute::Bold),
                    ]);

                for e in &eclipses {
                    let (ey, em, ed) = date_from_jd(e.greatest_eclipse);
                    let type_color = match e.eclipse_type {
                        solar::SolarEclipseType::Total => Color::Red,
                        solar::SolarEclipseType::Annular => Color::Yellow,
                        solar::SolarEclipseType::Partial => Color::DarkGrey,
                        solar::SolarEclipseType::Hybrid => Color::Magenta,
                    };
                    let type_str = match e.eclipse_type {
                        solar::SolarEclipseType::Total => "Total",
                        solar::SolarEclipseType::Annular => "Annular",
                        solar::SolarEclipseType::Partial => "Partial",
                        solar::SolarEclipseType::Hybrid => "Hybrid",
                    };

                    table.add_row(vec![
                        Cell::new(format!("{:4}-{:02}-{:02}", ey, em, ed as u32)),
                        Cell::new(type_str).fg(type_color),
                        Cell::new(format!("{:+.4}", e.gamma)),
                        Cell::new(format!("{:.3}", e.magnitude)),
                        Cell::new(jd_to_time_str(e.greatest_eclipse))
                            .add_attribute(Attribute::Bold),
                    ]);
                }

                println!();
                println!("    {}", table.to_string().replace('\n', "\n    "));
                println!(
                    "    {} solar eclipse(s) found. Times in UT.",
                    eclipses.len()
                );
            }
            println!();
        }
    }
}

fn cmd_ephemeris(body_name: String, start: Option<String>, days: u32, step: u32) {
    let body = parse_body(&body_name);
    let start_jd = parse_date_or_today(&start);

    header(&format!("{} Ephemeris — {} days", body.name(), days));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Date").add_attribute(Attribute::Bold),
            Cell::new("RA").add_attribute(Attribute::Bold),
            Cell::new("Dec").add_attribute(Attribute::Bold),
            Cell::new("Dist (AU)").add_attribute(Attribute::Bold),
            Cell::new("Elong").add_attribute(Attribute::Bold),
        ]);

    let mut i = 0u32;
    while i < days {
        let jd = start_jd + i as f64;
        let (dy, dm, dd) = date_from_jd(jd);

        let (ra, dec, dist) = match body {
            Body::Sun => {
                let eq = sun::apparent_equatorial(jd);
                let ecl = sun::apparent_ecliptic(jd);
                (eq.ra, eq.dec, ecl.distance)
            }
            Body::Moon => {
                let eq = moon::apparent_equatorial(jd);
                let ecl = moon::geocentric_ecliptic(jd);
                (eq.ra, eq.dec, ecl.distance / rustarium_core::bodies::AU_KM)
            }
            Body::Planet(p) => {
                let eq = planet::apparent_equatorial(p, jd);
                let geo = planet::geocentric_position(p, jd);
                (eq.ra, eq.dec, geo.distance)
            }
        };

        let elong = angular_elongation(jd, body);

        table.add_row(vec![
            Cell::new(format!("{:4}-{:02}-{:02}", dy, dm, dd as u32)),
            Cell::new(format_ra(ra)),
            Cell::new(format_dec(dec)),
            Cell::new(format!("{:.4}", dist)),
            Cell::new(format!("{:.1}°", elong.to_degrees())),
        ]);

        i += step;
    }

    println!();
    println!("    {}", table.to_string().replace('\n', "\n    "));
    println!();
}

// --- Helpers ---

fn angular_elongation(jd: JulianDay, body: Body) -> f64 {
    let sun_ecl = sun::geocentric_ecliptic(jd);
    let (body_lon, body_lat) = match body {
        Body::Sun => return 0.0,
        Body::Moon => {
            let m = moon::geocentric_ecliptic(jd);
            (m.longitude, m.latitude)
        }
        Body::Planet(p) => {
            let g = planet::geocentric_position(p, jd);
            (g.longitude, g.latitude)
        }
    };

    let cos_elong =
        body_lat.cos() * (body_lon - sun_ecl.longitude).cos();
    cos_elong.clamp(-1.0, 1.0).acos()
}

fn make_eq_fn(
    body: Body,
) -> impl Fn(JulianDay) -> rustarium_core::coords::EquatorialCoords {
    move |jd| match body {
        Body::Sun => sun::apparent_equatorial(jd),
        Body::Moon => moon::apparent_equatorial(jd),
        Body::Planet(p) => planet::apparent_equatorial(p, jd),
    }
}

fn planet_color(p: Planet) -> Color {
    match p {
        Planet::Mercury => Color::DarkGrey,
        Planet::Venus => Color::Yellow,
        Planet::Earth => Color::Cyan,
        Planet::Mars => Color::Red,
        Planet::Jupiter => Color::DarkRed,
        Planet::Saturn => Color::DarkYellow,
        Planet::Uranus => Color::Cyan,
        Planet::Neptune => Color::Blue,
    }
}

fn print_position_json(body: Body, jd: JulianDay) {
    let (ra, dec, dist, dist_unit) = match body {
        Body::Sun => {
            let eq = sun::apparent_equatorial(jd);
            let ecl = sun::apparent_ecliptic(jd);
            (eq.ra, eq.dec, ecl.distance, "AU")
        }
        Body::Moon => {
            let eq = moon::apparent_equatorial(jd);
            let ecl = moon::geocentric_ecliptic(jd);
            (eq.ra, eq.dec, ecl.distance, "km")
        }
        Body::Planet(p) => {
            let eq = planet::apparent_equatorial(p, jd);
            let geo = planet::geocentric_position(p, jd);
            (eq.ra, eq.dec, geo.distance, "AU")
        }
    };

    let json = serde_json::json!({
        "body": body.name(),
        "julian_day": jd.0,
        "equatorial": {
            "ra_deg": ra.to_degrees(),
            "dec_deg": dec.to_degrees(),
            "ra_hms": format_ra(ra),
            "dec_dms": format_dec(dec),
        },
        "distance": {
            "value": dist,
            "unit": dist_unit,
        }
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
