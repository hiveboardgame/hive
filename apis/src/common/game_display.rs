use crate::{
    i18n::{t_string, I18nKeys, Locale},
    responses::GameResponse,
};
use hudsoni::{Color, GameResult, GameStatus};
use leptos_i18n::I18nContext;
use shared_types::{Conclusion, PrettyString, TimeInfo, TimeMode, TournamentGameResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TournamentLink {
    pub name: String,
    pub href: String,
}

pub fn untimed_time_info() -> TimeInfo {
    TimeInfo {
        mode: TimeMode::Untimed,
        base: None,
        increment: None,
    }
}

pub fn game_time_info(game: &GameResponse) -> TimeInfo {
    TimeInfo {
        mode: game.time_mode,
        base: game.time_base,
        increment: game.time_increment,
    }
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
        (GameStatus::Adjudicated, TournamentGameResult::DoubeForfeit) => {
            ResultSummary::DoubleForfeit
        }
        _ => return None,
    };

    Some(match result {
        ResultSummary::Draw(result) => draw_str(result, &game.conclusion),
        ResultSummary::Winner(color) => winner_str(i18n, game, color, &game.conclusion),
        ResultSummary::DoubleForfeit => "The game ended as a double forfeit".to_string(),
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

fn draw_str(result: DrawResult, conclusion: &Conclusion) -> String {
    match result {
        DrawResult::Game => format!("{} {}", GameResult::Draw, conclusion.pretty_string()),
        DrawResult::Tournament => format!(
            "{} {}",
            TournamentGameResult::Draw,
            conclusion.pretty_string()
        ),
    }
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
    let winner_username = if winner.deleted {
        t_string!(i18n, profile.deleted_user).to_string()
    } else {
        winner.username.clone()
    };

    let additional_info = match conclusion {
        Conclusion::Timeout => "by timeout",
        Conclusion::Resigned => "by resignation",
        Conclusion::Board => "on the board",
        Conclusion::Committee => "by committee decision",
        _ => "",
    };

    format!("{winner_username} won {additional_info}")
}
