use crate::{
    common::{with_class, UserAction},
    components::molecules::user_row::UserRow,
    functions::users::search_users,
    i18n::*,
    responses::UserResponse,
};
use leptos::{html, leptos_dom::helpers::debounce, prelude::*};
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};
use wasm_bindgen::JsCast;

const MIN_SEARCH_LENGTH: usize = 2;
const MAX_SUGGESTIONS: usize = 10;

/// Usernames are ASCII (`db`'s `valid_username_char`), so the ASCII fold is exact.
fn sorted_by_username(users: BTreeMap<String, UserResponse>) -> Vec<(String, UserResponse)> {
    let mut users: Vec<_> = users.into_iter().collect();
    users.sort_by_cached_key(|(username, _)| username.to_ascii_lowercase());
    users
}

#[component]
pub fn UserSearch(
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] fallback_users: Option<Signal<BTreeMap<String, UserResponse>>>,
    #[prop(optional, into)] filtered_users: Option<Signal<HashSet<String>>>,
    #[prop(optional)] value: Option<Signal<Option<String>>>,
    /// Fires on every keystroke with the trimmed text (`None` when empty).
    /// Callers like the archive use it so the typed value is applied on
    /// Search even when no suggestion is selected.
    #[prop(optional)]
    on_input: Option<Callback<Option<String>>>,
    /// Drops the reserved-height hint/status rows below the input to keep the
    /// field tight (used by the archive where those rows add unwanted gaps).
    #[prop(optional)]
    compact: bool,
    actions: Vec<UserAction>,
) -> impl IntoView {
    let i18n = use_i18n();
    let pattern = RwSignal::new(String::new());
    let pattern_len = Signal::derive(move || pattern().len());
    let focused = RwSignal::new(false);
    // Root of the component; used by the input's blur handler to tell whether
    // focus left the component entirely or moved into one of its own controls.
    let root_ref = NodeRef::<html::Div>::new();

    let excluded_users = move || {
        filtered_users
            .as_ref()
            .map(|filtered_users| filtered_users().into_iter().collect::<Vec<_>>())
            .unwrap_or_default()
    };

    let user_search = LocalResource::new(move || {
        let pattern = pattern();
        let filtered_users = excluded_users();
        async move {
            if pattern.len() < MIN_SEARCH_LENGTH {
                None
            } else {
                let ranked: Vec<(String, UserResponse)> = search_users(pattern, filtered_users)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|user| (user.username.clone(), user))
                    .collect();
                Some(ranked)
            }
        }
    });

    let select_callback = actions.iter().find_map(|action| match action {
        UserAction::Select(callback) => Some(*callback),
        _ => None,
    });
    let wrapped_actions: Vec<UserAction> = actions
        .into_iter()
        .map(|a| match a {
            UserAction::Select(cb) => UserAction::Select(Callback::new(move |opt| {
                let _ = pattern.try_set(String::new());
                cb.run(opt);
            })),
            other => other,
        })
        .collect();

    // `pattern` drives the (debounced) suggestion search and the displayed
    // value. `on_input` fires synchronously in the handler below — not here —
    // so the typed value is applied immediately (e.g. clicking Search right
    // after typing, before the debounce elapses). Reading the value
    // synchronously in the handler also avoids touching a dead DOM Event later,
    // which trips Firefox's "Permission denied to access object".
    let mut debounced_update = debounce(Duration::from_millis(100), move |val: String| {
        let _ = pattern.try_set(val.clone());
        if val.is_empty() {
            if let Some(cb) = select_callback {
                cb.run(None);
            }
        }
    });

    let has_search_query = Signal::derive(move || pattern_len() >= MIN_SEARCH_LENGTH);
    let visible_users = Signal::derive(move || {
        if !has_search_query() {
            sorted_by_username(fallback_users.map(|f| f()).unwrap_or_default())
        } else {
            let mut users = user_search.get().flatten().unwrap_or_default();
            users.truncate(MAX_SUGGESTIONS);
            users
        }
    });

    // Typed-search results only show while the field is focused, so the
    // suggestion list disappears once the user clicks away. A persistent
    // fallback list (e.g. online players) still shows below the threshold.
    let show_suggestions = Signal::derive(move || {
        !visible_users.get().is_empty() && (!has_search_query() || focused.get())
    });

    // Fallback users (e.g. online players) stay in normal flow below the
    // search threshold; typed search results overlay the surrounding layout.
    let has_fallback = fallback_users.is_some();
    let fallback_status = Signal::derive(move || {
        fallback_users.map(|users| {
            users.with(|users| t_string!(i18n, home.online_players, count = users.len()))
        })
    });
    let results_are_dropdown = Signal::derive(move || !has_fallback || has_search_query());

    let display_value = Signal::derive(move || {
        let p = pattern();
        if p.is_empty() {
            value.as_ref().and_then(|v| v()).unwrap_or_default()
        } else {
            p
        }
    });

    let input_placeholder = move || {
        placeholder
            .clone()
            .unwrap_or_else(|| t_string!(i18n, home.search_players).to_string())
    };

    let has_select = select_callback.is_some();
    let wrapped_actions_stored = StoredValue::new(wrapped_actions);
    let show_clear =
        Signal::derive(move || has_select && value.as_ref().and_then(|v| v()).is_some());

    let do_clear = move |_| {
        let _ = pattern.try_set(String::new());
        if let Some(cb) = select_callback {
            cb.run(None);
        }
    };

    view! {
        <div node_ref=root_ref class="flex relative flex-col w-full min-w-0 shrink-0">
            <div class="flex relative gap-1 items-center">
                <input
                    class="ui-field-input"
                    type="text"
                    name="user-search"
                    autocomplete="off"
                    autocapitalize="none"
                    spellcheck="false"
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        if let Some(cb) = on_input {
                            let trimmed = val.trim();
                            cb.run((!trimmed.is_empty()).then(|| trimmed.to_string()));
                        }
                        debounced_update(val);
                    }
                    on:focus=move |_| {
                        let _ = focused.try_set(true);
                    }
                    on:blur=move |ev: leptos::ev::FocusEvent| {
                        if let (Some(root), Some(target)) = (
                            root_ref.try_get_untracked().flatten(),
                            ev.related_target(),
                        ) {
                            if let Some(node) = target.dyn_ref::<web_sys::Node>() {
                                if root.contains(Some(node)) {
                                    return;
                                }
                            }
                        }
                        let _ = focused.try_set(false);
                    }
                    placeholder=input_placeholder
                    prop:value=display_value
                    maxlength="20"
                />
                // The clear (×) button only exists for the select-style usage
                // (archive player filters). Reserving its slot keeps the two
                // player inputs the same width; other callers (online search,
                // invite) skip it so the input fills the row and lines up with
                // the suggestion list below.
                <Show when=move || has_select>
                    <div class="flex justify-center items-center shrink-0 size-10">
                        <Show when=show_clear>
                            <button
                                type="button"
                                class="ui-button ui-button-ghost ui-button-icon-lg"
                                aria-label="Clear selection"
                                on:click=do_clear
                            >
                                "×"
                            </button>
                        </Show>
                    </div>
                </Show>
            </div>
            {(!compact)
                .then(|| {
                    view! {
                        <div class="h-5">
                            <Show when=move || { pattern_len() > 0 && !has_search_query() }>
                                <span class="text-xs text-yellow-600">
                                    {move || format!("Minimum {MIN_SEARCH_LENGTH} characters")}
                                </span>
                            </Show>
                        </div>
                        <Suspense fallback=move || {
                            view! {
                                <div class="h-5">
                                    <p class="text-xs text-gray-500">"Searching..."</p>
                                </div>
                            }
                        }>
                            <div class="h-5">
                                <Show when=move || {
                                    !has_search_query() && fallback_status().is_some()
                                }>{move || fallback_status().unwrap_or_default()}</Show>
                                <Show when=move || {
                                    focused.get() && has_search_query()
                                        && !visible_users.get().is_empty()
                                }>{t!(i18n, home.found_players)}</Show>
                                <Show when=move || {
                                    focused.get() && has_search_query()
                                        && visible_users.get().is_empty()
                                }>
                                    <span class="text-xs text-gray-500">
                                        {move || format!("No users found for \"{}\"", pattern())}
                                    </span>
                                </Show>
                            </div>
                        </Suspense>
                    }
                })}
            <Transition>
                <Show when=show_suggestions>
                    // Keep input focus on suggestion click so blur
                    // doesn't hide the list before the click lands.
                    <div
                        class=move || {
                            if results_are_dropdown.get() {
                                with_class(
                                    "ui-dropdown-panel",
                                    "overflow-y-auto overflow-x-hidden absolute right-0 left-0 top-full z-50 mt-1 max-h-60",
                                )
                            } else {
                                with_class(
                                    "ui-dropdown-panel",
                                    "overflow-y-auto overflow-x-hidden mt-1 max-h-96",
                                )
                            }
                        }
                        on:mousedown=|ev| ev.prevent_default()
                    >
                        <For each=visible_users key=move |(_, user)| user.uid let:user>
                            <UserRow actions=wrapped_actions_stored.get_value() user=user.1 />
                        </For>
                    </div>
                </Show>
            </Transition>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Takeback;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn users(names: &[&str]) -> BTreeMap<String, UserResponse> {
        names
            .iter()
            .map(|name| {
                let user = UserResponse {
                    username: (*name).to_string(),
                    uid: Uuid::new_v4(),
                    patreon: false,
                    bot: false,
                    admin: false,
                    deleted: false,
                    ratings: HashMap::new(),
                    takeback: Takeback::default(),
                    lang: None,
                };
                ((*name).to_string(), user)
            })
            .collect()
    }

    fn order(names: &[&str]) -> Vec<String> {
        sorted_by_username(users(names))
            .into_iter()
            .map(|(username, _)| username)
            .collect()
    }

    #[test]
    fn case_does_not_split_the_list() {
        assert_eq!(
            order(&["Zed", "alice", "Bob", "charlie"]),
            vec!["alice", "Bob", "charlie", "Zed"],
        );
    }

    #[test]
    fn underscore_sorts_between_digits_and_letters() {
        assert_eq!(
            order(&["_ghost", "9lives", "Amy", "-dash"]),
            vec!["-dash", "9lives", "_ghost", "Amy"],
        );
    }
}
