use crate::functions::config::darkmode::ToggleDarkMode;
use leptos::*;

#[cfg(not(feature = "ssr"))]
fn initial_prefers_dark() -> bool {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    cookie.contains("darkmode=true")
}

#[cfg(feature = "ssr")]
fn initial_prefers_dark() -> bool {
    use_context::<actix_web::HttpRequest>()
        .and_then(|req| {
            req.cookies()
                .map(|cookies| {
                    cookies
                        .iter()
                        .any(|cookie| cookie.name() == "darkmode" && cookie.value() == "true")
                })
                .ok()
        })
        .unwrap_or(false)
}

#[derive(Clone)]
pub struct ColorScheme {
    pub action: Action<ToggleDarkMode, Result<bool, ServerFnError>>,
    pub prefers_dark: Signal<bool>,
}
pub fn provide_color_scheme() -> Signal<bool> {
    let initial = initial_prefers_dark();

    let toggle_dark_mode_action = create_server_action::<ToggleDarkMode>();
    // input is `Some(value)` when pending, and `None` if not pending
    let input = toggle_dark_mode_action.input();
    // value contains most recently-returned value
    let value = toggle_dark_mode_action.value();

    let prefers_dark_fn = move || {
        match (input(), value()) {
            // if there's some current input, use that optimistically
            (Some(submission), _) => submission.prefers_dark,
            // otherwise, if there was a previous value confirmed by server, use that
            (_, Some(Ok(value))) => value,
            // otherwise, use the initial value
            _ => initial,
        }
    };
    let prefers_dark = Signal::derive(prefers_dark_fn);

    provide_context(ColorScheme {
        action: toggle_dark_mode_action,
        prefers_dark,
    });
    prefers_dark
}
