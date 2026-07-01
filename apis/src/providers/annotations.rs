use crate::providers::{
    analysis::AnalysisSignal,
    game_state::{GameStateSignal, View},
};
use hudsoni::Position;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use shared_types::GameId;
use std::collections::HashMap;

/// White/Black mark each side, with a contrasting outline so they stay visible
/// on either board theme; red/green are high-contrast accents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationColor {
    White,
    Black,
    Red,
    Green,
}

impl AnnotationColor {
    pub fn fill(self) -> &'static str {
        match self {
            AnnotationColor::White => "#efe9dd",
            AnnotationColor::Black => "#2a323e",
            AnnotationColor::Red => "#d61a35",
            AnnotationColor::Green => "#3f9b3a",
        }
    }

    /// Outline/ink color. White is mode-aware — off-white on the dark board,
    /// mid-gray on the light one where pure white would vanish; green brightens
    /// on the dark board where the base tone reads too close to it.
    pub fn stroke(self, prefers_dark: bool) -> &'static str {
        match self {
            AnnotationColor::White => {
                if prefers_dark {
                    "#f2efe9"
                } else {
                    "#737880"
                }
            }
            AnnotationColor::Black => "#23262d",
            AnnotationColor::Red => "#d61a35",
            AnnotationColor::Green => {
                if prefers_dark {
                    "#5cc451"
                } else {
                    "#3f9b3a"
                }
            }
        }
    }

    pub fn all() -> [AnnotationColor; 4] {
        [
            AnnotationColor::White,
            AnnotationColor::Black,
            AnnotationColor::Red,
            AnnotationColor::Green,
        ]
    }

    /// lichess-style: a held modifier picks the color while drawing. Shift is
    /// avoided (it pops the native menu on Linux). `meta` is Cmd/Win/Super.
    pub fn from_modifiers(ctrl: bool, alt: bool, meta: bool) -> Option<Self> {
        match (ctrl, alt, meta) {
            (true, false, false) => Some(AnnotationColor::White),
            (false, true, false) => Some(AnnotationColor::Black),
            (true, true, false) => Some(AnnotationColor::Red),
            (false, false, true) => Some(AnnotationColor::Green),
            _ => None,
        }
    }
}

/// Point-marker shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerShape {
    Circle,
    Cross,
}

impl MarkerShape {
    pub fn all() -> [MarkerShape; 2] {
        [MarkerShape::Circle, MarkerShape::Cross]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Highlight {
    pub position: Position,
    pub color: AnnotationColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub position: Position,
    pub shape: MarkerShape,
    pub color: AnnotationColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrow {
    pub from: Position,
    pub to: Position,
    pub color: AnnotationColor,
}

/// All annotations attached to a single board position (analysis node / play turn).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationSet {
    pub highlights: Vec<Highlight>,
    pub markers: Vec<Marker>,
    pub arrows: Vec<Arrow>,
}

impl AnnotationSet {
    pub fn is_empty(&self) -> bool {
        self.highlights.is_empty() && self.markers.is_empty() && self.arrows.is_empty()
    }

    fn clear_point(&mut self, position: Position) {
        self.highlights.retain(|h| h.position != position);
        self.markers.retain(|m| m.position != position);
    }

    /// Redrawing the identical mark removes it; a different color/shape at the
    /// same hex replaces it.
    fn tap(&mut self, position: Position, tool: AnnotationTool, color: AnnotationColor) {
        match tool {
            AnnotationTool::Highlight => {
                if let Some(index) = self
                    .highlights
                    .iter()
                    .position(|h| h.position == position && h.color == color)
                {
                    self.highlights.remove(index);
                    return;
                }
                self.clear_point(position);
                self.highlights.push(Highlight { position, color });
            }
            AnnotationTool::Marker(shape) => {
                if let Some(index) = self
                    .markers
                    .iter()
                    .position(|m| m.position == position && m.shape == shape && m.color == color)
                {
                    self.markers.remove(index);
                    return;
                }
                self.clear_point(position);
                self.markers.push(Marker {
                    position,
                    shape,
                    color,
                });
            }
        }
    }

    /// Redrawing the identical arrow removes it; the same endpoints in a
    /// different color recolor it.
    fn add_arrow(&mut self, from: Position, to: Position, color: AnnotationColor) {
        if let Some(index) = self
            .arrows
            .iter()
            .position(|a| a.from == from && a.to == to)
        {
            if self.arrows[index].color == color {
                self.arrows.remove(index);
            } else {
                self.arrows[index].color = color;
            }
        } else {
            self.arrows.push(Arrow { from, to, color });
        }
    }
}

/// The tool applied on a tap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationTool {
    Highlight,
    Marker(MarkerShape),
}

/// Where annotations live: analysis in the `AnalysisTree` (so they save/load),
/// play in-memory keyed by the viewed turn (private, local-only).
#[derive(Clone, Copy)]
enum AnnotationBackend {
    Analysis(AnalysisSignal),
    Play {
        game_state: GameStateSignal,
        store: RwSignal<HashMap<(Option<GameId>, usize), AnnotationSet>>,
    },
}

impl AnnotationBackend {
    fn read_current(&self) -> AnnotationSet {
        match self {
            AnnotationBackend::Analysis(analysis) => analysis.tree.with(|tree| {
                tree.annotations
                    .get(&tree.current_annotation_key())
                    .cloned()
                    .unwrap_or_default()
            }),
            AnnotationBackend::Play { game_state, store } => {
                let key = play_key(*game_state);
                store.with(|map| map.get(&key).cloned().unwrap_or_default())
            }
        }
    }

    fn update_current(&self, mutate: impl FnOnce(&mut AnnotationSet)) {
        match self {
            AnnotationBackend::Analysis(analysis) => analysis.tree.update(|tree| {
                let key = tree.current_annotation_key();
                let mut set = tree.annotations.remove(&key).unwrap_or_default();
                mutate(&mut set);
                if !set.is_empty() {
                    tree.annotations.insert(key, set);
                }
            }),
            AnnotationBackend::Play { game_state, store } => {
                let key = play_key(*game_state);
                store.update(|map| {
                    let mut set = map.remove(&key).unwrap_or_default();
                    mutate(&mut set);
                    if !set.is_empty() {
                        map.insert(key, set);
                    }
                });
            }
        }
    }
}

/// Play-view key: which game, and a canonical ply count for the board on screen.
/// The ply count is identical whether the board is reached live or by scrubbing
/// history — `history_turn` is a 0-based ply index (`None` = initial position),
/// while `state.turn` is the move count. The game id keeps annotations from
/// leaking across the in-place `/game/:nanoid` route swap.
fn play_key(game_state: GameStateSignal) -> (Option<GameId>, usize) {
    game_state.signal.with(|gs| {
        let turn = match gs.view {
            View::History => gs.history_turn.map_or(0, |t| t + 1),
            View::Game => gs.state.turn,
        };
        (gs.game_id.clone(), turn)
    })
}

/// Per-page handle for reading/drawing annotations.
#[derive(Clone, Copy)]
pub struct AnnotationsSignal {
    backend: AnnotationBackend,
    /// Sticky annotate mode (toolbar toggle); suppresses piece moves while on.
    pub mode: RwSignal<bool>,
    pub tool: RwSignal<AnnotationTool>,
    pub color: RwSignal<AnnotationColor>,
    /// True while a draw-modifier is held (quick-draw); draws like `mode` and
    /// pops the palette open.
    pub quick_draw: RwSignal<bool>,
    /// In-progress drag `(from, to)` for the live preview.
    pub preview: RwSignal<Option<(Position, Position)>>,
}

impl AnnotationsSignal {
    fn new(backend: AnnotationBackend) -> Self {
        Self {
            backend,
            mode: RwSignal::new(false),
            tool: RwSignal::new(AnnotationTool::Highlight),
            color: RwSignal::new(AnnotationColor::White),
            quick_draw: RwSignal::new(false),
            preview: RwSignal::new(None),
        }
    }

    pub fn analysis(analysis: AnalysisSignal) -> Self {
        Self::new(AnnotationBackend::Analysis(analysis))
    }

    pub fn play(game_state: GameStateSignal) -> Self {
        Self::new(AnnotationBackend::Play {
            game_state,
            store: RwSignal::new(HashMap::new()),
        })
    }

    /// Annotations for the current position (reactive).
    pub fn current(&self) -> Signal<AnnotationSet> {
        let backend = self.backend;
        Signal::derive(move || backend.read_current())
    }

    pub fn apply_tap(&self, position: Position) {
        let color = self.color.get_untracked();
        let tool = self.tool.get_untracked();
        self.backend
            .update_current(|set| set.tap(position, tool, color));
    }

    pub fn apply_drag(&self, from: Position, to: Position) {
        let color = self.color.get_untracked();
        self.backend
            .update_current(|set| set.add_arrow(from, to, color));
    }

    pub fn clear_current(&self) {
        self.backend
            .update_current(|set| *set = AnnotationSet::default());
    }

    pub fn toggle_mode(&self) {
        self.mode.update(|on| *on = !*on);
    }
}
