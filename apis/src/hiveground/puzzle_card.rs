use super::{
    build_static_render_model,
    render::preview::{piece_glyph, three_d_options, DEFS, TILE_H, TILE_W},
    PieceShadow,
    RenderLayerKind,
};
use crate::common::SvgPos;
use hive_lib::{Bug, Color, Piece, State};
use resvg::{tiny_skia, usvg};
use std::{
    str::FromStr,
    sync::{Arc, LazyLock},
};

const WIDTH: f32 = 1200.0;
const HEIGHT: f32 = 630.0;
const BOARD_RIGHT: f32 = 680.0;
const BOARD_PADDING: f32 = 36.0;
const MAX_BOARD_SCALE: f32 = 2.4;
const TEXT_LEFT: f32 = 724.0;
const TEAL: &str = "#40afa1";
const AMBER: &str = "#e9ac43";
const RED: &str = "#d61a35";
const GREEN: &str = "#3cb043";
const MUTED: &str = "#9fb0b6";
const RESERVE_ORDER: [Bug; 8] = [
    Bug::Queen,
    Bug::Beetle,
    Bug::Grasshopper,
    Bug::Spider,
    Bug::Ant,
    Bug::Mosquito,
    Bug::Ladybug,
    Bug::Pillbug,
];

static FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut fonts = usvg::fontdb::Database::new();
    fonts.load_font_data(include_bytes!("../../og-fonts/Inter-Medium.ttf").to_vec());
    fonts.load_font_data(include_bytes!("../../og-fonts/Inter-ExtraBold.ttf").to_vec());
    Arc::new(fonts)
});

static LOGO: LazyLock<Option<tiny_skia::Pixmap>> = LazyLock::new(|| {
    tiny_skia::Pixmap::decode_png(include_bytes!("../../assets/inline_flat_dark.png")).ok()
});

pub struct PuzzleCard {
    pub label: String,
    pub date: Option<String>,
    pub solver_moves: i32,
    pub facts: Vec<String>,
}

pub fn puzzle_card_png(state: &State, card: &PuzzleCard) -> anyhow::Result<Vec<u8>> {
    let svg = puzzle_card_svg(state, card);
    let options = usvg::Options {
        fontdb: FONTS.clone(),
        font_family: "Inter".to_string(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(&svg, &options)?;
    let mut pixmap = tiny_skia::Pixmap::new(WIDTH as u32, HEIGHT as u32)
        .ok_or_else(|| anyhow::anyhow!("failed to allocate pixmap"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    if let Some(logo) = LOGO.as_ref() {
        let scale = 58.0 / logo.height() as f32;
        pixmap.draw_pixmap(
            0,
            0,
            logo.as_ref(),
            &tiny_skia::PixmapPaint {
                quality: tiny_skia::FilterQuality::Bicubic,
                ..Default::default()
            },
            tiny_skia::Transform::from_scale(scale, scale).post_translate(TEXT_LEFT - 6.0, 536.0),
            None,
        );
    }
    Ok(pixmap.encode_png()?)
}

fn puzzle_card_svg(state: &State, card: &PuzzleCard) -> String {
    let solver = state.turn_color;
    let side = match solver {
        Color::White => "White",
        Color::Black => "Black",
    };
    let moves = if card.solver_moves == 1 {
        "in 1 move".to_string()
    } else {
        format!("in {} moves", card.solver_moves)
    };
    let mut text = String::new();
    text.push_str(&format!(
        "<text x=\"{TEXT_LEFT}\" y=\"112\" font-size=\"22\" font-weight=\"800\" \
         letter-spacing=\"4\" fill=\"{TEAL}\">{}</text>",
        escape(&card.label.to_uppercase())
    ));
    if let Some(date) = &card.date {
        text.push_str(&format!(
            "<text x=\"{TEXT_LEFT}\" y=\"146\" font-size=\"22\" font-weight=\"500\" \
             fill=\"{MUTED}\">{}</text>",
            escape(date)
        ));
    }
    text.push_str(&format!(
        "<text x=\"{TEXT_LEFT}\" y=\"228\" font-size=\"60\" font-weight=\"800\" \
         fill=\"#ffffff\">{side} to win</text>\
         <text x=\"{TEXT_LEFT}\" y=\"298\" font-size=\"60\" font-weight=\"800\" \
         fill=\"{AMBER}\">{moves}</text>"
    ));
    let hand = reserve_row(state, solver);
    if !hand.is_empty() {
        text.push_str(&format!(
            "<text x=\"{TEXT_LEFT}\" y=\"368\" font-size=\"18\" font-weight=\"800\" \
         letter-spacing=\"3\" fill=\"{MUTED}\">{} IN HAND</text>",
            side.to_uppercase()
        ));
        text.push_str(&hand);
    }
    let chip_y = if hand.is_empty() { 352.0 } else { 478.0 };
    let mut chip_x = TEXT_LEFT;
    for fact in &card.facts {
        let width = fact.chars().count() as f32 * 11.0 + 36.0;
        text.push_str(&format!(
            "<rect x=\"{chip_x}\" y=\"{chip_y}\" width=\"{width}\" height=\"40\" rx=\"20\" \
             fill=\"#ffffff\" fill-opacity=\"0.08\" stroke=\"#ffffff\" stroke-opacity=\"0.18\"/>\
             <text x=\"{}\" y=\"{}\" font-size=\"20\" font-weight=\"500\" fill=\"#e6edf0\" \
             text-anchor=\"middle\">{}</text>",
            chip_x + width / 2.0,
            chip_y + 27.0,
            escape(fact)
        ));
        chip_x += width + 12.0;
    }
    text.push_str(&format!(
        "<text x=\"1160\" y=\"580\" font-size=\"24\" font-weight=\"800\" fill=\"#ffffff\" \
         text-anchor=\"end\">Can you find it?</text>"
    ));

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"{WIDTH}\" height=\"{HEIGHT}\" viewBox=\"0 0 {WIDTH} {HEIGHT}\">\
         <defs>\
           <linearGradient id=\"bg\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">\
             <stop offset=\"0\" stop-color=\"#2c383d\"/><stop offset=\"1\" stop-color=\"#151b1e\"/>\
           </linearGradient>\
           <radialGradient id=\"glow\"><stop offset=\"0\" stop-color=\"{TEAL}\" stop-opacity=\"0.22\"/>\
             <stop offset=\"1\" stop-color=\"{TEAL}\" stop-opacity=\"0\"/></radialGradient>\
           <radialGradient id=\"target\"><stop offset=\"0.45\" stop-color=\"{RED}\" stop-opacity=\"0.9\"/>\
             <stop offset=\"1\" stop-color=\"{RED}\" stop-opacity=\"0\"/></radialGradient>\
           <radialGradient id=\"own\"><stop offset=\"0.45\" stop-color=\"{GREEN}\" stop-opacity=\"0.85\"/>\
             <stop offset=\"1\" stop-color=\"{GREEN}\" stop-opacity=\"0\"/></radialGradient>\
           <pattern id=\"comb\" width=\"56\" height=\"97\" patternUnits=\"userSpaceOnUse\">\
             <path d=\"M28 0L56 16.2V48.5L28 64.7L0 48.5V16.2ZM28 64.7V97\" fill=\"none\" \
               stroke=\"#ffffff\" stroke-opacity=\"0.045\" stroke-width=\"1.5\"/>\
           </pattern>\
         </defs>\
         {defs}\
         <rect width=\"{WIDTH}\" height=\"{HEIGHT}\" fill=\"url(#bg)\"/>\
         <rect width=\"{WIDTH}\" height=\"{HEIGHT}\" fill=\"url(#comb)\"/>\
         <ellipse cx=\"{glow_x}\" cy=\"{glow_y}\" rx=\"360\" ry=\"300\" fill=\"url(#glow)\"/>\
         {board}\
         <rect x=\"{BOARD_RIGHT}\" width=\"{panel_w}\" height=\"{HEIGHT}\" fill=\"#000000\" fill-opacity=\"0.24\"/>\
         <rect x=\"{BOARD_RIGHT}\" width=\"4\" height=\"{HEIGHT}\" fill=\"{TEAL}\"/>\
         {text}</svg>",
        defs = *DEFS,
        glow_x = BOARD_RIGHT / 2.0,
        glow_y = HEIGHT / 2.0,
        board = board_group(state, solver.opposite_color()),
        panel_w = WIDTH - BOARD_RIGHT,
    )
}

fn board_group(state: &State, target: Color) -> String {
    let model = build_static_render_model(&state.board);
    let tiles = three_d_options();
    let mut glyphs = String::new();
    let mut queens = String::new();
    let last_moved = state
        .history
        .moves
        .last()
        .and_then(|(piece, _)| Piece::from_str(piece).ok());
    let mut last = None;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for stack in &model.stacks {
        for layer in &stack.layers {
            let RenderLayerKind::Piece { piece, shadow, .. } = &layer.kind else {
                continue;
            };
            let (cx, cy) = SvgPos::center_for_level(stack.position, layer.level, true);
            min_x = min_x.min(cx);
            min_y = min_y.min(cy);
            max_x = max_x.max(cx);
            max_y = max_y.max(cy);
            if piece.bug() == Bug::Queen && layer.level == 0 {
                let gradient = if piece.color() == target {
                    "target"
                } else {
                    "own"
                };
                queens.push_str(&format!(
                    "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"48\" fill=\"url(#{gradient})\"/>"
                ));
            }
            if Some(*piece) == last_moved {
                last = Some((cx, cy));
            }
            glyphs.push_str(&piece_glyph(
                *piece,
                *shadow == PieceShadow::Design,
                &tiles,
                cx,
                cy,
            ));
        }
    }
    if min_x > max_x {
        return String::new();
    }
    let content_w = (max_x - min_x) + TILE_W;
    let content_h = (max_y - min_y) + TILE_H;
    let scale = ((BOARD_RIGHT - 2.0 * BOARD_PADDING) / content_w)
        .min((HEIGHT - 2.0 * BOARD_PADDING) / content_h)
        .min(MAX_BOARD_SCALE);
    let (center_x, center_y) = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
    let last = last
        .map(|(x, y)| {
            format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{AMBER}\" stroke-width=\"3\" \
                 stroke-dasharray=\"7 5\" stroke-linejoin=\"round\"/>",
                hexagon(x, y, 31.0)
            )
        })
        .unwrap_or_default();
    format!(
        "<g transform=\"translate({:.2},{:.2}) scale({scale:.4}) translate({:.2},{:.2})\">\
         {queens}{glyphs}{last}</g>",
        BOARD_RIGHT / 2.0,
        HEIGHT / 2.0,
        -center_x,
        -center_y,
    )
}

fn reserve_row(state: &State, color: Color) -> String {
    let reserve = state.board.reserve(color, state.game_type);
    let tiles = three_d_options();
    let mut row = String::new();
    let mut x = TEXT_LEFT + TILE_W * 0.65;
    let y = 424.0;
    let scale = 1.3;
    for bug in RESERVE_ORDER {
        let Some(pieces) = reserve.get(&bug) else {
            continue;
        };
        let Some(piece) = pieces
            .iter()
            .filter_map(|piece| Piece::from_str(piece).ok())
            .min_by_key(|piece| piece.order())
        else {
            continue;
        };
        row.push_str(&format!(
            "<g transform=\"translate({x:.2},{y:.2}) scale({scale})\">{}</g>",
            piece_glyph(piece, false, &tiles, 0.0, 0.0)
        ));
        if pieces.len() > 1 {
            row.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"12\" fill=\"#151b1e\" stroke=\"#ffffff\" \
                 stroke-opacity=\"0.6\" stroke-width=\"1.5\"/>\
                 <text x=\"{:.2}\" y=\"{:.2}\" font-size=\"15\" font-weight=\"800\" fill=\"#ffffff\" \
                 text-anchor=\"middle\">{}</text>",
                x + 24.0,
                y - 28.0,
                x + 24.0,
                y - 22.8,
                pieces.len()
            ));
        }
        x += TILE_W * scale + 8.0;
    }
    row
}

fn hexagon(x: f32, y: f32, radius: f32) -> String {
    let mut path = String::new();
    for corner in 0..6 {
        let angle = std::f32::consts::FRAC_PI_3 * corner as f32 - std::f32::consts::FRAC_PI_2;
        let (px, py) = (x + radius * angle.cos(), y + radius * angle.sin());
        path.push_str(&format!(
            "{}{px:.2} {py:.2}",
            if corner == 0 { "M" } else { "L" }
        ));
    }
    path.push('Z');
    path
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_lib::History;

    #[test]
    fn renders_a_card() {
        let pgn = include_str!("../../../engine/test_pgns/valid/p_game.pgn");
        let history = History::from_pgn_str(pgn).expect("parse test pgn");
        let state = State::new_from_history(&history).expect("valid state");
        let card = PuzzleCard {
            label: "Puzzle of the day".to_string(),
            date: Some("Sunday 12 October".to_string()),
            solver_moves: 2,
            facts: vec!["Difficulty 1450".to_string()],
        };
        let png = puzzle_card_png(&state, &card).expect("render png");
        assert!(png.len() > 8 && &png[1..4] == b"PNG");
        if let Ok(out) = std::env::var("PUZZLE_CARD_OUT") {
            std::fs::write(out, &png).expect("write card");
        }
    }
}
