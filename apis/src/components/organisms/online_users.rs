use crate::functions::users::search_users;
use crate::i18n::*;
use crate::{
    common::UserAction, components::molecules::user_row::UserRow,
    providers::online_users::OnlineUsersSignal,
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use std::collections::BTreeMap;
use std::time::Duration;
#[component]
pub fn OnlineUsers() -> impl IntoView {
    let i18n = use_i18n();
    let online_users = expect_context::<OnlineUsersSignal>();
    let pattern = RwSignal::new(String::new());
    let user_search = Resource::new(pattern, async move |pattern| {
        if pattern.is_empty() {
            BTreeMap::new()
        } else {
            let user_search = search_users(pattern).await;
            let mut btree = BTreeMap::new();
            for user in user_search.unwrap_or_default() {
                btree.insert(user.username.clone(), user);
            }
            btree
        }
    });
    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        pattern.set(event_target_value(&ev));
    });
    let users = move || {
        if pattern().is_empty() {
            online_users.signal.get().username_user
        } else {
            user_search.get().unwrap_or_default()
        }
    };
    let num = move || online_users.signal.get().username_user.len();
    view! {
        <div class="flex flex-col m-2 w-fit">
            <input
                class="p-1 w-64"
                type="text"
                on:input=debounced_search
                placeholder=move || t_string!(i18n, home.search_players)
                value=pattern
                maxlength="20"
            />
            <Show
                when=move || pattern().is_empty()
                fallback=move || { t!(i18n, home.found_players) }
            >
                {t!(i18n, home.online_players, count = num)}
            </Show>
            <div class="overflow-y-auto max-h-96">
                <Transition>
                    <For each=users key=move |(_, user)| user.uid let:user>
                        <UserRow
                            actions=vec![UserAction::Challenge]
                            user=StoredValue::new(user.1)
                        />
                    </For>
                </Transition>
            </div>
        </div>
    }
}
