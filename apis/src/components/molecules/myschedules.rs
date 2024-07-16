use crate::components::atoms::{
    profile_link::ProfileLink,
    schedule_controls::{GameDateControls, ProposeDateControls},
};
use crate::providers::schedules::SchedulesContext;
use crate::responses::{GameResponse, ScheduleResponse};
use chrono::{Duration, Utc};
use leptos::*;
use std::collections::HashMap;
use uuid::Uuid;

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";

#[component]
pub fn MySchedules(
    games_hashmap: Memo<HashMap<Uuid, GameResponse>>,
    user_id: Uuid,
) -> impl IntoView {
    let ctx = expect_context::<SchedulesContext>();
    let get_game = move |game_id: Uuid| games_hashmap().get(&game_id).cloned();
    let own_slice = move || ctx.own.get();
    let get_schedules = move |game_id: Uuid| {
        own_slice()
            .get(&game_id)
            //context has all my games so this should never fail
            .unwrap_or(&vec![])
            .iter()
            .filter(|s| s.start_t + Duration::hours(1) > Utc::now())
            .cloned()
            .collect::<Vec<_>>()
    };
    view! {
        <span class="font-bold text-md">My Schedules:</span>
        <For
            each=move || {
                let mut ret = Vec::new();
                own_slice()
                    .iter()
                    .for_each(|(key, value)| {
                        if let Some(game) = get_game(*key) {
                            if game.white_player.uid == user_id || game.black_player.uid == user_id
                            {
                                ret.push((game, value.len()))
                            }
                        }
                    });
                ret
            }

            key=|g| (g.0.uuid, g.1)
            let:game
        >
            <div class="flex-col p-3 w-full justify-betwween h-fit dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
                <div class="flex justify-center mb-4">
                    <div class="flex justify-between items-center w-fit">
                        <ProfileLink
                            patreon=game.0.white_player.patreon
                            username=game.0.white_player.username.clone()
                            extend_tw_classes="truncate max-w-[120px] "
                            user_is_hoverable=store_value(game.0.white_player.clone())
                        />
                        vs.
                        <ProfileLink
                            patreon=game.0.black_player.patreon
                            username=game.0.black_player.username.clone()
                            extend_tw_classes=" truncate max-w-[120px]"
                            user_is_hoverable=store_value(game.0.black_player.clone())
                        />
                    </div>
                    <a class=BUTTON_STYLE href=format!("/game/{}", &game.0.game_id)>
                        "Join Game"
                    </a>
                </div>

                <For
                    each=move || { get_schedules(game.0.uuid) }

                    key=|schedule: &ScheduleResponse| (
                        schedule.id,
                        schedule.start_t,
                        schedule.agreed,
                    )

                    let:schedule
                >
                    <GameDateControls player_id=user_id schedule=schedule.clone()/>

                </For>
                <ProposeDateControls game_id=game.0.game_id/>

            </div>
        </For>
    }
}
