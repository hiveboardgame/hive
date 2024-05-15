use crate::common::MoveConfirm;
use crate::functions::config::confirm_mode::ToggleConfirmMode;
use leptos::*;

#[cfg(not(feature = "ssr"))]
fn initial_prefers_confirm() -> MoveConfirm {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    if cookie.contains("confirm_mode=Clock") {
        return MoveConfirm::Clock;
    } else if cookie.contains("confirm_mode=Single") {
        return MoveConfirm::Single;
    }
    return MoveConfirm::Double;
}

#[cfg(feature = "ssr")]
fn initial_prefers_confirm() -> MoveConfirm {
    use std::str::FromStr;

    if let Some(request) = use_context::<actix_web::HttpRequest>() {
        if let Ok(cookies) = request.cookies() {
            for cookie in cookies.iter() {
                if cookie.name() == "confirm_mode" {
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
    pub action: Action<ToggleConfirmMode, Result<MoveConfirm, ServerFnError>>,
    pub preferred_confirm: Signal<MoveConfirm>,
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
            let initial = initial_prefers_confirm();
            match (input(), value()) {
                // if there's some current input, use that optimistically
                (Some(submission), _) => submission.move_confirm,
                // otherwise, if there was a previous value confirmed by server, use that
                (_, Some(Ok(value))) => value,
                // otherwise, use the initial value
                _ => initial,
            }
        };

        ConfirmMode {
            action: toggle_move_confirm_action,
            preferred_confirm: Signal::derive(prefers_confirm_fn),
        }
    }
}
