use super::TileDesign;

pub const THREE_D_SHADOW_PATH: &str = "/assets/tiles/3d/shadow.svg";
pub const THREE_D_SHADOW_PREFIX: &str = "hive_shadow_";

pub fn inline_tile_sprite(design: &TileDesign) -> Option<(&'static str, &'static str)> {
    match design {
        TileDesign::ThreeD => Some(("/assets/tiles/3d/3d.svg", "hive_3d_")),
        TileDesign::Carbon3D => Some(("/assets/tiles/carbon-3d/carbon-3d.svg", "hive_carbon_3d_")),
        _ => None,
    }
}

/// Scope trusted repository SVG assets before sharing their definitions in one document.
/// Asset serialization keeps double-quoted attributes and unquoted local url() references.
pub fn namespace_svg(svg: &str, prefix: &str) -> String {
    let start = svg
        .find("<svg")
        .and_then(|start| svg[start..].find('>').map(|end| start + end + 1))
        .unwrap_or(0);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    svg[start..end]
        .replace(" id=\"", &format!(" id=\"{prefix}"))
        .replace("href=\"#", &format!("href=\"#{prefix}"))
        .replace("url(#", &format!("url(#{prefix}"))
}
