use crate::common::TileRotation;

use crate::functions::config::tile_rotation::ChangeTileRotation;
use leptos::*;

#[cfg(not(feature = "ssr"))]
fn initial_tile_rotation() -> TileRotation {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    if cookie.contains("tile_rotation=Yes") {
        return TileRotation::Yes;
    }
    TileRotation::No
}

#[cfg(feature = "ssr")]
fn initial_tile_rotation() -> TileRotation {
    use std::str::FromStr;

    if let Some(request) = use_context::<actix_web::HttpRequest>() {
        if let Ok(cookies) = request.cookies() {
            for cookie in cookies.iter() {
                if cookie.name() == "tile_rotation" {
                    if let Ok(tile_rotation) = TileRotation::from_str(cookie.value()) {
                        return tile_rotation;
                    }
                }
            }
        }
    };
    TileRotation::No
}

#[derive(Clone)]
pub struct TileRotationConfig {
    pub action: Action<ChangeTileRotation, Result<TileRotation, ServerFnError>>,
    pub preferred_tile_rotation: Signal<TileRotation>,
}

impl Default for TileRotationConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TileRotationConfig {
    pub fn new() -> Self {
        let change_tile_rotation = create_server_action::<ChangeTileRotation>();
        // input is `Some(value)` when pending, and `None` if not pending
        let input = change_tile_rotation.input();
        // value contains most recently-returned value
        let value = change_tile_rotation.value();

        let prefers_tile_rotation_fn = move || {
            let initial = initial_tile_rotation();
            match (input(), value()) {
                // if there's some current input, use that optimistically
                (Some(submission), _) => submission.tile_rotation,
                // otherwise, if there was a previous value confirmed by server, use that
                (_, Some(Ok(value))) => value,
                // otherwise, use the initial value
                _ => initial,
            }
        };

        TileRotationConfig {
            action: change_tile_rotation,
            preferred_tile_rotation: Signal::derive(prefers_tile_rotation_fn),
        }
    }
}
