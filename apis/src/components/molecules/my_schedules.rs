use crate::providers::schedules::SchedulesContext;
use crate::responses::GameResponse;
use crate::responses::ScheduleResponse;
use crate::{
    components::atoms::{
        profile_link::ProfileLink,
        schedule_controls::{GameDateControls, ProposeDateControls},
    },
    pages::tournament::INFO_STYLE,
};
use chrono::{Duration, Utc};
use hive_lib::GameStatus;
use leptos::prelude::*;
use shared_types::Conclusion;
use shared_types::GameId;
use std::collections::HashMap;
use uuid::Uuid;

#[component]
pub fn MySchedules(
    games_hashmap: Memo<HashMap<GameId, GameResponse>>,
    user_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let has_schedules = move || {
        user_id().is_some_and(|user_id| {
            games_hashmap.get().iter().any(|(_game_id, game)| {
                game.game_status == GameStatus::NotStarted
                    && (game.white_player.uid == user_id || game.black_player.uid == user_id)
                    && game.conclusion == Conclusion::Unknown
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
    let get_game = move |game_id: GameId| games_hashmap().get(&game_id).cloned();
    let own_slice = move || ctx.own.get();
    let get_schedules = move |game_id: GameId| {
        own_slice()
            .get(&game_id)
            .unwrap_or(&HashMap::new())
            .values()
            .filter(|s| s.start_t + Duration::hours(1) > Utc::now())
            .cloned()
            .collect::<Vec<ScheduleResponse>>()
    };
    let my_schedules = move || {
        let mut ret = Vec::new();
        let user_id = user_id();
        own_slice().iter().for_each(|(key, value)| {
            if let Some(game) = get_game(key.clone()) {
                if game.white_player.uid == user_id || game.black_player.uid == user_id {
                    ret.push((game, value.len()))
                }
            }
        });
        ret
    };
    view! {
        <details class="m-2 w-80">
            <summary class=INFO_STYLE>My Schedules:</summary>
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
                    view! {
                        <div class="flex flex-col justify-between p-3 w-full h-fit dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
                            <div class="flex justify-center mb-4">
                                <div class="flex gap-1 justify-between items-center p-1 w-fit">
                                    <ProfileLink
                                        patreon=white_patreon
                                        bot=white_bot
                                        username=white_username
                                        extend_tw_classes="truncate max-w-[120px] "
                                    />
                                    vs.
                                    <ProfileLink
                                        patreon=black_patreon
                                        bot=black_bot
                                        username=black_username
                                        extend_tw_classes=" truncate max-w-[120px]"
                                    />
                                </div>
                            </div>

                            <For
                                each=move || { get_schedules(game_id()) }

                                key=|schedule: &ScheduleResponse| (
                                    schedule.id,
                                    schedule.start_t,
                                    schedule.agreed,
                                )

                                let:schedule
                            >
                                <GameDateControls player_id=user_id() schedule=schedule.clone() />

                            </For>
                            <ProposeDateControls game_id=game_id() />
                            <a
                                class="flex gap-1 justify-center items-center place-self-center px-4 py-2 w-2/5 font-bold text-white rounded no-link-style bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                                href=format!("/game/{}", &game_id())
                            >
                                "Join Game"
                            </a>
                        </div>
                    }
                }

            </For>
        </details>
    }
}
