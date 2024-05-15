use crate::common::TileDesign;
use crate::functions::config::tile_design::ChangeTileDesign;
use leptos::*;

#[cfg(not(feature = "ssr"))]
fn initial_tile_design() -> TileDesign {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    if cookie.contains("tile_design=Flat") {
        return TileDesign::Flat;
    }
    TileDesign::Official
}

#[cfg(feature = "ssr")]
fn initial_tile_design() -> TileDesign {
    use std::str::FromStr;

    if let Some(request) = use_context::<actix_web::HttpRequest>() {
        if let Ok(cookies) = request.cookies() {
            for cookie in cookies.iter() {
                if cookie.name() == "tile_design" {
                    if let Ok(tile_design) = TileDesign::from_str(cookie.value()) {
                        return tile_design;
                    }
                }
            }
        }
    };
    TileDesign::Official
}

#[derive(Clone)]
pub struct TileDesignConfig {
    pub action: Action<ChangeTileDesign, Result<TileDesign, ServerFnError>>,
    pub preferred_tile_design: Signal<TileDesign>,
}

impl Default for TileDesignConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TileDesignConfig {
    pub fn new() -> Self {
        let change_tile_design = create_server_action::<ChangeTileDesign>();
        // input is `Some(value)` when pending, and `None` if not pending
        let input = change_tile_design.input();
        // value contains most recently-returned value
        let value = change_tile_design.value();

        let prefers_tile_design_fn = move || {
            let initial = initial_tile_design();
            match (input(), value()) {
                // if there's some current input, use that optimistically
                (Some(submission), _) => submission.tile_design,
                // otherwise, if there was a previous value confirmed by server, use that
                (_, Some(Ok(value))) => value,
                // otherwise, use the initial value
                _ => initial,
            }
        };

        TileDesignConfig {
            action: change_tile_design,
            preferred_tile_design: Signal::derive(prefers_tile_design_fn),
        }
    }
}
