use super::drag::{DragPhase, DraggedPiece, PieceDrag};
use crate::{common::PieceType, providers::annotations::AnnotationColor};
use hive_lib::{Color, Piece, Position};
use leptos::prelude::*;
use web_sys::{MouseEvent, PointerEvent};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HivegroundCapabilities {
    pub select_board_piece: bool,
    pub select_reserve_piece: bool,
    pub select_target: bool,
    pub preselect_piece: bool,
    pub inspect_stacks: bool,
    pub drag_color: Option<Color>,
}

impl HivegroundCapabilities {
    pub const fn none() -> Self {
        Self {
            select_board_piece: false,
            select_reserve_piece: false,
            select_target: false,
            preselect_piece: false,
            inspect_stacks: false,
            drag_color: None,
        }
    }

    pub const fn live_selection() -> Self {
        Self {
            select_board_piece: true,
            select_reserve_piece: true,
            select_target: true,
            preselect_piece: true,
            inspect_stacks: true,
            drag_color: None,
        }
    }

    pub const fn analysis_selection() -> Self {
        Self {
            select_board_piece: true,
            select_reserve_piece: true,
            select_target: true,
            preselect_piece: false,
            inspect_stacks: true,
            drag_color: None,
        }
    }

    pub const fn board_inspection() -> Self {
        Self {
            select_board_piece: false,
            select_reserve_piece: false,
            select_target: false,
            preselect_piece: false,
            inspect_stacks: true,
            drag_color: None,
        }
    }

    // Pillbug throws of enemy pieces stay click-only: pressing an enemy piece must keep panning.
    fn can_drag(&self, piece: Piece, piece_type: PieceType) -> bool {
        self.drag_color == Some(piece.color())
            && matches!(
                piece_type,
                PieceType::Board | PieceType::Reserve | PieceType::Inactive
            )
    }
}

#[derive(Clone, Copy, Default)]
pub struct HivegroundActions {
    pub dispatch: Option<Callback<HivegroundAction>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HivegroundAction {
    SelectBoardPiece {
        piece: Piece,
        position: Position,
    },
    SelectReservePiece {
        piece: Piece,
        position: Position,
    },
    SelectTarget {
        position: Position,
    },
    DropOnTarget {
        position: Position,
        dragged: Piece,
        dragged_type: PieceType,
    },
    ResetSelection,
    PreselectPiece {
        piece: Piece,
        position: Position,
        piece_type: PieceType,
    },
}

#[derive(Clone, Copy)]
struct DragState {
    drag: RwSignal<Option<PieceDrag>>,
    // The browser follows a drop with a click; it must not re-select or cancel.
    swallow_click: RwSignal<bool>,
}

impl DragState {
    fn new() -> Self {
        Self {
            drag: RwSignal::new(None),
            swallow_click: RwSignal::new(false),
        }
    }
}

#[derive(Clone, Copy)]
pub struct HivegroundInteraction {
    capabilities: Signal<HivegroundCapabilities>,
    expanded_stack: RwSignal<Option<Position>>,
    actions: HivegroundActions,
    drag: DragState,
}

impl HivegroundInteraction {
    pub fn static_view() -> Self {
        Self::new(HivegroundCapabilities::none(), HivegroundActions::default())
    }

    // Capability profiles are deliberately static-or-reactive: analysis uses a
    // fixed profile, while live play updates as auth and player identity settle.
    pub fn new(
        capabilities: impl Into<Signal<HivegroundCapabilities>>,
        actions: HivegroundActions,
    ) -> Self {
        Self {
            capabilities: capabilities.into(),
            expanded_stack: RwSignal::new(None::<Position>),
            actions,
            drag: DragState::new(),
        }
    }

    // The reserve and board share one drag so a piece can be dragged between their SVGs.
    pub fn disable_stack_inspection(self) -> Self {
        let capabilities = self.capabilities;
        Self {
            capabilities: Signal::derive(move || {
                let mut capabilities = capabilities.get();
                capabilities.inspect_stacks = false;
                capabilities
            }),
            expanded_stack: RwSignal::new(None::<Position>),
            actions: self.actions,
            drag: self.drag,
        }
    }

    pub fn click_piece(
        &self,
        evt: MouseEvent,
        piece: Piece,
        position: Position,
        piece_type: PieceType,
    ) {
        evt.stop_propagation();
        if self.is_click_swallowed() {
            return;
        }
        // A draw-modifier click is a quick-draw gesture (board handles it), not a selection.
        if AnnotationColor::from_modifiers(evt.ctrl_key(), evt.alt_key(), evt.meta_key()).is_some()
        {
            return;
        }
        self.select_piece(piece, position, piece_type);
    }

    pub fn click_target(&self, evt: MouseEvent, position: Position) {
        evt.stop_propagation();
        if self.is_click_swallowed() {
            return;
        }
        let capabilities = self.capabilities.get_untracked();
        if capabilities.select_target {
            self.dispatch(HivegroundAction::SelectTarget { position });
        }
    }

    pub fn click_active(&self, evt: MouseEvent) {
        evt.stop_propagation();
        if self.is_click_swallowed() {
            return;
        }
        self.cancel_selection();
    }

    pub fn cancel_selection(&self) {
        self.dispatch(HivegroundAction::ResetSelection);
    }

    pub fn press_piece(
        &self,
        evt: PointerEvent,
        piece: Piece,
        position: Position,
        piece_type: PieceType,
    ) {
        // A non-primary pointer means another finger is already down: that's a pinch.
        if evt.button() != 0
            || !evt.is_primary()
            || AnnotationColor::from_modifiers(evt.ctrl_key(), evt.alt_key(), evt.meta_key())
                .is_some()
            || !self
                .capabilities
                .get_untracked()
                .can_drag(piece, piece_type)
        {
            return;
        }
        let dragged = DraggedPiece {
            piece,
            position,
            piece_type,
        };
        self.drag.drag.set(Some(PieceDrag::pressed(
            dragged,
            (evt.client_x(), evt.client_y()),
            evt.pointer_id(),
        )));
    }

    // Runs before the piece's own press: a second finger turns the gesture into a pinch, and
    // a fresh first finger means any drag still recorded lost its release.
    pub fn pointer_down(&self) {
        self.drag.swallow_click.set(false);
        self.cancel_drag();
    }

    pub fn drag_to(&self, pointer_id: i32, pointer: (i32, i32)) {
        let Some(drag) = self.own_drag(pointer_id) else {
            return;
        };
        match drag.phase {
            DragPhase::Pressed { .. } if drag.left_slop(pointer) => {
                self.select_piece(
                    drag.dragged.piece,
                    drag.dragged.position,
                    drag.dragged.piece_type,
                );
            }
            DragPhase::Pressed { .. } => return,
            DragPhase::Dragging => {}
        }
        self.drag.drag.set(Some(PieceDrag {
            phase: DragPhase::Dragging,
            pointer,
            ..drag
        }));
    }

    pub fn release_drag(&self, pointer_id: i32, drop: Option<Position>) {
        let Some(drag) = self.own_drag(pointer_id) else {
            return;
        };
        self.drag.drag.set(None);
        if !drag.is_dragging() {
            return;
        }
        self.drag.swallow_click.set(true);
        match drop {
            Some(position) if drag.dropped_on_origin(position) => {}
            Some(position) => self.dispatch(HivegroundAction::DropOnTarget {
                position,
                dragged: drag.dragged.piece,
                dragged_type: drag.dragged.piece_type,
            }),
            None => self.cancel_selection(),
        }
    }

    fn own_drag(&self, pointer_id: i32) -> Option<PieceDrag> {
        self.drag
            .drag
            .get_untracked()
            .filter(|drag| drag.pointer_id == pointer_id)
    }

    pub fn cancel_drag(&self) {
        self.drag.drag.set(None);
    }

    pub fn cancel_drag_of(&self, pointer_id: i32) {
        if self.own_drag(pointer_id).is_some() {
            self.cancel_drag();
        }
    }

    pub fn is_click_swallowed(&self) -> bool {
        self.drag.swallow_click.get_untracked()
    }

    pub fn dragging(&self) -> Option<PieceDrag> {
        self.drag.drag.get().filter(PieceDrag::is_dragging)
    }

    pub fn is_dragging_untracked(&self) -> bool {
        self.drag
            .drag
            .with_untracked(|drag| drag.is_some_and(|drag| drag.is_dragging()))
    }

    pub fn can_drag_any(&self) -> bool {
        self.capabilities
            .with(|capabilities| capabilities.drag_color.is_some())
    }

    pub fn expand_stack(&self, position: Position) {
        if self.can_inspect_stacks() {
            self.expanded_stack.set(Some(position));
        }
    }

    pub fn collapse_stack(&self) {
        self.expanded_stack.set(None);
    }

    pub fn can_inspect_stacks(&self) -> bool {
        self.capabilities
            .with(|capabilities| capabilities.inspect_stacks)
    }

    pub fn is_viewport_pan_allowed(&self) -> bool {
        self.expanded_stack.with_untracked(Option::is_none)
            && self.drag.drag.with_untracked(Option::is_none)
    }

    pub fn stack_level_multiplier(&self, position: Position) -> usize {
        if !self.can_inspect_stacks() {
            return 1;
        }

        match self.expanded_stack.get() {
            Some(pos) if position == pos => 13,
            _ => 1,
        }
    }

    fn select_piece(&self, piece: Piece, position: Position, piece_type: PieceType) {
        let capabilities = self.capabilities.get_untracked();
        match piece_type {
            PieceType::Board if capabilities.select_board_piece => {
                self.dispatch(HivegroundAction::SelectBoardPiece { piece, position });
            }
            PieceType::Reserve if capabilities.select_reserve_piece => {
                self.dispatch(HivegroundAction::SelectReservePiece { piece, position });
            }
            PieceType::Move | PieceType::Spawn | PieceType::Premove
                if capabilities.select_target =>
            {
                self.dispatch(HivegroundAction::SelectTarget { position });
            }
            PieceType::Board | PieceType::Reserve | PieceType::Inactive
                if capabilities.preselect_piece =>
            {
                self.dispatch(HivegroundAction::PreselectPiece {
                    piece,
                    position,
                    piece_type,
                });
            }
            _ => {}
        }
    }

    fn dispatch(&self, action: HivegroundAction) {
        if let Some(dispatch) = self.actions.dispatch {
            dispatch.run(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::Owner;
    use std::sync::{Arc, Mutex};

    const FINGER: i32 = 1;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    fn recording_interaction() -> (HivegroundInteraction, Arc<Mutex<Vec<HivegroundAction>>>) {
        let actions = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&actions);
        let capabilities = HivegroundCapabilities {
            drag_color: Some(Color::White),
            ..HivegroundCapabilities::live_selection()
        };
        let interaction = HivegroundInteraction::new(
            capabilities,
            HivegroundActions {
                dispatch: Some(Callback::new(move |action| {
                    recorded.lock().expect("lock").push(action);
                })),
            },
        );
        (interaction, actions)
    }

    fn press(interaction: HivegroundInteraction, dragged: DraggedPiece, pointer: (i32, i32)) {
        interaction
            .drag
            .drag
            .set(Some(PieceDrag::pressed(dragged, pointer, FINGER)));
    }

    #[test]
    fn drag_selects_past_slop_and_drops_on_target() {
        Owner::new().with(|| {
            let (interaction, actions) = recording_interaction();
            let origin = Position::new(0, 0);
            let target = Position::new(1, 0);
            let dragged = DraggedPiece {
                piece: piece("wA1"),
                position: origin,
                piece_type: PieceType::Board,
            };
            press(interaction, dragged, (100, 100));

            interaction.drag_to(FINGER, (102, 101));
            assert!(actions.lock().expect("lock").is_empty());
            assert!(interaction.dragging().is_none());

            interaction.drag_to(FINGER, (140, 100));
            interaction.drag_to(FINGER, (160, 100));
            interaction.release_drag(FINGER, Some(target));

            assert_eq!(
                *actions.lock().expect("lock"),
                vec![
                    HivegroundAction::SelectBoardPiece {
                        piece: piece("wA1"),
                        position: origin,
                    },
                    HivegroundAction::DropOnTarget {
                        position: target,
                        dragged: piece("wA1"),
                        dragged_type: PieceType::Board,
                    },
                ]
            );
            assert!(interaction.is_click_swallowed());
            assert!(interaction.is_viewport_pan_allowed());
        });
    }

    #[test]
    fn release_without_movement_leaves_the_click_alone() {
        Owner::new().with(|| {
            let (interaction, actions) = recording_interaction();
            let dragged = DraggedPiece {
                piece: piece("wA1"),
                position: Position::new(0, 0),
                piece_type: PieceType::Board,
            };
            press(interaction, dragged, (100, 100));
            interaction.release_drag(FINGER, Some(Position::new(3, 3)));

            assert!(actions.lock().expect("lock").is_empty());
            assert!(!interaction.is_click_swallowed());
        });
    }

    #[test]
    fn dropping_back_on_origin_keeps_the_selection() {
        Owner::new().with(|| {
            let (interaction, actions) = recording_interaction();
            let origin = Position::new(0, 0);
            let dragged = DraggedPiece {
                piece: piece("wA1"),
                position: origin,
                piece_type: PieceType::Board,
            };
            press(interaction, dragged, (100, 100));
            interaction.drag_to(FINGER, (140, 100));
            interaction.release_drag(FINGER, Some(origin));

            assert_eq!(
                *actions.lock().expect("lock"),
                vec![HivegroundAction::SelectBoardPiece {
                    piece: piece("wA1"),
                    position: origin,
                }]
            );
        });
    }

    #[test]
    fn a_second_finger_cancels_the_drag_instead_of_dropping() {
        Owner::new().with(|| {
            let (interaction, actions) = recording_interaction();
            let dragged = DraggedPiece {
                piece: piece("wA1"),
                position: Position::new(0, 0),
                piece_type: PieceType::Board,
            };
            interaction.pointer_down();
            press(interaction, dragged, (100, 100));
            interaction.drag_to(FINGER + 1, (160, 100));
            interaction.cancel_drag_of(FINGER + 1);
            assert!(interaction.dragging().is_none());
            assert!(!interaction.is_viewport_pan_allowed());

            interaction.pointer_down();
            interaction.drag_to(FINGER, (160, 100));
            interaction.release_drag(FINGER + 1, Some(Position::new(1, 0)));
            interaction.release_drag(FINGER, Some(Position::new(1, 0)));

            assert!(actions.lock().expect("lock").is_empty());
            assert!(interaction.is_viewport_pan_allowed());
        });
    }

    #[test]
    fn a_drag_that_lost_its_release_is_cleared_by_the_next_press() {
        Owner::new().with(|| {
            let (interaction, _) = recording_interaction();
            let dragged = DraggedPiece {
                piece: piece("wA1"),
                position: Position::new(0, 0),
                piece_type: PieceType::Board,
            };
            press(interaction, dragged, (100, 100));
            interaction.drag_to(FINGER, (160, 100));
            assert!(!interaction.is_viewport_pan_allowed());

            interaction.pointer_down();
            assert!(interaction.is_viewport_pan_allowed());
        });
    }

    #[test]
    fn only_own_pieces_drag_so_enemy_pieces_still_pan() {
        let capabilities = HivegroundCapabilities {
            drag_color: Some(Color::White),
            ..HivegroundCapabilities::live_selection()
        };
        assert!(capabilities.can_drag(piece("wA1"), PieceType::Board));
        assert!(!capabilities.can_drag(piece("bA1"), PieceType::Board));
        assert!(capabilities.can_drag(piece("wA1"), PieceType::Inactive));
        assert!(!capabilities.can_drag(piece("bA1"), PieceType::Inactive));
        assert!(!capabilities.can_drag(piece("wA1"), PieceType::Covered));
        assert!(!HivegroundCapabilities::live_selection().can_drag(piece("wA1"), PieceType::Board));
    }
}
