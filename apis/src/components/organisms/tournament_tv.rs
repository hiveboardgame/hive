use crate::{
    common::format_game_result,
    components::{
        molecules::thumbnail_pieces::ThumbnailPieces,
        organisms::tournament_inspector::{player_standing, TournamentSelection},
    },
    functions::games::get::get_game_from_nanoid,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        games::GamesSignal,
        ArenaState,
        ArenaStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
    },
    responses::GameResponse,
};
use chrono::{DateTime, Utc};
use hive_lib::{Color, GameStatus};
use leptos::prelude::*;
use reactive_stores::{ArcField, Store};
use shared_types::{tournament_view::ArenaGameResponse, GameId, TimeMode, TournamentStatus};
use std::{collections::HashMap, sync::Arc};

fn arena_featured_game_id(
    status: TournamentStatus,
    featured_game_id: Option<&GameId>,
) -> Option<&GameId> {
    (status == TournamentStatus::InProgress)
        .then_some(featured_game_id)
        .flatten()
}

fn resolve_featured_game<'a, T>(
    game_id: &GameId,
    live: &'a HashMap<GameId, T>,
    own_realtime: &'a HashMap<GameId, T>,
    own_untimed: &'a HashMap<GameId, T>,
    own_correspondence: &'a HashMap<GameId, T>,
    fallback: Option<(&GameId, &'a T)>,
) -> Option<&'a T> {
    live.get(game_id)
        .or_else(|| own_realtime.get(game_id))
        .or_else(|| own_untimed.get(game_id))
        .or_else(|| own_correspondence.get(game_id))
        .or_else(|| {
            fallback
                .filter(|(id, _)| *id == game_id)
                .map(|(_, game)| game)
        })
}

#[component]
fn ArenaTvPlayerRow(
    common: Store<TournamentCommon>,
    game: Arc<GameResponse>,
    color: Color,
    now: Signal<DateTime<Utc>>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let identity_game = Arc::clone(&game);
    let clock_game = Arc::clone(&game);
    let active = arena_tv_active_color(&game) == Some(color);
    let clock_class = if active {
        "ml-auto px-2 py-1 font-mono text-lg font-bold tabular-nums rounded bg-pillbug-teal/20 shrink-0"
    } else {
        "ml-auto px-2 py-1 font-mono text-lg font-bold tabular-nums rounded bg-black/5 dark:bg-white/10 shrink-0"
    };

    view! {
        <div class="flex gap-2 items-center py-1.5 min-w-0">
            {move || {
                let player = match color {
                    Color::White => identity_game.white_player.clone(),
                    Color::Black => identity_game.black_player.clone(),
                };
                let player_id = player.uid;
                let rank = player_standing(&common.standings().get(), player.uid)
                    .map(|(rank, _)| rank);
                let name = if player.deleted {
                    t_string!(i18n, profile.deleted_user).to_string()
                } else {
                    player.username.clone()
                };
                let recorded_rating = match color {
                    Color::White => identity_game.white_rating,
                    Color::Black => identity_game.black_rating,
                }
                    .filter(|rating| rating.is_finite())
                    .map(|rating| rating.round().max(0.0) as u64);
                let rating = recorded_rating
                    .unwrap_or_else(|| player.rating_for_speed(&identity_game.speed));
                view! {
                    <span class="font-semibold tabular-nums text-gray-500 dark:text-gray-400 shrink-0">
                        {rank.map_or_else(|| String::from("—"), |rank| format!("#{rank}"))}
                    </span>
                    <button
                        type="button"
                        class="font-semibold truncate hover:text-pillbug-teal"
                        on:click=move |_| {
                            selection.set(Some(TournamentSelection::Player(player_id)))
                        }
                    >
                        {name}
                    </button>
                    <span class="text-sm italic text-gray-500 dark:text-gray-400 shrink-0">
                        {rating}
                    </span>
                }
            }}
            {move || {
                if clock_game.finished {
                    return None;
                }
                arena_tv_clock_text(&clock_game, color, now.get())
                    .map(|clock| {
                        view! { <span class=clock_class>{clock}</span> }
                    })
            }}
        </div>
    }
}

fn arena_tv_active_color(game: &GameResponse) -> Option<Color> {
    (!game.finished
        && matches!(&game.game_status, GameStatus::InProgress)
        && game.last_interaction.is_some()
        && game.arena_move_due_at.is_none())
    .then(|| {
        if game.turn.is_multiple_of(2) {
            Color::White
        } else {
            Color::Black
        }
    })
}

fn arena_tv_clock_text(game: &GameResponse, color: Color, now: DateTime<Utc>) -> Option<String> {
    if game.finished {
        return None;
    }
    if game.time_mode == TimeMode::Untimed {
        return Some(String::from("∞"));
    }
    let mut remaining = match color {
        Color::White => game.white_time_left,
        Color::Black => game.black_time_left,
    }?;
    if arena_tv_active_color(game) == Some(color) {
        let elapsed = game
            .last_interaction
            .and_then(|interaction| now.signed_duration_since(interaction).to_std().ok())
            .unwrap_or_default();
        remaining = remaining.checked_sub(elapsed).unwrap_or_default();
    }
    Some(game.time_mode.time_remaining(remaining))
}

#[component]
fn ArenaTournamentTvGame(
    common: Store<TournamentCommon>,
    game: GameResponse,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let board = StoredValue::new(game.create_state().board);
    let game = Arc::new(game);
    let now = use_ticking_now();
    let href = format!("/game/{}", game.game_id);
    let board_label_game = Arc::clone(&game);
    let result_game = Arc::clone(&game);

    view! {
        <section
            class="min-w-0"
            data-testid="tournament-tv"
            data-featured-game-id=game.game_id.0.clone()
        >
            <ArenaTvPlayerRow common game=Arc::clone(&game) color=Color::Black now selection />
            <a
                class="block overflow-hidden w-full rounded-lg border border-gray-300 shadow-sm dark:border-gray-700 aspect-square bg-even-light dark:bg-surface-row-even"
                href=href
                aria-label=move || {
                    let white = if board_label_game.white_player.deleted {
                        t_string!(i18n, profile.deleted_user).to_string()
                    } else {
                        board_label_game.white_player.username.clone()
                    };
                    let black = if board_label_game.black_player.deleted {
                        t_string!(i18n, profile.deleted_user).to_string()
                    } else {
                        board_label_game.black_player.username.clone()
                    };
                    format!("{white} {} {black}", t_string!(i18n, tournaments.view.common.versus))
                }
            >
                <ThumbnailPieces board />
            </a>
            <ArenaTvPlayerRow common game=Arc::clone(&game) color=Color::White now selection />
            {move || {
                result_game
                    .finished
                    .then(|| {
                        let result = format_game_result(i18n, &result_game)
                            .unwrap_or_else(|| {
                                t_string!(i18n, tournaments.view.slots.game.finished).to_string()
                            });
                        view! { <p class="pt-1 font-semibold text-center">{result}</p> }
                    })
            }}
        </section>
    }
}

#[component]
pub fn TournamentTv(
    common: Store<TournamentCommon>,
    arena: Store<ArenaState>,
    games: GamesSignal,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let featured_game_request = Memo::new(move |_| {
        let lifecycle = common.lifecycle().get();
        let featured_game_id = arena.featured_game_id().get();
        let game_id =
            arena_featured_game_id(lifecycle.status, featured_game_id.as_ref()).cloned()?;
        let exists = arena.games().with(|games| games.contains_key(&game_id));
        if !exists {
            return None;
        }
        let game: ArcField<ArenaGameResponse> = arena.games().at_key(game_id.clone()).into();
        let finished_at = game.try_get()?.game.finished_at;
        Some((game_id, finished_at))
    });
    let fallback_game = Resource::new(
        move || featured_game_request.get(),
        |request| async move {
            match request {
                Some(request) => {
                    let game_id = request.0.clone();
                    Some((request, get_game_from_nanoid(game_id).await.ok()))
                }
                None => None,
            }
        },
    );
    let last_fallback = RwSignal::new(None::<GameResponse>);
    Effect::new(move |_| {
        if let Some(Some((_, Some(game)))) = fallback_game.get() {
            last_fallback.set(Some(game));
        }
    });
    let featured_game = move || {
        let request = featured_game_request.get()?;
        let game_id = &request.0;
        let fallback = fallback_game
            .get()
            .flatten()
            .filter(|(loaded_request, _)| loaded_request == &request)
            .and_then(|(_, game)| game)
            .or_else(|| last_fallback.get());
        games.live.with(|live| {
            games.own.with(|own| {
                resolve_featured_game(
                    game_id,
                    &live.live_games,
                    &own.realtime,
                    &own.untimed,
                    &own.correspondence,
                    fallback.as_ref().map(|game| (&game.game_id, game)),
                )
                .cloned()
            })
        })
    };

    view! {
        {move || {
            featured_game().map(|game| view! { <ArenaTournamentTvGame common game selection /> })
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_tv_resolver_uses_live_then_each_own_bucket_then_fallback() {
        let game_id = GameId(String::from("featured"));
        let mut live = HashMap::from([(game_id.clone(), 1)]);
        let mut own_realtime = HashMap::from([(game_id.clone(), 2)]);
        let mut own_untimed = HashMap::from([(game_id.clone(), 3)]);
        let mut own_correspondence = HashMap::from([(game_id.clone(), 4)]);
        let fallback = 5;
        let resolved = |live: &HashMap<GameId, i32>,
                        own_realtime: &HashMap<GameId, i32>,
                        own_untimed: &HashMap<GameId, i32>,
                        own_correspondence: &HashMap<GameId, i32>| {
            resolve_featured_game(
                &game_id,
                live,
                own_realtime,
                own_untimed,
                own_correspondence,
                Some((&game_id, &fallback)),
            )
            .copied()
        };

        assert_eq!(
            resolved(&live, &own_realtime, &own_untimed, &own_correspondence),
            Some(1),
        );
        live.clear();
        assert_eq!(
            resolved(&live, &own_realtime, &own_untimed, &own_correspondence),
            Some(2),
        );
        own_realtime.clear();
        assert_eq!(
            resolved(&live, &own_realtime, &own_untimed, &own_correspondence),
            Some(3),
        );
        own_untimed.clear();
        assert_eq!(
            resolved(&live, &own_realtime, &own_untimed, &own_correspondence),
            Some(4),
        );
        own_correspondence.clear();
        assert_eq!(
            resolved(&live, &own_realtime, &own_untimed, &own_correspondence),
            Some(5),
        );
        assert_eq!(
            resolve_featured_game(
                &game_id,
                &live,
                &own_realtime,
                &own_untimed,
                &own_correspondence,
                None,
            ),
            None,
        );
    }

    #[test]
    fn arena_tv_rejects_a_previous_feature_as_fallback() {
        let previous_id = GameId(String::from("previous"));
        let featured_id = GameId(String::from("featured"));
        let empty = HashMap::<GameId, i32>::new();
        assert_eq!(
            resolve_featured_game(
                &featured_id,
                &empty,
                &empty,
                &empty,
                &empty,
                Some((&previous_id, &1)),
            ),
            None,
        );
    }
}
