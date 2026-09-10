use leptos::prelude::*;
use leptos_router::hooks::use_url;
use std::collections::HashMap;

#[cfg(not(feature = "ssr"))]
use leptos::{
    ev,
    leptos_dom::helpers::{
        document,
        request_animation_frame,
        set_timeout_with_handle,
        window,
        window_event_listener,
    },
};
#[cfg(not(feature = "ssr"))]
use std::time::Duration;
#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;

#[cfg(not(feature = "ssr"))]
const RESTORE_RETRY_DELAY: Duration = Duration::from_millis(50);
const RESTORE_RETRY_LIMIT: u16 = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PagePosition {
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NavigationRequest {
    id: u64,
    location: String,
}

#[derive(Clone)]
pub struct NavigationFocusState {
    active_location: ArcRwSignal<Option<String>>,
    history_request: ArcRwSignal<Option<NavigationRequest>>,
    pending_focus: ArcRwSignal<Option<NavigationRequest>>,
    pending_restore: ArcRwSignal<Option<NavigationRequest>>,
    positions: ArcRwSignal<HashMap<String, PagePosition>>,
    request_sequence: ArcRwSignal<u64>,
}

impl NavigationFocusState {
    pub fn new(initial_location: Option<String>) -> Self {
        let positions = initial_location
            .as_ref()
            .map(|location| (location.clone(), PagePosition::default()))
            .into_iter()
            .collect();
        Self {
            active_location: ArcRwSignal::new(initial_location),
            history_request: ArcRwSignal::new(None),
            pending_focus: ArcRwSignal::new(None),
            pending_restore: ArcRwSignal::new(None),
            positions: ArcRwSignal::new(positions),
            request_sequence: ArcRwSignal::new(0),
        }
    }

    #[cfg(not(feature = "ssr"))]
    pub fn begin_history_navigation(&self, target: Option<String>) {
        let Some(target) = target else {
            return;
        };
        self.remember_browser_position();
        let request = self.next_request(target.clone());
        self.pending_focus.set(None);
        self.pending_restore.set(Some(request.clone()));

        // A native restoration scroll can fire before the router reacts to
        // popstate. Switch identities now so it cannot overwrite the page we
        // are leaving.
        self.active_location.set(Some(target));
        self.history_request.set(Some(request));
    }

    fn next_request(&self, location: String) -> NavigationRequest {
        let mut id = 0;
        self.request_sequence.update(|sequence| {
            *sequence = sequence.wrapping_add(1);
            id = *sequence;
        });
        NavigationRequest { id, location }
    }

    fn begin_push_navigation(&self, location: String) -> NavigationRequest {
        let request = self.next_request(location.clone());
        self.active_location.set(Some(location.clone()));
        self.history_request.set(None);
        self.pending_restore.set(None);
        self.pending_focus.set(Some(request.clone()));
        self.positions
            .update(|positions| _ = positions.insert(location, PagePosition::default()));
        request
    }

    fn activate_initial_location(&self, location: String) {
        self.active_location.set(Some(location.clone()));
        self.history_request.set(None);
        self.pending_focus.set(None);
        self.pending_restore.set(None);
        self.positions.update(|positions| {
            positions.entry(location).or_default();
        });
    }

    fn take_history_request(&self, location: &str) -> Option<NavigationRequest> {
        let request = self.history_request.get_untracked();
        if !request
            .as_ref()
            .is_some_and(|request| request.location == location)
        {
            return None;
        }
        self.history_request.set(None);
        request
    }

    fn position_for(&self, location: &str) -> PagePosition {
        self.positions
            .with_untracked(|positions| positions.get(location).copied())
            .unwrap_or_default()
    }

    #[cfg(not(feature = "ssr"))]
    fn remember_browser_position(&self) {
        if self.pending_restore.get_untracked().is_some() {
            return;
        }
        let Some(location) = self.active_location.get_untracked() else {
            return;
        };
        if current_browser_location_key().as_deref() != Some(location.as_str()) {
            return;
        }
        let position = PagePosition {
            x: window().scroll_x().unwrap_or_default(),
            y: window().scroll_y().unwrap_or_default(),
        };
        self.positions.update(|positions| {
            positions.insert(location, position);
        });
    }

    #[cfg(not(feature = "ssr"))]
    pub(crate) fn pending_focus(&self) -> Option<NavigationRequest> {
        self.pending_focus.get()
    }

    #[cfg(not(feature = "ssr"))]
    pub(crate) fn focus_request_is_current(&self, request: &NavigationRequest) -> bool {
        self.request_is_current(request)
            && self.pending_focus.get_untracked().as_ref() == Some(request)
    }

    #[cfg(not(feature = "ssr"))]
    pub(crate) fn consume_focus(&self, request: &NavigationRequest) -> bool {
        if !self.focus_request_is_current(request) {
            return false;
        }
        self.pending_focus.set(None);
        true
    }

    #[cfg(not(feature = "ssr"))]
    fn restore_request_is_current(&self, request: &NavigationRequest) -> bool {
        self.request_is_current(request)
            && self.pending_restore.get_untracked().as_ref() == Some(request)
    }

    #[cfg(not(feature = "ssr"))]
    fn request_is_current(&self, request: &NavigationRequest) -> bool {
        self.request_sequence.get_untracked() == request.id
            && self.active_location.get_untracked().as_deref() == Some(request.location.as_str())
            && current_browser_location_key().as_deref() == Some(request.location.as_str())
    }
}

#[component]
pub fn NavigationFocus(state: NavigationFocusState) -> impl IntoView {
    let url = use_url();
    let previous_location = StoredValue::new(None::<String>);
    let mounted = ArcRwSignal::new(true);
    on_cleanup({
        let mounted = mounted.clone();
        move || mounted.set(false)
    });

    #[cfg(not(feature = "ssr"))]
    {
        let state = state.clone();
        let mounted = mounted.clone();
        let scroll_handle = window_event_listener(ev::scroll, move |_| {
            if mounted.get_untracked() {
                state.remember_browser_position();
            }
        });
        on_cleanup(move || scroll_handle.remove());
    }

    Effect::new(move |_| {
        let url = url.get();
        let location = location_key(url.path(), url.search(), url.hash());
        let pending_history = state.history_request.get();
        let previous = previous_location.get_value();
        previous_location.set_value(Some(location.clone()));

        if pending_history
            .as_ref()
            .is_some_and(|request| request.location == location)
        {
            let request = state
                .take_history_request(&location)
                .expect("matching history request should remain pending");
            let target = state.position_for(&location);
            restore_page_position(
                request,
                target,
                RESTORE_RETRY_LIMIT,
                mounted.clone(),
                state.clone(),
            );
        } else if previous.as_deref() != Some(location.as_str()) {
            if should_reset_page(previous.as_deref(), &location, false) {
                #[cfg(not(feature = "ssr"))]
                state.remember_browser_position();
                let request = state.begin_push_navigation(location);
                reset_page_position_and_focus(request, mounted.clone(), state.clone());
            } else {
                state.activate_initial_location(location);
                #[cfg(not(feature = "ssr"))]
                state.remember_browser_position();
            }
        }
    });
}

fn location_key(pathname: &str, search: &str, hash: &str) -> String {
    let mut location = pathname.to_string();
    let search = search.strip_prefix('?').unwrap_or(search);
    if !search.is_empty() {
        location.push('?');
        location.push_str(search);
    }
    if !hash.is_empty() {
        if !hash.starts_with('#') {
            location.push('#');
        }
        location.push_str(hash);
    }
    location
}

#[cfg(not(feature = "ssr"))]
pub(crate) fn current_browser_location_key() -> Option<String> {
    let location = window().location();
    Some(location_key(
        &location.pathname().ok()?,
        &location.search().ok()?,
        &location.hash().ok()?,
    ))
}

fn should_reset_page(previous: Option<&str>, current: &str, from_history: bool) -> bool {
    !from_history
        && previous.is_some_and(|previous| {
            // Filtering updates the URL while a user is still typing. Keep focus in
            // that page; only a different path or anchor requests heading focus.
            previous.split(['?', '#']).next() != current.split(['?', '#']).next()
                || previous.split_once('#').map(|(_, hash)| hash)
                    != current.split_once('#').map(|(_, hash)| hash)
        })
}

#[cfg(any(not(feature = "ssr"), test))]
fn restoration_target(
    target: PagePosition,
    maximum: PagePosition,
    retries_remaining: u16,
) -> Option<PagePosition> {
    let reachable = target.x <= maximum.x + 1.0 && target.y <= maximum.y + 1.0;
    if !reachable && retries_remaining > 0 {
        return None;
    }
    Some(PagePosition {
        x: target.x.clamp(0.0, maximum.x.max(0.0)),
        y: target.y.clamp(0.0, maximum.y.max(0.0)),
    })
}

#[cfg(not(feature = "ssr"))]
fn maximum_page_position() -> PagePosition {
    document()
        .document_element()
        .map(|root| PagePosition {
            x: f64::from((root.scroll_width() - root.client_width()).max(0)),
            y: f64::from((root.scroll_height() - root.client_height()).max(0)),
        })
        .unwrap_or_default()
}

#[cfg(not(feature = "ssr"))]
fn restore_page_position(
    request: NavigationRequest,
    target: PagePosition,
    retries_remaining: u16,
    mounted: ArcRwSignal<bool>,
    state: NavigationFocusState,
) {
    request_animation_frame(move || {
        if !mounted.get_untracked() || !state.restore_request_is_current(&request) {
            return;
        }
        if let Some(position) =
            restoration_target(target, maximum_page_position(), retries_remaining)
        {
            state.pending_restore.set(None);
            state.positions.update(|positions| {
                positions.insert(request.location, position);
            });
            window().scroll_to_with_x_and_y(position.x, position.y);
            return;
        }

        let request_for_timeout = request.clone();
        let mounted_for_timeout = mounted.clone();
        let state_for_timeout = state.clone();
        let _ = set_timeout_with_handle(
            move || {
                if !mounted_for_timeout.get_untracked()
                    || !state_for_timeout.restore_request_is_current(&request_for_timeout)
                {
                    return;
                }
                restore_page_position(
                    request_for_timeout,
                    target,
                    retries_remaining - 1,
                    mounted_for_timeout,
                    state_for_timeout,
                );
            },
            RESTORE_RETRY_DELAY,
        );
    });
}

#[cfg(feature = "ssr")]
fn restore_page_position(
    _request: NavigationRequest,
    _target: PagePosition,
    _retries_remaining: u16,
    _mounted: ArcRwSignal<bool>,
    _state: NavigationFocusState,
) {
}

#[cfg(not(feature = "ssr"))]
fn reset_page_position_and_focus(
    request: NavigationRequest,
    mounted: ArcRwSignal<bool>,
    state: NavigationFocusState,
) {
    request_animation_frame(move || {
        if !mounted.get_untracked() || !state.request_is_current(&request) {
            return;
        }
        request_animation_frame(move || {
            if !mounted.get_untracked() || !state.request_is_current(&request) {
                return;
            }
            window().scroll_to_with_x_and_y(0.0, 0.0);

            let Ok(Some(heading)) =
                document().query_selector("main h1:not([data-navigation-focus-managed])")
            else {
                return;
            };
            let _ = heading.set_attribute("tabindex", "-1");
            if let Ok(heading) = heading.dyn_into::<web_sys::HtmlElement>() {
                if state.consume_focus(&request) {
                    let _ = heading.focus();
                }
            }
        });
    });
}

#[cfg(feature = "ssr")]
fn reset_page_position_and_focus(
    _request: NavigationRequest,
    _mounted: ArcRwSignal<bool>,
    _state: NavigationFocusState,
) {
}

#[cfg(test)]
mod tests {
    use super::{location_key, restoration_target, should_reset_page, PagePosition};

    #[test]
    fn location_key_includes_path_query_and_hash() {
        assert_eq!(location_key("/tournaments", "", ""), "/tournaments");
        assert_eq!(
            location_key("/tournaments", "page=2", "#standings"),
            "/tournaments?page=2#standings",
        );
        assert_ne!(
            location_key("/tournaments", "page=1", ""),
            location_key("/tournaments", "page=2", ""),
        );
        assert_ne!(
            location_key("/tournaments", "", "#one"),
            location_key("/tournaments", "", "#two"),
        );
    }

    #[test]
    fn query_updates_preserve_typing_focus_while_route_changes_reset_it() {
        assert!(!should_reset_page(
            Some("/tournaments?q=a"),
            "/tournaments?q=arena",
            false
        ));
        assert!(!should_reset_page(
            Some("/tournaments?q=arena"),
            "/tournaments",
            false
        ));
        assert!(should_reset_page(
            Some("/tournaments?q=arena"),
            "/tournaments/mine/joined",
            false
        ));
        assert!(!should_reset_page(
            Some("/tournaments"),
            "/tournaments/mine/joined",
            true
        ));
    }

    #[test]
    fn restoration_waits_until_the_saved_offset_is_reachable() {
        assert_eq!(
            restoration_target(
                PagePosition { x: 0.0, y: 900.0 },
                PagePosition { x: 0.0, y: 500.0 },
                1,
            ),
            None,
        );
    }

    #[test]
    fn restoration_uses_the_exact_saved_offset_once_reachable() {
        let target = PagePosition { x: 12.0, y: 900.0 };
        assert_eq!(
            restoration_target(
                target,
                PagePosition {
                    x: 100.0,
                    y: 1_500.0,
                },
                10,
            ),
            Some(target),
        );
    }

    #[test]
    fn restoration_clamps_when_the_retry_window_expires() {
        assert_eq!(
            restoration_target(
                PagePosition { x: 300.0, y: 900.0 },
                PagePosition { x: 40.0, y: 500.0 },
                0,
            ),
            Some(PagePosition { x: 40.0, y: 500.0 }),
        );
    }
}
