use super::model::{HivegroundRenderModel, RenderLayer};
use hive_lib::Position;
use leptos::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "ssr")]
pub use preview::{board_to_png, board_to_svg, PreviewOpts};

/// SSR previews stay visually aligned with the live board without a Leptos
/// runtime.
#[cfg(feature = "ssr")]
mod preview {
    use crate::{
        common::{resolve_piece_paint, ShadowHref, SvgPos, TileDesign},
        hiveground::{build_static_render_model, PieceShadow, RenderLayerKind},
        providers::config::TileOptions,
    };
    use hive_lib::Board;
    use std::sync::LazyLock;

    pub struct PreviewOpts {
        pub width: u32,
        pub height: u32,
        pub background: String,
    }

    impl Default for PreviewOpts {
        fn default() -> Self {
            Self {
                width: 1200,
                height: 630,
                background: "#47545a".to_string(),
            }
        }
    }

    // Match `PieceGlyph` so previews do not drift from the live board.
    const GLYPH_TRANSFORM: &str = "scale(0.56, 0.56) translate(-45, -50)";
    const TILE_W: f32 = 88.337 * 0.56;
    const TILE_H: f32 = 104.229 * 0.56;
    const PADDING: f32 = 48.0;
    // Tiny openings look noisy if a single tile fills the card.
    const MAX_SCALE: f32 = 1.8;

    fn three_d_options() -> TileOptions {
        TileOptions {
            design: TileDesign::ThreeD,
            ..TileOptions::default()
        }
    }

    /// resvg will not fetch external tile assets, so inline namespaced defs.
    static DEFS: LazyLock<String> = LazyLock::new(|| {
        let mut defs = String::from("<defs>");
        defs.push_str(&namespace_ids(
            include_str!("../../assets/tiles/3d/3d.svg"),
            "t3_",
        ));
        defs.push_str(&namespace_ids(
            include_str!("../../assets/tiles/common/all.svg"),
            "tc_",
        ));
        defs.push_str("</defs>");
        defs
    });

    // usvg reparses inlined defs on each render; HTTP-level PNG caching absorbs
    // repeats, and pre-rasterized glyphs can wait until cold renders hurt.
    pub fn board_to_png(board: &Board, opts: &PreviewOpts) -> anyhow::Result<Vec<u8>> {
        let svg = board_to_svg(board, opts);
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(opts.width, opts.height)
            .ok_or_else(|| anyhow::anyhow!("failed to allocate pixmap"))?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        Ok(pixmap.encode_png()?)
    }

    pub fn board_to_svg(board: &Board, opts: &PreviewOpts) -> String {
        let model = build_static_render_model(board);
        let tiles = three_d_options();

        // Bounds keep centering independent of DOM layout.
        let mut glyphs = String::new();
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for stack in &model.stacks {
            for layer in &stack.layers {
                let RenderLayerKind::Piece { piece, shadow, .. } = &layer.kind else {
                    continue; // OG cards should show only the static board.
                };
                let (cx, cy) = SvgPos::center_for_level(stack.position, layer.level, true);
                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);
                let design_shadow = *shadow == PieceShadow::Design;
                glyphs.push_str(&piece_glyph(*piece, design_shadow, &tiles, cx, cy));
            }
        }

        let (w, h) = (opts.width as f32, opts.height as f32);
        let wrapper = if min_x <= max_x {
            let content_w = (max_x - min_x) + TILE_W;
            let content_h = (max_y - min_y) + TILE_H;
            let scale = ((w - 2.0 * PADDING) / content_w)
                .min((h - 2.0 * PADDING) / content_h)
                .min(MAX_SCALE);
            let (bcx, bcy) = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
            format!(
                "<g transform=\"translate({:.2},{:.2}) scale({:.4}) translate({:.2},{:.2})\">",
                w / 2.0,
                h / 2.0,
                scale,
                -bcx,
                -bcy,
            )
        } else {
            "<g>".to_string()
        };

        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" \
             xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" \
             viewBox=\"0 0 {w} {h}\">\
             <rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>\
             {defs}{wrapper}{glyphs}</g></svg>",
            bg = opts.background,
            defs = *DEFS,
        )
    }

    fn piece_glyph(
        piece: hive_lib::Piece,
        design_shadow: bool,
        tiles: &TileOptions,
        cx: f32,
        cy: f32,
    ) -> String {
        // The 3D theme's shadow symbol is not useful outside the browser.
        let shadow_href = if design_shadow {
            ShadowHref::for_design(&tiles.design)
        } else {
            ShadowHref::None
        };
        let paint = resolve_piece_paint(piece, tiles, shadow_href);
        let tile = local_ref(&paint.tile_href.0);
        let bug = local_ref(&paint.bug_href.0);
        let rotate = paint
            .rotation
            .map(|deg| format!("rotate({deg})"))
            .unwrap_or_default();
        let dots = paint
            .dots_href
            .as_ref()
            .map(|d| {
                format!(
                    "<use href=\"{}\" fill=\"{}\"></use>",
                    local_ref(&d.0),
                    paint.dot_color
                )
            })
            .unwrap_or_default();
        let pos = format!("translate({cx:.2},{cy:.2})");
        format!(
            "<g>\
               <g transform=\"{pos}\"><g transform=\"{GLYPH_TRANSFORM}\"><use href=\"{tile}\"></use></g></g>\
               <g transform=\"{pos}\"><g transform=\"{rotate}\"><g transform=\"{GLYPH_TRANSFORM}\">\
                 <use href=\"{bug}\"></use>{dots}\
               </g></g></g>\
             </g>"
        )
    }

    /// resvg needs local fragments for inlined asset symbols.
    fn local_ref(href: &str) -> String {
        let frag = href.rsplit('#').next().unwrap_or_default();
        let prefix = if href.contains("/3d/3d.svg") {
            "t3_"
        } else if href.contains("/common/all.svg") {
            "tc_"
        } else {
            "" // unused shadow refs can remain unresolved.
        };
        format!("#{prefix}{frag}")
    }

    /// Namespacing lets multiple SVG files share common ids like `a`.
    fn namespace_ids(svg: &str, prefix: &str) -> String {
        let start = svg.find('>').map(|i| i + 1).unwrap_or(0);
        let end = svg.rfind("</svg>").unwrap_or(svg.len());
        let mut content = svg[start..end].to_string();
        for id in collect_ids(&content) {
            // Delimited replacements keep `a` from corrupting ids like `a1`.
            content = content
                .replace(&format!("id=\"{id}\""), &format!("id=\"{prefix}{id}\""))
                .replace(
                    &format!("href=\"#{id}\""),
                    &format!("href=\"#{prefix}{id}\""),
                )
                .replace(&format!("url(#{id})"), &format!("url(#{prefix}{id})"));
        }
        content
    }

    fn collect_ids(s: &str) -> Vec<String> {
        let mut ids = Vec::new();
        let mut rest = s;
        while let Some(i) = rest.find("id=\"") {
            let after = &rest[i + 4..];
            if let Some(j) = after.find('"') {
                ids.push(after[..j].to_string());
                rest = &after[j + 1..];
            } else {
                break;
            }
        }
        ids.sort();
        ids.dedup();
        ids
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use hive_lib::{History, State};

        #[test]
        fn renders_png_for_a_real_game() {
            let pgn = include_str!("../../../engine/test_pgns/valid/p_game.pgn");
            let history = History::from_pgn_str(pgn.to_string()).expect("parse test pgn");
            let state = State::new_from_history(&history).expect("valid state");
            let png = board_to_png(&state.board, &PreviewOpts::default()).expect("render png");
            assert!(png.len() > 8 && &png[1..4] == b"PNG", "valid PNG header");
        }
    }
}

type LayersByPosition = HashMap<Position, Vec<RenderLayer>>;

pub fn layers_by_position(model: Memo<HivegroundRenderModel>) -> Memo<LayersByPosition> {
    Memo::new(move |_| {
        model.with(|model| {
            model
                .stacks
                .iter()
                .map(|stack| (stack.position, stack.layers.clone()))
                .collect()
        })
    })
}

pub fn layers_for_position(
    layers_by_position: Memo<LayersByPosition>,
    position: Position,
) -> Signal<Vec<RenderLayer>> {
    Signal::derive(move || {
        layers_by_position.with(|layers| layers.get(&position).cloned().unwrap_or_default())
    })
}

pub fn stack_positions(model: Memo<HivegroundRenderModel>) -> Signal<Vec<Position>> {
    Signal::derive(move || {
        model.with(|model| model.stacks.iter().map(|stack| stack.position).collect())
    })
}
