use crate::{hiveground::LastMoveDirection, providers::config::TileOptions};

// Sprite URLs carry `?v=` because assets are served without Cache-Control, so browsers keep
// stale copies; bump it here and in piece_paint.rs whenever a sprite file changes.
const LAST_MOVE_COLOR: &str = "#d33682";
const PREMOVE_COLOR: &str = "#6c71c4";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct OverlayPaint {
    pub straight: bool,
    pub href: &'static str,
    pub color: Option<&'static str>,
}

impl OverlayPaint {
    pub fn active(tile_options: &TileOptions) -> Self {
        Self {
            straight: tile_options.is_three_d(),
            href: "/assets/tiles/common/all.svg?v=3#active",
            color: None,
        }
    }

    pub fn target(tile_options: &TileOptions) -> Self {
        Self {
            straight: tile_options.is_three_d(),
            href: "/assets/tiles/common/all.svg?v=3#target",
            color: None,
        }
    }

    pub fn vacate_target(tile_options: &TileOptions) -> Self {
        Self {
            straight: tile_options.is_three_d(),
            href: "/assets/tiles/common/all.svg?v=3#target_vacate",
            color: None,
        }
    }

    pub fn last_move(tile_options: &TileOptions, direction: LastMoveDirection) -> Self {
        Self::move_marker(tile_options, direction, LAST_MOVE_COLOR)
    }

    pub fn premove(tile_options: &TileOptions, direction: LastMoveDirection) -> Self {
        Self::move_marker(tile_options, direction, PREMOVE_COLOR)
    }

    fn move_marker(
        tile_options: &TileOptions,
        direction: LastMoveDirection,
        color: &'static str,
    ) -> Self {
        let straight = tile_options.is_three_d();
        let href = match direction {
            LastMoveDirection::To if straight => {
                "/assets/tiles/3d/last_move_to.svg?v=2#last_move_to"
            }
            LastMoveDirection::To => "/assets/tiles/common/all.svg?v=3#last_move_to",
            LastMoveDirection::From => "/assets/tiles/common/all.svg?v=3#last_move_from",
        };
        Self {
            straight,
            href,
            color: Some(color),
        }
    }
}
