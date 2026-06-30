use crate::{
    common::UserAction,
    components::{
        atoms::rating::icon_for_speed,
        molecules::{empty_state::EmptyState, user_row::UserRow},
    },
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
    let top_users = LocalResource::new({
        let auth_context = auth_context.clone();
        move || {
            let _viewer_id = auth_context
                .user
                .with(|account| account.as_ref().map(|account| account.id));
            async move { get_top_users(speed(), 10).await }
        }
    });
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
                                view! {
                                    <section class="w-full max-w-xs ui-panel">
                                        <EmptyState title="Couldn't fetch top users" class="m-2" />
                                    </section>
                                },
                            )
                        }
                        Ok(users) => {
                            let is_empty = users.is_empty();
                            let show_unranked_placeholder = auth_context
                                .user
                                .with(|account| {
                                    account
                                        .as_ref()
                                        .is_some_and(|account| {
                                            !users.iter().any(|(_, user)| user.uid == account.id)
                                        })
                                });
                            let users = StoredValue::new(users);
                            Either::Right(
                                view! {
                                    <section class="ui-panel w-full max-w-xs">
                                        <div class="ui-panel-header py-2">
                                            <div class="flex min-w-0 items-center gap-2">
                                                <Icon
                                                    icon=icon_for_speed(speed())
                                                    attr:class="size-4 flex-shrink-0 text-pillbug-teal"
                                                />
                                                <h2 class="truncate text-sm font-bold text-gray-900 dark:text-gray-100">
                                                    {speed().to_string()}
                                                </h2>
                                            </div>
                                        </div>
                                        <div class="ui-panel-body p-2">
                                            <Show
                                                when=move || !is_empty || show_unranked_placeholder
                                                fallback=|| {
                                                    view! {
                                                        <EmptyState
                                                            title="No one yet"
                                                            class="py-4"
                                                        />
                                                    }
                                                }
                                            >
                                                <div class="overflow-hidden rounded-lg border border-black/5 dark:border-white/10">
                                                    <For
                                                        each=move || users.get_value()

                                                        key=|(_,user)| user.uid
                                                        let:((rank,user))
                                                    >
                                                        <div class="flex items-center gap-2">
                                                            <span class="w-6 text-right text-xs font-semibold text-gray-500 dark:text-gray-400">
                                                                {rank}
                                                            </span>
                                                            <div class="min-w-0 flex-1">
                                                                <UserRow
                                                                    actions=vec![UserAction::Challenge]
                                                                    user
                                                                    game_speed=StoredValue::new(speed())
                                                                />
                                                            </div>
                                                        </div>
                                                    </For>
                                                    <Show when=move || show_unranked_placeholder>
                                                        <div class="flex items-center gap-2">
                                                            <span class="w-6 text-right text-xs font-semibold text-gray-500 dark:text-gray-400"></span>
                                                            <div class="flex h-10 min-w-0 flex-1 items-center rounded px-3 py-1 text-xs font-semibold text-gray-500 ui-dense-table-row dark:text-gray-400">
                                                                "Not ranked yet"
                                                            </div>
                                                        </div>
                                                    </Show>
                                                </div>
                                            </Show>
                                        </div>
                                    </section>
                                },
                            )
                        }
                    })
            }}

        </Transition>
    }
}
