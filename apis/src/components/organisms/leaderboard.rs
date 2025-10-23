use crate::common::UserAction;
use crate::components::atoms::rating::icon_for_speed;
use crate::{
    components::molecules::user_row::UserRow,
    functions::users::{get_top_users, get_users_around_position},
    providers::AuthContext,
};
use leptos::either::Either;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn Leaderboard(speed: GameSpeed) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let speed = Signal::derive(move || speed);

    let user_id = Signal::derive(move || auth_context.user.with(|u| u.as_ref().map(|u| u.id)));

    let top_users_resource = Resource::new(
        move || speed(),
        move |speed| async move { get_top_users(speed, 10).await },
    );

    let user_position_resource = Resource::new(
        move || (speed(), user_id()),
        move |(speed, user_id_opt)| async move {
            match user_id_opt {
                Some(user_id) => get_users_around_position(speed, user_id, 5).await.ok(),
                None => None,
            }
        },
    );

    view! {
        <div class="flex flex-col gap-4">
            <Transition>
                {move || {
                    top_users_resource
                        .get()
                        .map(|data| {
                            match data {
                                Err(e) => {
                                    log!("Error fetching top users: {:?}", e);
                                    Either::Left(
                                        view! {
                                            <pre class="m-2 h-6">"Couldn't fetch top users"</pre>
                                        },
                                    )
                                }
                                Ok(users) => {
                                    let users = StoredValue::new(users);
                                    let is_empty = move || users.with_value(|u| u.is_empty());
                                    Either::Right(

                                        view! {
                                            <div class="flex flex-col m-2 rounded-lg w-fit">
                                                <div class="flex gap-1 items-center">
                                                    <Icon icon=icon_for_speed(speed()) />
                                                    {speed().to_string()}
                                                    " - Top 10"
                                                </div>
                                                <div class=move || {
                                                    format!(
                                                        "p-1 h-6 {}",
                                                        if !is_empty() { "hidden" } else { "flex" },
                                                    )
                                                }>{move || if is_empty() { "No one yet" } else { "" }}</div>
                                                <div class="rounded-lg overflow-hidden">
                                                    <For
                                                        each=move || { users.get_value() }
                                                        key=|user| user.uid
                                                        let:user
                                                    >
                                                        <UserRow
                                                            actions=vec![UserAction::Challenge]
                                                            user
                                                            game_speed=StoredValue::new(speed())
                                                        />
                                                    </For>
                                                </div>
                                            </div>
                                        },
                                    )
                                }
                            }
                        })
                }}
            </Transition>

            <Transition>
                {move || {
                    user_position_resource
                        .get()
                        .and_then(|data| data)
                        .map(|users| {
                            let users = StoredValue::new(users);

                            view! {
                                <div class="flex flex-col m-2 rounded-lg w-fit">
                                    <div class="flex gap-1 items-center">
                                        <Icon icon=icon_for_speed(speed()) />
                                        {speed().to_string()}
                                        " - Your Position"
                                    </div>
                                    <div class="rounded-lg overflow-hidden">
                                        <For
                                            each=move || { users.get_value() }
                                            key=|user| user.uid
                                            let:user
                                        >
                                            <UserRow
                                                actions=vec![UserAction::Challenge]
                                                user
                                                game_speed=StoredValue::new(speed())
                                            />
                                        </For>
                                    </div>
                                </div>
                            }
                        })
                }}
            </Transition>
        </div>
    }
}
