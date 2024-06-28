use crate::components::molecules::time_row::TimeRow;
use crate::providers::AuthContext;
use crate::responses::{TournamentResponse, UserResponse};
use chrono::Local;
use leptos::*;
use shared_types::{GameSpeed, TimeInfo, TournamentStatus};

#[component]
pub fn TournamentRow(tournament: TournamentResponse) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let game_speed =
        GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
    let user_qualifies = move |user: UserResponse| {
        let rating = user.rating_for_speed(&game_speed) as i32;
        match (tournament.band_lower, tournament.band_upper) {
            (None, None) => true,
            (None, Some(upper)) => rating < upper,
            (Some(lower), None) => rating > lower,
            (Some(lower), Some(upper)) => rating > lower && rating < upper,
        }
    };
    let starts = move || {
        if matches!(tournament.status, TournamentStatus::NotStarted) {
            match tournament.start_at {
                None => "Start up to organizer".to_string(),
                Some(time) => time
                    .with_timezone(&Local)
                    .format("Starts on: %d/%m/%Y %H:%M")
                    .to_string(),
            }
        } else {
            tournament.status.pretty_string()
        }
    };
    let range = move || {
        let lower = match tournament.band_lower {
            None => "any".to_string(),
            Some(lower) => lower.to_string(),
        };

        let upper = match tournament.band_upper {
            None => "any".to_string(),
            Some(upper) => upper.to_string(),
        };

        format!("Min rating: {lower} Max rating: {upper}")
    };

    let seats_taken = format!("{}/{}", tournament.players.len(), tournament.seats);
    let time_info = TimeInfo {
        mode: tournament.time_mode,
        base: tournament.time_base,
        increment: tournament.time_increment,
    };
    view! {
        <article class="flex relative justify-between items-center px-2 py-4 mx-2 w-5/6 h-32 duration-300 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="flex flex-col">
                <div>"Tournament name: " {tournament.name}</div>
                <div>"Seats taken: " {seats_taken}</div>
            </div>
            <div class="flex flex-col">
                <TimeRow time_info/>
                <div class="flex gap-1">
                    <div>{tournament.mode}</div>
                    <div>{tournament.rounds} " Rounds"</div>
                </div>
            </div>
            <div class="flex flex-col">
                <div>{range}</div>
                <Show when=move || tournament.invite_only>
                <div>Invite only </div>
                </Show>
                <div>{starts}</div>
            </div>
            <a
                class="absolute top-0 left-0 z-10 w-full h-full"
                href=format!("/tournament/{}", tournament.tournament_id.0)
            ></a>
        </article>
    }
}
