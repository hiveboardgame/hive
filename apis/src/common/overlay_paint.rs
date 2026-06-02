use crate::{hiveground::LastMoveDirection, providers::config::TileOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct OverlayPaint {
    pub straight: bool,
    pub href: &'static str,
}

impl OverlayPaint {
    pub fn active(tile_options: &TileOptions) -> Self {
        Self {
            straight: tile_options.is_three_d(),
            href: "/assets/tiles/common/all.svg#active",
        }
    }

    pub fn target(tile_options: &TileOptions) -> Self {
        Self {
            straight: tile_options.is_three_d(),
            href: "/assets/tiles/common/all.svg#target",
        }
    }

    pub fn last_move(tile_options: &TileOptions, direction: LastMoveDirection) -> Self {
        let straight = tile_options.is_three_d();
        let href = match direction {
            LastMoveDirection::To if straight => "/assets/tiles/3d/last_move_to.svg#last_move_to",
            LastMoveDirection::To => "/assets/tiles/common/all.svg#last_move_to",
            LastMoveDirection::From => "/assets/tiles/common/all.svg#last_move_from",
        };
        Self { straight, href }
    }
}
