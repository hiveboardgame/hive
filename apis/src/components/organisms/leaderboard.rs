use crate::{
    common::UserAction,
    components::{atoms::rating::icon_for_speed, molecules::user_row::UserRow},
    functions::users::get_top_users,
    providers::AuthContext,
};
use leptos::{either::Either, logging::log, prelude::*};
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn Leaderboard(speed: GameSpeed) -> impl IntoView {
    let speed = Signal::derive(move || speed);
    let auth_context = expect_context::<AuthContext>();
    let top_users = LocalResource::new(move || async move { get_top_users(speed(), 10).await });
    Effect::watch(
        auth_context.logout.version(),
        move |_, _, _| {
            top_users.refetch();
        },
        false,
    );
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
                            let is_empty = users.is_empty();
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
                                                if !is_empty { "hidden" } else { "flex" },
                                            )
                                        }>{move || if is_empty { "No one yet" } else { "" }}</div>
                                        <div class="overflow-hidden rounded-lg">
                                            <For
                                                each=move || { users.clone() }

                                                key=|(_,user)| user.uid
                                                let:((rank,user))
                                            >
                                                <div class="flex gap-2 items-center">
                                                    <span class="w-6 text-sm text-right text-stone-500">
                                                        {rank}
                                                    </span>
                                                    <UserRow
                                                        actions=vec![UserAction::Challenge]
                                                        user
                                                        game_speed=StoredValue::new(speed())
                                                    />
                                                </div>
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
