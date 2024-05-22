use crate::{
    components::molecules::user_row::UserRow,
    providers::{online_users::OnlineUsersSignal, user_search::UserSearchSignal, ApiRequests},
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::*;
use std::time::Duration;

#[component]
pub fn OnlineUsers() -> impl IntoView {
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
    let text = move || {
        if pattern().is_empty() {
            let num = online_users.signal.get().username_user.len();
            if num == 1 {
                format!("{} online player", num)
            } else {
                format!("{} online players", num)
            }
        } else {
            String::from("Found:")
        }
    };
    view! {
        <div class="flex flex-col m-2 w-fit">
            <input
                class="p-1 w-64"
                type="text"
                on:input=debounced_search
                placeholder="Search players"
                prop:value=pattern
                attr:maxlength="20"
            />

            {text}
            <div class="overflow-y-auto h-96">
                <For each=users key=move |(_, user)| user.uid let:user>
                    <UserRow user=store_value(user.1)/>
                </For>

            </div>
        </div>
    }
}
