use crate::custom_body::{CustomBody, PropagationMethod, SmallBodyType};
use crate::nbody::orbital_elements::OrbitalElements;
use serde::Deserialize;

/// Raw SBDB API response structure.
/// Only the fields we need are deserialized; unknown fields are ignored.
#[derive(Debug, Deserialize)]
pub struct SbdbResponse {
    pub object: Option<SbdbObject>,
    pub orbit: Option<SbdbOrbit>,
    pub phys_par: Option<Vec<SbdbPhysParam>>,
}

#[derive(Debug, Deserialize)]
pub struct SbdbObject {
    pub fullname: Option<String>,
    pub des: Option<String>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub spkid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SbdbOrbit {
    pub elements: Option<Vec<SbdbElement>>,
    pub epoch: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SbdbElement {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct SbdbPhysParam {
    pub name: String,
    pub value: Option<String>,
}

impl SbdbResponse {
    /// Convert a parsed SBDB response into a CustomBody.
    pub fn to_custom_body(&self) -> Result<CustomBody, String> {
        let obj = self.object.as_ref().ok_or("missing 'object' field")?;
        let orbit = self.orbit.as_ref().ok_or("missing 'orbit' field")?;
        let elements = orbit
            .elements
            .as_ref()
            .ok_or("missing 'orbit.elements' field")?;

        // Extract orbital elements by name
        let a = get_element(elements, "a")?;
        let e = get_element(elements, "e")?;
        let i = get_element(elements, "i")?;
        let om = get_element(elements, "om")?;
        let w = get_element(elements, "w")?;
        let ma = get_element(elements, "ma")?;

        // Epoch (JD)
        let epoch_jd = match &orbit.epoch {
            Some(serde_json::Value::String(s)) => s
                .parse::<f64>()
                .map_err(|_| format!("invalid epoch: {}", s))?,
            Some(serde_json::Value::Number(n)) => n
                .as_f64()
                .ok_or_else(|| "invalid epoch number".to_string())?,
            _ => return Err("missing epoch".into()),
        };

        // Body name
        let name = obj
            .name
            .clone()
            .or_else(|| obj.fullname.clone())
            .or_else(|| obj.des.clone())
            .unwrap_or_else(|| "Unknown".into());

        // Body type from kind field
        let body_type = classify_body(obj.kind.as_deref());

        // Physical parameters (optional)
        let gm = get_phys_param(&self.phys_par, "GM").unwrap_or(0.0);
        let diameter_km = get_phys_param(&self.phys_par, "diameter");
        let abs_magnitude_h = get_phys_param(&self.phys_par, "H");

        Ok(CustomBody {
            name,
            designation: obj.des.clone(),
            body_type,
            propagation: PropagationMethod::Keplerian {
                elements: OrbitalElements::from_au_and_degrees(a, e, i, om, w, ma),
                epoch_jd,
            },
            gm,
            diameter_km,
            abs_magnitude_h,
            horizons_id: None,
        })
    }
}

fn get_element(elements: &[SbdbElement], name: &str) -> Result<f64, String> {
    elements
        .iter()
        .find(|el| el.name == name)
        .ok_or_else(|| format!("missing orbital element: {}", name))?
        .value
        .parse::<f64>()
        .map_err(|_| format!("invalid value for element: {}", name))
}

fn get_phys_param(params: &Option<Vec<SbdbPhysParam>>, name: &str) -> Option<f64> {
    params
        .as_ref()?
        .iter()
        .find(|p| p.name == name)?
        .value
        .as_ref()?
        .parse::<f64>()
        .ok()
}

fn classify_body(kind: Option<&str>) -> SmallBodyType {
    match kind {
        Some(k) if k.starts_with('a') => SmallBodyType::Asteroid,
        Some(k) if k.starts_with('c') => SmallBodyType::Comet,
        // Check designation patterns for TNOs and dwarf planets
        _ => SmallBodyType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ceres_sbdb_response() {
        let json = r#"{
            "object": {
                "fullname": "1 Ceres",
                "des": "1",
                "name": "Ceres",
                "kind": "an",
                "spkid": "2000001"
            },
            "orbit": {
                "epoch": "2460200.5",
                "elements": [
                    {"name": "e", "value": "0.07600902910070946"},
                    {"name": "a", "value": "2.766044736305795"},
                    {"name": "i", "value": "10.59351035990559"},
                    {"name": "om", "value": "80.30554898681753"},
                    {"name": "w", "value": "73.59764315927306"},
                    {"name": "ma", "value": "130.036"}
                ]
            },
            "phys_par": [
                {"name": "diameter", "value": "939.4"},
                {"name": "GM", "value": "62.6284"},
                {"name": "H", "value": "3.33"}
            ]
        }"#;

        let resp: SbdbResponse = serde_json::from_str(json).expect("parse SBDB JSON");
        let body = resp.to_custom_body().expect("convert to CustomBody");

        assert_eq!(body.name, "Ceres");
        assert_eq!(body.body_type, SmallBodyType::Asteroid);
        assert!((body.epoch_jd() - 2460200.5).abs() < 0.001);
        assert!((body.gm - 62.6284).abs() < 0.01);
        assert_eq!(body.diameter_km, Some(939.4));
        assert_eq!(body.abs_magnitude_h, Some(3.33));
        // Check orbital elements round-trip
        let a_au = body.elements().unwrap().semi_major_axis_km / crate::bodies::AU_KM;
        assert!(
            (a_au - 2.766).abs() < 0.001,
            "a = {} AU",
            a_au
        );
    }

    #[test]
    fn parse_comet_response() {
        let json = r#"{
            "object": {
                "fullname": "1P/Halley",
                "des": "1P",
                "name": "Halley",
                "kind": "cn"
            },
            "orbit": {
                "epoch": "2449400.5",
                "elements": [
                    {"name": "e", "value": "0.9671429085"},
                    {"name": "a", "value": "17.83414429"},
                    {"name": "i", "value": "162.2626906"},
                    {"name": "om", "value": "58.42008098"},
                    {"name": "w", "value": "111.3324851"},
                    {"name": "ma", "value": "38.3842644"}
                ]
            },
            "phys_par": []
        }"#;

        let resp: SbdbResponse = serde_json::from_str(json).expect("parse");
        let body = resp.to_custom_body().expect("convert");
        assert_eq!(body.name, "Halley");
        assert_eq!(body.body_type, SmallBodyType::Comet);
        assert!(body.elements().unwrap().eccentricity > 0.96);
    }

    #[test]
    fn missing_element_returns_error() {
        let json = r#"{
            "object": {"fullname": "Test", "kind": "an"},
            "orbit": {
                "epoch": "2460200.5",
                "elements": [
                    {"name": "e", "value": "0.1"},
                    {"name": "a", "value": "2.5"}
                ]
            }
        }"#;
        let resp: SbdbResponse = serde_json::from_str(json).expect("parse");
        let result = resp.to_custom_body();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing orbital element"));
    }
}
