use serde::{Deserialize, Deserializer, Serialize};

/// Accepts `true`/`false` or `1`/`0` for a flag.
///
/// The backing column is `INTEGER`, and the generator that produces
/// `country_reference.json` has emitted both encodings, so tolerate both rather
/// than making a whole-world seed load fail on a JSON literal style. Always
/// re-serialises as a JSON boolean.
fn flexible_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolOrInt {
        Bool(bool),
        Int(i64),
    }
    Ok(match BoolOrInt::deserialize(d)? {
        BoolOrInt::Bool(b) => b,
        BoolOrInt::Int(i) => i != 0,
    })
}

/// A country's identity and where to draw it — the reference data every other
/// table joins against by `country_code`. Distinct from
/// [`crate::models::country::Country`], which is a researched policy dossier
/// and exists for only a handful of countries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryReference {
    pub country_code: String,
    pub country_name: String,
    #[serde(default)]
    pub alpha3: Option<String>,
    /// ISO 3166-1 numeric, zero-padded to 3 chars. This is the join key onto
    /// the basemap's topojson `feature.id`, which carries no alpha-2 code.
    /// `None` for codes outside ISO 3166-1 (currently only XK).
    #[serde(default)]
    pub iso_numeric: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub subregion: Option<String>,
    /// Label point: the centroid of the country's largest landmass, not of all
    /// its landmasses averaged. `None` when the basemap has no geometry.
    #[serde(default)]
    pub centroid_lat: Option<f64>,
    #[serde(default)]
    pub centroid_lon: Option<f64>,
    /// Spans every landmass, unlike the centroid — it is used for zoom framing,
    /// so a client can derive a fly-to altitude without a geometry index.
    #[serde(default)]
    pub bbox_min_lon: Option<f64>,
    #[serde(default)]
    pub bbox_min_lat: Option<f64>,
    #[serde(default)]
    pub bbox_max_lon: Option<f64>,
    #[serde(default)]
    pub bbox_max_lat: Option<f64>,
    /// True when the basemap has geometry to draw. Replaces every hardcoded
    /// country-code array in the codebase.
    #[serde(deserialize_with = "flexible_bool")]
    pub include_on_globe: bool,
    /// Lower sorts first in a fetch cycle. 0 = actively researched.
    pub priority_tier: i64,
}
