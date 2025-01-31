use crate::i18n::*;
use crate::{components::molecules::time_row::TimeRow, providers::game_state::GameStateSignal};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::prelude::*;
use shared_types::{Conclusion, PrettyString, TimeInfo, TournamentGameResult, TournamentId};

#[component]
pub fn GameInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateSignal>();
    let game_info = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| {
            (
                TimeInfo {
                    mode: gr.time_mode,
                    base: gr.time_base,
                    increment: gr.time_increment,
                },
                gr.rated,
            )
        })
    });
    let game_status = create_read_slice(game_state.signal, |gs| gs.state.game_status.clone());
    let tournament_game_result = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map(|gr| gr.tournament_game_result.clone())
    });
    let game_conclusion = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| gr.conclusion.clone())
    });
    let white_username = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map(|gr| gr.white_player.username.clone())
            .unwrap_or_default()
    });
    let black_username = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map(|gr| gr.black_player.username.clone())
            .unwrap_or_default()
    });
    let tournament_info = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| {
            (
                gr.tournament.is_some(),
                gr.tournament.as_ref().map(|t| t.name.clone()),
                gr.tournament.as_ref().map(|t| t.tournament_id.clone()),
            )
        })
    });
    let winner_str = move |color: Color, conclusion: Conclusion| {
        let winner_username = match color {
            Color::White => white_username(),
            Color::Black => black_username(),
        };

        let additional_info = match conclusion {
            Conclusion::Timeout => "by timeout",
            Conclusion::Resigned => "by resignation",
            Conclusion::Board => "on the board",
            Conclusion::Committee => "by committee decision",
            _ => "",
        };

        format!("{} won {}", winner_username, additional_info)
    };
    let game_result_str = move || match (game_status(), tournament_game_result()) {
        (GameStatus::Finished(result), _) => {
            let conclusion = game_conclusion();

            match (result, conclusion) {
                (GameResult::Draw, Some(conclusion)) => {
                    format!("{} {}", GameResult::Draw, conclusion.pretty_string())
                }
                (GameResult::Winner(color), Some(conclusion)) => winner_str(color, conclusion),
                _ => String::new(),
            }
        }
        (GameStatus::NotStarted, Some(result)) => {
            let conclusion = game_conclusion();

            match (result, conclusion) {
                (TournamentGameResult::Draw, Some(conclusion)) => {
                    format!(
                        "{} {}",
                        TournamentGameResult::Draw,
                        conclusion.pretty_string()
                    )
                }
                (TournamentGameResult::Winner(color), Some(conclusion)) => {
                    winner_str(color, conclusion)
                }
                (TournamentGameResult::DoubeForfeit, _) => {
                    "The game ended as a double forfeit".to_string()
                }
                _ => String::new(),
            }
        }
        _ => String::new(),
    };
    move || {
        if let (Some((time_info, rated)), Some((is_tournament, name, nanoid))) =
            (game_info(), tournament_info())
        {
            let rated = if rated {
                t!(i18n, game.rated).into_any()
            } else {
                t!(i18n, game.casual).into_any()
            };
            let name = Signal::derive(move || name.clone());
            let name = move || {
                if let Some(name) = name() {
                    format!("played in {}", name)
                } else {
                    String::new()
                }
            };
            let nanoid = Signal::derive(move || nanoid.clone());
            let link = move || {
                if let Some(TournamentId(id)) = nanoid() {
                    format!("/tournament/{}", id)
                } else {
                    String::new()
                }
            };
            view! {
                <div class=extend_tw_classes>
                    <div class="flex gap-1 items-center">
                        <TimeRow time_info=time_info.into() extend_tw_classes="whitespace-nowrap" />
                        <div>{rated}</div>
                        <Show when=move || is_tournament>
                            <a href=link>{name()}</a>
                        </Show>
                        <Show when=move || {
                            matches!(game_status(), GameStatus::Finished(_))
                                || tournament_game_result().is_some()
                        }>
                            <div>{game_result_str}</div>
                        </Show>
                    </div>
                </div>
            }
            .into_any()
        } else {
            view! { "" }.into_any()
        }
    }
}
