use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
pub struct ProfileCatalog {
    pub countries: Vec<(String, String)>,
    pub locations: Vec<String>,
}

pub fn catalog() -> &'static ProfileCatalog {
    static CATALOG: OnceLock<ProfileCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("profile_catalog.json"))
            .expect("valid bundled profile catalog")
    })
}

pub fn flag_emoji(code: &str) -> String {
    code.bytes()
        .filter_map(|c| char::from_u32(0x1F1E6 + u32::from(c.saturating_sub(b'A'))))
        .collect()
}
