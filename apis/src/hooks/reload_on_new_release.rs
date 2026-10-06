use crate::providers::websocket::{ConnectionReadyState, WebsocketContext};
use leptos::{prelude::*, task::spawn_local_scoped_with_cancellation};
use leptos_router::{hooks::use_location, location::Location};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

const CLIENT_RELEASE: Option<&str> = option_env!("HIVE_RELEASE_SHA");

// A tab keeps running the bundle it loaded. After a deploy it reconnects to the new server; once
// that server runs another release, the next navigation becomes a full load of the new one.
pub fn use_reload_on_new_release() -> Signal<bool> {
    let outdated = RwSignal::new(false);
    let Some(client_release) = CLIENT_RELEASE else {
        return outdated.into();
    };
    let websocket = expect_context::<WebsocketContext>();

    Effect::new(move |was_open: Option<bool>| {
        let open = websocket.ready_state.get() == ConnectionReadyState::Open;
        if open && was_open != Some(true) && !outdated.get_untracked() {
            spawn_local_scoped_with_cancellation(async move {
                if server_release()
                    .await
                    .is_some_and(|server| server != client_release)
                {
                    outdated.set(true);
                }
            });
        }
        open
    });

    let location = use_location();
    Effect::new(move |previous_path: Option<String>| {
        let path = location.pathname.get();
        if previous_path.is_some_and(|previous| previous != path) {
            if outdated.get_untracked() {
                load_current_route(&location);
            } else {
                // The destination may already contain unsaved work by the time this finishes.
                // Only record the update; a later navigation or explicit refresh will apply it.
                spawn_local_scoped_with_cancellation(async move {
                    if server_release()
                        .await
                        .is_some_and(|server| server != client_release)
                    {
                        outdated.set(true);
                    }
                });
            }
        }
        path
    });
    outdated.into()
}

fn load_current_route(location: &Location) {
    let browser_location = window().location();
    let Ok(origin) = browser_location.origin() else {
        return;
    };
    let mut destination = format!("{origin}{}", location.pathname.get_untracked());
    let search = location.search.get_untracked();
    if !search.is_empty() {
        destination.push('?');
        destination.push_str(&search);
    }
    destination.push_str(&location.hash.get_untracked());

    // The router can publish the destination before committing it to browser history.
    if browser_location.href().ok().as_deref() == Some(destination.as_str()) {
        let _ = browser_location.reload();
    } else {
        let _ = browser_location.set_href(&destination);
    }
}

async fn server_release() -> Option<String> {
    let response: Response = JsFuture::from(window().fetch_with_str("/health"))
        .await
        .ok()?
        .dyn_into()
        .ok()?;
    if !response.ok() {
        return None;
    }
    JsFuture::from(response.text().ok()?)
        .await
        .ok()?
        .as_string()
}
