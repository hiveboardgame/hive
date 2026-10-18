use crate::{
    common::{display_username, format_game_rating, format_time_control, game_speed_label},
    functions::games::get::get_game_from_nanoid,
    i18n::*,
    responses::GameResponse,
};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use leptos_meta::Meta;
use leptos_router::hooks::use_location;
use shared_types::{rounded_rating, GameId};

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

fn meta_tags(
    title: Signal<String>,
    description: Signal<String>,
    url: String,
    image: String,
) -> impl IntoView {
    view! {
        <Meta name="description" content=move || description.get() />
        <Meta property="og:url" content=url.clone() />
        <Meta property="og:type" content="website" />
        <Meta property="og:title" content=move || title.get() />
        <Meta property="og:description" content=move || description.get() />
        <Meta property="og:image" content=image.clone() />
        <Meta name="twitter:card" content="summary_large_image" />
        <Meta property="twitter:domain" content="hivegame.com" />
        <Meta property="twitter:url" content=url />
        <Meta name="twitter:title" content=move || title.get() />
        <Meta name="twitter:description" content=move || description.get() />
        <Meta name="twitter:image" content=image />
    }
}

fn default_meta() -> impl IntoView {
    let i18n = use_i18n();
    let (nanoid, turn) = pick_iconic_game();
    let image = match turn {
        Some(turn) => format!("https://hivegame.com/og/game/{nanoid}.png?turn={turn}"),
        None => format!("https://hivegame.com/og/game/{nanoid}.png"),
    };
    meta_tags(
        Signal::derive(move || t_string!(i18n, game.site_title).to_string()),
        Signal::derive(move || t_string!(i18n, game.site_description).to_string()),
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
    let i18n = use_i18n();
    let nanoid = game.game_id.0.clone();
    let (white_deleted, white_username, white_rating) = (
        game.white_player.deleted,
        game.white_player.username.clone(),
        rating(game.white_rating),
    );
    let (black_deleted, black_username, black_rating) = (
        game.black_player.deleted,
        game.black_player.username.clone(),
        rating(game.black_rating),
    );
    let title = Signal::derive(move || {
        let white = format!(
            "{}{}",
            display_username(i18n, white_deleted, &white_username),
            white_rating
        );
        let black = format!(
            "{}{}",
            display_username(i18n, black_deleted, &black_username),
            black_rating
        );
        t_string!(i18n, common.versus_players, white = white, black = black).to_string()
    });
    let description = Signal::derive(move || game_description(i18n, &game));
    meta_tags(
        title,
        description,
        format!("https://hivegame.com/game/{nanoid}"),
        format!("https://hivegame.com/og/game/{nanoid}.png"),
    )
}

fn rating(rating: Option<f64>) -> String {
    match rating {
        Some(rating) => rounded_rating(rating)
            .map(|rating| format!(" ({rating})"))
            .unwrap_or_default(),
        None => String::new(),
    }
}

fn game_description(i18n: I18nContext<Locale, I18nKeys>, game: &GameResponse) -> String {
    let mut parts = Vec::new();
    if let Some(clock) = game.time_control() {
        parts.push(format_time_control(i18n, Some(clock)));
    }
    parts.push(game_speed_label(i18n, game.speed).to_string());
    parts.push(format_game_rating(i18n, game.rated));
    parts.push(status_text(i18n, game));
    parts.join(" · ")
}

fn status_text(i18n: I18nContext<Locale, I18nKeys>, game: &GameResponse) -> String {
    match &game.game_status {
        GameStatus::Finished(GameResult::Winner(Color::White)) => {
            t_string!(i18n, game.tournament_result.white_won)
        }
        GameStatus::Finished(GameResult::Winner(Color::Black)) => {
            t_string!(i18n, game.tournament_result.black_won)
        }
        GameStatus::Finished(GameResult::Draw) => t_string!(i18n, game.tournament_result.draw),
        GameStatus::Finished(_) | GameStatus::Adjudicated => t_string!(i18n, common.finished),
        _ => t_string!(i18n, common.in_progress),
    }
    .to_string()
}
