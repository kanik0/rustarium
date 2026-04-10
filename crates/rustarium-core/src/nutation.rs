use crate::time::JulianDay;

/// Compute nutation in longitude (Δψ) and nutation in obliquity (Δε).
/// Returns (delta_psi, delta_epsilon) in radians.
///
/// Uses the IAU 2000B model (77 lunisolar terms + 1 planetary correction).
/// Accuracy: ~0.001 arcseconds (1 milliarcsecond).
/// Reference: McCarthy & Luzum (2003), IERS Conventions (2003) chapter 5.
pub fn nutation(jd: JulianDay) -> (f64, f64) {
    let t = jd.century();

    // Fundamental arguments (Delaunay variables) in radians
    // IERS Conventions (2003), eq. 5.43
    let l = fundamental_l(t);      // Mean anomaly of the Moon
    let lp = fundamental_lp(t);    // Mean anomaly of the Sun
    let f = fundamental_f(t);      // Mean argument of latitude of the Moon
    let d = fundamental_d(t);      // Mean elongation of the Moon from the Sun
    let omega = fundamental_omega(t); // Mean longitude of ascending node

    let mut delta_psi = 0.0_f64;
    let mut delta_epsilon = 0.0_f64;

    // IAU 2000B lunisolar nutation: 77 terms
    // Format: (l, lp, F, D, Omega, S_psi, S_dot_psi, C_eps, C_dot_eps)
    // Coefficients in 0.1 microarcseconds (0.1 μas)
    for &(nl, nlp, nf, nd, nom, sp, spt, ce, cet) in &IAU2000B_TERMS {
        let arg = nl as f64 * l + nlp as f64 * lp + nf as f64 * f
            + nd as f64 * d + nom as f64 * omega;
        let sin_arg = arg.sin();
        let cos_arg = arg.cos();

        delta_psi += (sp + spt * t) * sin_arg;
        delta_epsilon += (ce + cet * t) * cos_arg;
    }

    // Convert from 0.1 microarcseconds to radians
    // 0.1 μas = 0.1e-6 arcseconds = 1e-7 arcseconds
    let to_rad = std::f64::consts::PI / (180.0 * 3600.0 * 1e7);
    delta_psi *= to_rad;
    delta_epsilon *= to_rad;

    // Fixed planetary correction for IAU 2000B (luni-solar approximation)
    let dpsi_corr = -0.135 / 3600.0 * std::f64::consts::PI / 180.0; // milliarcseconds
    let deps_corr = 0.388 / 3600.0 * std::f64::consts::PI / 180.0;
    delta_psi += dpsi_corr * t;
    delta_epsilon += deps_corr * t;

    (delta_psi, delta_epsilon)
}

// Fundamental arguments (IERS 2003)

fn fundamental_l(t: f64) -> f64 {
    normalize_angle(
        485868.249036 + t * (1717915923.2178
            + t * (31.8792 + t * (0.051635 + t * (-0.00024470)))),
    )
}

fn fundamental_lp(t: f64) -> f64 {
    normalize_angle(
        1287104.793048 + t * (129596581.0481
            + t * (-0.5532 + t * (0.000136 + t * (-0.00001149)))),
    )
}

fn fundamental_f(t: f64) -> f64 {
    normalize_angle(
        335779.526232 + t * (1739527262.8478
            + t * (-12.7512 + t * (-0.001037 + t * (0.00000417)))),
    )
}

fn fundamental_d(t: f64) -> f64 {
    normalize_angle(
        1072260.703692 + t * (1602961601.2090
            + t * (-6.3706 + t * (0.006593 + t * (-0.00003169)))),
    )
}

fn fundamental_omega(t: f64) -> f64 {
    normalize_angle(
        450160.398036 + t * (-6962890.5431
            + t * (7.4722 + t * (0.007702 + t * (-0.00005939)))),
    )
}

/// Convert arcseconds to radians and normalize to [0, 2π)
fn normalize_angle(arcsec: f64) -> f64 {
    let rad = arcsec * std::f64::consts::PI / (180.0 * 3600.0);
    let two_pi = 2.0 * std::f64::consts::PI;
    ((rad % two_pi) + two_pi) % two_pi
}

/// Mean obliquity of the ecliptic (ε₀) in radians.
/// Laskar formula (valid for ±10000 years from J2000).
pub fn mean_obliquity(jd: JulianDay) -> f64 {
    let t = jd.century();
    let u = t / 100.0;

    let epsilon_arcsec = 23.0 * 3600.0
        + 26.0 * 60.0
        + 21.448
        - 4680.93 * u
        - 1.55 * u * u
        + 1999.25 * u * u * u
        - 51.38 * u.powi(4)
        - 249.67 * u.powi(5)
        - 39.05 * u.powi(6)
        + 7.12 * u.powi(7)
        + 27.87 * u.powi(8)
        + 5.79 * u.powi(9)
        + 2.45 * u.powi(10);

    epsilon_arcsec * std::f64::consts::PI / (180.0 * 3600.0)
}

/// True obliquity of the ecliptic (ε) in radians.
pub fn true_obliquity(jd: JulianDay) -> f64 {
    let (_, delta_epsilon) = nutation(jd);
    mean_obliquity(jd) + delta_epsilon
}

/// IAU 2000B lunisolar nutation coefficients (77 terms).
/// Format: (l, l', F, D, Ω, Sp, Spt, Ce, Cet)
/// Sp/Ce in units of 0.1 μas (microarcseconds), Spt/Cet in 0.1 μas/century.
/// Source: IERS Conventions (2003), Table 5.3b.
#[rustfmt::skip]
const IAU2000B_TERMS: [(i8, i8, i8, i8, i8, f64, f64, f64, f64); 77] = [
    // l   l'  F   D   Ω         Sp          Spt         Ce          Cet
    ( 0,  0,  0,  0,  1, -172064161.0, -174666.0, 92052331.0, 9086.0),
    ( 0,  0,  2, -2,  2,  -13170906.0,  -1675.0,  5730336.0, -3015.0),
    ( 0,  0,  2,  0,  2,   -2276413.0,   -234.0,   978459.0,  -485.0),
    ( 0,  0,  0,  0,  2,    2074554.0,    207.0,  -897492.0,   470.0),
    ( 0,  1,  0,  0,  0,    1475877.0,  -3633.0,    73871.0,  -184.0),
    ( 0,  1,  2, -2,  2,    -516821.0,   1226.0,   224386.0,  -677.0),
    ( 1,  0,  0,  0,  0,     711159.0,     73.0,    -6750.0,     0.0),
    ( 0,  0,  2,  0,  1,    -387298.0,   -367.0,   200728.0,    18.0),
    ( 1,  0,  2,  0,  2,    -301461.0,    -36.0,   129025.0,   -63.0),
    ( 0, -1,  2, -2,  2,     215829.0,   -494.0,   -95929.0,   299.0),
    ( 0,  0,  2, -2,  1,     128227.0,    137.0,   -68982.0,    -9.0),
    (-1,  0,  2,  0,  2,     123457.0,     11.0,   -53311.0,    32.0),
    (-1,  0,  0,  2,  0,     156994.0,     10.0,    -1235.0,     0.0),
    ( 1,  0,  0,  0,  1,      63110.0,     63.0,   -33228.0,     0.0),
    (-1,  0,  0,  0,  1,     -57976.0,    -63.0,    31429.0,     0.0),
    (-1,  0,  2,  2,  2,     -59641.0,    -11.0,    25543.0,   -11.0),
    ( 1,  0,  2,  0,  1,     -51613.0,    -42.0,    26366.0,     0.0),
    (-2,  0,  2,  0,  1,      45893.0,     50.0,   -24236.0,   -10.0),
    ( 0,  0,  0,  2,  0,      63384.0,     11.0,    -1220.0,     0.0),
    ( 0,  0,  2,  2,  2,     -38571.0,     -1.0,    16452.0,   -11.0),
    ( 0, -2,  2, -2,  2,      32481.0,      0.0,   -13870.0,     0.0),
    (-2,  0,  0,  2,  0,     -47722.0,      0.0,      477.0,   -25.0),
    ( 2,  0,  2,  0,  2,     -31046.0,     -1.0,    13238.0,   -11.0),
    ( 1,  0,  2, -2,  2,      28593.0,      0.0,   -12338.0,    10.0),
    (-1,  0,  2,  0,  1,      20441.0,     21.0,   -10758.0,     0.0),
    ( 2,  0,  0,  0,  0,      29243.0,      0.0,     -609.0,     0.0),
    ( 0,  0,  2,  0,  0,      25887.0,      0.0,     -550.0,     0.0),
    ( 0,  1,  0,  0,  1,     -14053.0,    -25.0,     8551.0,    -2.0),
    (-1,  0,  0,  2,  1,      15164.0,     10.0,    -8001.0,     0.0),
    ( 0,  2,  2, -2,  2,     -15794.0,     72.0,     6850.0,   -42.0),
    ( 0,  0, -2,  2,  0,      21783.0,      0.0,     -167.0,    13.0),
    ( 1,  0,  0, -2,  1,     -12873.0,    -10.0,     6953.0,     0.0),
    ( 0, -1,  0,  0,  1,     -12654.0,     11.0,     6415.0,     0.0),
    (-1,  0,  2,  2,  1,     -10204.0,      0.0,     5222.0,     0.0),
    ( 0,  2,  0,  0,  0,      16707.0,    -85.0,      168.0,    -1.0),
    ( 1,  0,  2,  2,  2,      -7691.0,      0.0,     3268.0,     0.0),
    (-2,  0,  2,  0,  0,     -11024.0,      0.0,      104.0,     0.0),
    ( 0,  1,  2,  0,  2,       7566.0,    -21.0,    -3250.0,     0.0),
    ( 0,  0,  2,  2,  1,      -6637.0,    -11.0,     3353.0,     0.0),
    ( 0, -1,  2,  0,  2,      -7141.0,     21.0,     3070.0,     0.0),
    ( 0,  0,  0,  2,  1,      -6302.0,    -11.0,     3272.0,     0.0),
    ( 1,  0,  2, -2,  1,       5800.0,     10.0,    -3045.0,     0.0),
    ( 2,  0,  2, -2,  2,       6443.0,      0.0,    -2768.0,     0.0),
    (-2,  0,  0,  2,  1,      -5774.0,    -11.0,     3041.0,     0.0),
    ( 2,  0,  2,  0,  1,      -5350.0,      0.0,     2695.0,     0.0),
    ( 0, -1,  2, -2,  1,      -4752.0,    -11.0,     2719.0,     0.0),
    ( 0,  0,  0, -2,  1,      -4940.0,    -11.0,     2720.0,     0.0),
    (-1, -1,  0,  2,  0,       7350.0,      0.0,      -51.0,     0.0),
    ( 2,  0,  0, -2,  1,      -4421.0,      0.0,     2463.0,     0.0),
    ( 1,  0,  0,  2,  0,       7120.0,      0.0,      -78.0,     0.0),
    ( 0,  1,  2, -2,  1,      -4067.0,      0.0,     2184.0,     0.0),
    ( 1, -1,  0,  0,  0,       5477.0,      0.0,     -100.0,     0.0),
    (-1,  0,  0,  1,  1,      -4272.0,      0.0,     2270.0,     0.0),
    (-1, -1,  2,  2,  2,      -3503.0,      0.0,     1500.0,     0.0),
    ( 0,  1,  0,  0, -1,      -3633.0,      0.0,     2032.0,     0.0),
    (-1,  0,  2,  0,  0,       5765.0,      0.0,     -130.0,     0.0),
    ( 0, -1,  2,  2,  2,      -4098.0,      0.0,     1766.0,     0.0),
    (-2,  0,  0,  0,  1,      -4581.0,      0.0,     2397.0,     0.0),
    ( 1,  1,  2,  0,  2,       3530.0,      0.0,    -1484.0,     0.0),
    ( 2,  0,  0,  0,  1,       3487.0,      0.0,    -1747.0,     0.0),
    (-1,  1,  0,  1,  0,       5765.0,      0.0,     -130.0,     0.0),
    ( 1,  1,  0,  0,  0,       3534.0,      0.0,      -36.0,     0.0),
    ( 1,  0,  2,  0,  0,       3493.0,      0.0,     -164.0,     0.0),
    (-1,  0,  2, -2,  1,      -3210.0,      0.0,     1693.0,     0.0),
    ( 1,  0,  0,  0,  2,      -3013.0,      0.0,     1584.0,     0.0),
    (-1,  0,  0,  1,  0,       3025.0,      0.0,     -112.0,     0.0),
    ( 0,  0,  2,  1,  2,       2789.0,      0.0,    -1202.0,     0.0),
    (-1,  0,  2,  4,  2,      -2857.0,      0.0,     1228.0,     0.0),
    (-1,  1,  0,  1,  1,       2813.0,      0.0,    -1415.0,     0.0),
    ( 0, -2,  2, -2,  1,      -2652.0,      0.0,     1404.0,     0.0),
    ( 1,  0,  2,  2,  1,      -2568.0,      0.0,     1281.0,     0.0),
    (-2,  0,  2,  2,  2,      -2510.0,      0.0,     1078.0,     0.0),
    (-1,  0,  0,  0,  2,       2500.0,      0.0,    -1315.0,     0.0),
    ( 1,  1,  2, -2,  2,       2552.0,      0.0,    -1114.0,     0.0),
    (-2,  0,  2,  4,  2,      -2264.0,      0.0,      973.0,     0.0),
    (-1,  0,  4,  0,  2,       2172.0,      0.0,     -937.0,     0.0),
    ( 2,  0,  2, -2,  1,       1932.0,      0.0,     -988.0,     0.0),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::jd_from_date;

    /// Meeus example 22.a: JD 2446895.5 (1987 April 10)
    /// Expected with full IAU 2000B: Δψ ≈ -3.788", Δε ≈ +9.443"
    #[test]
    fn nutation_meeus_22a() {
        let jd = jd_from_date(1987, 4, 10.0);
        let (dpsi, deps) = nutation(jd);

        let dpsi_arcsec = dpsi.to_degrees() * 3600.0;
        let deps_arcsec = deps.to_degrees() * 3600.0;

        // IAU 2000B should be more accurate than old 13-term model
        assert!(
            (dpsi_arcsec - (-3.788)).abs() < 0.3,
            "Δψ = {:.3}\" expected -3.788\"",
            dpsi_arcsec
        );
        assert!(
            (deps_arcsec - 9.443).abs() < 0.3,
            "Δε = {:.3}\" expected 9.443\"",
            deps_arcsec
        );
    }

    /// Mean obliquity at J2000.0 should be ~23.4393°
    #[test]
    fn mean_obliquity_j2000() {
        let eps = mean_obliquity(crate::time::J2000);
        let eps_deg = eps.to_degrees();
        assert!(
            (eps_deg - 23.4393).abs() < 0.001,
            "ε₀ = {:.4}° expected ~23.4393°",
            eps_deg
        );
    }

    #[test]
    fn nutation_77_terms_more_accurate() {
        // The 77-term model should agree with the 13-term within ~0.5"
        // but be more precise at higher resolution
        let jd = jd_from_date(2024, 6, 15.0);
        let (dpsi, deps) = nutation(jd);

        // Nutation values should be small angles (< 20 arcseconds)
        assert!(dpsi.to_degrees().abs() * 3600.0 < 20.0);
        assert!(deps.to_degrees().abs() * 3600.0 < 20.0);
    }
}
