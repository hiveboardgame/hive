use crate::{functions::games::get::get_game_from_nanoid, responses::GameResponse};
use hudsoni::{Color, GameResult, GameStatus};
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
                    .map(|game| match game {
                        Some(game) => game_meta(game).into_any(),
                        None => default_meta().into_any(),
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
