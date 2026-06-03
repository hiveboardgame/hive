use super::model::{LastMoveDirection, PieceShadow};
use crate::{
    common::{resolve_piece_paint, OverlayPaint, PiecePaint, ShadowHref},
    providers::config::TileOptions,
};
use hive_lib::Piece;

#[derive(Clone, Debug, PartialEq)]
pub struct HivegroundPaint {
    tile_options: TileOptions,
}

impl HivegroundPaint {
    pub fn new(tile_options: &TileOptions) -> Self {
        Self {
            tile_options: tile_options.clone(),
        }
    }

    pub fn piece(&self, piece: Piece, shadow: PieceShadow) -> PiecePaint {
        let shadow_href = match shadow {
            PieceShadow::Design => ShadowHref::for_design(&self.tile_options.design),
            PieceShadow::None => ShadowHref::None,
        };
        resolve_piece_paint(piece, &self.tile_options, shadow_href)
    }

    pub fn active(&self) -> OverlayPaint {
        OverlayPaint::active(&self.tile_options)
    }

    pub fn target(&self) -> OverlayPaint {
        OverlayPaint::target(&self.tile_options)
    }

    pub fn last_move(&self, direction: LastMoveDirection) -> OverlayPaint {
        OverlayPaint::last_move(&self.tile_options, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::TileDesign;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    #[test]
    fn palette_paints_ordinary_piece_with_design_shadow() {
        let paint =
            HivegroundPaint::new(&TileOptions::default()).piece(piece("wQ"), PieceShadow::Design);

        assert_eq!(paint.shadow_href, ShadowHref::CommonDropShadow);
        assert_eq!(
            paint.tile_href.0,
            "/assets/tiles/official/official.svg#white"
        );
    }

    #[test]
    fn palette_uses_no_shadow_for_ghost_pieces() {
        let paint =
            HivegroundPaint::new(&TileOptions::default()).piece(piece("wQ"), PieceShadow::None);

        assert_eq!(paint.shadow_href, ShadowHref::None);
    }

    #[test]
    fn palette_paints_three_d_shadow_for_three_d_designs() {
        let tile_options = TileOptions {
            design: TileDesign::Carbon3D,
            ..TileOptions::default()
        };

        let paint = HivegroundPaint::new(&tile_options).piece(piece("wQ"), PieceShadow::Design);

        assert_eq!(paint.shadow_href, ShadowHref::ThreeDShadow);
    }
}
