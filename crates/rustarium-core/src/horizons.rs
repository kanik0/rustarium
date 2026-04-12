use crate::custom_body::{CustomBody, EphemerisPoint, PropagationMethod, SmallBodyType};
use serde::Deserialize;

// ===== Spacecraft Catalog =====

/// A known spacecraft with its JPL Horizons ID.
pub struct SpacecraftEntry {
    pub name: &'static str,
    pub horizons_id: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub launch_year: u16,
    pub status: &'static str,
}

pub const SPACECRAFT_CATALOG: &[SpacecraftEntry] = &[
    SpacecraftEntry {
        name: "Voyager 1",
        horizons_id: "-31",
        aliases: &["voyager1", "vgr1"],
        description: "Interstellar probe, farthest human-made object",
        launch_year: 1977,
        status: "Active",
    },
    SpacecraftEntry {
        name: "Voyager 2",
        horizons_id: "-32",
        aliases: &["voyager2", "vgr2"],
        description: "Interstellar probe, visited 4 planets",
        launch_year: 1977,
        status: "Active",
    },
    SpacecraftEntry {
        name: "New Horizons",
        horizons_id: "-98",
        aliases: &["newhorizons", "nh"],
        description: "Pluto and Kuiper Belt flyby",
        launch_year: 2006,
        status: "Active",
    },
    SpacecraftEntry {
        name: "JWST",
        horizons_id: "-170",
        aliases: &["james webb", "webb", "jwst"],
        description: "Space telescope at Sun-Earth L2",
        launch_year: 2021,
        status: "Active",
    },
    SpacecraftEntry {
        name: "Parker Solar Probe",
        horizons_id: "-96",
        aliases: &["parker", "psp"],
        description: "Closest approach to the Sun",
        launch_year: 2018,
        status: "Active",
    },
    SpacecraftEntry {
        name: "Juno",
        horizons_id: "-61",
        aliases: &["juno"],
        description: "Jupiter orbiter",
        launch_year: 2011,
        status: "Active",
    },
    SpacecraftEntry {
        name: "Lucy",
        horizons_id: "-49",
        aliases: &["lucy"],
        description: "Jupiter Trojan asteroid tour",
        launch_year: 2021,
        status: "Active",
    },
    SpacecraftEntry {
        name: "OSIRIS-APEX",
        horizons_id: "-64",
        aliases: &["osiris", "osiris-rex", "osiris-apex", "osirisrex"],
        description: "Asteroid sample return, now visiting Apophis",
        launch_year: 2016,
        status: "Active",
    },
    SpacecraftEntry {
        name: "Cassini",
        horizons_id: "-82",
        aliases: &["cassini"],
        description: "Saturn orbiter (deorbited 2017)",
        launch_year: 1997,
        status: "Completed",
    },
    SpacecraftEntry {
        name: "Mars Reconnaissance Orbiter",
        horizons_id: "-74",
        aliases: &["mro"],
        description: "Mars orbiter with HiRISE camera",
        launch_year: 2005,
        status: "Active",
    },
    SpacecraftEntry {
        name: "Perseverance",
        horizons_id: "-168",
        aliases: &["percy", "mars2020"],
        description: "Mars rover in Jezero crater",
        launch_year: 2020,
        status: "Active",
    },
];

/// Search the spacecraft catalog by name or alias (case-insensitive substring match).
pub fn search_catalog(query: &str) -> Vec<&'static SpacecraftEntry> {
    let q = query.to_lowercase();
    let q = q.trim();
    if q.is_empty() {
        return SPACECRAFT_CATALOG.iter().collect();
    }
    SPACECRAFT_CATALOG
        .iter()
        .filter(|e| {
            e.name.to_lowercase().contains(q)
                || e.horizons_id == q
                || e.aliases.iter().any(|a| a.to_lowercase().contains(q))
        })
        .collect()
}

// ===== Horizons Response Parser =====

/// Raw Horizons API JSON response.
#[derive(Debug, Deserialize)]
pub struct HorizonsResponse {
    pub result: Option<String>,
    pub signature: Option<HorizonsSignature>,
}

#[derive(Debug, Deserialize)]
pub struct HorizonsSignature {
    pub source: Option<String>,
    pub version: Option<String>,
}

impl HorizonsResponse {
    /// Parse the Horizons vector table response into a CustomBody.
    /// The `name` and `horizons_id` are provided from the catalog.
    pub fn to_custom_body(
        &self,
        name: &str,
        horizons_id: &str,
        catalog_entry: Option<&SpacecraftEntry>,
    ) -> Result<CustomBody, String> {
        let result = self.result.as_deref().ok_or("missing 'result' field")?;

        // Check for error messages
        if result.contains("Cannot find matching object")
            || result.contains("No ephemeris for target")
        {
            return Err(format!(
                "Horizons: no data for '{}' (ID {})",
                name, horizons_id
            ));
        }

        let table = parse_vector_table(result)?;
        if table.is_empty() {
            return Err("no ephemeris points found in Horizons response".into());
        }

        Ok(CustomBody {
            name: name.to_string(),
            designation: catalog_entry.map(|e| e.description.to_string()),
            body_type: SmallBodyType::Spacecraft,
            propagation: PropagationMethod::Ephemeris { table },
            gm: 0.0,
            diameter_km: None,
            abs_magnitude_h: None,
            horizons_id: Some(horizons_id.to_string()),
        })
    }
}

/// Parse the ASCII vector table between $$SOE and $$EOE markers.
/// Expected format (VECTORS output with AU-D units):
///
/// ```text
/// $$SOE
/// 2460310.500000000 = A.D. 2024-Jan-01 00:00:00.0000 TDB
///  X = 1.234567890E+02  Y =-5.678901234E+01  Z = 3.456789012E+00
///  VX= 1.234567890E-03 VY=-5.678901234E-04 VZ= 3.456789012E-05
/// $$EOE
/// ```
pub fn parse_vector_table(result: &str) -> Result<Vec<EphemerisPoint>, String> {
    let soe = result
        .find("$$SOE")
        .ok_or("$$SOE marker not found in Horizons result")?;
    let eoe = result
        .find("$$EOE")
        .ok_or("$$EOE marker not found in Horizons result")?;

    if eoe <= soe {
        return Err("$$EOE before $$SOE".into());
    }

    let block = &result[soe + 5..eoe];
    let lines: Vec<&str> = block
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // Each entry starts with a JD line, followed by X/Y/Z, VX/VY/VZ, and optionally LT/RG/RR.
    // We detect entry boundaries by looking for JD lines (start with a digit).
    let mut points = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        // Find next JD line (starts with a digit)
        let jd_line = lines[i];
        let jd = match parse_jd_from_line(jd_line) {
            Some(jd) => jd,
            None => {
                i += 1;
                continue;
            }
        };

        // Next line must be X/Y/Z
        if i + 1 >= lines.len() {
            break;
        }
        let xyz_line = lines[i + 1];
        let (x, y, z) = match parse_xyz(xyz_line) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };

        // Next line must be VX/VY/VZ
        let (vx, vy, vz) = if i + 2 < lines.len() {
            parse_vxyz(lines[i + 2]).unwrap_or((0.0, 0.0, 0.0))
        } else {
            (0.0, 0.0, 0.0)
        };

        points.push(EphemerisPoint {
            jd,
            x,
            y,
            z,
            vx,
            vy,
            vz,
        });

        // Skip all lines for this entry (JD + XYZ + VXYZ + optional LT/RG/RR)
        i += 3;
        // Skip any remaining lines that don't start with a digit (LT/RG/RR, blanks)
        while i < lines.len() && parse_jd_from_line(lines[i]).is_none() {
            i += 1;
        }
    }

    Ok(points)
}

fn parse_jd_from_line(line: &str) -> Option<f64> {
    // First token is the JD number
    line.split_whitespace().next()?.parse::<f64>().ok()
}

fn parse_xyz(line: &str) -> Option<(f64, f64, f64)> {
    // Format: " X = 1.234E+02 Y =-5.678E+01 Z = 3.456E+00"
    let x = extract_value(line, "X")?;
    let y = extract_value(line, "Y")?;
    let z = extract_value(line, "Z")?;
    Some((x, y, z))
}

fn parse_vxyz(line: &str) -> Option<(f64, f64, f64)> {
    // Format: " VX= 1.234E-03 VY=-5.678E-04 VZ= 3.456E-05"
    let vx = extract_value(line, "VX")?;
    let vy = extract_value(line, "VY")?;
    let vz = extract_value(line, "VZ")?;
    Some((vx, vy, vz))
}

/// Extract a numeric value after "KEY=" or "KEY =" from a Horizons line.
fn extract_value(line: &str, key: &str) -> Option<f64> {
    // Find the key (e.g., "X", "VX") followed by optional spaces and "="
    let key_pos = line.find(key)?;
    let after_key = &line[key_pos + key.len()..];
    // Skip spaces and "="
    let after_eq = after_key.trim_start().strip_prefix('=')?.trim_start();
    // Take the next token (possibly starting with '-')
    let token: String = after_eq
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    token.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_catalog_by_name() {
        let results = search_catalog("Voyager");
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|e| e.name == "Voyager 1"));
        assert!(results.iter().any(|e| e.name == "Voyager 2"));
    }

    #[test]
    fn search_catalog_by_alias() {
        let results = search_catalog("webb");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "JWST");
    }

    #[test]
    fn search_catalog_by_id() {
        let results = search_catalog("-31");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Voyager 1");
    }

    #[test]
    fn search_catalog_empty_returns_all() {
        let results = search_catalog("");
        assert_eq!(results.len(), SPACECRAFT_CATALOG.len());
    }

    #[test]
    fn parse_horizons_vector_table() {
        let result = r#"
Some header text
*******************************************************************************
$$SOE
2460310.500000000 = A.D. 2024-Jan-01 00:00:00.0000 TDB
 X = 1.532467890E+02 Y =-3.678901234E+01 Z = 6.456789012E+00
 VX= 3.234567890E-04 VY= 1.678901234E-03 VZ=-2.456789012E-05
2460315.500000000 = A.D. 2024-Jan-06 00:00:00.0000 TDB
 X = 1.534567890E+02 Y =-3.668901234E+01 Z = 6.446789012E+00
 VX= 3.244567890E-04 VY= 1.688901234E-03 VZ=-2.446789012E-05
$$EOE
*******************************************************************************
"#;

        let points = parse_vector_table(result).expect("parse table");
        assert_eq!(points.len(), 2);
        assert!((points[0].jd - 2460310.5).abs() < 0.001);
        assert!((points[0].x - 153.2467890).abs() < 0.01);
        assert!((points[0].y - (-36.78901234)).abs() < 0.01);
        assert!((points[1].jd - 2460315.5).abs() < 0.001);
    }

    #[test]
    fn parse_horizons_to_custom_body() {
        let json = r#"{
            "result": "header\n$$SOE\n2460310.500000000 = A.D. 2024-Jan-01 TDB\n X = 1.0E+02 Y =-5.0E+01 Z = 3.0E+00\n VX= 1.0E-03 VY=-5.0E-04 VZ= 3.0E-05\n$$EOE\nfooter",
            "signature": {"source": "NASA/JPL", "version": "1.0"}
        }"#;

        let resp: HorizonsResponse = serde_json::from_str(json).expect("parse JSON");
        let body = resp
            .to_custom_body("Voyager 1", "-31", Some(&SPACECRAFT_CATALOG[0]))
            .expect("to_custom_body");

        assert_eq!(body.name, "Voyager 1");
        assert_eq!(body.body_type, SmallBodyType::Spacecraft);
        assert_eq!(body.horizons_id, Some("-31".into()));
        assert!(body.ephemeris_table().is_some());
        assert_eq!(body.ephemeris_table().unwrap().len(), 1);
    }

    #[test]
    fn extract_value_various_formats() {
        // Space before =
        assert!((extract_value(" X = 1.5E+02 Y =-3.0E+01", "X").unwrap() - 150.0).abs() < 0.1);
        // No space before =
        assert!((extract_value(" VX= 3.2E-04 VY=-5.0E-04", "VX").unwrap() - 3.2e-4).abs() < 1e-8);
        // Negative value
        assert!((extract_value(" Y =-3.678E+01 Z = 6.0", "Y").unwrap() - (-36.78)).abs() < 0.01);
    }
}
