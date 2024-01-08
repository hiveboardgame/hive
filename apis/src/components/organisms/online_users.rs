use leptos::*;

use crate::{
    components::molecules::user_row::UserRow,
    providers::{auth_context::AuthContext, online_users::OnlineUsersSignal},
};

#[component]
pub fn OnlineUsers() -> impl IntoView {
    let online_users = expect_context::<OnlineUsersSignal>();
    let auth_context = expect_context::<AuthContext>();
    let username = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user.username),
        _ => None,
    };
    let online_players = move || {
        let mut users = (online_users.signal)().username_user;
        username().and_then(|name| users.remove(&name));
        users
    };
    view! {
        <div class="flex flex-col m-2">
            Online players <ol>

                <For each=online_players key=|(key, _)| key.to_owned() let:a_user>
                    <UserRow username=store_value(a_user.0) rating=a_user.1.rating/>
                </For>
            </ol>
        </div>
    }
}
