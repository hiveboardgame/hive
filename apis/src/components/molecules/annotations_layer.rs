use crate::{
    common::SvgPos,
    providers::{
        analysis::AnalysisSignal,
        annotations::{AnnotationTool, AnnotationsSignal, MarkerShape},
        game_state::{GameStateSignal, View},
        Config,
    },
};
use hive_lib::{Position, State};
use leptos::prelude::*;

/// Markers/arrows are translucent so the piece below stays visible.
const SHAPE_OPACITY: &str = "0.8";

/// Draws the current position's annotations as inline SVG inside the board's
/// transform `<g>`. `pointer-events: none` so it never eats the board's handlers
/// (arrows are hit-tested by the board instead).
#[component]
pub fn AnnotationsLayer(
    annotations: AnnotationsSignal,
    history_state: Memo<State>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let config = expect_context::<Config>().0;
    let in_analysis = use_context::<AnalysisSignal>().is_some();
    let board_view = create_read_slice(game_state.signal, |gs| gs.view.clone());
    let last_turn = game_state.is_last_turn_as_signal();
    let current = annotations.current();

    let elements = move || {
        let set = current.get();
        let (straight, prefers_dark) = config.with(|c| (c.tile.is_three_d(), c.prefers_dark));
        // Scrubbing play history shows an earlier (possibly differently stacked)
        // board, so anchor marks to that board rather than the live one — matching
        // Board's piece-source choice.
        let show_history = board_view.get() == View::History && !last_turn.get() && !in_analysis;
        // Anchor to the top of the stack so marks track the visible piece.
        let center = move |position: Position| {
            let top_level = if show_history {
                history_state.with(|s| s.board.level(position).saturating_sub(1))
            } else {
                game_state
                    .signal
                    .with(|gs| gs.state.board.level(position).saturating_sub(1))
            };
            SvgPos::center_for_level(position, top_level, straight)
        };

        let mut views = Vec::new();

        for highlight in &set.highlights {
            let (cx, cy) = center(highlight.position);
            let path = rounded_hex_path(cx, cy, HIGHLIGHT_SIZE);
            let fill = highlight.color.fill();
            let stroke = highlight.color.stroke(prefers_dark);
            views.push(
                view! {
                    <path
                        d=path
                        fill=fill
                        fill-opacity="0.35"
                        stroke=stroke
                        stroke-width="2"
                        stroke-opacity="0.95"
                        stroke-linejoin="round"
                    />
                }
                .into_any(),
            );
        }

        for marker in &set.markers {
            let (cx, cy) = center(marker.position);
            views.push(marker_view(
                cx,
                cy,
                marker.shape,
                marker.color.fill(),
                marker.color.stroke(prefers_dark),
            ));
        }

        for arrow in &set.arrows {
            let (ax, ay) = center(arrow.from);
            let (bx, by) = center(arrow.to);
            if let Some(view) = arrow_view(
                ax,
                ay,
                bx,
                by,
                arrow.color.stroke(prefers_dark),
                SHAPE_OPACITY,
                false,
            ) {
                views.push(view);
            }
        }

        // Dashed live preview: an arrow once the drag leaves the start hex, else
        // the mark a release-in-place would leave.
        if let Some((from, to)) = annotations.preview.get() {
            let stroke = annotations.color.get().stroke(prefers_dark);
            if from != to {
                let (ax, ay) = center(from);
                let (bx, by) = center(to);
                if let Some(view) = arrow_view(ax, ay, bx, by, stroke, "0.5", true) {
                    views.push(view);
                }
            } else {
                let (cx, cy) = center(from);
                let fill = annotations.color.get().fill();
                let tool = annotations.tool.get();
                views.push(mark_preview_view(tool, cx, cy, fill, stroke));
            }
        }

        views
    };

    view! { <g pointer-events="none">{elements}</g> }
}

/// Inset from the full 30 so the highlight sits inside the tile border.
const HIGHLIGHT_SIZE: f32 = 26.0;

/// Pointy-top hexagon path centered at `(cx, cy)` with slightly rounded corners
/// to match the piece tiles' rounding.
fn rounded_hex_path(cx: f32, cy: f32, size: f32) -> String {
    let dx = 0.866_025_4 * size; // √3/2 · size
    let corners = [
        (cx + dx, cy - 0.5 * size),
        (cx + dx, cy + 0.5 * size),
        (cx, cy + size),
        (cx - dx, cy + 0.5 * size),
        (cx - dx, cy - 0.5 * size),
        (cx, cy - size),
    ];
    let r = size * 0.18; // corner radius — just a touch
    let n = corners.len();
    // Point `r` from `a` toward `b`, where each rounded corner begins/ends.
    let toward = |a: (f32, f32), b: (f32, f32)| {
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let len = vx.hypot(vy).max(f32::EPSILON);
        (a.0 + vx / len * r, a.1 + vy / len * r)
    };
    let mut d = String::new();
    for i in 0..n {
        let cur = corners[i];
        let p_in = toward(cur, corners[(i + n - 1) % n]);
        let p_out = toward(cur, corners[(i + 1) % n]);
        d.push_str(&format!(
            "{} {} {} Q {} {} {} {} ",
            if i == 0 { "M" } else { "L" },
            p_in.0,
            p_in.1,
            cur.0,
            cur.1,
            p_out.0,
            p_out.1
        ));
    }
    d.push('Z');
    d
}

fn marker_view(
    cx: f32,
    cy: f32,
    shape: MarkerShape,
    fill: &'static str,
    stroke: &'static str,
) -> AnyView {
    match shape {
        MarkerShape::Circle => view! {
            <circle
                cx=cx
                cy=cy
                r="15"
                fill=fill
                stroke=stroke
                stroke-width="3"
                opacity=SHAPE_OPACITY
            />
        }
        .into_any(),
        MarkerShape::Cross => {
            let d = 12.0;
            view! {
                <g stroke=stroke stroke-width="6" stroke-linecap="round" opacity=SHAPE_OPACITY>
                    <line x1=cx - d y1=cy - d x2=cx + d y2=cy + d />
                    <line x1=cx - d y1=cy + d x2=cx + d y2=cy - d />
                </g>
            }
            .into_any()
        }
    }
}

/// Dashed, faint preview of the mark a release-in-place would leave.
fn mark_preview_view(
    tool: AnnotationTool,
    cx: f32,
    cy: f32,
    fill: &'static str,
    stroke: &'static str,
) -> AnyView {
    match tool {
        AnnotationTool::Highlight => {
            let path = rounded_hex_path(cx, cy, HIGHLIGHT_SIZE);
            view! {
                <path
                    d=path
                    fill=fill
                    fill-opacity="0.18"
                    stroke=stroke
                    stroke-width="2"
                    stroke-opacity="0.8"
                    stroke-dasharray="6 5"
                    stroke-linejoin="round"
                />
            }
            .into_any()
        }
        AnnotationTool::Marker(MarkerShape::Circle) => view! {
            <circle
                cx=cx
                cy=cy
                r="15"
                fill=fill
                stroke=stroke
                stroke-width="3"
                stroke-dasharray="5 4"
                opacity="0.5"
            />
        }
        .into_any(),
        AnnotationTool::Marker(MarkerShape::Cross) => {
            let d = 12.0;
            view! {
                <g
                    stroke=stroke
                    stroke-width="6"
                    stroke-linecap="round"
                    stroke-dasharray="5 5"
                    opacity="0.5"
                >
                    <line x1=cx - d y1=cy - d x2=cx + d y2=cy + d />
                    <line x1=cx - d y1=cy + d x2=cx + d y2=cy - d />
                </g>
            }
            .into_any()
        }
    }
}

/// Arrow `(ax,ay)` → `(bx,by)`, tip on the target. `dashed` = provisional
/// preview. `None` if degenerate.
fn arrow_view(
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    stroke: &'static str,
    opacity: &'static str,
    dashed: bool,
) -> Option<AnyView> {
    let dx = bx - ax;
    let dy = by - ay;
    let len = dx.hypot(dy);
    if len < 1.0 {
        return None;
    }
    let (ux, uy) = (dx / len, dy / len);
    let head_len = 22.0;
    let head_w = 16.0;
    let tip = (bx, by);
    let base = (tip.0 - ux * head_len, tip.1 - uy * head_len);
    // Perpendicular unit vector for the head's base corners.
    let (px, py) = (-uy, ux);
    let left = (base.0 + px * head_w / 2.0, base.1 + py * head_w / 2.0);
    let right = (base.0 - px * head_w / 2.0, base.1 - py * head_w / 2.0);
    let head_points = format!(
        "{},{} {},{} {},{}",
        tip.0, tip.1, left.0, left.1, right.0, right.1
    );
    let dash = if dashed { "8 6" } else { "none" };

    Some(
        view! {
            <g opacity=opacity>
                <line
                    x1=ax
                    y1=ay
                    x2=base.0
                    y2=base.1
                    stroke=stroke
                    stroke-width="7"
                    stroke-linecap="round"
                    stroke-dasharray=dash
                />
                <polygon points=head_points fill=stroke stroke=stroke stroke-width="1" />
            </g>
        }
        .into_any(),
    )
}
