use crate::i18n::*;
use crate::{
    common::UserAction,
    components::molecules::user_row::UserRow,
    providers::{online_users::OnlineUsersSignal, user_search::UserSearchSignal, ApiRequests},
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use std::time::Duration;
#[component]
pub fn OnlineUsers() -> impl IntoView {
    let i18n = use_i18n();
    let user_search = expect_context::<UserSearchSignal>();
    let online_users = expect_context::<OnlineUsersSignal>();
    let pattern = RwSignal::new(String::new());
    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        pattern.set(event_target_value(&ev));
        if pattern().is_empty() {
            user_search.signal.update(|s| s.clear());
        } else {
            let api = ApiRequests::new();
            api.search_user(pattern());
        }
    });
    let users = move || {
        if pattern().is_empty() {
            online_users.signal.get().username_user
        } else {
            user_search.signal.get()
        }
    };
    let num = move || online_users.signal.get().username_user.len();
    //TODO: Uncoment out code
    view! {
        <div class="flex flex-col m-2 w-fit">
            <input
                class="p-1 w-64"
                type="text"
                on:input=debounced_search
                //placeholder={t!(i18n, home.search_players)}
                //value=pattern
                maxlength="20"
            />
            <Show
                when=move || pattern().is_empty()
                fallback=move || { t!(i18n, home.found_players) }
            >
                {t!(i18n, home.online_players, count = num)}
            </Show>
            <div class="overflow-y-auto max-h-96">
                <For each=users key=move |(_, user)| user.uid let:user>
                    <UserRow actions=vec![UserAction::Challenge] user=StoredValue::new(user.1) />
                </For>

            </div>
        </div>
    }
}
