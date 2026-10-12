use crate::{
    functions::{
        games::get::get_game_from_nanoid,
        puzzles::{daily_puzzle, get_puzzle},
    },
    responses::{GameResponse, PuzzleResponse},
};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::prelude::*;
use leptos_meta::Meta;
use leptos_router::hooks::use_location;
use shared_types::GameId;

const SITE_TITLE: &str = "HiveGame.com • Play Hive Online";
const SITE_DESCRIPTION: &str = "The best place to play Hive online — the strategy game with no board. Free, no ads, with rated games, tournaments, and play against friends, random opponents or bots.";

/// Real positions keep generic link previews specific without another image
/// pipeline.
const ICONIC_GAMES: &[(&str, Option<usize>)] = &[
    ("iQldhayhc_NC", Some(9)),
    ("tjjdVF2ETE_Q", Some(32)),
    ("taD7qPxjU_k6", Some(41)),
    ("qEPb0RhDx8w4", Some(31)),
    ("oT_MZ9lQ-An5", Some(72)),
];

/// One owner prevents duplicate head tags and lets crawlers get SSR game cards.
/// The document title stays with the `Title` atom; cards read `og:title`.
#[component]
pub fn OG() -> impl IntoView {
    let location = use_location();
    let game_id = Memo::new(move |_| {
        location
            .pathname
            .get()
            .strip_prefix("/game/")
            .map(|rest| rest.split('/').next().unwrap_or(rest).to_string())
            .filter(|nanoid| !nanoid.is_empty())
            .map(GameId)
    });

    let puzzle_id = Memo::new(move |_| {
        location
            .pathname
            .get()
            .strip_prefix("/puzzles/")
            .filter(|rest| !rest.is_empty() && !rest.contains('/'))
            .filter(|rest| !matches!(*rest, "history" | "browse" | "daily"))
            .map(str::to_string)
    });
    let puzzle = Resource::new_blocking(
        move || puzzle_id.get(),
        |puzzle_id| async move {
            match puzzle_id {
                Some(id) => get_puzzle(id).await.ok(),
                None => None,
            }
        },
    );

    let puzzle_section = Memo::new(move |_| {
        let pathname = location.pathname.get();
        matches!(
            pathname.as_str(),
            "/puzzles" | "/puzzles/daily" | "/puzzles/browse" | "/puzzles/history"
        )
        .then_some(pathname)
    });
    let daily = Resource::new_blocking(
        move || puzzle_section.get(),
        |section| async move {
            let section = section?;
            let daily = daily_puzzle().await.ok().flatten()?;
            Some((section, daily))
        },
    );

    let game = Resource::new_blocking(
        move || game_id.get(),
        |game_id| async move {
            match game_id {
                Some(id) => get_game_from_nanoid(id).await.ok(),
                None => None,
            }
        },
    );

    view! {
        <Suspense>
            {move || {
                game.get()
                    .zip(puzzle.get())
                    .zip(daily.get())
                    .map(|((game, puzzle), daily)| match (game, puzzle, daily) {
                        (Some(game), _, _) => game_meta(game).into_any(),
                        (None, Some(puzzle), _) => {
                            let url = format!("https://hivegame.com/puzzles/{}", puzzle.nanoid);
                            puzzle_meta(puzzle, url).into_any()
                        }
                        (None, None, Some((section, daily))) if section == "/puzzles/daily" => {
                            puzzle_meta(
                                    daily.puzzle,
                                    "https://hivegame.com/puzzles/daily".to_string(),
                                )
                                .into_any()
                        }
                        (None, None, Some((section, daily))) => {
                            puzzles_meta(section, daily.puzzle).into_any()
                        }
                        (None, None, None) => default_meta().into_any(),
                    })
            }}
        </Suspense>
    }
}

fn meta_tags(title: String, description: String, url: String, image: String) -> impl IntoView {
    view! {
        <Meta name="description" content=description.clone() />
        <Meta property="og:url" content=url.clone() />
        <Meta property="og:type" content="website" />
        <Meta property="og:title" content=title.clone() />
        <Meta property="og:description" content=description.clone() />
        <Meta property="og:image" content=image.clone() />
        <Meta name="twitter:card" content="summary_large_image" />
        <Meta property="twitter:domain" content="hivegame.com" />
        <Meta property="twitter:url" content=url />
        <Meta name="twitter:title" content=title />
        <Meta name="twitter:description" content=description />
        <Meta name="twitter:image" content=image />
    }
}

fn default_meta() -> impl IntoView {
    let (nanoid, turn) = pick_iconic_game();
    let image = match turn {
        Some(turn) => format!("https://hivegame.com/og/game/{nanoid}.png?turn={turn}"),
        None => format!("https://hivegame.com/og/game/{nanoid}.png"),
    };
    meta_tags(
        SITE_TITLE.to_string(),
        SITE_DESCRIPTION.to_string(),
        "https://hivegame.com".to_string(),
        image,
    )
}

/// An atomic counter is fair enough for rotation and avoids RNG cfg splits.
fn pick_iconic_game() -> (&'static str, Option<usize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    if ICONIC_GAMES.is_empty() {
        return ("", None);
    }
    let index = NEXT.fetch_add(1, Ordering::Relaxed) % ICONIC_GAMES.len();
    ICONIC_GAMES[index]
}

fn game_meta(game: GameResponse) -> impl IntoView {
    let nanoid = &game.game_id.0;
    let title = format!(
        "{}{} vs {}{}",
        game.white_player.username,
        rating(game.white_rating),
        game.black_player.username,
        rating(game.black_rating),
    );
    meta_tags(
        title,
        game_description(&game),
        format!("https://hivegame.com/game/{nanoid}"),
        format!("https://hivegame.com/og/game/{nanoid}.png"),
    )
}

fn puzzle_task(puzzle: &PuzzleResponse) -> String {
    let side = shared_types::start_state(
        &puzzle.game_type,
        puzzle.history.as_deref(),
        puzzle.position.as_deref(),
    )
    .map(|state| match state.turn_color {
        Color::White => "White",
        Color::Black => "Black",
    })
    .unwrap_or("White");
    let moves = if puzzle.solver_moves == 1 {
        "1 move".to_string()
    } else {
        format!("{} moves", puzzle.solver_moves)
    };
    format!("{side} to win in {moves}")
}

fn puzzle_facts(puzzle: &PuzzleResponse) -> String {
    let mut facts = vec![format!("Difficulty {}", puzzle.rating)];
    if puzzle.plays >= 10 {
        facts.push(format!(
            "{}% of players solve it",
            (puzzle.solves * 100 / puzzle.plays).clamp(0, 100)
        ));
    }
    facts.join(" · ")
}

fn puzzle_meta(puzzle: PuzzleResponse, url: String) -> impl IntoView {
    let nanoid = &puzzle.nanoid;
    let (title, description, image) = match puzzle.daily_day {
        Some(day) => (
            format!("Puzzle of the day: {}", puzzle_task(&puzzle)),
            format!(
                "The Hive puzzle for {}. Can you find the forced win? {} · Free on HiveGame.com",
                day.format("%A %-d %B"),
                puzzle_facts(&puzzle)
            ),
            format!("https://hivegame.com/og/puzzle/{nanoid}.png?day={day}"),
        ),
        None => (
            format!("{} · Hive puzzle", puzzle_task(&puzzle)),
            format!(
                "Can you find the forced win? {} · Free on HiveGame.com",
                puzzle_facts(&puzzle)
            ),
            format!("https://hivegame.com/og/puzzle/{nanoid}.png"),
        ),
    };
    meta_tags(title, description, url, image)
}

fn puzzles_meta(section: String, daily: PuzzleResponse) -> impl IntoView {
    let image = match daily.daily_day {
        Some(day) => format!(
            "https://hivegame.com/og/puzzle/{}.png?day={day}",
            daily.nanoid
        ),
        None => format!("https://hivegame.com/og/puzzle/{}.png", daily.nanoid),
    };
    meta_tags(
        "Hive puzzles · HiveGame.com".to_string(),
        "Find the forced win: free Hive puzzles from real games and the Hive Puzzles book, a puzzle of the day, and a puzzle rating that grows with you.".to_string(),
        format!("https://hivegame.com{section}"),
        image,
    )
}

fn rating(rating: Option<f64>) -> String {
    match rating {
        Some(rating) => format!(" ({})", rating.round() as i64),
        None => String::new(),
    }
}

fn game_description(game: &GameResponse) -> String {
    let mut parts = Vec::new();
    if let (Some(base), Some(increment)) = (game.time_base, game.time_increment) {
        parts.push(format!("{}+{}", base / 60, increment));
    }
    parts.push(game.speed.to_string());
    parts.push(if game.rated { "Rated" } else { "Casual" }.to_string());
    parts.push(status_text(game));
    parts.join(" · ")
}

fn status_text(game: &GameResponse) -> String {
    match &game.game_status {
        GameStatus::Finished(GameResult::Winner(Color::White)) => "White won",
        GameStatus::Finished(GameResult::Winner(Color::Black)) => "Black won",
        GameStatus::Finished(GameResult::Draw) => "Draw",
        GameStatus::Finished(_) | GameStatus::Adjudicated => "Finished",
        _ => "In progress",
    }
    .to_string()
}
