use crate::{
    common::UserAction,
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

const DEFAULT_INPUT_CLASS: &str = "w-full min-w-0 min-h-10 px-3 rounded-lg shadow-sm input input-bordered focus:ring-2 focus:ring-pillbug-teal/50";

#[component]
pub fn UserSearch(
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] fallback_users: Option<Signal<BTreeMap<String, UserResponse>>>,
    #[prop(optional)] filtered_users: Option<HashSet<String>>,
    #[prop(optional)] filtered_users_signal: Option<Signal<HashSet<String>>>,
    #[prop(optional)] show_count: Option<Signal<String>>,
    #[prop(optional)] value: Option<Signal<Option<String>>>,
    /// Fires on every keystroke with the trimmed text (`None` when empty).
    /// Callers like the archive use it so the typed value is applied on
    /// Search even when no suggestion is selected.
    #[prop(optional)]
    on_input: Option<Callback<Option<String>>>,
    /// Overrides the input's CSS classes so callers can match a form's styling.
    #[prop(optional, into)]
    input_class: Option<String>,
    /// Overrides the root container's CSS classes. Defaults to a full-width
    /// flex column; callers like the home online-users list pass a fixed width
    /// (`w-64` + margins) so the box doesn't fill its grid column edge-to-edge.
    /// Must keep `relative` so the suggestion dropdown positions against it.
    #[prop(optional, into)]
    container_class: Option<String>,
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
    // focus left the component entirely (collapse the list) or just moved into
    // our own subtree — e.g. the challenge `<dialog>` opening from a found row,
    // which must not tear the list (and that dialog) down with it.
    let root_ref = NodeRef::<html::Div>::new();

    let excluded_users = move || {
        filtered_users_signal
            .as_ref()
            .map(|s| s())
            .or_else(|| filtered_users.clone())
            .unwrap_or_default()
    };

    let user_search = Resource::new(
        move || (pattern(), excluded_users()),
        async move |(pattern, filtered_users)| {
            if pattern.len() < MIN_SEARCH_LENGTH {
                None
            } else {
                let user_search = search_users(pattern).await;
                let btree: BTreeMap<String, UserResponse> = user_search
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|user| !filtered_users.contains(&user.username))
                    .map(|user| (user.username.clone(), user))
                    .collect();
                Some(btree)
            }
        },
    );

    let mut clear_callback: Option<Callback<()>> = None;
    let wrapped_actions: Vec<UserAction> = actions
        .into_iter()
        .map(|a| match a {
            UserAction::Select(cb) => {
                let cb_clear = cb;
                clear_callback = Some(Callback::new(move |_| {
                    cb_clear.run(None);
                }));
                UserAction::Select(Callback::new(move |opt| {
                    pattern.set(String::new());
                    cb.run(opt);
                }))
            }
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
        pattern.set(val.clone());
        if val.is_empty() {
            if let Some(ref cb) = clear_callback {
                cb.run(());
            }
        }
    });

    let users = Signal::derive(move || {
        if pattern_len() < MIN_SEARCH_LENGTH {
            fallback_users.map(|f| f()).unwrap_or_default()
        } else {
            user_search
                .get()
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .take(MAX_SUGGESTIONS)
                .collect::<BTreeMap<_, _>>()
        }
    });

    // Typed-search results only show while the field is focused, so the
    // suggestion list disappears once the user clicks away. A persistent
    // fallback list (e.g. online players) still shows below the threshold.
    let show_suggestions = Signal::derive(move || {
        !users.get().is_empty() && (pattern_len() < MIN_SEARCH_LENGTH || focused.get())
    });

    // The persistent fallback list (e.g. online players, shown below the
    // search threshold) must stay in normal flow so it reserves layout space
    // instead of overlaying the content beneath it. Typed suggestions still
    // use an absolute dropdown overlay so they don't push the page around.
    let has_fallback = fallback_users.is_some();
    let as_dropdown = Signal::derive(move || !(has_fallback && pattern_len() < MIN_SEARCH_LENGTH));

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

    // clear_callback is set only when actions contain UserAction::Select; used for × button and backspace-clear
    let has_select = clear_callback.is_some();
    let wrapped_actions_stored = StoredValue::new(wrapped_actions);
    let show_clear =
        Signal::derive(move || has_select && value.as_ref().and_then(|v| v()).is_some());

    let do_clear = move |_| {
        pattern.set(String::new());
        if let Some(ref cb) = clear_callback {
            cb.run(());
        }
    };

    let resolved_input_class = input_class.unwrap_or_else(|| DEFAULT_INPUT_CLASS.to_string());
    let resolved_container_class = container_class
        .unwrap_or_else(|| "flex relative flex-col w-full min-w-0 shrink-0".to_string());

    view! {
        <div node_ref=root_ref class=resolved_container_class>
            <div class="flex relative gap-1 items-center">
                <input
                    class=resolved_input_class
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
                    on:focus=move |_| focused.set(true)
                    on:blur=move |ev: leptos::ev::FocusEvent| {
                        use wasm_bindgen::JsCast;
                        if let (Some(root), Some(target)) = (root_ref.get(), ev.related_target()) {
                            if let Some(node) = target.dyn_ref::<web_sys::Node>() {
                                if root.contains(Some(node)) {
                                    return;
                                }
                            }
                        }
                        focused.set(false);
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
                                class="flex justify-center items-center w-full h-full text-2xl leading-none text-gray-500 rounded-lg transition-colors hover:text-gray-700 hover:bg-gray-100 dark:hover:text-gray-200 dark:hover:bg-gray-700"
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
                            <Show when=move || {
                                pattern_len() > 0 && pattern().len() < MIN_SEARCH_LENGTH
                            }>
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
                                    pattern_len() < MIN_SEARCH_LENGTH && show_count.is_some()
                                }>{show_count.unwrap()}</Show>
                                <Show when=move || {
                                    focused.get() && pattern_len() >= MIN_SEARCH_LENGTH
                                        && !users.get().is_empty()
                                }>{t!(i18n, home.found_players)}</Show>
                                <Show when=move || {
                                    focused.get() && pattern_len() >= MIN_SEARCH_LENGTH
                                        && users.get().is_empty()
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
                {move || {
                    show_suggestions
                        .get()
                        .then(|| {
                            view! {
                                // Keep input focus on suggestion click so blur
                                // doesn't hide the list before the click lands.
                                <div
                                    class=move || {
                                        if as_dropdown.get() {
                                            "overflow-y-auto overflow-x-hidden absolute right-0 left-0 top-full z-50 mt-1 max-h-60 bg-white rounded-lg border border-gray-200 shadow-lg dark:bg-gray-800 dark:border-gray-700"
                                        } else {
                                            "overflow-y-auto overflow-x-hidden mt-1 max-h-60 bg-white rounded-lg border border-gray-200 shadow-lg dark:bg-gray-800 dark:border-gray-700"
                                        }
                                    }
                                    on:mousedown=|ev| {
                                        let in_dialog = ev
                                            .target()
                                            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                                            .and_then(|el| el.closest("dialog").ok().flatten())
                                            .is_some();
                                        if !in_dialog {
                                            ev.prevent_default();
                                        }
                                    }
                                >
                                    <For each=users key=move |(_, user)| user.uid let:user>
                                        <UserRow
                                            actions=wrapped_actions_stored.get_value()
                                            user=user.1
                                            selection_mode=has_select
                                            full_width=true
                                        />
                                    </For>
                                </div>
                            }
                        })
                }}
            </Transition>
        </div>
    }
}
