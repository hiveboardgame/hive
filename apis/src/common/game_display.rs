use crate::{
    i18n::{t_string, td_string, I18nKeys, Locale},
    responses::GameResponse,
};
use hive_lib::{Color, ColorChoice, GameResult, GameStatus};
use leptos_i18n::I18nContext;
use shared_types::{Conclusion, GameSpeed, TimeMode, TournamentGameResult};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TournamentLink {
    pub name: String,
    pub href: String,
}

pub fn game_tournament_link(game: &GameResponse) -> Option<TournamentLink> {
    game.tournament.as_ref().map(|tournament| TournamentLink {
        name: tournament.name.clone(),
        href: format!("/tournament/{}", tournament.tournament_id),
    })
}

pub fn format_game_rating(i18n: I18nContext<Locale, I18nKeys>, rated: bool) -> String {
    if rated {
        t_string!(i18n, game.rated).to_string()
    } else {
        t_string!(i18n, game.casual).to_string()
    }
}

pub fn display_username(
    i18n: I18nContext<Locale, I18nKeys>,
    deleted: bool,
    username: &str,
) -> String {
    if deleted {
        t_string!(i18n, common.deleted_user).to_string()
    } else {
        username.to_string()
    }
}

pub fn game_speed_label(i18n: I18nContext<Locale, I18nKeys>, speed: GameSpeed) -> &'static str {
    match speed {
        GameSpeed::Bullet => t_string!(i18n, game.speeds.bullet),
        GameSpeed::Blitz => t_string!(i18n, game.speeds.blitz),
        GameSpeed::Rapid => t_string!(i18n, game.speeds.rapid),
        GameSpeed::Classic => t_string!(i18n, game.speeds.classic),
        GameSpeed::Correspondence => t_string!(i18n, game.speeds.correspondence),
        GameSpeed::Untimed => t_string!(i18n, game.speeds.untimed),
        GameSpeed::Puzzle => t_string!(i18n, game.speeds.puzzle),
    }
}

pub fn format_game_result(
    i18n: I18nContext<Locale, I18nKeys>,
    game: &GameResponse,
) -> Option<String> {
    let result = match (&game.game_status, &game.tournament_game_result) {
        (GameStatus::Finished(GameResult::Draw), _) => ResultSummary::Draw(DrawResult::Game),
        (GameStatus::Adjudicated, TournamentGameResult::Draw) => {
            ResultSummary::Draw(DrawResult::Tournament)
        }
        (GameStatus::Finished(GameResult::Winner(color)), _)
        | (GameStatus::Adjudicated, TournamentGameResult::Winner(color)) => {
            ResultSummary::Winner(*color)
        }
        (GameStatus::Adjudicated, TournamentGameResult::DoubleForfeit) => {
            ResultSummary::DoubleForfeit
        }
        _ => return None,
    };

    Some(match result {
        ResultSummary::Draw(result) => draw_str(i18n, result, &game.conclusion),
        ResultSummary::Winner(color) => winner_str(i18n, game, color, &game.conclusion),
        ResultSummary::DoubleForfeit => t_string!(i18n, game.result.double_forfeit).to_string(),
    })
}

#[derive(Clone, Copy)]
enum DrawResult {
    Game,
    Tournament,
}

#[derive(Clone, Copy)]
enum ResultSummary {
    Draw(DrawResult),
    Winner(Color),
    DoubleForfeit,
}

fn draw_str(
    i18n: I18nContext<Locale, I18nKeys>,
    result: DrawResult,
    conclusion: &Conclusion,
) -> String {
    let result = match result {
        DrawResult::Game => GameResult::Draw.to_string(),
        DrawResult::Tournament => TournamentGameResult::Draw.to_string(),
    };
    t_string!(
        i18n,
        game.result.draw_summary,
        result = result,
        conclusion = conclusion_label(i18n, conclusion)
    )
    .to_string()
}

fn winner_str(
    i18n: I18nContext<Locale, I18nKeys>,
    game: &GameResponse,
    color: Color,
    conclusion: &Conclusion,
) -> String {
    let winner = match color {
        Color::White => &game.white_player,
        Color::Black => &game.black_player,
    };
    let winner = display_username(i18n, winner.deleted, &winner.username);
    match conclusion {
        Conclusion::Timeout => {
            t_string!(i18n, game.result.winner_timeout, winner = winner).to_string()
        }
        Conclusion::Resigned => {
            t_string!(i18n, game.result.winner_resignation, winner = winner).to_string()
        }
        Conclusion::Board => t_string!(i18n, game.result.winner_board, winner = winner).to_string(),
        Conclusion::Committee => {
            t_string!(i18n, game.result.winner_committee, winner = winner).to_string()
        }
        Conclusion::Withdrawal => {
            t_string!(i18n, game.result.winner_withdrawal, winner = winner).to_string()
        }
        Conclusion::Forfeit => {
            t_string!(i18n, game.result.winner_forfeit, winner = winner).to_string()
        }
        _ => t_string!(i18n, game.result.winner, winner = winner).to_string(),
    }
}

pub fn conclusion_label(
    i18n: I18nContext<Locale, I18nKeys>,
    conclusion: &Conclusion,
) -> &'static str {
    match conclusion {
        Conclusion::Board => t_string!(i18n, game.conclusion.board),
        Conclusion::Committee => t_string!(i18n, game.conclusion.committee),
        Conclusion::Draw => t_string!(i18n, game.conclusion.draw),
        Conclusion::Forfeit => t_string!(i18n, game.conclusion.forfeit),
        Conclusion::Withdrawal => t_string!(i18n, common.withdrawal),
        Conclusion::Repetition => t_string!(i18n, game.conclusion.repetition),
        Conclusion::Resigned => t_string!(i18n, game.conclusion.resigned),
        Conclusion::Timeout => t_string!(i18n, game.conclusion.timeout),
        Conclusion::Unknown => t_string!(i18n, common.unknown),
    }
}

pub fn game_result_label(i18n: I18nContext<Locale, I18nKeys>, result: &GameResult) -> String {
    match result {
        GameResult::Unknown => t_string!(i18n, common.unknown).to_string(),
        _ => result.to_string(),
    }
}

pub fn tournament_game_result_label(
    i18n: I18nContext<Locale, I18nKeys>,
    result: &TournamentGameResult,
) -> String {
    match result {
        TournamentGameResult::Unknown => t_string!(i18n, common.unknown).to_string(),
        _ => result.to_string(),
    }
}

pub fn color_choice_label(
    i18n: I18nContext<Locale, I18nKeys>,
    color: &ColorChoice,
) -> &'static str {
    match color {
        ColorChoice::White => t_string!(i18n, common.white),
        ColorChoice::Black => t_string!(i18n, common.black),
        ColorChoice::Random => t_string!(i18n, common.random),
    }
}

pub fn format_clock_time_remaining(
    i18n: I18nContext<Locale, I18nKeys>,
    mode: TimeMode,
    left: Duration,
) -> String {
    let total = left.as_secs();
    let days = total / 86400;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    if mode == TimeMode::Untimed {
        String::new()
    } else if days > 0 {
        if days > 1 || hours == 24 {
            t_string!(i18n, game.clock.days, days = days).to_string()
        } else {
            t_string!(i18n, game.clock.days_hours, days = days, hours = hours % 24).to_string()
        }
    } else if hours > 0 {
        t_string!(
            i18n,
            game.clock.hours_minutes,
            hours = hours,
            minutes = minutes
        )
        .to_string()
    } else {
        mode.time_remaining(left)
    }
}

pub fn result_marker_label(i18n: I18nContext<Locale, I18nKeys>, marker: &str) -> &str {
    match marker {
        "W" => t_string!(i18n, common.short_win),
        "D" => t_string!(i18n, common.short_draw),
        "L" => t_string!(i18n, common.short_loss),
        "B" => t_string!(i18n, common.short_bye),
        _ => marker,
    }
}

pub fn game_speed_label_for_locale(locale: Locale, speed: GameSpeed) -> &'static str {
    match speed {
        GameSpeed::Bullet => td_string!(locale, game.speeds.bullet),
        GameSpeed::Blitz => td_string!(locale, game.speeds.blitz),
        GameSpeed::Rapid => td_string!(locale, game.speeds.rapid),
        GameSpeed::Classic => td_string!(locale, game.speeds.classic),
        GameSpeed::Correspondence => td_string!(locale, game.speeds.correspondence),
        GameSpeed::Untimed => td_string!(locale, game.speeds.untimed),
        GameSpeed::Puzzle => td_string!(locale, game.speeds.puzzle),
    }
}
