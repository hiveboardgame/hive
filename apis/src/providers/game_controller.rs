use leptos::prelude::{provide_context, RwSignal};

use crate::common::GameActionResponse;

#[derive(Clone)]
pub struct GameController {
    pub response: RwSignal<Option<GameActionResponse>>,
}

pub fn provide_game_controller() {
    provide_context(GameController {
        response: RwSignal::new(None),
    });
}
