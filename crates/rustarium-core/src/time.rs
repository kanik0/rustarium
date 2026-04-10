use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Julian Day number. All internal time is in this form.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct JulianDay(pub f64);

/// J2000.0 epoch: 2000-Jan-01 12:00:00 TT (JD 2451545.0)
pub const J2000: JulianDay = JulianDay(2451545.0);

/// Julian days per Julian century
pub const DAYS_PER_CENTURY: f64 = 36525.0;

impl JulianDay {
    /// Julian centuries from J2000.0: T = (JD - 2451545.0) / 36525.0
    pub fn century(self) -> f64 {
        (self.0 - J2000.0) / DAYS_PER_CENTURY
    }

    /// Julian millennia from J2000.0 (used by VSOP87)
    pub fn millennium(self) -> f64 {
        self.century() / 10.0
    }

    pub fn from_century(t: f64) -> Self {
        JulianDay(J2000.0 + t * DAYS_PER_CENTURY)
    }
}

impl std::ops::Add<f64> for JulianDay {
    type Output = JulianDay;
    fn add(self, days: f64) -> JulianDay {
        JulianDay(self.0 + days)
    }
}

impl std::ops::Sub<f64> for JulianDay {
    type Output = JulianDay;
    fn sub(self, days: f64) -> JulianDay {
        JulianDay(self.0 - days)
    }
}

impl std::ops::Sub for JulianDay {
    type Output = f64;
    fn sub(self, other: JulianDay) -> f64 {
        self.0 - other.0
    }
}

/// Convert a Gregorian calendar date to Julian Day.
/// Meeus, "Astronomical Algorithms", chapter 7.
///
/// `day` can include a fractional part for the time of day.
/// Month: 1..=12, Day: 1..=31 (with fraction for hours).
pub fn jd_from_date(year: i32, month: u32, day: f64) -> JulianDay {
    let (y, m) = if month <= 2 {
        (year as f64 - 1.0, month as f64 + 12.0)
    } else {
        (year as f64, month as f64)
    };

    // Gregorian calendar reform: October 15, 1582
    // Before this date, use Julian calendar (B = 0)
    let b = if year > 1582 || (year == 1582 && month > 10) || (year == 1582 && month == 10 && day >= 15.0) {
        let a = (y / 100.0).floor();
        2.0 - a + (a / 4.0).floor()
    } else {
        0.0
    };

    let jd = (365.25 * (y + 4716.0)).floor()
        + (30.6001 * (m + 1.0)).floor()
        + day
        + b
        - 1524.5;

    JulianDay(jd)
}

/// Convert a Gregorian calendar date+time to Julian Day.
pub fn jd_from_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: f64,
) -> JulianDay {
    let day_frac = day as f64 + hour as f64 / 24.0 + min as f64 / 1440.0 + sec / 86400.0;
    jd_from_date(year, month, day_frac)
}

/// Convert Julian Day to Gregorian calendar date.
/// Returns (year, month, day_with_fraction).
pub fn date_from_jd(jd: JulianDay) -> (i32, u32, f64) {
    let jd_val = jd.0 + 0.5;
    let z = jd_val.floor();
    let f = jd_val - z;

    let a = if z < 2299161.0 {
        z
    } else {
        let alpha = ((z - 1867216.25) / 36524.25).floor();
        z + 1.0 + alpha - (alpha / 4.0).floor()
    };

    let b = a + 1524.0;
    let c = ((b - 122.1) / 365.25).floor();
    let d = (365.25 * c).floor();
    let e = ((b - d) / 30.6001).floor();

    let day = b - d - (30.6001 * e).floor() + f;
    let month = if e < 14.0 { e - 1.0 } else { e - 13.0 };
    let year = if month > 2.0 { c - 4716.0 } else { c - 4715.0 };

    (year as i32, month as u32, day)
}

/// ΔT = TT − UT1 in seconds.
/// Polynomial approximations from Espenak & Meeus (2006), extended.
/// Valid approximately from -500 to +2150.
pub fn delta_t(jd: JulianDay) -> f64 {
    let (year, month, _) = date_from_jd(jd);
    let y = year as f64 + (month as f64 - 0.5) / 12.0;

    if y < -500.0 {
        let u = (y - 1820.0) / 100.0;
        -20.0 + 32.0 * u * u
    } else if y < 500.0 {
        let u = y / 100.0;
        10583.6
            + u * (-1014.41
                + u * (33.78311
                    + u * (-5.952053
                        + u * (-0.1798452 + u * (0.022174192 + u * 0.0090316521)))))
    } else if y < 1600.0 {
        let u = (y - 1000.0) / 100.0;
        1574.2
            + u * (-556.01
                + u * (71.23472
                    + u * (0.319781
                        + u * (-0.8503463 + u * (-0.005050998 + u * 0.0083572073)))))
    } else if y < 1700.0 {
        let t = y - 1600.0;
        120.0 + t * (-0.9808 + t * (-0.01532 + t * (1.0 / 7129.0)))
    } else if y < 1800.0 {
        let t = y - 1700.0;
        8.83 + t * (0.1603 + t * (-0.0059285 + t * (0.00013336 + t * (-1.0 / 1174000.0))))
    } else if y < 1860.0 {
        let t = y - 1800.0;
        13.72 + t * (-0.332447
            + t * (0.0068612
                + t * (0.0041116
                    + t * (-0.00037436
                        + t * (0.0000121272
                            + t * (-0.0000001699 + t * 0.000000000875))))))
    } else if y < 1900.0 {
        let t = y - 1860.0;
        7.62 + t * (0.5737 + t * (-0.251754 + t * (0.01680668
            + t * (-0.0004473624 + t * (1.0 / 233174.0)))))
    } else if y < 1920.0 {
        let t = y - 1900.0;
        -2.79 + t * (1.494119 + t * (-0.0598939 + t * (0.0061966 + t * (-0.000197))))
    } else if y < 1941.0 {
        let t = y - 1920.0;
        21.20 + t * (0.84493 + t * (-0.076100 + t * 0.0020936))
    } else if y < 1961.0 {
        let t = y - 1950.0;
        29.07 + t * (0.407 + t * (-1.0 / 233.0 + t * (1.0 / 2547.0)))
    } else if y < 1986.0 {
        let t = y - 1975.0;
        45.45 + t * (1.067 + t * (-1.0 / 260.0 + t * (-1.0 / 718.0)))
    } else if y < 2005.0 {
        let t = y - 2000.0;
        63.86 + t * (0.3345 + t * (-0.060374 + t * (0.0017275
            + t * (0.000651814 + t * 0.00002373599))))
    } else if y < 2050.0 {
        let t = y - 2000.0;
        62.92 + t * (0.32217 + t * 0.005589)
    } else if y < 2150.0 {
        let u = (y - 1820.0) / 100.0;
        -20.0 + 32.0 * u * u - 0.5628 * (2150.0 - y)
    } else {
        let u = (y - 1820.0) / 100.0;
        -20.0 + 32.0 * u * u
    }
}

/// Convert JD in UT1 to JD in Terrestrial Time (TT).
pub fn jd_ut_to_tt(jd_ut: JulianDay) -> JulianDay {
    let dt = delta_t(jd_ut);
    JulianDay(jd_ut.0 + dt / 86400.0)
}

/// Convert JD in TT to JD in UT1.
pub fn jd_tt_to_ut(jd_tt: JulianDay) -> JulianDay {
    let dt = delta_t(jd_tt);
    JulianDay(jd_tt.0 - dt / 86400.0)
}

/// Greenwich Mean Sidereal Time at a given UT1 Julian Day.
/// Returns radians in [0, 2π).
/// Meeus, chapter 12, equation 12.4.
pub fn greenwich_mean_sidereal_time(jd_ut: JulianDay) -> f64 {
    let t = jd_ut.century();
    // GMST in degrees
    let gmst = 280.46061837
        + 360.98564736629 * (jd_ut.0 - 2451545.0)
        + 0.000387933 * t * t
        - t * t * t / 38710000.0;

    normalize_radians(gmst.to_radians())
}

/// Greenwich Apparent Sidereal Time.
/// GMST corrected for nutation in longitude.
pub fn greenwich_apparent_sidereal_time(
    jd_ut: JulianDay,
    nutation_longitude: f64,
    true_obliquity: f64,
) -> f64 {
    let gmst = greenwich_mean_sidereal_time(jd_ut);
    let correction = nutation_longitude * true_obliquity.cos();
    normalize_radians(gmst + correction)
}

/// Normalize an angle to [0, 2π).
pub fn normalize_radians(mut angle: f64) -> f64 {
    let two_pi = 2.0 * PI;
    angle %= two_pi;
    if angle < 0.0 {
        angle += two_pi;
    }
    angle
}

/// Normalize an angle to [0°, 360°).
pub fn normalize_degrees(mut angle: f64) -> f64 {
    angle %= 360.0;
    if angle < 0.0 {
        angle += 360.0;
    }
    angle
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-5;

    /// Meeus example 7.a: 1957 October 4.81 UT = JD 2436116.31
    #[test]
    fn jd_from_date_meeus_7a() {
        let jd = jd_from_date(1957, 10, 4.81);
        assert!((jd.0 - 2436116.31).abs() < EPSILON);
    }

    /// Meeus example 7.b: 333 January 27.5 = JD 1842713.0
    #[test]
    fn jd_from_date_meeus_7b() {
        let jd = jd_from_date(333, 1, 27.5);
        assert!((jd.0 - 1842713.0).abs() < EPSILON);
    }

    /// J2000.0 epoch: 2000 January 1.5 TT = JD 2451545.0
    #[test]
    fn jd_j2000() {
        let jd = jd_from_date(2000, 1, 1.5);
        assert!((jd.0 - 2451545.0).abs() < EPSILON);
    }

    /// Round-trip: JD -> date -> JD
    #[test]
    fn jd_roundtrip() {
        let original = JulianDay(2460000.5);
        let (y, m, d) = date_from_jd(original);
        let recovered = jd_from_date(y, m, d);
        assert!((original.0 - recovered.0).abs() < 1e-8);
    }

    #[test]
    fn julian_century_at_j2000() {
        assert!((J2000.century()).abs() < EPSILON);
    }

    #[test]
    fn delta_t_year_2000() {
        // ΔT ≈ 63.83 seconds around 2000
        let jd = jd_from_date(2000, 1, 1.0);
        let dt = delta_t(jd);
        assert!((dt - 63.83).abs() < 1.0); // within 1 second
    }

    /// Meeus example 12.a: GMST on 1987 April 10 at 0h UT
    /// Expected: 13h 10m 46.3668s
    #[test]
    fn gmst_meeus_12a() {
        let jd = jd_from_date(1987, 4, 10.0);
        let gmst = greenwich_mean_sidereal_time(jd);
        // Convert expected to radians: 13h 10m 46.3668s = 13.179546h
        // = 13.179546 * 15 = 197.693° = 3.4507... rad
        let expected_hours = 13.0 + 10.0 / 60.0 + 46.3668 / 3600.0;
        let expected_rad = expected_hours * 15.0_f64.to_radians();
        assert!(
            (gmst - expected_rad).abs() < 1e-4,
            "GMST: {} expected: {}",
            gmst,
            expected_rad
        );
    }

    #[test]
    fn normalize_radians_negative() {
        let angle = normalize_radians(-PI / 2.0);
        assert!((angle - 3.0 * PI / 2.0).abs() < EPSILON);
    }
}
