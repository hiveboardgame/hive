//! The outlines of the tile artwork, so a ghost's coloured border can follow the tile exactly.
//! Paths are copied from `assets/tiles`, in the artwork's own coordinates.

use crate::common::TileDesign;

pub struct TileOutline {
    pub paths: &'static [&'static str],
    pub transform: &'static str,
    /// How much `transform` scales, to give a stroke a width in artwork units.
    pub scale: f32,
}

/// Every flat design shares flat.svg's outline (official was 1.7% shorter until it was
/// brought in line).
const FLAT: TileOutline = TileOutline {
    paths: &["M0 0a6.9 6.9 0 0 1 3.592 1.708 7 7 0 0 1 1.342 1.671c.326.572 11.761 19.972 12.074 20.942a6.85 6.85 0 0 1 .105 3.877 7 7 0 0 1-.674 1.657c-.283.496-11.474 20.355-12.078 21.052a6.87 6.87 0 0 1-5.197 2.385c-.607 0-23.293.147-24.033.032a6.88 6.88 0 0 1-4.918-3.376c-.324-.569-11.76-19.973-12.072-20.929a6.9 6.9 0 0 1 .564-5.557c.285-.498 11.469-20.347 12.078-21.05a6.87 6.87 0 0 1 3.274-2.1 6.8 6.8 0 0 1 1.916-.278C-23.422.034-.73-.112 0 0"],
    transform: "matrix(1.42938 -.83963 -.82525 -1.45429 83.936 78.34)",
    scale: 1.665,
};

/// The 3d tile is its top face (path55) plus its side (path54) from 3d.svg; carbon-3d is the
/// same shape.
const THREE_D: TileOutline = TileOutline {
    paths: &[
        "M271.3 72.1q-2.65-2.9-7.35-5.8-27.55-16.5-54.35-32Q156.85 3.05 153.95 2q-9.7-3.4-20.5-1.05-5.75 1.3-9.45 3.7-6.3 3.4-54.85 30.95Q16.4 65.8 14.3 67.35q-8.15 6.6-11.85 17.1Q.9 89.7.9 95.45q-.25 18.75-.3 34.7L0 214.2q.1 7.1.35 8.3 1.196 4.876 3.3 9 2.556 5.006 6.45 8.9 2.6 2.85 7.35 5.75 105.5 62.75 110 64.3 10.5 3.7 20.45 1.05 5.5-1.3 9.45-3.65 106.6-60.1 109.5-62.5l.25-.25q8.15-6.55 11.8-17.05 1.6-5 1.6-11.05.5-32.3.8-63 .05-3.9.15-7.6v-26.7q.1-27.65-.45-29.75-2.6-10.75-9.7-17.85",
        "M281.45 146.4q-.1 3.7-.15 7.6-.3 30.7-.8 63 0 6.05-1.6 11.05-3.65 10.5-11.8 17.05l-.25.25q-2.9 2.4-109.5 62.5-3.95 2.35-9.45 3.65-9.95 2.65-20.45-1.05-4.5-1.55-110-64.3-4.75-2.9-7.35-5.75-3.894-3.894-6.45-8.9-2.104-4.124-3.3-9-.25-1.2-.35-8.3-.1 25.5.35 27.75 1.196 4.99 3.3 9.15 2.556 5.083 6.45 8.95 3.4 3.45 7.35 5.8 105.5 62.75 110 64.3 3.15 1.1 6.4 1.6 5.35.8 11.05 0 1.5-.2 3-.55 4.2-1.05 9.45-3.65 106.6-60.15 109.5-62.75h.25q8.15-6.55 11.8-17.05 1.6-5.55 1.6-11.3.5-32.05.8-62.75.15-15.5.15-27.3",
    ],
    transform: "translate(.016 .003)scale(.31379)",
    scale: 0.31379,
};

pub fn tile_outline(design: &TileDesign) -> &'static TileOutline {
    match design {
        TileDesign::ThreeD | TileDesign::Carbon3D => &THREE_D,
        _ => &FLAT,
    }
}
