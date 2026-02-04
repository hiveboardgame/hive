use crate::{
    common::UserAction, components::molecules::user_row::UserRow, functions::users::search_users,
    i18n::*, responses::UserResponse,
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

const MIN_SEARCH_LENGTH: usize = 2;
const MAX_SUGGESTIONS: usize = 10;

#[component]
pub fn UserSearch(
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] fallback_users: Option<Signal<BTreeMap<String, UserResponse>>>,
    #[prop(optional)] filtered_users: Option<HashSet<String>>,
    #[prop(optional)] filtered_users_signal: Option<Signal<HashSet<String>>>,
    #[prop(optional)] show_count: Option<Signal<String>>,
    #[prop(optional)] value: Option<Signal<Option<String>>>,
    actions: Vec<UserAction>,
) -> impl IntoView {
    let i18n = use_i18n();
    let pattern = RwSignal::new(String::new());
    let pattern_len = Signal::derive(move || pattern().len());

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
                let btree = user_search
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
                let cb_clear = cb.clone();
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

    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        let val = event_target_value(&ev);
        pattern.set(val.clone());
        if val.is_empty() {
            if let Some(ref cb) = clear_callback {
                cb.run(());
            }
        }
    });

    let users = Signal::derive(move || {
        let all = if pattern_len() < MIN_SEARCH_LENGTH {
            fallback_users.map(|f| f()).unwrap_or_default()
        } else {
            user_search.get().flatten().unwrap_or_default()
        };
        all.into_iter().take(MAX_SUGGESTIONS).collect::<BTreeMap<_, _>>()
    });

    let display_value = Signal::derive(move || {
        let p = pattern();
        if p.is_empty() {
            value
                .as_ref()
                .and_then(|v| v())
                .unwrap_or_default()
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
    let show_clear = Signal::derive(move || {
        has_select
            && value
                .as_ref()
                .and_then(|v| v())
                .is_some()
    });

    let do_clear = move |_| {
        pattern.set(String::new());
        if let Some(ref cb) = clear_callback {
            cb.run(());
        }
    };

    view! {
        <div class="flex flex-col w-full min-w-0 shrink-0 relative">
            <div class="relative flex items-center gap-1">
                <input
                    class="input input-bordered w-full p-1 rounded-lg min-w-0"
                    type="text"
                    on:input=debounced_search
                    placeholder=input_placeholder
                    prop:value=display_value
                    maxlength="20"
                />
                <Show when=show_clear>
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm btn-square shrink-0"
                        aria-label="Clear selection"
                        on:click=do_clear
                    >
                        "×"
                    </button>
                </Show>
            </div>
            <div class="h-5">
                <Show when=move || { pattern_len() > 0 && pattern().len() < MIN_SEARCH_LENGTH }>
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
                        pattern_len() >= MIN_SEARCH_LENGTH && !users.get().is_empty()
                    }>{t!(i18n, home.found_players)}</Show>
                    <Show when=move || {
                        pattern_len() >= MIN_SEARCH_LENGTH && users.get().is_empty()
                    }>
                        <span class="text-xs text-gray-500">
                            {move || format!("No users found for \"{}\"", pattern())}
                        </span>
                    </Show>
                </div>
            </Suspense>
            <Show when=move || { pattern_len() >= MIN_SEARCH_LENGTH }>
                <Transition>
                    <div class="absolute top-full left-0 right-0 z-50 mt-1 overflow-y-auto overflow-x-hidden max-h-60 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shadow-lg">
                        <For each=users key=move |(_, user)| user.uid let:user>
                            <UserRow actions=wrapped_actions_stored.get_value() user=user.1 selection_mode=has_select />
                        </For>
                    </div>
                </Transition>
            </Show>
        </div>
    }
}
