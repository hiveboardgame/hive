use crate::{common::PieceType, providers::annotations::AnnotationColor};
use hive_lib::{Piece, Position};
use leptos::prelude::*;
use web_sys::MouseEvent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HivegroundCapabilities {
    pub select_board_piece: bool,
    pub select_reserve_piece: bool,
    pub select_target: bool,
    pub preselect_piece: bool,
    pub inspect_stacks: bool,
}

impl HivegroundCapabilities {
    pub const fn none() -> Self {
        Self {
            select_board_piece: false,
            select_reserve_piece: false,
            select_target: false,
            preselect_piece: false,
            inspect_stacks: false,
        }
    }

    pub const fn live_selection() -> Self {
        Self {
            select_board_piece: true,
            select_reserve_piece: true,
            select_target: true,
            preselect_piece: true,
            inspect_stacks: true,
        }
    }

    pub const fn analysis_selection() -> Self {
        Self {
            select_board_piece: true,
            select_reserve_piece: true,
            select_target: true,
            preselect_piece: false,
            inspect_stacks: true,
        }
    }

    pub const fn board_inspection() -> Self {
        Self {
            select_board_piece: false,
            select_reserve_piece: false,
            select_target: false,
            preselect_piece: false,
            inspect_stacks: true,
        }
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
    ResetSelection,
    PreselectPiece {
        piece: Piece,
        position: Position,
        piece_type: PieceType,
    },
}

#[derive(Clone, Copy)]
pub struct HivegroundInteraction {
    capabilities: Signal<HivegroundCapabilities>,
    expanded_stack: RwSignal<Option<Position>>,
    actions: HivegroundActions,
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
        }
    }

    pub fn disable_stack_inspection(self) -> Self {
        let capabilities = self.capabilities;
        Self::new(
            Signal::derive(move || {
                let mut capabilities = capabilities.get();
                capabilities.inspect_stacks = false;
                capabilities
            }),
            self.actions,
        )
    }

    pub fn click_piece(
        &self,
        evt: MouseEvent,
        piece: Piece,
        position: Position,
        piece_type: PieceType,
    ) {
        evt.stop_propagation();
        // A draw-modifier click is a quick-draw gesture (board handles it), not a selection.
        if AnnotationColor::from_modifiers(evt.ctrl_key(), evt.alt_key(), evt.meta_key()).is_some()
        {
            return;
        }
        let capabilities = self.capabilities.get_untracked();
        match piece_type {
            PieceType::Board if capabilities.select_board_piece => {
                self.dispatch(HivegroundAction::SelectBoardPiece { piece, position });
            }
            PieceType::Reserve if capabilities.select_reserve_piece => {
                self.dispatch(HivegroundAction::SelectReservePiece { piece, position });
            }
            PieceType::Move | PieceType::Spawn if capabilities.select_target => {
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

    pub fn click_target(&self, evt: MouseEvent, position: Position) {
        evt.stop_propagation();
        let capabilities = self.capabilities.get_untracked();
        if capabilities.select_target {
            self.dispatch(HivegroundAction::SelectTarget { position });
        }
    }

    pub fn click_active(&self, evt: MouseEvent) {
        evt.stop_propagation();
        self.cancel_selection();
    }

    pub fn cancel_selection(&self) {
        self.dispatch(HivegroundAction::ResetSelection);
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
            .with_untracked(|capabilities| capabilities.inspect_stacks)
    }

    pub fn is_viewport_pan_allowed(&self) -> bool {
        self.expanded_stack.with_untracked(Option::is_none)
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

    fn dispatch(&self, action: HivegroundAction) {
        if let Some(dispatch) = self.actions.dispatch {
            dispatch.run(action);
        }
    }
}
