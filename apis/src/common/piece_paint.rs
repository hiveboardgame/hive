use super::config_options::{TileDesign, TileDots, TileRotation};
use crate::providers::config::TileOptions;
use hive_lib::{Bug, Color, Piece};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PiecePaint {
    pub shadow_href: ShadowHref,
    pub tile_href: TileHref,
    pub bug_href: BugHref,
    pub dots_href: Option<DotsHref>,
    pub dot_color: &'static str,
    pub rotation: Option<usize>,
    pub three_d: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShadowHref {
    None,
    CommonDropShadow,
    ThreeDShadow,
}

impl ShadowHref {
    pub fn for_design(design: &TileDesign) -> Self {
        match design {
            TileDesign::ThreeD | TileDesign::Carbon3D => Self::ThreeDShadow,
            TileDesign::Community
            | TileDesign::HighContrast
            | TileDesign::Official
            | TileDesign::Flat
            | TileDesign::Pride
            | TileDesign::Carbon => Self::CommonDropShadow,
        }
    }

    pub fn href(self) -> &'static str {
        match self {
            Self::None => "#no_ds",
            Self::CommonDropShadow => "/assets/tiles/common/all.svg#drop_shadow",
            Self::ThreeDShadow => "/assets/tiles/3d/shadow.svg#dshadow",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TileHref(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BugHref(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DotsHref(pub String);

#[derive(Clone, Copy)]
struct BugDotPalette {
    ant: &'static str,
    beetle: &'static str,
    grasshopper: &'static str,
    spider: &'static str,
    fallback: &'static str,
}

impl BugDotPalette {
    fn color_for(self, bug: Bug) -> &'static str {
        match bug {
            Bug::Ant => self.ant,
            Bug::Beetle => self.beetle,
            Bug::Grasshopper => self.grasshopper,
            Bug::Spider => self.spider,
            _ => self.fallback,
        }
    }
}

const OFFICIAL_DOT_PALETTE: BugDotPalette = BugDotPalette {
    ant: "#289ee0",
    beetle: "#9a7fc7",
    grasshopper: "#42b23c",
    spider: "#a4572a",
    fallback: "#FF0000",
};

const FLAT_DOT_PALETTE: BugDotPalette = BugDotPalette {
    ant: "#3574a5",
    beetle: "#7a4fab",
    grasshopper: "#3f9b3a",
    spider: "#993c1e",
    fallback: "#FF0000",
};

const HIGH_CONTRAST_DOT_PALETTE: BugDotPalette = BugDotPalette {
    ant: "#3574a5",
    beetle: "#ac5bb3",
    grasshopper: "#3f9b3a",
    spider: "#d66138",
    fallback: "#FF0000",
};

pub fn resolve_piece_paint(
    piece: Piece,
    tile_options: &TileOptions,
    shadow_href: ShadowHref,
) -> PiecePaint {
    PiecePaint {
        shadow_href,
        tile_href: TileHref(piece_tile_href(piece, &tile_options.design)),
        bug_href: BugHref(piece_bug_href(piece, &tile_options.design)),
        dots_href: piece_dots_href(piece, &tile_options.dots),
        dot_color: piece_dot_color(piece, &tile_options.design),
        rotation: piece_rotation_degrees(piece, &tile_options.rotation),
        three_d: tile_options.is_three_d(),
    }
}

fn piece_dot_color(piece: Piece, design: &TileDesign) -> &'static str {
    match design {
        TileDesign::Official | TileDesign::ThreeD => OFFICIAL_DOT_PALETTE.color_for(piece.bug()),
        TileDesign::Flat => FLAT_DOT_PALETTE.color_for(piece.bug()),
        TileDesign::HighContrast | TileDesign::Community => {
            HIGH_CONTRAST_DOT_PALETTE.color_for(piece.bug())
        }
        TileDesign::Pride => match piece.color() {
            Color::White => "#3a3a3a",
            Color::Black => "#ead9b6",
        },
        TileDesign::Carbon3D | TileDesign::Carbon => match piece.color() {
            Color::White => "#2d2d2d",
            Color::Black => "#dcdcdc",
        },
    }
}

fn piece_dots_href(piece: Piece, dots: &TileDots) -> Option<DotsHref> {
    match dots {
        TileDots::No => None,
        TileDots::Angled => Some(DotsHref(format!(
            "/assets/tiles/common/all.svg#a{}",
            piece.order()
        ))),
        TileDots::Vertical => Some(DotsHref(format!(
            "/assets/tiles/common/all.svg#v{}",
            piece.order()
        ))),
    }
}

fn piece_bug_href(piece: Piece, design: &TileDesign) -> String {
    let bug = piece.bug();
    let color = piece.color();
    let asset_path = tile_asset_path(design);
    match design {
        TileDesign::Official
        | TileDesign::Flat
        | TileDesign::HighContrast
        | TileDesign::Community => format!("{asset_path}#{}", bug.name()),
        TileDesign::ThreeD | TileDesign::Pride | TileDesign::Carbon3D | TileDesign::Carbon => {
            format!("{asset_path}#{}{}", color.name(), bug.name())
        }
    }
}

fn piece_tile_href(piece: Piece, design: &TileDesign) -> String {
    let color = piece.color();
    format!("{}#{}", tile_asset_path(design), color.name())
}

fn piece_rotation_degrees(piece: Piece, rotation: &TileRotation) -> Option<usize> {
    match rotation {
        TileRotation::No => None,
        TileRotation::Yes => Some(piece.order().saturating_sub(1) * 60),
    }
}

fn tile_asset_path(design: &TileDesign) -> &'static str {
    match design {
        TileDesign::Official => "/assets/tiles/official/official.svg",
        TileDesign::Flat => "/assets/tiles/flat/flat.svg",
        TileDesign::ThreeD => "/assets/tiles/3d/3d.svg",
        TileDesign::HighContrast => "/assets/tiles/high-contrast/high-contrast.svg",
        TileDesign::Community => "/assets/tiles/community/community.svg",
        TileDesign::Pride => "/assets/tiles/lgbtq/lgbtq.svg",
        TileDesign::Carbon3D => "/assets/tiles/carbon-3d/carbon-3d.svg",
        TileDesign::Carbon => "/assets/tiles/carbon/carbon.svg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    #[test]
    fn piece_asset_hrefs_follow_tile_design() {
        let white_ant = piece("wA1");
        let black_beetle = piece("bB2");

        assert_eq!(
            piece_tile_href(white_ant, &TileDesign::Official),
            "/assets/tiles/official/official.svg#white"
        );
        assert_eq!(
            piece_bug_href(white_ant, &TileDesign::Official),
            "/assets/tiles/official/official.svg#Ant"
        );
        assert_eq!(
            piece_tile_href(black_beetle, &TileDesign::ThreeD),
            "/assets/tiles/3d/3d.svg#black"
        );
        assert_eq!(
            piece_bug_href(black_beetle, &TileDesign::ThreeD),
            "/assets/tiles/3d/3d.svg#blackBeetle"
        );
        assert_eq!(
            piece_bug_href(black_beetle, &TileDesign::Carbon3D),
            "/assets/tiles/carbon-3d/carbon-3d.svg#blackBeetle"
        );
        assert_eq!(
            piece_bug_href(white_ant, &TileDesign::Pride),
            "/assets/tiles/lgbtq/lgbtq.svg#whiteAnt"
        );
    }

    #[test]
    fn piece_dots_and_rotation_match_options() {
        let ant = piece("wA3");
        let queen = piece("wQ");

        assert_eq!(
            piece_dots_href(ant, &TileDots::Vertical),
            Some(DotsHref("/assets/tiles/common/all.svg#v3".to_string()))
        );
        assert_eq!(
            piece_dots_href(ant, &TileDots::Angled),
            Some(DotsHref("/assets/tiles/common/all.svg#a3".to_string()))
        );
        assert_eq!(piece_dots_href(ant, &TileDots::No), None);
        assert_eq!(piece_rotation_degrees(ant, &TileRotation::Yes), Some(120));
        assert_eq!(piece_rotation_degrees(queen, &TileRotation::Yes), Some(0));
        assert_eq!(piece_rotation_degrees(ant, &TileRotation::No), None);
    }

    #[test]
    fn dot_colors_match_current_design_rules() {
        assert_eq!(
            piece_dot_color(piece("wA1"), &TileDesign::Official),
            "#289ee0"
        );
        assert_eq!(piece_dot_color(piece("wB1"), &TileDesign::Flat), "#7a4fab");
        assert_eq!(
            piece_dot_color(piece("wS1"), &TileDesign::Community),
            "#d66138"
        );
        assert_eq!(piece_dot_color(piece("wQ"), &TileDesign::Pride), "#3a3a3a");
        assert_eq!(piece_dot_color(piece("bQ"), &TileDesign::Carbon), "#dcdcdc");
    }
}
