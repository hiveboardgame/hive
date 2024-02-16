use crate::common::config_options::TileDots;
use crate::functions::config::tile_dots::ChangeTileDots;
use leptos::*;

#[cfg(not(feature = "ssr"))]
fn initial_tile_dots() -> TileDots {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    if cookie.contains("tile_dots=No") {
        return TileDots::No;
    }
    TileDots::Yes
}

#[cfg(feature = "ssr")]
fn initial_tile_dots() -> TileDots {
    use std::str::FromStr;

    if let Some(request) = use_context::<actix_web::HttpRequest>() {
        if let Ok(cookies) = request.cookies() {
            for cookie in cookies.iter() {
                if cookie.name() == "tile_dots" {
                    if let Ok(tile_dots) = TileDots::from_str(cookie.value()) {
                        return tile_dots;
                    }
                }
            }
        }
    };
    TileDots::Yes
}

#[derive(Clone)]
pub struct TileDotsConfig {
    pub action: Action<ChangeTileDots, Result<TileDots, ServerFnError>>,
    pub preferred_tile_dots: Signal<TileDots>,
}

impl Default for TileDotsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TileDotsConfig {
    pub fn new() -> Self {
        let change_tile_dots = create_server_action::<ChangeTileDots>();
        // input is `Some(value)` when pending, and `None` if not pending
        let input = change_tile_dots.input();
        // value contains most recently-returned value
        let value = change_tile_dots.value();

        let prefers_tile_dots_fn = move || {
            let initial = initial_tile_dots();
            match (input(), value()) {
                // if there's some current input, use that optimistically
                (Some(submission), _) => submission.tile_dots,
                // otherwise, if there was a previous value confirmed by server, use that
                (_, Some(Ok(value))) => value,
                // otherwise, use the initial value
                _ => initial,
            }
        };

        TileDotsConfig {
            action: change_tile_dots,
            preferred_tile_dots: Signal::derive(prefers_tile_dots_fn),
        }
    }
}
