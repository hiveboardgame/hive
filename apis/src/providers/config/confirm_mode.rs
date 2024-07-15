use crate::common::MoveConfirm;
use crate::functions::config::confirm_mode::ToggleConfirmMode;
use leptos::*;
use shared_types::GameSpeed;
use std::collections::HashMap;

#[cfg(not(feature = "ssr"))]
fn initial_prefers_confirm(game_speed: GameSpeed) -> MoveConfirm {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    if cookie.contains(&format!("{game_speed}_confirm_mode=Clock")) {
        return MoveConfirm::Clock;
    } else if cookie.contains(&format!("{game_speed}_confirm_mode=Single")) {
        return MoveConfirm::Single;
    }
    MoveConfirm::Double
}

#[cfg(feature = "ssr")]
fn initial_prefers_confirm(game_speed: GameSpeed) -> MoveConfirm {
    use std::str::FromStr;

    if let Some(request) = use_context::<actix_web::HttpRequest>() {
        if let Ok(cookies) = request.cookies() {
            for cookie in cookies.iter() {
                if cookie.name() == format!("{game_speed}_confirm_mode") {
                    if let Ok(confirm_mode) = MoveConfirm::from_str(cookie.value()) {
                        return confirm_mode;
                    }
                }
            }
        }
    };
    MoveConfirm::Double
}

#[derive(Clone)]
pub struct ConfirmMode {
    pub action: Action<ToggleConfirmMode, Result<(GameSpeed, MoveConfirm), ServerFnError>>,
    pub preferred_confirms: Signal<HashMap<GameSpeed, MoveConfirm>>,
}

impl Default for ConfirmMode {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfirmMode {
    pub fn new() -> Self {
        let toggle_move_confirm_action = create_server_action::<ToggleConfirmMode>();
        // input is `Some(value)` when pending, and `None` if not pending
        let input = toggle_move_confirm_action.input();
        // value contains most recently-returned value
        let value = toggle_move_confirm_action.value();

        let prefers_confirm_fn = move || {
            let mut move_confirms = HashMap::new();
            for game_speed in GameSpeed::all() {
                let initial = initial_prefers_confirm(game_speed.clone());
                move_confirms.insert(game_speed, initial);
            }
            match (input(), value()) {
                (Some(submission), _) => {
                    move_confirms.insert(submission.game_speed, submission.move_confirm);
                }

                (_, Some(Ok(value))) => {
                    move_confirms.insert(value.0, value.1);
                }

                _ => {}
            }
            move_confirms
        };

        ConfirmMode {
            action: toggle_move_confirm_action,
            preferred_confirms: Signal::derive(prefers_confirm_fn),
        }
    }
}
