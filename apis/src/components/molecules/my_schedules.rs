use crate::{
    common::with_class,
    components::atoms::{
        profile_link::ProfileLink,
        schedule_controls::{GameDateControls, ProposeDateControls},
    },
    providers::schedules::SchedulesContext,
    responses::{GameResponse, ScheduleResponse},
};
use chrono::{Duration, Utc};
use hudsoni::GameStatus;
use leptos::prelude::*;
use shared_types::{Conclusion, GameId};
use std::collections::HashMap;
use uuid::Uuid;

#[component]
pub fn MySchedules(
    games_hashmap: Memo<HashMap<GameId, GameResponse>>,
    user_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let has_schedules = move || {
        user_id().is_some_and(|user_id| {
            games_hashmap.with(|games| {
                games.iter().any(|(_game_id, game)| {
                    game.game_status == GameStatus::NotStarted
                        && (game.white_player.uid == user_id || game.black_player.uid == user_id)
                        && game.conclusion == Conclusion::Unknown
                })
            })
        })
    };

    view! {
        <Show when=has_schedules>
            <MySchedulesInner
                games_hashmap
                user_id=Signal::derive(move || user_id().expect("User_id is some"))
            />
        </Show>
    }
}

#[component]
fn MySchedulesInner(
    games_hashmap: Memo<HashMap<GameId, GameResponse>>,
    user_id: Signal<Uuid>,
) -> impl IntoView {
    let ctx = expect_context::<SchedulesContext>();
    let get_game = move |game_id: GameId| games_hashmap.with(|games| games.get(&game_id).cloned());
    let get_schedules = move |game_id: GameId| {
        ctx.own.with(|own| {
            own.get(&game_id)
                .unwrap_or(&HashMap::new())
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .filter(|s| s.start_t + Duration::hours(1) > Utc::now())
        })
    };
    let my_schedules = move || {
        let mut ret = Vec::new();
        let user_id = user_id();
        ctx.own.with(|own| {
            own.iter().for_each(|(key, value)| {
                if let Some(game) = get_game(key.clone()) {
                    if game.white_player.uid == user_id || game.black_player.uid == user_id {
                        ret.push((game, value.len()))
                    }
                }
            });
        });
        ret
    };
    view! {
        <details class="w-full min-w-0 h-fit ui-panel">
            <summary class="ui-panel-summary">"My Schedules"</summary>
            <div class="space-y-2 ui-panel-body">
                <For each=my_schedules key=|g| (g.0.uuid, g.1) let:game>

                    {
                        let (gr, _) = game;
                        let game_id = Signal::derive(move || gr.game_id.clone());
                        let white_username = gr.white_player.username;
                        let black_username = gr.black_player.username;
                        let white_patreon = gr.white_player.patreon;
                        let black_patreon = gr.black_player.patreon;
                        let white_bot = gr.white_player.bot;
                        let black_bot = gr.black_player.bot;
                        let white_deleted = gr.white_player.deleted;
                        let black_deleted = gr.black_player.deleted;
                        view! {
                            <div class=with_class(
                                "ui-card-row",
                                "flex h-fit w-full flex-col justify-between gap-3 p-3",
                            )>
                                <div class="flex justify-center">
                                    <div class="flex gap-1 justify-between items-center p-1 w-fit">
                                        <ProfileLink
                                            patreon=white_patreon
                                            bot=white_bot
                                            username=white_username
                                            deleted=white_deleted
                                            extend_tw_classes="truncate max-w-[120px] "
                                        />
                                        vs.
                                        <ProfileLink
                                            patreon=black_patreon
                                            bot=black_bot
                                            username=black_username
                                            deleted=black_deleted
                                            extend_tw_classes=" truncate max-w-[120px]"
                                        />
                                    </div>
                                </div>

                                <div class="flex flex-col gap-2">
                                    <For
                                        each=move || { get_schedules(game_id()) }

                                        key=|schedule: &ScheduleResponse| (
                                            schedule.id,
                                            schedule.start_t,
                                            schedule.agreed,
                                        )

                                        let:schedule
                                    >
                                        <GameDateControls
                                            player_id=user_id()
                                            schedule=schedule.clone()
                                        />

                                    </For>
                                </div>
                                <ProposeDateControls game_id=game_id() />
                                <a
                                    class="place-self-center ui-button ui-button-primary ui-button-md no-link-style"
                                    href=format!("/game/{}", &game_id())
                                >
                                    "Join Game"
                                </a>
                            </div>
                        }
                    }

                </For>
            </div>
        </details>
    }
}
