use crate::{
    common::{display_username, format_clock_time_remaining, format_game_result, TvInterest},
    components::{
        atoms::rating::Rating,
        molecules::thumbnail_pieces::ThumbnailPieces,
        organisms::tournament_inspector::{player_standing, TournamentSelection},
    },
    functions::games::get::get_game_from_nanoid,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        games::GamesSignal,
        websocket::use_tv_subscription,
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
use leptos_i18n::I18nContext;
use reactive_stores::{ArcField, Store};
use shared_types::{
    rounded_rating,
    tournament_view::ArenaGameResponse,
    GameId,
    TimeMode,
    TournamentStatus,
};
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
                let name = display_username(i18n, player.deleted, &player.username);
                let recorded_rating = match color {
                    Color::White => identity_game.white_rating,
                    Color::Black => identity_game.black_rating,
                };
                let rating = recorded_rating
                    .and_then(rounded_rating)
                    .map(u64::from)
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
                        <Rating value=rating />
                    </span>
                }
            }}
            {move || {
                if clock_game.finished {
                    return None;
                }
                arena_tv_clock_text(i18n, &clock_game, color, now.get())
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

fn arena_tv_clock_text(
    i18n: I18nContext<Locale, I18nKeys>,
    game: &GameResponse,
    color: Color,
    now: DateTime<Utc>,
) -> Option<String> {
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
    Some(format_clock_time_remaining(i18n, game.time_mode, remaining))
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
                    let white = display_username(
                        i18n,
                        board_label_game.white_player.deleted,
                        &board_label_game.white_player.username,
                    );
                    let black = display_username(
                        i18n,
                        board_label_game.black_player.deleted,
                        &board_label_game.black_player.username,
                    );
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
    use_tv_subscription(Signal::derive(move || {
        featured_game_request
            .get()
            .map(|(game_id, _)| TvInterest::Game(game_id))
            .unwrap_or(TvInterest::None)
    }));
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
    let featured_game = Memo::new_with_compare(
        move |_| {
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
                    .filter(|game| !game.history.is_empty())
                    .cloned()
                })
            })
        },
        |previous, next| {
            match (
                previous.and_then(Option::as_ref),
                next.and_then(Option::as_ref),
            ) {
                (Some(previous), Some(next)) => {
                    // GameResponse equality omits clock-only changes. Player records
                    // can also refresh independently of the persisted game revision.
                    previous.game_id != next.game_id
                        || previous.updated_at != next.updated_at
                        || previous.white_player != next.white_player
                        || previous.black_player != next.black_player
                }
                (None, None) => false,
                _ => true,
            }
        },
    );

    view! {
        <Transition fallback=|| ()>
            {move || {
                #[cfg(feature = "ssr")] fallback_game.with(|_| ());
                featured_game
                    .get()
                    .map(|game| {
                        // SSR must wait for the fetch; hydration renders through the compared memo.
                        view! { <ArenaTournamentTvGame common game selection /> }
                    })
            }}
        </Transition>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ssr")]
    mod ssr {
        use super::*;
        use crate::{
            components::organisms::tournament_overview_rail::TournamentOverviewRail,
            providers::{provide_config, TournamentFormatState, TournamentState},
            responses::{TournamentLifecycleDetails, TournamentMemberships, TournamentStandings},
        };
        use actix_web::{rt::System, test as actix_test, web::Data, App};
        use db_lib::{
            get_conn,
            models::{Challenge, Game, NewChallenge, NewGame, NewUser, User},
            DbPool,
        };
        use hive_lib::{ColorChoice, GameType};
        use leptos_actix::{generate_route_list, LeptosRoutes};
        use leptos_router::{
            components::{Route, Router, Routes},
            path,
        };
        use shared_types::{
            tournament::{arena::Config as ArenaConfig, BotAdmission},
            tournament_view::CompactTournamentGameResponse,
            ChallengeDetails,
            ChallengeVisibility,
            GameSpeed,
            GameStart,
            RealtimeClock,
            TournamentId,
        };
        use std::{num::NonZeroU32, time::Duration};
        use uuid::Uuid;

        mod test_database {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../db/tests/common/test_db_support.rs"
            ));
        }

        #[component]
        fn AsyncArena(
            #[prop(default = GameId(String::from("featured")))] game_id: GameId,
        ) -> impl IntoView {
            provide_config();
            let timestamp = DateTime::from_timestamp(0, 0).unwrap();
            let common = Store::new(TournamentCommon {
                lifecycle: TournamentLifecycleDetails {
                    tournament_id: TournamentId(String::from("ssr-arena")),
                    name: String::from("SSR Arena"),
                    description: None,
                    status: TournamentStatus::InProgress,
                    seats: None,
                    min_seats: 2,
                    invite_only: false,
                    band_upper: None,
                    band_lower: None,
                    starts_at: Some(timestamp),
                    started_at: Some(timestamp),
                    finished_at: None,
                    created_at: timestamp,
                    start_setup: None,
                },
                memberships: TournamentMemberships::default(),
                standings: TournamentStandings::default(),
                bot_admission: BotAdmission::HumansAndBots,
            });
            let arena = Store::new(ArenaState {
                configuration: ArenaConfig::new(
                    NonZeroU32::new(3600).unwrap(),
                    RealtimeClock {
                        base_seconds: NonZeroU32::new(300).unwrap(),
                        increment_seconds: 0,
                    },
                ),
                games: HashMap::from([(
                    game_id.clone(),
                    ArenaGameResponse {
                        ordinal: 0,
                        game: CompactTournamentGameResponse {
                            game_id: game_id.clone(),
                            participants: [Uuid::from_u128(1), Uuid::from_u128(2)],
                            finished: false,
                            status: GameStatus::InProgress,
                            start: GameStart::Arena,
                            speed: GameSpeed::Blitz,
                            ratings: [Some(1500); 2],
                            berserked: [false; 2],
                            finished_at: None,
                        },
                        outcome: None,
                        awarded_points: None,
                        doubled: None,
                    },
                )]),
                player_stats: HashMap::new(),
                featured_game_id: Some(game_id),
            });
            arena.games().update_keys();
            arena.player_stats().update_keys();
            let games = GamesSignal::new(Signal::derive(|| None));
            provide_context(games);
            let selection = RwSignal::new(None);
            let tournament = Resource::new(
                || (),
                |_| async {
                    tokio::task::yield_now().await;
                },
            );
            let state = TournamentState {
                common,
                format: TournamentFormatState::Arena(arena),
            };
            view! {
                <Transition fallback=|| ()>
                    {move || {
                        tournament
                            .get()
                            .map(|()| {
                                view! { <TournamentOverviewRail tournament=state selection /> }
                            })
                    }}
                </Transition>
            }
        }

        async fn render_arena(pool: DbPool, game_id: GameId) -> String {
            tokio::task::spawn_blocking(move || {
                System::new().block_on(async move {
                    let app_fn = move || {
                        let game_id = game_id.clone();
                        view! {
                            <I18nContextProvider>
                                <Router>
                                    <Routes fallback=|| ()>
                                        <Route
                                            path=path!("/")
                                            view=move || {
                                                view! { <AsyncArena game_id=game_id.clone() /> }
                                            }
                                        />
                                    </Routes>
                                </Router>
                            </I18nContextProvider>
                        }
                    };
                    let routes = generate_route_list(app_fn.clone());
                    let app = actix_test::init_service(
                        App::new()
                            .app_data(Data::new(pool))
                            .leptos_routes(routes, app_fn),
                    )
                    .await;
                    let response = actix_test::call_service(
                        &app,
                        actix_test::TestRequest::get().uri("/").to_request(),
                    )
                    .await;
                    assert!(response.status().is_success());
                    let body = tokio::time::timeout(
                        Duration::from_secs(2),
                        actix_test::read_body(response),
                    )
                    .await
                    .expect("the Arena SSR response must finish after the featured-game fetch");
                    String::from_utf8(body.to_vec()).unwrap()
                })
            })
            .await
            .expect("the SSR test worker must finish")
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn featured_game_fetch_does_not_prevent_arena_ssr_completion() {
            let db = test_database::test_db().await;
            // A missing Game still exercises an asynchronous fetch that must not stall SSR.
            render_arena(db.pool.clone(), GameId(String::from("featured"))).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolved_featured_game_is_in_ssr_html_before_hydration() {
            let db = test_database::test_db().await;
            let mut conn = get_conn(&db.pool).await.unwrap();
            let white = User::create(
                NewUser::new("ssr_white", "password", "ssr_white@example.test").unwrap(),
                &mut conn,
            )
            .await
            .unwrap();
            let black = User::create(
                NewUser::new("ssr_black", "password", "ssr_black@example.test").unwrap(),
                &mut conn,
            )
            .await
            .unwrap();
            let challenge = NewChallenge::new(
                white.id,
                None,
                &ChallengeDetails {
                    rated: false,
                    game_type: GameType::Base,
                    visibility: ChallengeVisibility::Public,
                    opponent: None,
                    color_choice: ColorChoice::Random,
                    time_mode: TimeMode::RealTime,
                    time_base: Some(300),
                    time_increment: Some(0),
                    band_lower: None,
                    band_upper: None,
                },
                &mut conn,
            )
            .await
            .unwrap();
            let challenge = Challenge::create(&challenge, &mut conn).await.unwrap();
            let mut game = NewGame::new(white.id, black.id, &challenge).unwrap();
            game.history = String::from("wA1 ;");
            game.turn = 1;
            game.current_player_id = black.id;
            game.game_status = GameStatus::InProgress.to_string();
            let game = Game::create(game, &mut conn).await.unwrap();
            drop(conn);

            let html = render_arena(db.pool.clone(), GameId(game.nanoid.clone())).await;
            assert!(
                html.contains(&format!("data-featured-game-id=\"{}\"", game.nanoid)),
                "SSR must render the resolved featured game rather than serialize its data beside an empty placeholder"
            );
        }
    }

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
