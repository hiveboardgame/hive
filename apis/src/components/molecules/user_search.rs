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

#[component]
pub fn UserSearch(
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] fallback_users: Option<Signal<BTreeMap<String, UserResponse>>>,
    #[prop(optional)] filtered_users: Option<HashSet<String>>,
    #[prop(optional)] show_count: Option<Signal<String>>,
    actions: Vec<UserAction>,
) -> impl IntoView {
    let i18n = use_i18n();
    let pattern = RwSignal::new(String::new());
    let pattern_len = Signal::derive(move || pattern().len());
    let user_search = Resource::new(
        move || (pattern(), filtered_users.clone()),
        async move |(pattern, filtered_users)| {
            if pattern.len() < MIN_SEARCH_LENGTH {
                None
            } else {
                let user_search = search_users(pattern).await;
                let btree = user_search
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|user| {
                        filtered_users
                            .as_ref()
                            .is_none_or(|filtered| !filtered.contains(&user.username))
                    })
                    .map(|user| (user.username.clone(), user))
                    .collect();
                Some(btree)
            }
        },
    );

    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        pattern.set(event_target_value(&ev));
    });

    let users = Signal::derive(move || {
        if pattern_len() < MIN_SEARCH_LENGTH {
            fallback_users.map(|f| f()).unwrap_or_default()
        } else {
            user_search.get().flatten().unwrap_or_default()
        }
    });

    let input_placeholder = move || {
        placeholder
            .clone()
            .unwrap_or_else(|| t_string!(i18n, home.search_players).to_string())
    };

    view! {
        <div class="flex flex-col m-2 w-fit">
            <div class="relative">
                <input
                    class="p-1 w-64"
                    type="text"
                    on:input=debounced_search
                    placeholder=input_placeholder
                    prop:value=pattern
                    maxlength="20"
                />
                <div class="h-5">
                    <Show when=move || { pattern_len() > 0 && pattern().len() < MIN_SEARCH_LENGTH }>
                        <span class="text-xs text-yellow-600">
                            {move || format!("Minimum {MIN_SEARCH_LENGTH} characters")}
                        </span>
                    </Show>
                </div>
            </div>
            <Transition>
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
            </Transition>
            <Transition>
                <div class="overflow-y-auto max-h-96">
                    <For each=users key=move |(_, user)| user.uid let:user>
                        <UserRow actions=actions.clone() user=user.1 />
                    </For>
                </div>
            </Transition>
        </div>
    }
}
