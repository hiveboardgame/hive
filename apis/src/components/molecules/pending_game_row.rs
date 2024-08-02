use crate::components::atoms::profile_link::ProfileLink;
use crate::responses::GameResponse;
use chrono::{DateTime, Local, Utc};
use leptos::*;

#[component]
pub fn PendingGameRow(schedule: Option<DateTime<Utc>>, game: GameResponse) -> impl IntoView {
    let date_str = if let Some(time) = schedule {
        format!(
            "Scheduled at {}",
            time.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
        )
    } else {
        "Not yet scheduled".to_owned()
    };
    view! {
        <div class="flex flex-col items-center p-3 w-full sm:flex-row sm:justify-between min-w-fit dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <div class="flex flex-col items-center h-fit w-fit">
                <div class="flex gap-1 items-center p-1">
                    <div class="flex shrink">
                        <ProfileLink
                            patreon=game.white_player.patreon
                            username=game.white_player.username.clone()
                            extend_tw_classes="truncate max-w-[120px]"
                        />
                        {format!("({})", game.white_rating())}
                    </div>
                    vs.
                    <div class="flex shrink">
                        <ProfileLink
                            patreon=game.black_player.patreon
                            username=game.black_player.username.clone()
                            extend_tw_classes="truncate max-w-[120px]"
                        />
                        {format!("({})", game.black_rating())}
                    </div>
                </div>
                <div class=format!("flex {}", if schedule.is_some() {"font-bold"} else {""})>{date_str}</div>
            </div>
            <a
                class="flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                href=format!("/game/{}", &game.game_id)
            >
                "Join Game"
            </a>
        </div>
    }
}
