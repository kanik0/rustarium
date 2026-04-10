use owo_colors::OwoColorize;
use rustarium_core::time::{date_from_jd, JulianDay};

// --- Colors ---

pub fn body_colored(name: &str) -> String {
    match name {
        "Sun" => name.yellow().bold().to_string(),
        "Moon" => name.bright_white().bold().to_string(),
        "Mercury" => name.bright_black().to_string(),
        "Venus" => name.bright_yellow().to_string(),
        "Earth" => name.bright_cyan().to_string(),
        "Mars" => name.red().to_string(),
        "Jupiter" => name.bright_red().to_string(),
        "Saturn" => name.yellow().to_string(),
        "Uranus" => name.cyan().to_string(),
        "Neptune" => name.blue().to_string(),
        _ => name.bright_magenta().to_string(),
    }
}

// --- Layout ---

pub fn header(text: &str) {
    let bar = "─".repeat(text.len() + 4);
    println!();
    println!("  {}", bar.dimmed());
    println!("  {} {} {}", "│".dimmed(), text.bold(), "│".dimmed());
    println!("  {}", bar.dimmed());
}

pub fn section(text: &str) {
    println!();
    println!("  {} {}", "▸".bright_blue(), text.bold());
}

pub fn kv(key: &str, value: &str) {
    println!("    {:<22} {}", key.dimmed(), value);
}

pub fn kv_colored(key: &str, value: &str) {
    println!("    {:<22} {}", key.dimmed(), value.bright_white());
}

pub fn banner() {
    println!(
        "{}",
        r#"
    ╭─────────────────────────────────────╮
    │  ✦  RUSTARIUM                       │
    │     Solar System Prediction Engine   │
    ╰─────────────────────────────────────╯"#
            .bright_blue()
    );
}

// --- Time formatting ---

/// Format a JD as "HH:MM" in UT.
pub fn jd_to_time_str(jd: JulianDay) -> String {
    let (_, _, d) = date_from_jd(jd);
    let frac = d - d.floor();
    let total_seconds = frac * 86400.0;
    let h = (total_seconds / 3600.0).floor() as u32;
    let min = ((total_seconds - h as f64 * 3600.0) / 60.0).floor() as u32;
    format!("{:02}:{:02}", h, min)
}

/// Format a JD as "HH:MM TZ (HH:MM UT)" using timezone info.
pub fn jd_to_local_time_str(jd: JulianDay, tz: &TzInfo) -> String {
    let ut_str = jd_to_time_str(jd);
    let offset = tz.offset_at(jd);
    let local_jd = JulianDay(jd.0 + offset / 24.0);
    let local_str = jd_to_time_str(local_jd);
    let tz_name = tz.name_at(jd);
    format!("{} {} ({}{}UT)", local_str, tz_name, ut_str, " ".dimmed())
}

/// Format a JD as "HH:MM TZ" (short form for tables).
pub fn jd_to_local_time_short(jd: JulianDay, tz: &TzInfo) -> String {
    let offset = tz.offset_at(jd);
    let local_jd = JulianDay(jd.0 + offset / 24.0);
    jd_to_time_str(local_jd)
}

pub fn moon_phase_icon(fraction: f64) -> &'static str {
    match (fraction * 8.0).round() as u32 {
        0 => "🌑",
        1 => "🌒",
        2 => "🌓",
        3 => "🌔",
        4 => "🌕",
        5 => "🌖",
        6 => "🌗",
        7 => "🌘",
        _ => "🌑",
    }
}

// --- Timezone ---

/// Timezone information for a location.
#[derive(Debug, Clone)]
pub struct TzInfo {
    /// Standard time name (e.g., "CET")
    pub std_name: &'static str,
    /// Daylight saving time name (e.g., "CEST")
    pub dst_name: &'static str,
    /// Standard UTC offset in hours (e.g., 1.0 for CET)
    pub std_offset: f64,
    /// DST UTC offset in hours (e.g., 2.0 for CEST)
    pub dst_offset: f64,
    /// DST rule
    pub dst_rule: DstRule,
}

#[derive(Debug, Clone, Copy)]
pub enum DstRule {
    /// No DST (e.g., Japan)
    None,
    /// EU: last Sunday of March to last Sunday of October
    Europe,
    /// US: second Sunday of March to first Sunday of November
    NorthAmerica,
    /// Australia: first Sunday of October to first Sunday of April (southern hemisphere)
    Australia,
}

impl TzInfo {
    pub fn utc() -> Self {
        Self {
            std_name: "UT",
            dst_name: "UT",
            std_offset: 0.0,
            dst_offset: 0.0,
            dst_rule: DstRule::None,
        }
    }

    /// Get the UTC offset (hours) at a given JD.
    pub fn offset_at(&self, jd: JulianDay) -> f64 {
        if self.is_dst(jd) {
            self.dst_offset
        } else {
            self.std_offset
        }
    }

    /// Get the timezone name at a given JD.
    pub fn name_at(&self, jd: JulianDay) -> &'static str {
        if self.is_dst(jd) {
            self.dst_name
        } else {
            self.std_name
        }
    }

    fn is_dst(&self, jd: JulianDay) -> bool {
        let (year, month, day) = date_from_jd(jd);
        let day = day as u32;

        match self.dst_rule {
            DstRule::None => false,
            DstRule::Europe => {
                // Last Sunday of March to last Sunday of October
                let mar_last_sun = last_sunday_of_month(year, 3);
                let oct_last_sun = last_sunday_of_month(year, 10);
                (month == 3 && day >= mar_last_sun)
                    || (month > 3 && month < 10)
                    || (month == 10 && day < oct_last_sun)
            }
            DstRule::NorthAmerica => {
                // Second Sunday of March to first Sunday of November
                let mar_2nd_sun = nth_sunday_of_month(year, 3, 2);
                let nov_1st_sun = nth_sunday_of_month(year, 11, 1);
                (month == 3 && day >= mar_2nd_sun)
                    || (month > 3 && month < 11)
                    || (month == 11 && day < nov_1st_sun)
            }
            DstRule::Australia => {
                // First Sunday of October to first Sunday of April (southern hemisphere)
                let oct_1st_sun = nth_sunday_of_month(year, 10, 1);
                let apr_1st_sun = nth_sunday_of_month(year, 4, 1);
                (month == 10 && day >= oct_1st_sun)
                    || (month > 10)
                    || (month < 4)
                    || (month == 4 && day < apr_1st_sun)
            }
        }
    }
}

/// Day of month for the last Sunday of a given month.
fn last_sunday_of_month(year: i32, month: u32) -> u32 {
    let days_in_month = match month {
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    let last_day_dow = day_of_week(year, month, days_in_month);
    days_in_month - last_day_dow // Sunday = 0
}

/// Day of month for the nth Sunday (n=1 first, n=2 second, etc.)
fn nth_sunday_of_month(year: i32, month: u32, n: u32) -> u32 {
    let first_dow = day_of_week(year, month, 1);
    let first_sunday = if first_dow == 0 { 1 } else { 8 - first_dow };
    first_sunday + (n - 1) * 7
}

/// Day of week: 0=Sunday, 1=Monday, ..., 6=Saturday (Zeller-like)
fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    let jd = rustarium_core::time::jd_from_date(year, month, day as f64);
    ((jd.0 + 1.5) as i64 % 7) as u32
}

// --- City database ---

/// City with location and timezone.
pub struct CityData {
    pub location: rustarium_core::coords::GeoLocation,
    pub tz: TzInfo,
    pub label: &'static str,
}

pub fn lookup_city(name: &str) -> Option<CityData> {
    use rustarium_core::coords::GeoLocation;

    let cet = TzInfo {
        std_name: "CET",
        dst_name: "CEST",
        std_offset: 1.0,
        dst_offset: 2.0,
        dst_rule: DstRule::Europe,
    };
    let gmt = TzInfo {
        std_name: "GMT",
        dst_name: "BST",
        std_offset: 0.0,
        dst_offset: 1.0,
        dst_rule: DstRule::Europe,
    };
    let est = TzInfo {
        std_name: "EST",
        dst_name: "EDT",
        std_offset: -5.0,
        dst_offset: -4.0,
        dst_rule: DstRule::NorthAmerica,
    };
    let pst = TzInfo {
        std_name: "PST",
        dst_name: "PDT",
        std_offset: -8.0,
        dst_offset: -7.0,
        dst_rule: DstRule::NorthAmerica,
    };
    let jst = TzInfo {
        std_name: "JST",
        dst_name: "JST",
        std_offset: 9.0,
        dst_offset: 9.0,
        dst_rule: DstRule::None,
    };
    let aest = TzInfo {
        std_name: "AEST",
        dst_name: "AEDT",
        std_offset: 10.0,
        dst_offset: 11.0,
        dst_rule: DstRule::Australia,
    };

    Some(match name.to_lowercase().as_str() {
        "rome" | "roma" => CityData {
            location: GeoLocation::from_degrees(41.9028, 12.4964, 21.0),
            tz: cet.clone(),
            label: "Roma",
        },
        "milan" | "milano" => CityData {
            location: GeoLocation::from_degrees(45.4642, 9.1900, 120.0),
            tz: cet.clone(),
            label: "Milano",
        },
        "naples" | "napoli" => CityData {
            location: GeoLocation::from_degrees(40.8518, 14.2681, 17.0),
            tz: cet,
            label: "Napoli",
        },
        "london" | "londra" => CityData {
            location: GeoLocation::from_degrees(51.5074, -0.1278, 11.0),
            tz: gmt,
            label: "London",
        },
        "paris" | "parigi" => CityData {
            location: GeoLocation::from_degrees(48.8566, 2.3522, 35.0),
            tz: TzInfo {
                std_name: "CET",
                dst_name: "CEST",
                std_offset: 1.0,
                dst_offset: 2.0,
                dst_rule: DstRule::Europe,
            },
            label: "Paris",
        },
        "berlin" | "berlino" => CityData {
            location: GeoLocation::from_degrees(52.5200, 13.4050, 34.0),
            tz: TzInfo {
                std_name: "CET",
                dst_name: "CEST",
                std_offset: 1.0,
                dst_offset: 2.0,
                dst_rule: DstRule::Europe,
            },
            label: "Berlin",
        },
        "madrid" => CityData {
            location: GeoLocation::from_degrees(40.4168, -3.7038, 650.0),
            tz: TzInfo {
                std_name: "CET",
                dst_name: "CEST",
                std_offset: 1.0,
                dst_offset: 2.0,
                dst_rule: DstRule::Europe,
            },
            label: "Madrid",
        },
        "new york" | "newyork" | "ny" => CityData {
            location: GeoLocation::from_degrees(40.7128, -74.0060, 10.0),
            tz: est,
            label: "New York",
        },
        "los angeles" | "la" => CityData {
            location: GeoLocation::from_degrees(34.0522, -118.2437, 71.0),
            tz: pst,
            label: "Los Angeles",
        },
        "tokyo" => CityData {
            location: GeoLocation::from_degrees(35.6762, 139.6503, 40.0),
            tz: jst,
            label: "Tokyo",
        },
        "sydney" => CityData {
            location: GeoLocation::from_degrees(-33.8688, 151.2093, 3.0),
            tz: aest,
            label: "Sydney",
        },
        _ => return None,
    })
}

/// Resolve location + timezone from CLI args. Returns (location, timezone, label).
pub fn resolve_location(
    lat: &Option<f64>,
    lon: &Option<f64>,
    city: &Option<String>,
) -> (rustarium_core::coords::GeoLocation, TzInfo, String) {
    use rustarium_core::coords::GeoLocation;

    if let (Some(lat), Some(lon)) = (lat, lon) {
        return (
            GeoLocation::from_degrees(*lat, *lon, 0.0),
            TzInfo::utc(),
            format!("{:.2}°N {:.2}°E", lat, lon),
        );
    }

    let city_name = city.as_deref().unwrap_or("roma");

    match lookup_city(city_name) {
        Some(data) => (data.location, data.tz, data.label.to_string()),
        None => {
            eprintln!(
                "  {} Unknown city '{}'. Use --lat/--lon or try: roma, milano, london, paris, berlin, madrid, ny, tokyo, sydney",
                "✗".red(),
                city_name
            );
            std::process::exit(1);
        }
    }
}

// --- Date/body parsing ---

pub fn parse_date_or_today(date_str: &Option<String>) -> JulianDay {
    match date_str {
        Some(s) => parse_date(s),
        None => today_jd(),
    }
}

pub fn parse_date(s: &str) -> JulianDay {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<f64>(),
        ) {
            return rustarium_core::time::jd_from_date(y, m, d);
        }
    }
    eprintln!(
        "  {} Invalid date '{}'. Use YYYY-MM-DD format.",
        "✗".red(),
        s
    );
    std::process::exit(1);
}

pub fn today_jd() -> JulianDay {
    let now = chrono::Local::now();
    rustarium_core::time::jd_from_datetime(
        now.format("%Y").to_string().parse().unwrap(),
        now.format("%m").to_string().parse().unwrap(),
        now.format("%d").to_string().parse().unwrap(),
        now.format("%H").to_string().parse().unwrap(),
        now.format("%M").to_string().parse().unwrap(),
        0.0,
    )
}

pub fn parse_body(name: &str) -> rustarium_core::bodies::Body {
    use rustarium_core::bodies::{Body, Planet};
    match name.to_lowercase().as_str() {
        "sun" | "sole" => Body::Sun,
        "moon" | "luna" => Body::Moon,
        "mercury" | "mercurio" => Body::Planet(Planet::Mercury),
        "venus" | "venere" => Body::Planet(Planet::Venus),
        "earth" | "terra" => Body::Planet(Planet::Earth),
        "mars" | "marte" => Body::Planet(Planet::Mars),
        "jupiter" | "giove" => Body::Planet(Planet::Jupiter),
        "saturn" | "saturno" => Body::Planet(Planet::Saturn),
        "uranus" | "urano" => Body::Planet(Planet::Uranus),
        "neptune" | "nettuno" => Body::Planet(Planet::Neptune),
        _ => {
            eprintln!(
                "  {} Unknown body '{}'. Available: sun, moon, mercury, venus, mars, jupiter, saturn, uranus, neptune",
                "✗".red(),
                name
            );
            std::process::exit(1);
        }
    }
}
