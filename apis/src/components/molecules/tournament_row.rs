use crate::components::molecules::time_row::TimeRow;
use crate::responses::TournamentResponse;
use chrono::Local;
use leptos::*;
use shared_types::{TimeInfo, TournamentStatus};

#[component]
pub fn TournamentRow(tournament: TournamentResponse) -> impl IntoView {
    let starts = move || {
        if matches!(tournament.status, TournamentStatus::NotStarted) {
            match tournament.starts_at {
                None => "Start up to organizer".to_string(),
                Some(time) => time
                    .with_timezone(&Local)
                    .format("Start: %d/%m/%Y %H:%M")
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

        format!("Elo: {lower}-{upper}")
    };

    let seats_taken = format!("{}/{} players", tournament.players.len(), tournament.seats);
    let time_info = TimeInfo {
        mode: tournament.time_mode,
        base: tournament.time_base,
        increment: tournament.time_increment,
    };
    view! {
        <article class="flex relative flex-col justify-between items-center px-2 py-4 mx-2 w-5/6 h-32 duration-300 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="flex justify-center w-full font-bold break-words">{tournament.name}</div>
            <div class="flex flex-row justify-between w-full">
                <div class="flex flex-col">
                    <div class="flex gap-1">
                        <div>{tournament.mode}</div>
                    </div>
                    <TimeRow time_info/>
                    <div>{seats_taken}</div>
                </div>
                <div class="flex flex-col">
                    <div>{range}</div>
                    <Show when=move || tournament.invite_only>
                        <div>Invite only</div>
                    </Show>
                    <div>{starts}</div>
                </div>
            </div>
            <a
                class="absolute top-0 left-0 z-10 w-full h-full"
                href=format!("/tournament/{}", tournament.tournament_id.0)
            ></a>
        </article>
    }
}
