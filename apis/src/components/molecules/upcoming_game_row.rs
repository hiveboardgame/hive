use crate::components::atoms::profile_link::ProfileLink;
use crate::components::molecules::time_row::TimeRow;
use crate::providers::AuthContext;
use crate::responses::GameResponse;
use chrono::{DateTime, Duration, Local, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TimeInfo;

#[component]
pub fn UpcomingGameRow(
    game_data: (DateTime<Utc>, GameResponse),
    current_time: RwSignal<DateTime<Local>>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let (start_time, game) = game_data;
    let local_time = start_time.with_timezone(&Local);
    let formatted_time = local_time.format("%Y-%m-%d %H:%M UTC%z").to_string();
    let white_username = StoredValue::new(game.white_player.username.clone());
    let black_username = StoredValue::new(game.black_player.username.clone());
    let tournament_name = StoredValue::new(
        game.tournament
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Unknown Tournament".to_string()),
    );
    let tournament_id = game
        .tournament
        .as_ref()
        .map(|t| t.tournament_id.0.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let time_info = TimeInfo {
        mode: game.time_mode,
        base: game.time_base,
        increment: game.time_increment,
    };

    let show_button = move || {
        let now = current_time.get();
        let time_until_start = local_time.signed_duration_since(now);
        time_until_start <= Duration::minutes(10)
    };

    let user_is_player = move || {
        auth_context.user.with(|user| {
            user.as_ref().is_some_and(|u| {
                u.username == white_username.get_value() || u.username == black_username.get_value()
            })
        })
    };

    view! {
        <div class="flex flex-col p-4 w-full rounded-lg duration-300 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="flex mb-3 w-full text-center">
                <a
                    class="overflow-hidden w-full text-lg font-bold text-blue-500 break-words hover:underline no-link-style hyphens-auto"
                    href=format!("/tournament/{}", tournament_id)
                    title=tournament_name.get_value()
                >
                    {tournament_name.get_value()}
                </a>
            </div>

            <div class="flex flex-col gap-3 items-center">
                <div class="flex gap-2 items-center">
                    <ProfileLink
                        username=white_username.get_value()
                        patreon=game.white_player.patreon
                        bot=game.white_player.bot
                        extend_tw_classes="font-semibold"
                    />
                    <span class="text-sm opacity-75">vs</span>
                    <ProfileLink
                        username=black_username.get_value()
                        patreon=game.black_player.patreon
                        bot=game.black_player.bot
                        extend_tw_classes="font-semibold"
                    />
                </div>

                <TimeRow time_info extend_tw_classes="text-sm" />

                <Show when=show_button>
                    <a
                        class="flex items-center justify-center px-3 py-1 text-sm font-medium text-white rounded no-link-style bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 min-w-[5rem]"
                        href=format!("/game/{}", game.game_id)
                    >
                        {move || {
                            if user_is_player() {
                                view! { "Join Game" }.into_any()
                            } else {
                                view! {
                                    <Icon icon=icondata_ai::AiEyeOutlined attr:class="mr-1 w-4 h-4" />
                                    "Watch"
                                }
                                    .into_any()
                            }
                        }}
                    </a>
                </Show>

                <div class="text-sm opacity-75">{formatted_time}</div>
            </div>
        </div>
    }
}
