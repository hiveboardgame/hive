use crate::common::UserAction;
use crate::components::atoms::rating::icon_for_speed;
use crate::{components::molecules::user_row::UserRow, functions::users::get_top_users};
use leptos::either::Either;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn Leaderboard(speed: GameSpeed) -> impl IntoView {
    let speed = Signal::derive(move || speed);
    let top_users = OnceResource::new(get_top_users(speed(), 10));
    view! {
        <Transition>
            {move || {
                top_users
                    .get()
                    .map(|data| match data {
                        Err(e) => {
                            log!("Error is: {:?}", e);
                            Either::Left(
                                view! { <pre class="m-2 h-6">"Couldn't fetch top users"</pre> },
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
                                            :
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

                                                key=|users| users.uid
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
                    })
            }}

        </Transition>
    }
}
