use crate::{
    common::{
        display_standing_rows,
        game_point_presentation,
        primary_score_presentation,
        primary_value_text,
        result_marker_color,
        result_marker_label,
        standings_value_text,
        tournament_game_number,
        tournament_player_name as player_name,
        unfinished_tournament_game_label,
        use_fitted_pagination,
    },
    components::{
        atoms::{color_hex::ColorHex, rating::Rating},
        molecules::{
            game_previews::GamePreview,
            pagination_controls::PaginationControls,
            panel::Panel,
            time_row::TimeRow,
            tournament_standings_controls::TournamentStandingsControls,
        },
        organisms::tournament_encounter_summary::{
            EncounterParticipant,
            TournamentEncounterSummary,
        },
    },
    functions::games::get::get_game_from_nanoid,
    i18n::*,
    providers::{
        RoundRobinRound,
        RoundRobinState,
        RoundRobinStateStoreFields,
        SwissRound,
        SwissState,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatState,
    },
    responses::{TournamentMemberships, TournamentStandings},
};
use hive_lib::{Color, GameStatus};
use leptos::{html, portal::Portal, prelude::*};
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament::{
        elimination::{ClinchPolicy, SeriesPhase},
        round_robin::{Config as RoundRobinConfig, PrimaryScore as RoundRobinPrimaryScore},
        standings::Value,
        swiss::{
            Config as SwissConfig,
            PrimaryScore as DoubleSwissPrimaryScore,
            System as SwissSystem,
        },
        AdjudicatedSideResult,
        FormatConfig,
        GameOutcome,
        Resolution,
        Score,
    },
    tournament_view::{
        RoundRobinMatchResponse,
        SlotResponse,
        SwissByeResponse,
        SwissMatchCompletionResponse,
    },
    GameId,
    GameSpeed,
};
use std::collections::HashMap;
use tournamint::{MatchScore, PointSystem};
use uuid::Uuid;

fn format_game_points(score: Score, point_system: PointSystem) -> String {
    standings_value_text(Value::Score(score), game_point_presentation(point_system))
}

fn slot_state(slot: &SlotResponse) -> String {
    let i18n = use_i18n();
    match slot.resolution {
        Some(Resolution::Result(GameOutcome::Played(_))) => {
            t_string!(i18n, tournaments.ui.played).to_string()
        }
        Some(Resolution::Result(GameOutcome::Adjudicated(outcome)))
            if matches!(
                (outcome.white(), outcome.black()),
                (
                    AdjudicatedSideResult::DoubleForfeit,
                    AdjudicatedSideResult::DoubleForfeit
                )
            ) =>
        {
            t_string!(i18n, game.tournament_result.double_forfeit).to_string()
        }
        Some(Resolution::Result(GameOutcome::Adjudicated(_))) => {
            t_string!(i18n, tournaments.ui.adjudicated).to_string()
        }
        Some(Resolution::Withdrawal(_)) => t_string!(i18n, common.withdrawal).to_string(),
        Some(Resolution::Clinched) => {
            t_string!(i18n, tournaments.ui.not_played_match_already_decided).to_string()
        }
        Some(Resolution::CancelledByWithdrawal) => {
            t_string!(i18n, tournaments.ui.not_played_withdrawal).to_string()
        }
        None => match slot.game.as_ref() {
            Some(game)
                if game.finished
                    || matches!(
                        game.status,
                        GameStatus::Finished(_) | GameStatus::Adjudicated
                    ) =>
            {
                t_string!(i18n, tournaments.ui.finished_result_pending).to_string()
            }
            Some(game) if game.status == GameStatus::InProgress => {
                t_string!(i18n, common.in_progress).to_string()
            }
            _ if slot.scheduled_at.is_some() => t_string!(i18n, common.scheduled).to_string(),
            Some(game) if game.status == GameStatus::NotStarted => {
                t_string!(i18n, common.not_started).to_string()
            }
            Some(_) => t_string!(i18n, common.in_progress).to_string(),
            None => t_string!(i18n, tournaments.ui.planned).to_string(),
        },
    }
}

fn unresolved_slot_state_priority(slot: &SlotResponse) -> u8 {
    if slot.game.as_ref().is_some_and(|game| {
        game.finished
            || matches!(
                game.status,
                GameStatus::Finished(_) | GameStatus::Adjudicated | GameStatus::InProgress
            )
    }) {
        0
    } else if slot.scheduled_at.is_some() {
        1
    } else if slot.game.is_some() {
        2
    } else {
        3
    }
}

fn is_played_slot(slot: &SlotResponse) -> bool {
    matches!(
        slot.resolution,
        Some(Resolution::Result(GameOutcome::Played(_)))
    ) && slot.game_id().is_some()
}

fn ordered_game_slots(
    slots: Vec<ArcField<SlotResponse>>,
    format: Option<TournamentFormatState>,
) -> Vec<ArcField<SlotResponse>> {
    let Some(format) = format else { return slots };
    let mut ordered = slots
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            let number = slot
                .try_get()
                .and_then(|value| tournament_game_number(format, &value, index + 1));
            ((number.is_none(), number.unwrap_or(index + 1)), slot)
        })
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(order, _)| *order);
    ordered.into_iter().map(|(_, slot)| slot).collect()
}

#[component]
fn TournamentGameRow(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    point_system: PointSystem,
    game_number: Signal<Option<usize>>,
    total_games: usize,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        {move || {
            let game_slot = game_slot.try_get()?;
            let memberships = common.memberships().get();
            let white_name = player_name(&memberships, game_slot.participants[0]);
            let black_name = player_name(&memberships, game_slot.participants[1]);
            let state = slot_state(&game_slot);
            let number = game_number.get();
            let score = game_slot
                .awarded_game_points
                .map(|points| {
                    format!(
                        "{} – {}",
                        format_game_points(points[0], point_system),
                        format_game_points(points[1], point_system),
                    )
                });
            let game_id = game_slot.game_id().cloned();
            Some(
                view! {
                    <div class="grid gap-y-1 gap-x-3 py-2 px-2 text-xs grid-cols-[minmax(0,1fr)_auto] dark:odd:bg-surface-row-odd dark:even:bg-surface-row-even odd:bg-odd-light even:bg-even-light">
                        <div class="min-w-0">

                            <div class="font-bold text-gray-700 dark:text-gray-300">
                                {number
                                    .map_or_else(
                                        || {
                                            unfinished_tournament_game_label(
                                                    i18n.get_locale(),
                                                    &game_slot,
                                                )
                                                .to_string()
                                        },
                                        |number| {
                                            t_string!(
                                                i18n, tournaments.ui.game_of, number = number, value1 = total_games
                                            )
                                                .to_string()
                                        },
                                    )}
                            </div>
                            <div class="flex flex-wrap gap-x-1 text-gray-600 dark:text-gray-400">
                                <span class="truncate">{white_name}</span>
                                <span>"–"</span>
                                <span class="truncate">{black_name}</span>
                            </div>
                        </div>
                        <div class="text-right shrink-0">
                            <div class="font-bold tabular-nums">
                                {score.unwrap_or_else(|| String::from("—"))}
                            </div>
                            {number.map(|_| view! { <div class="text-gray-500">{state}</div> })}
                            {game_id
                                .map(|game_id| {
                                    view! {
                                        <a
                                            class="font-bold hover:underline"
                                            href=format!("/game/{game_id}")
                                        >
                                            {move || { t_string!(i18n, common.open_game).to_string() }}
                                        </a>
                                    }
                                })}
                        </div>
                    </div>
                },
            )
        }}
    }
}

#[component]
fn TournamentGamePlaceholder(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    point_system: PointSystem,
) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let slot = game_slot.try_get()?;
        let memberships = common.memberships().get();
        let state = slot_state(&slot);
        let game_id = slot.game_id().cloned();
        let speed = GameSpeed::from(slot.clock);
        let players = slot
            .participants
            .map(|player| player_name(&memberships, player));

        let label = t_string!(
            i18n,
            common.versus_players,
            white = players[0].clone(),
            black = players[1].clone()
        )
        .to_string();
        let result = slot.awarded_game_points.map(|points| {
            format!(
                "{} – {} · {state}",
                format_game_points(points[0], point_system),
                format_game_points(points[1], point_system),
            )
        });
        Some(view! {
            <article class="flex overflow-hidden relative flex-col items-center w-full min-w-0 ui-card-row">
                <div class="grid gap-1 py-1.5 px-2 w-full text-sm">
                    {players
                        .into_iter()
                        .enumerate()
                        .map(|(side, name)| {
                            let color = if side == 0 { Color::White } else { Color::Black };
                            let rating = slot
                                .game
                                .as_ref()
                                .and_then(|game| game.ratings[side].map(u64::from))
                                .or_else(|| {
                                    memberships
                                        .players
                                        .get(&slot.participants[side])
                                        .map(|player| player.rating_for_speed(&speed))
                                });
                            view! {
                                <div class="grid gap-1.5 items-center h-5 grid-cols-[1rem_minmax(0,1fr)_auto]">
                                    <ColorHex color=Signal::derive(move || color) />
                                    <span class="font-medium truncate" title=name.clone()>
                                        {name.clone()}
                                    </span>
                                    <span class="text-xs text-gray-500">
                                        {rating.map(|rating| view! { <Rating value=rating /> })}
                                    </span>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
                <p class="px-2 mb-1 h-10 text-sm leading-5 text-center line-clamp-2">
                    {game_id.as_ref().map(|_| result.clone().unwrap_or_else(|| state.to_string()))}
                </p>
                <div class="flex justify-center items-center w-full aspect-square shrink-0">

                    <span class="text-sm font-semibold">
                        {if game_id.is_some() {
                            t_string!(i18n, common.open_game).to_string()
                        } else if slot.resolution.is_some() {
                            result.unwrap_or_else(|| state.to_string())
                        } else if slot.scheduled_at.is_some() {
                            state.to_string()
                        } else {
                            t_string!(i18n, tournaments.ui.not_available_yet).to_string()
                        }}
                    </span>
                </div>
                {game_id
                    .map(|game_id| {
                        view! {
                            <a
                                class="absolute inset-0 z-10 rounded-lg no-link-style focus-visible:outline-2 focus-visible:outline-pillbug-teal"
                                href=format!("/game/{game_id}")
                                aria-label=move || {
                                    t_string!(i18n, tournaments.ui.open_game, label = label.clone())
                                        .to_string()
                                }
                            ></a>
                        }
                    })}
            </article>
        })
    }
}

#[component]
fn LazyTournamentGamePreview(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    game_id: GameId,
    point_system: PointSystem,
) -> impl IntoView {
    let game = Resource::new(move || game_id.clone(), get_game_from_nanoid);
    view! {
        {move || match game.get() {
            Some(Ok(game)) => view! { <GamePreview game drawer=true /> }.into_any(),
            None | Some(Err(_)) => {
                view! {
                    <TournamentGamePlaceholder common game_slot=game_slot.clone() point_system />
                }
                    .into_any()
            }
        }}
    }
}

#[component]
fn TournamentDrawerSlot(
    common: Store<TournamentCommon>,
    game_slot: ArcField<SlotResponse>,
    point_system: PointSystem,
    game_number: Signal<Option<usize>>,
    total_games: Signal<usize>,
) -> impl IntoView {
    let i18n = use_i18n();
    let preview_slot = game_slot.clone();
    let played_game_id = Memo::new(move |_| {
        preview_slot
            .try_with(|slot| {
                is_played_slot(slot)
                    .then(|| slot.game_id().cloned())
                    .flatten()
            })
            .flatten()
    });
    let placeholder_slot = game_slot.clone();
    let label_slot = game_slot.clone();
    view! {
        <div class="flex flex-col gap-1 w-60 min-w-0 max-w-full">

            <p class="text-xs font-bold">
                {move || match game_number.get() {
                    Some(number) => {
                        Some(
                            t_string!(
                                i18n, tournaments.ui.game_of, number = number, value1 = total_games.get()
                            )
                                .to_string(),
                        )
                    }
                    None => {
                        label_slot
                            .try_get()
                            .map(|slot| {
                                unfinished_tournament_game_label(i18n.get_locale(), &slot)
                                    .to_string()
                            })
                    }
                }}
            </p>
            <Show when=move || played_game_id.with(Option::is_none)>
                <TournamentGamePlaceholder common game_slot=placeholder_slot.clone() point_system />
            </Show>
            <For
                each=move || played_game_id.get().into_iter()
                key=|game_id| game_id.clone()
                children=move |game_id| {
                    view! {
                        <LazyTournamentGamePreview
                            common
                            game_slot=game_slot.clone()
                            game_id
                            point_system
                        />
                    }
                }
            />
        </div>
    }
}

#[derive(Clone)]
pub struct TournamentDrawerSeries {
    pub(crate) label: Option<String>,
    pub(crate) slots: Vec<ArcField<SlotResponse>>,
    pub(crate) participants: [Uuid; 2],
    pub(crate) score: Option<String>,
    pub(crate) sets: Option<Vec<TournamentDrawerSet>>,
    pub(crate) outcome: Option<String>,
}

#[derive(Clone)]
pub(crate) struct TournamentDrawerSet {
    pub(crate) slots: Vec<ArcField<SlotResponse>>,
    pub(crate) score: String,
    pub(crate) phase: SeriesPhase,
}

#[component]
pub(crate) fn TournamentGamesDrawer(
    common: Store<TournamentCommon>,
    #[prop(optional)] numbering_format: Option<TournamentFormatState>,
    #[prop(into)] series: Signal<Vec<TournamentDrawerSeries>>,
    point_system: PointSystem,
    #[prop(optional, into)] match_points: Signal<Option<String>>,
    close: Callback<()>,
) -> impl IntoView {
    // Mount both overlay siblings outside transformed bracket/pairing route wrappers.
    view! {
        <Portal>
            <TournamentGamesDrawerBody
                common
                numbering_format
                series
                point_system
                match_points
                close
            />
        </Portal>
    }
}

#[component]
fn TournamentGamesDrawerBody(
    common: Store<TournamentCommon>,
    numbering_format: Option<TournamentFormatState>,
    series: Signal<Vec<TournamentDrawerSeries>>,
    point_system: PointSystem,
    match_points: Signal<Option<String>>,
    close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let names = Memo::new(move |_| {
        let participants = series.with(|series| {
            series
                .last()
                .map(|series| series.participants)
                .unwrap_or_default()
        });
        common
            .memberships()
            .with(|memberships| participants.map(|player| player_name(memberships, player)))
    });

    let title = move || {
        series.with(|series| {
            series
                .first()
                .and_then(|series| series.label.clone())
                .unwrap_or(t_string!(i18n, tournaments.ui.matchup_games).to_string())
        })
    };
    view! {
        <button
            type="button"
            class="fixed inset-0 z-40 bg-black/35"
            aria-label=move || t_string!(i18n, tournaments.ui.close_matchup_games).to_string()
            on:click=move |_| close.run(())
        ></button>
        <aside
            class="overflow-y-auto overscroll-contain fixed inset-0 z-50 bg-white shadow-2xl md:inset-y-4 md:right-4 md:left-auto md:rounded-xl dark:bg-gray-900 max-h-dvh md:w-[min(34rem,calc(100vw-2rem))]"
            data-testid="round-robin-matchup-drawer"
        >
            <header class="sticky top-0 z-10 py-3 px-4 bg-white border-b border-gray-200 dark:bg-gray-900 dark:border-gray-700">
                <div class="flex gap-3 justify-between items-start">
                    <div class="min-w-0">

                        <p class="font-bold tracking-wide text-gray-500 uppercase text-[10px]">
                            {title}
                        </p>
                        <h2 class="overflow-hidden min-w-0 text-lg font-bold leading-tight wrap-break-word">
                            <span>{move || names.get()[0].clone()}</span>
                            <span class="mx-1.5 font-normal text-gray-500">
                                {move || t_string!(i18n, tournaments.view.common.versus)}
                            </span>
                            <span>{move || names.get()[1].clone()}</span>
                        </h2>
                    </div>

                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm shrink-0"
                        on:click=move |_| close.run(())
                    >
                        {t!(i18n, common.close)}
                    </button>
                </div>
            </header>
            <div class="p-3 space-y-5 sm:p-4">
                {move || {
                    match_points
                        .get()
                        .map(|score| {
                            view! {
                                <p class="font-semibold tabular-nums">
                                    {move || {
                                        t_string!(
                                            i18n, tournaments.ui.match_points, score = score.clone()
                                        )
                                            .to_string()
                                    }}
                                </p>
                            }
                        })
                }}
                <For
                    each=move || series.with(|series| 0..series.len())
                    key=|index| *index
                    children=move |index| {
                        let series = Signal::derive(move || {
                            series.with(|series| series.get(index).cloned())
                        });
                        view! {
                            <TournamentGamesSeries common numbering_format series point_system />
                        }
                    }
                />
            </div>
        </aside>
    }
}

#[component]
fn TournamentGamesSeries(
    common: Store<TournamentCommon>,
    numbering_format: Option<TournamentFormatState>,
    series: Signal<Option<TournamentDrawerSeries>>,
    point_system: PointSystem,
) -> impl IntoView {
    let matchup_score = move || {
        let series = series.get()?;
        Some(series.score.unwrap_or_else(|| {
            matchup_score_text(
                point_system,
                &series
                    .slots
                    .iter()
                    .filter_map(|slot| slot.try_get())
                    .collect::<Vec<_>>(),
                series.participants[0],
            )
        }))
    };
    let orientation = move || {
        let series = series.get()?;
        series.label.map(|_| {
            common.memberships().with(|memberships| {
                t_string!(
                    use_i18n(),
                    common.versus_players,
                    white = player_name(memberships, series.participants[0]),
                    black = player_name(memberships, series.participants[1])
                )
                .to_string()
            })
        })
    };
    view! {
        <section class="space-y-3 [&+section]:pt-4 [&+section]:border-t [&+section]:border-gray-200 dark:[&+section]:border-gray-700">
            <div>
                <div class="flex gap-4 justify-between items-center">
                    {move || {
                        series
                            .get()
                            .and_then(|series| series.label.clone())
                            .map(|label| view! { <h3 class="font-bold">{label}</h3> })
                    }}
                    <Show when=move || {
                        series
                            .with(|series| {
                                series.as_ref().is_some_and(|series| series.sets.is_none())
                            })
                    }>
                        <strong class="ml-auto text-lg tabular-nums">{matchup_score}</strong>
                    </Show>
                </div>
                {move || {
                    orientation()
                        .map(|orientation| {
                            view! { <p class="text-xs text-gray-500">{orientation}</p> }
                        })
                }}
                {move || {
                    series
                        .get()
                        .and_then(|series| series.outcome)
                        .map(|outcome| view! { <p class="font-semibold">{outcome}</p> })
                }}
            </div>
            <For
                each=move || {
                    series
                        .with(|series| {
                            0..series
                                .as_ref()
                                .map_or(0, |series| series.sets.as_ref().map_or(1, Vec::len))
                        })
                }
                key=|index| *index
                children=move |set_index| {
                    view! {
                        <TournamentGamesSet common numbering_format series set_index point_system />
                    }
                }
            />
        </section>
    }
}

#[component]
fn TournamentGamesSet(
    common: Store<TournamentCommon>,
    numbering_format: Option<TournamentFormatState>,
    series: Signal<Option<TournamentDrawerSeries>>,
    set_index: usize,
    point_system: PointSystem,
) -> impl IntoView {
    let i18n = use_i18n();
    let slots = Signal::derive(move || {
        let slots = series.with(|series| match series.as_ref() {
            Some(TournamentDrawerSeries {
                sets: Some(sets), ..
            }) => sets
                .get(set_index)
                .map(|set| set.slots.clone())
                .unwrap_or_default(),
            Some(series) => series.slots.clone(),
            None => Vec::new(),
        });
        ordered_game_slots(slots, numbering_format)
    });
    let total_games = Signal::derive(move || slots.with(Vec::len));
    let details = move || series.get()?.sets?.get(set_index).cloned();
    view! {
        <div class="space-y-3">
            {move || {
                details()
                    .map(|set| {
                        let phase = set.phase;
                        let rules = t_string!(
                            i18n, tournaments.ui.game_count_with_details, value1 = phase.games_per_set, value2 = match phase.clinch {
                                ClinchPolicy::PlayAll => t_string!(i18n, tournaments.ui.all_games_played).to_string(),
                                ClinchPolicy::EarlyClinch => t_string!(i18n, tournaments.ui.ends_when_clinched).to_string(),
                            }
                        )
                            .to_string();

                        view! {
                            <div>
                                <div class="flex gap-4 justify-between items-center">

                                    <h3 class="font-bold">
                                        {move || {
                                            t_string!(i18n, tournaments.ui.set, value1 = set_index + 1)
                                                .to_string()
                                        }}
                                    </h3>
                                    <strong class="tabular-nums">{set.score}</strong>
                                </div>
                                <div class="flex flex-wrap gap-x-2 items-center text-xs text-gray-600 dark:text-gray-300">
                                    <span>{rules}</span>
                                    <TimeRow
                                        time_control=Some(phase.clock)
                                        extend_tw_classes="text-xs"
                                    />
                                </div>
                            </div>
                        }
                    })
            }}
            <div class="grid gap-3 justify-center items-start grid-cols-[repeat(auto-fit,minmax(min(100%,14rem),15rem))]">
                <ForEnumerate
                    each=move || slots.get()
                    key=|slot| slot.with_untracked(|slot| slot.id)
                    children=move |index, game_slot| {
                        let numbered_slot = game_slot.clone();
                        let game_number = Signal::derive(move || {
                            let slot = numbered_slot.try_get()?;
                            numbering_format
                                .map_or(
                                    Some(index.get() + 1),
                                    |format| {
                                        tournament_game_number(format, &slot, index.get() + 1)
                                    },
                                )
                        });
                        view! {
                            <TournamentDrawerSlot
                                common
                                game_slot
                                point_system
                                game_number
                                total_games
                            />
                        }
                    }
                />
            </div>
        </div>
    }
}

pub(crate) fn swiss_round_progress(
    rounds: &[SwissRound],
    round_index: u32,
) -> Option<(usize, usize)> {
    let round = rounds
        .iter()
        .find(|round| round.round_index == round_index)?;
    let total = round.encounters.len() + round.byes.len();
    let resolved = round
        .encounters
        .iter()
        .filter(|encounter| encounter.completion.is_some())
        .count()
        + round.byes.len();
    Some((resolved, total))
}

const CROSS_HEAD: &str =
    "px-1 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:text-gray-300";
const CROSS_CELL: &str = "p-0 text-center";
const CROSS_STICKY_NUMBER_HEAD: &str = "sticky left-0 z-20 w-8 min-w-8 bg-white px-1 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:bg-gray-900 dark:text-gray-300";
const CROSS_STICKY_PLAYER_HEAD: &str = "sticky left-8 z-20 bg-white px-1 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:bg-gray-900 dark:text-gray-300";
const CROSS_STICKY_NUMBER_CELL: &str =
    "sticky left-0 z-10 w-8 min-w-8 bg-white px-1 py-1 text-center font-bold tabular-nums text-gray-500 dark:bg-gray-900 dark:text-gray-400";
const CROSS_STICKY_PLAYER_CELL: &str =
    "sticky left-8 z-10 max-w-40 bg-white px-1 py-1 text-left whitespace-nowrap dark:bg-gray-900";

#[derive(Clone)]
struct RoundRobinCell {
    slots: Vec<ArcField<SlotResponse>>,
    match_result: Option<RoundRobinMatchResponse>,
}

impl PartialEq for RoundRobinCell {
    fn eq(&self, other: &Self) -> bool {
        self.match_result == other.match_result
            && self.slots.len() == other.slots.len()
            && self.slots.iter().zip(&other.slots).all(|(left, right)| {
                left.with_untracked(|slot| slot.id) == right.with_untracked(|slot| slot.id)
            })
    }
}

#[derive(Clone, PartialEq)]
struct RoundRobinTable {
    order: Vec<(Option<u32>, Uuid, String)>,
    cells: HashMap<(Uuid, Uuid), RoundRobinCell>,
}

fn unordered(left: Uuid, right: Uuid) -> (Uuid, Uuid) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn round_robin_cells(
    rounds: &[RoundRobinRound],
    round_robin: Store<RoundRobinState>,
) -> HashMap<(Uuid, Uuid), RoundRobinCell> {
    let mut cells = HashMap::new();
    for round in rounds {
        for slot in &round.slots {
            let slot_field: ArcField<SlotResponse> =
                round_robin.slots().at_key(slot.slot_id).into();
            let participants = slot_field.get_untracked().participants;
            let key = unordered(participants[0], participants[1]);
            cells
                .entry(key)
                .or_insert_with(|| RoundRobinCell {
                    slots: Vec::new(),
                    match_result: None,
                })
                .slots
                .push(slot_field);
        }
    }
    for encounter in round_robin.matches().get() {
        if let Some(cell) = cells.get_mut(&unordered(
            encounter.participants[0],
            encounter.participants[1],
        )) {
            cell.match_result = Some(encounter);
        }
    }
    cells
}

fn ranked(
    memberships: &TournamentMemberships,
    standings: &TournamentStandings,
    configuration: &RoundRobinConfig,
) -> Vec<(Option<u32>, Uuid, String)> {
    if let Some(standings) = &standings.snapshot {
        return display_standing_rows(standings, &memberships.withdrawn)
            .into_iter()
            .map(|standing| {
                (
                    standing.rank,
                    standing.row.user_id,
                    primary_value_text(
                        standing.row.primary_score,
                        &FormatConfig::RoundRobin(configuration.clone()),
                    ),
                )
            })
            .collect();
    }
    let mut players = memberships
        .players
        .values()
        .map(|player| (None, player.uid, String::from("—")))
        .collect::<Vec<_>>();
    players.sort_by(|left, right| {
        memberships.players[&left.1]
            .username
            .cmp(&memberships.players[&right.1].username)
    });
    players
}

fn round_robin_matrix_order(
    memberships: &TournamentMemberships,
    standings: &TournamentStandings,
    configuration: &RoundRobinConfig,
    viewer: Option<Uuid>,
) -> Vec<(Option<u32>, Uuid, String)> {
    let mut players = ranked(memberships, standings, configuration);
    if let Some(position) =
        viewer.and_then(|viewer| players.iter().position(|(_, player, _)| *player == viewer))
    {
        let viewer = players.remove(position);
        players.insert(0, viewer);
    }
    players
}

fn result_marker(slot: &SlotResponse, player: Uuid) -> &'static str {
    let Some(resolution) = slot.resolution else {
        return if slot.game.is_some() { "*" } else { "–" };
    };
    let outcome = match resolution {
        Resolution::Result(outcome) | Resolution::Withdrawal(outcome) => outcome,
        Resolution::Clinched | Resolution::CancelledByWithdrawal => return "–",
    };
    match if slot.participants[0] == player {
        outcome.white().score()
    } else {
        outcome.black().score()
    } {
        MatchScore::Win => "W",
        MatchScore::Draw => "D",
        MatchScore::Loss => "L",
    }
}

#[component]
fn RoundRobinResultStrip(
    slots: Vec<ArcField<SlotResponse>>,
    round_robin: Store<RoundRobinState>,
    player: Uuid,
    #[prop(default = false)] roomy: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let marker_gap = if roomy { "gap-1" } else { "gap-px" };
    view! {
        <span class=format!(
            "inline-flex items-center justify-center font-mono font-bold leading-none {marker_gap}",
        )>
            {move || {
                ordered_game_slots(
                        slots.clone(),
                        Some(TournamentFormatState::RoundRobin(round_robin)),
                    )
                    .into_iter()
                    .filter_map(|slot| {
                        let slot = slot.try_get()?;
                        let marker = result_marker(&slot, player);
                        let color = result_marker_color(marker);
                        Some(
                            view! {
                                <span class=format!(
                                    "inline-flex w-[1ch] shrink-0 justify-center {color}",
                                )>{move || result_marker_label(i18n, marker)}</span>
                            },
                        )
                    })
                    .collect_view()
            }}
        </span>
    }
}

#[component]
fn RoundRobinMatrixCell(
    slots: Vec<ArcField<SlotResponse>>,
    round_robin: Store<RoundRobinState>,
    point_system: PointSystem,
    primary_match_points: bool,
    row: Uuid,
    column: Uuid,
    pair: (Uuid, Uuid),
    opened: RwSignal<Option<(Uuid, Uuid)>>,
    hovered: RwSignal<Option<(Uuid, Uuid)>>,
    match_result: Signal<Option<RoundRobinMatchResponse>>,
) -> impl IntoView {
    let strip = slots.clone();
    let score = move || {
        let slots = slots
            .iter()
            .filter_map(|slot| slot.try_get())
            .collect::<Vec<_>>();
        if primary_match_points {
            match_result.with(|result| round_robin_match_score_text(result.as_ref(), row))
        } else {
            matchup_score_text(point_system, &slots, row)
        }
    };
    view! {
        <td class=move || {
            let on_axis = opened
                .get()
                .is_some_and(|(selected_row, selected_column)| {
                    selected_row == row || selected_column == column
                })
                || hovered
                    .get()
                    .is_some_and(|(selected_row, selected_column)| {
                        selected_row == row || selected_column == column
                    });
            if on_axis {
                format!("{CROSS_CELL} bg-pillbug-teal/10")
            } else {
                CROSS_CELL.to_string()
            }
        }>
            <button
                type="button"
                class=move || {
                    if opened.get().is_some_and(|opened| unordered(opened.0, opened.1) == pair) {
                        "flex h-10 w-full min-w-10 flex-col items-center justify-center bg-pillbug-teal px-0.5 font-bold tabular-nums text-white ring-2 ring-inset ring-pillbug-teal/30"
                    } else {
                        "flex h-10 w-full min-w-10 flex-col items-center justify-center px-0.5 tabular-nums hover:bg-pillbug-teal/20"
                    }
                }
                on:mouseenter=move |_| hovered.set(Some((row, column)))
                on:click=move |_| opened.set(Some((row, column)))
            >
                <span class="text-[9px]">
                    <RoundRobinResultStrip slots=strip round_robin player=row />
                </span>
                <strong class="mt-0.5 leading-none text-[10px]">{score}</strong>
            </button>
        </td>
    }
}

fn round_robin_match_score_text(
    encounter: Option<&RoundRobinMatchResponse>,
    player: Uuid,
) -> String {
    encounter
        .and_then(|encounter| {
            encounter.completion.map(|completed| {
                let side = usize::from(encounter.participants[1] == player);
                format!(
                    "{}–{}",
                    completed.match_points[side].value(),
                    completed.match_points[1 - side].value()
                )
            })
        })
        .unwrap_or_else(|| String::from("—"))
}

fn matchup_score_text(point_system: PointSystem, slots: &[SlotResponse], player: Uuid) -> String {
    let mut points = [0u32; 2];
    let mut has_award = false;
    for slot in slots {
        let Some(award) = slot.awarded_game_points else {
            continue;
        };
        let side = usize::from(slot.participants[1] == player);
        points[0] = points[0].saturating_add(award[side].value());
        points[1] = points[1].saturating_add(award[1 - side].value());
        has_award = true;
    }
    if !has_award {
        return String::from("—");
    }
    let presentation = game_point_presentation(point_system);
    let value = format!(
        "{}–{}",
        standings_value_text(Value::Score(Score::new(points[0])), presentation),
        standings_value_text(Value::Score(Score::new(points[1])), presentation),
    );
    value
}

#[component]
fn RoundRobinMatrixPlayerRow(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    point_system: PointSystem,
    primary_match_points: bool,
    row: Uuid,
    table: Memo<RoundRobinTable>,
    player_order: Memo<Vec<Uuid>>,
    opened: RwSignal<Option<(Uuid, Uuid)>>,
    hovered: RwSignal<Option<(Uuid, Uuid)>>,
) -> impl IntoView {
    view! {
        <tr class=move || {
            if opened.get().is_some_and(|(selected_row, _)| selected_row == row)
                || hovered.get().is_some_and(|(selected_row, _)| selected_row == row)
            {
                "ui-dense-table-row !bg-pillbug-teal/25"
            } else {
                "ui-dense-table-row"
            }
        }>
            <td class=CROSS_STICKY_NUMBER_CELL>
                {move || {
                    player_order
                        .with(|players| {
                            players.iter().position(|player| *player == row).unwrap_or(0) + 1
                        })
                }}
            </td>
            <td class=CROSS_STICKY_PLAYER_CELL>
                {move || {
                    let memberships = common.memberships().get();
                    let name = player_name(&memberships, row);
                    let name_title = name.clone();
                    let withdrawn = memberships.withdrawn.contains(&row);
                    view! {
                        <span
                            class=if withdrawn {
                                "block max-w-40 truncate line-through"
                            } else {
                                "block max-w-40 truncate"
                            }
                            title=name_title
                        >
                            {name}
                        </span>
                    }
                }}
            </td>
            <For
                each=move || player_order.get()
                key=|player| *player
                children=move |column| {
                    if row == column {
                        return view! {
                            <td class=format!(
                                "{CROSS_CELL} bg-gray-100 text-gray-400 dark:bg-gray-800 dark:text-gray-600",
                            )>"—"</td>
                        }
                            .into_any();
                    }
                    let pair = unordered(row, column);
                    let Some(cell) = table.with_untracked(|table| table.cells.get(&pair).cloned())
                    else {
                        return view! {
                            <td class=CROSS_CELL>
                                <span class="text-gray-400 dark:text-gray-600">"·"</span>
                            </td>
                        }
                            .into_any();
                    };
                    let match_result = Signal::derive(move || {
                        table
                            .with(|table| {
                                table.cells.get(&pair).and_then(|cell| cell.match_result.clone())
                            })
                    });
                    view! {
                        <RoundRobinMatrixCell
                            round_robin
                            slots=cell.slots
                            match_result
                            primary_match_points
                            point_system
                            row
                            column
                            pair
                            opened
                            hovered
                        />
                    }
                        .into_any()
                }
            />
            <td class=CROSS_CELL>
                <strong>
                    {move || {
                        table
                            .with(|table| {
                                table
                                    .order
                                    .iter()
                                    .find(|(_, player, _)| *player == row)
                                    .map(|(_, _, total)| total.clone())
                                    .unwrap_or_default()
                            })
                    }}
                </strong>
            </td>
        </tr>
    }
}

#[component]
fn RoundRobinPlayerMatchup(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    point_system: PointSystem,
    primary_match_points: bool,
    player: Uuid,
    opponent: Uuid,
    opponent_number: usize,
    slots: Vec<ArcField<SlotResponse>>,
    match_result: Option<RoundRobinMatchResponse>,
) -> impl IntoView {
    let opponent_name = player_name(&common.memberships().get_untracked(), opponent);
    let score_slots = slots.clone();
    let score = move || {
        if primary_match_points {
            return round_robin_match_score_text(match_result.as_ref(), player);
        }
        matchup_score_text(
            point_system,
            &score_slots
                .iter()
                .filter_map(|slot| slot.try_get())
                .collect::<Vec<_>>(),
            player,
        )
    };
    let progress_slots = slots.clone();
    let resolved = Memo::new(move |_| {
        progress_slots
            .iter()
            .filter(|slot| slot.try_get().is_some_and(|slot| slot.resolution.is_some()))
            .count()
    });
    let total = slots.len();
    let row_slots = slots.clone();
    let rows = move || {
        ordered_game_slots(row_slots.clone(), Some(TournamentFormatState::RoundRobin(round_robin)))
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            let numbered_slot = slot.clone();
            let game_number = Signal::derive(move || {
                tournament_game_number(
                    TournamentFormatState::RoundRobin(round_robin),
                    &numbered_slot.try_get()?, index + 1,
                )
            });
            view! { <TournamentGameRow common game_slot=slot point_system game_number total_games=total /> }
        })
        .collect_view()
    };

    view! {
        <details class="overflow-hidden rounded border border-gray-300 dark:border-gray-700">
            <summary class="grid gap-y-1 gap-x-3 items-center py-2 px-3 cursor-pointer grid-cols-[minmax(0,1fr)_auto] marker:text-gray-400">
                <span class="min-w-0 text-sm font-bold truncate">
                    <span class="mr-2 font-mono text-xs text-gray-500 dark:text-gray-400">
                        {format!("#{opponent_number}")}
                    </span>
                    {opponent_name}
                </span>
                <strong class="text-sm tabular-nums">{score}</strong>
                <span class="text-xs">
                    <RoundRobinResultStrip slots=slots.clone() round_robin player roomy=true />
                </span>
                {move || {
                    (resolved() < total)
                        .then(|| {
                            view! {
                                <span class="tabular-nums text-right text-gray-500 dark:text-gray-400 text-[10px]">
                                    {move || {
                                        t_string!(
                                            use_i18n(), tournaments.ui.resolved_game_count, resolved_games = resolved(), total_games = total
                                        )
                                            .to_string()
                                    }}
                                </span>
                            }
                        })
                }}
            </summary>
            <div class="border-t border-gray-200 dark:border-gray-700">{rows}</div>
        </details>
    }
}

#[component]
fn RoundRobinPlayerLedger(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    point_system: PointSystem,
    primary_match_points: bool,
    player: Uuid,
    table: Memo<RoundRobinTable>,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="space-y-3">
            {move || {
                let table = table.get();
                let order = &table.order;
                let cells = &table.cells;
                let Some((rank, _, total_score)) = order
                    .iter()
                    .find(|(_, candidate, _)| *candidate == player)
                    .cloned() else {
                    return view! {
                        <p class="ui-empty-state">
                            {t!(i18n, tournaments.ui.choose_a_player_to_see_their_results)}
                        </p>
                    }
                        .into_any()
                };
                let player_name = player_name(&common.memberships().get(), player);
                let withdrawn = common.memberships().get().withdrawn.contains(&player);
                let mut resolved_games = 0;
                let mut total_games = 0;
                let matchups = order
                    .iter()
                    .enumerate()
                    .filter_map(|(index, (_, opponent, _))| {
                        if *opponent == player {
                            return None;
                        }
                        let cell = cells.get(&unordered(player, *opponent))?;
                        resolved_games
                            += cell
                                .slots
                                .iter()
                                .filter(|slot| {
                                    slot.try_get().is_some_and(|slot| slot.resolution.is_some())
                                })
                                .count();
                        total_games += cell.slots.len();
                        Some((index + 1, *opponent, cell.slots.clone(), cell.match_result.clone()))
                    })
                    .collect::<Vec<_>>();
                let matchup_views = matchups
                    .into_iter()
                    .map(|(opponent_number, opponent, slots, match_result)| {
                        view! {
                            <RoundRobinPlayerMatchup
                                common
                                round_robin
                                primary_match_points
                                match_result
                                point_system
                                player
                                opponent
                                opponent_number
                                slots
                            />
                        }
                    })
                    .collect_view();
                view! {
                    <div class="flex gap-3 justify-between items-end px-1">
                        <div class="min-w-0">
                            <p class=if withdrawn {
                                "block max-w-full font-bold line-through truncate"
                            } else {
                                "block max-w-full font-bold truncate"
                            }>{player_name}</p>

                            <p class="text-xs tabular-nums text-gray-500 dark:text-gray-400">
                                {t_string!(
                                    i18n, tournaments.ui.rank_points, value1 = rank
                                        .map(|rank| rank.to_string())
                                        .unwrap_or_else(|| String::from("—")), total_score = total_score
                                )
                                    .to_string()}
                            </p>
                        </div>

                        <p class="text-xs tabular-nums text-right text-gray-500 dark:text-gray-400 shrink-0">
                            {move || {
                                t_string!(
                                    i18n, tournaments.ui.resolved_game_count, resolved_games = resolved_games, total_games = total_games
                                )
                                    .to_string()
                            }}
                        </p>
                    </div>
                    {if matchup_views.is_empty() {
                        view! {
                            <p class="text-sm text-gray-500">
                                {t!(i18n, tournaments.ui.no_opponent_results_yet)}
                            </p>
                        }
                            .into_any()
                    } else {
                        view! { <div class="space-y-2">{matchup_views}</div> }.into_any()
                    }}
                }
                    .into_any()
            }}
        </div>
    }
}

#[component]
pub fn RoundRobinCrosstable(
    common: Store<TournamentCommon>,
    round_robin: Store<RoundRobinState>,
    user_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let point_system = round_robin
        .configuration()
        .get_untracked()
        .rules
        .point_system;
    let primary_match_points = round_robin
        .configuration()
        .get_untracked()
        .rules
        .primary_score
        == RoundRobinPrimaryScore::MatchPoints;
    let i18n = use_i18n();
    let hovered = RwSignal::new(None::<(Uuid, Uuid)>);
    let opened = RwSignal::new(None::<(Uuid, Uuid)>);
    let close_drawer = Callback::new(move |()| opened.set(None));
    let mobile_choice = RwSignal::new(None::<Uuid>);
    let table = Memo::new(move |_| {
        let viewer = user_id.get();
        let memberships = common.memberships().get();
        let standings = common.standings().get();
        let configuration = round_robin.configuration().get();
        let rounds = round_robin.rounds().get();
        RoundRobinTable {
            order: round_robin_matrix_order(&memberships, &standings, &configuration, viewer),
            cells: round_robin_cells(&rounds, round_robin),
        }
    });
    let player_order = Memo::new(move |_| {
        table.with(|table| {
            table
                .order
                .iter()
                .map(|(_, player, _)| *player)
                .collect::<Vec<_>>()
        })
    });
    let mobile_player = Signal::derive(move || {
        table.with(|table| {
            mobile_choice
                .get()
                .filter(|chosen| table.order.iter().any(|(_, player, _)| player == chosen))
                .or_else(|| table.order.first().map(|(_, player, _)| *player))
        })
    });
    let opened_cell = Memo::new(move |_| {
        let (left, right) = opened.get()?;
        table.with(|table| table.cells.get(&unordered(left, right)).cloned())
    });

    view! {
        <Panel title=move || common.lifecycle().get().name class="min-w-0">
            <div class="space-y-3 md:hidden" data-testid="round-robin-player-results">
                <div>

                    <label class="block mb-1 font-bold tracking-wide text-gray-500 uppercase text-[10px]">
                        {t!(i18n, tournaments.ui.player_results)}
                    </label>
                    <select
                        class="w-full ui-field-select"
                        prop:value=move || {
                            mobile_player.get().map(|player| player.to_string()).unwrap_or_default()
                        }
                        on:change=move |event| {
                            mobile_choice.set(Uuid::parse_str(&event_target_value(&event)).ok())
                        }
                    >
                        {move || {
                            table
                                .with(|table| table.order.clone())
                                .into_iter()
                                .enumerate()
                                .map(|(index, (_, player, _))| {
                                    let name = player_name(&common.memberships().get(), player);
                                    let withdrawn = common
                                        .memberships()
                                        .get()
                                        .withdrawn
                                        .contains(&player);
                                    view! {
                                        <option
                                            class=withdrawn.then_some("line-through")
                                            value=player.to_string()
                                        >
                                            {format!("#{} {name}", index + 1)}
                                        </option>
                                    }
                                })
                                .collect_view()
                        }}
                    </select>
                </div>
                {move || {
                    mobile_player
                        .get()
                        .map(|player| {
                            view! {
                                <RoundRobinPlayerLedger
                                    common
                                    round_robin
                                    point_system
                                    primary_match_points
                                    player
                                    table
                                />
                            }
                        })
                }}
            </div>
            <div class="hidden space-y-3 md:block">
                <div class="overflow-x-auto" data-testid="round-robin-crosstable">
                    <table class="min-w-full text-xs border-collapse">
                        <thead>
                            <tr>
                                <th class=CROSS_STICKY_NUMBER_HEAD>"#"</th>
                                <th class=CROSS_STICKY_PLAYER_HEAD>{t!(i18n, common.player)}</th>
                                <For
                                    each=move || player_order.get()
                                    key=|player| *player
                                    children=move |player| {
                                        view! {
                                            <th class=move || {
                                                let selected = opened
                                                    .get()
                                                    .is_some_and(|(_, column)| column == player)
                                                    || hovered
                                                        .get()
                                                        .is_some_and(|(_, column)| column == player);
                                                if selected {
                                                    format!("{CROSS_HEAD} bg-pillbug-teal/20")
                                                } else {
                                                    CROSS_HEAD.to_string()
                                                }
                                            }>
                                                <span
                                                    class="inline-block py-1 w-10 font-bold tabular-nums"
                                                    title=move || player_name(
                                                        &common.memberships().get(),
                                                        player,
                                                    )
                                                >
                                                    {move || {
                                                        player_order
                                                            .with(|players| {
                                                                players.iter().position(|id| *id == player).unwrap_or(0) + 1
                                                            })
                                                    }}
                                                </span>
                                            </th>
                                        }
                                    }
                                />
                                <th class=CROSS_HEAD>
                                    {t!(i18n, tournaments.view.encounters.total)}
                                </th>
                            </tr>
                        </thead>
                        <tbody on:mouseleave=move |_| { hovered.set(None) }>
                            <For
                                each=move || player_order.get()
                                key=|player| *player
                                children=move |row| {
                                    view! {
                                        <RoundRobinMatrixPlayerRow
                                            common
                                            round_robin
                                            primary_match_points
                                            point_system
                                            row
                                            table
                                            player_order
                                            opened
                                            hovered
                                        />
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>
            </div>
            <For
                each=move || opened.get().filter(|_| opened_cell.with(Option::is_some)).into_iter()
                key=|pair| *pair
                children=move |(left, right)| {
                    let match_points = Signal::derive(move || {
                        opened_cell
                            .with(|cell| {
                                cell.as_ref()?
                                    .match_result
                                    .as_ref()
                                    .map(|encounter| round_robin_match_score_text(
                                        Some(encounter),
                                        left,
                                    ))
                            })
                    });
                    let series = Memo::new_with_compare(
                        move |_| {
                            opened_cell
                                .get()
                                .into_iter()
                                .map(|cell| {
                                    TournamentDrawerSeries {
                                        label: None,
                                        slots: cell.slots,
                                        participants: [left, right],
                                        score: None,
                                        sets: None,
                                        outcome: None,
                                    }
                                })
                                .collect::<Vec<_>>()
                        },
                        |_, _| true,
                    );
                    view! {
                        <TournamentGamesDrawer
                            common
                            numbering_format=TournamentFormatState::RoundRobin(round_robin)
                            match_points
                            series
                            point_system
                            close=close_drawer
                        />
                    }
                }
            />
        </Panel>
    }
}

#[derive(Clone)]
enum SwissRoundItem {
    Encounter(SwissEncounterView),
    Bye(SwissByeResponse),
}

#[derive(Clone)]
struct SwissEncounterView {
    pairing_index: u32,
    participants: [Uuid; 2],
    pre_round_primary_scores: [Score; 2],
    rating_snapshots: [Option<u32>; 2],
    slots: Vec<ArcField<SlotResponse>>,
    completion: Option<SwissMatchCompletionResponse>,
}

fn is_double_swiss(configuration: &SwissConfig) -> bool {
    matches!(&configuration.system, SwissSystem::DoubleSwiss(_))
}

fn swiss_primary_score_text(configuration: &SwissConfig, score: Score) -> String {
    standings_value_text(
        Value::Score(score),
        primary_score_presentation(&FormatConfig::Swiss(configuration.clone())),
    )
}

fn swiss_rating(
    configuration: &SwissConfig,
    memberships: &TournamentMemberships,
    player: Uuid,
    snapshot: Option<u32>,
) -> Option<u64> {
    snapshot.map(u64::from).or_else(|| {
        let speed = GameSpeed::from(configuration.clock);
        memberships
            .players
            .get(&player)
            .map(|user| user.rating_for_speed(&speed))
    })
}

fn swiss_encounter_rating(
    configuration: &SwissConfig,
    memberships: &TournamentMemberships,
    encounter: &SwissEncounterView,
    player: Uuid,
) -> Option<u64> {
    let side = encounter
        .participants
        .iter()
        .position(|participant| *participant == player)?;
    let snapshot = encounter.rating_snapshots[side].or_else(|| {
        encounter.slots.iter().find_map(|slot| {
            let slot = slot.try_get()?;
            slot.resolution?;
            let side = slot
                .participants
                .iter()
                .position(|participant| *participant == player)?;
            slot.game.as_ref()?.ratings[side]
        })
    });
    swiss_rating(configuration, memberships, player, snapshot)
}

fn swiss_encounter_result(
    configuration: &SwissConfig,
    encounter: &SwissEncounterView,
    left: Uuid,
) -> String {
    let match_points = matches!(
        &configuration.system,
        SwissSystem::DoubleSwiss(system)
            if system.primary_score == DoubleSwissPrimaryScore::MatchPoints
    );
    if let Some(completion) = encounter.completion {
        let points = if match_points {
            completion.match_points
        } else {
            completion.game_points
        };
        let side = usize::from(encounter.participants[1] == left);
        return format!(
            "{} – {}",
            swiss_primary_score_text(configuration, points[side]),
            swiss_primary_score_text(configuration, points[1 - side]),
        );
    }
    if match_points {
        return String::from("—");
    }
    let mut totals = [0u32; 2];
    let mut scored = 0usize;
    for slot in encounter.slots.iter().filter_map(|slot| slot.try_get()) {
        let Some(points) = slot.awarded_game_points else {
            continue;
        };
        let oriented = if slot.participants[0] == left {
            points
        } else {
            [points[1], points[0]]
        };
        let Some(left_total) = totals[0].checked_add(oriented[0].value()) else {
            return String::from("—");
        };
        let Some(right_total) = totals[1].checked_add(oriented[1].value()) else {
            return String::from("—");
        };
        totals = [left_total, right_total];
        scored += 1;
    }
    if scored == 0 {
        return String::from("—");
    }
    format!(
        "{} – {}",
        format_game_points(Score::new(totals[0]), configuration.game_point_system),
        format_game_points(Score::new(totals[1]), configuration.game_point_system),
    )
}

fn swiss_encounter_progress(configuration: &SwissConfig, encounter: &SwissEncounterView) -> String {
    let i18n = use_i18n();
    let slots = encounter
        .slots
        .iter()
        .filter_map(|slot| slot.try_get())
        .collect::<Vec<_>>();
    let finished = slots
        .iter()
        .filter(|slot| slot.resolution.is_some())
        .count();
    if is_double_swiss(configuration) && finished > 0 {
        return t_string!(i18n, tournaments.ui.of_2_finished, finished = finished).to_string();
    }

    slots
        .iter()
        .min_by_key(|slot| unresolved_slot_state_priority(slot))
        .map_or(t_string!(i18n, common.pending).to_string(), slot_state)
        .to_string()
}

fn swiss_round_items(round: &SwissRound, swiss: Store<SwissState>) -> Vec<SwissRoundItem> {
    round
        .encounters
        .iter()
        .map(|encounter| {
            SwissRoundItem::Encounter(SwissEncounterView {
                pairing_index: encounter.pairing_index,
                participants: encounter.participants,
                pre_round_primary_scores: encounter.pre_round_primary_scores,
                rating_snapshots: encounter.rating_snapshots,
                completion: encounter.completion,
                slots: encounter
                    .slot_ids
                    .iter()
                    .map(|slot_id| swiss.slots().at_key(*slot_id).into())
                    .collect(),
            })
        })
        .chain(round.byes.iter().copied().map(SwissRoundItem::Bye))
        .collect()
}

fn swiss_round_item_matches(
    memberships: &TournamentMemberships,
    item: &SwissRoundItem,
    needle: &str,
) -> bool {
    if needle.is_empty() {
        return true;
    }
    match item {
        SwissRoundItem::Encounter(encounter) => encounter.participants.iter().any(|player| {
            player_name(memberships, *player)
                .to_lowercase()
                .contains(needle)
        }),
        SwissRoundItem::Bye(bye_) => player_name(memberships, bye_.player)
            .to_lowercase()
            .contains(needle),
    }
}

#[component]
fn SwissRoundLedgerItem(
    common: Store<TournamentCommon>,
    configuration: SwissConfig,
    item: SwissRoundItem,
    round_index: u32,
    open_pairing: Callback<(u32, u32)>,
    interactive: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    match item {
        SwissRoundItem::Encounter(encounter) => {
            let pairing_index = encounter.pairing_index;
            let participants = encounter.participants;
            let memberships = common.memberships().get_untracked();
            let left_name = player_name(&memberships, participants[0]);
            let right_name = player_name(&memberships, participants[1]);
            view! {
                {move || {
                    let first_slot = encounter.slots.first().and_then(|slot| slot.try_get());
                    let game_id = first_slot.as_ref().and_then(|slot| slot.game_id().cloned());
                    let color = |player| {
                        first_slot
                            .as_ref()
                            .and_then(|slot| {
                                slot.participants
                                    .iter()
                                    .position(|participant| *participant == player)
                                    .map(|side| if side == 0 { Color::White } else { Color::Black })
                            })
                    };
                    let detail = |side: usize| {
                        swiss_primary_score_text(
                            &configuration,
                            encounter.pre_round_primary_scores[side],
                        )
                    };
                    let left = EncounterParticipant {
                        name: left_name.clone(),
                        detail: Some(detail(0)),
                        rating: swiss_encounter_rating(
                            &configuration,
                            &memberships,
                            &encounter,
                            participants[0],
                        ),
                        color: color(participants[0]),
                    };
                    let right = EncounterParticipant {
                        name: right_name.clone(),
                        detail: Some(detail(1)),
                        rating: swiss_encounter_rating(
                            &configuration,
                            &memberships,
                            &encounter,
                            participants[1],
                        ),
                        color: color(participants[1]),
                    };
                    let result = swiss_encounter_result(
                        &configuration,
                        &encounter,
                        first_slot.as_ref().map_or(participants[0], SlotResponse::white),
                    );
                    let progress = swiss_encounter_progress(&configuration, &encounter);
                    if interactive {

                        view! {
                            <article
                                data-page-row
                                class="min-w-0 rounded border @container border-black/10 dark:border-white/10"
                            >
                                <button
                                    type="button"
                                    class="block py-3 px-2 w-full text-left rounded sm:px-3 hover:bg-pillbug-teal/10 focus-visible:outline-pillbug-teal"
                                    on:click=move |_| open_pairing.run((round_index, pairing_index))
                                >
                                    <TournamentEncounterSummary
                                        identity=format!("#{}", pairing_index + 1)
                                        left
                                        right=Some(right)
                                        result
                                        progress
                                        action=move || {
                                            t_string!(i18n, tournaments.ui.view_match).to_string()
                                        }
                                        game_count=2
                                    />
                                </button>
                            </article>
                        }
                            .into_any()
                    } else if let Some(game_id) = game_id {

                        view! {
                            <article
                                data-page-row
                                class="min-w-0 rounded border @container border-black/10 dark:border-white/10"
                            >
                                <a
                                    class="block py-3 px-2 w-full text-left rounded sm:px-3 no-link-style hover:bg-pillbug-teal/10 focus-visible:outline-pillbug-teal"
                                    href=format!("/game/{game_id}")
                                >
                                    <TournamentEncounterSummary
                                        identity=format!("#{}", pairing_index + 1)
                                        left
                                        right=Some(right)
                                        result
                                        progress
                                        action=move || {
                                            t_string!(i18n, common.open_game).to_string()
                                        }
                                    />
                                </a>
                            </article>
                        }
                            .into_any()
                    } else {
                        view! {
                            <article
                                data-page-row
                                class="py-3 px-2 min-w-0 rounded border sm:px-3 @container border-black/10 dark:border-white/10"
                            >
                                <TournamentEncounterSummary
                                    identity=format!("#{}", pairing_index + 1)
                                    left
                                    right=Some(right)
                                    result
                                    progress
                                />
                            </article>
                        }
                            .into_any()
                    }
                }}
            }
            .into_any()
        }
        SwissRoundItem::Bye(bye_) => {
            let memberships = common.memberships().get_untracked();
            let (name, rating, score) = {
                (
                    player_name(&memberships, bye_.player),
                    swiss_rating(
                        &configuration,
                        &memberships,
                        bye_.player,
                        bye_.rating_snapshot,
                    ),
                    swiss_primary_score_text(&configuration, bye_.pre_round_primary_score),
                )
            };

            let state = t_string!(i18n, common.bye).to_string();

            let participant = EncounterParticipant {
                name,
                color: None,
                rating,
                detail: Some(score),
            };
            view! {
                <article
                    data-page-row
                    class="py-3 px-2 min-w-0 rounded border sm:px-3 @container border-black/10 dark:border-white/10"
                >
                    <TournamentEncounterSummary
                        identity=String::from("—")
                        left=participant
                        right=None
                        result=state
                    />
                </article>
            }
            .into_any()
        }
    }
}

#[component]
pub fn SwissRoundBrowser(
    common: Store<TournamentCommon>,
    swiss: Store<SwissState>,
) -> impl IntoView {
    let i18n = use_i18n();
    let selected = RwSignal::new(swiss.rounds().get_untracked().len().saturating_sub(1));
    let search = RwSignal::new(String::new());
    let container = NodeRef::<html::Div>::new();
    let opened_pairing = RwSignal::new(None::<(u32, u32)>);
    let open_pairing = Callback::new(move |pairing| opened_pairing.set(Some(pairing)));
    let close_drawer = Callback::new(move |()| opened_pairing.set(None));
    let opened_encounter = Memo::new(move |_| {
        let (round_index, pairing_index) = opened_pairing.get()?;
        swiss.rounds().with(|rounds| {
            rounds
                .iter()
                .find(|round| round.round_index == round_index)?
                .encounters
                .iter()
                .find(|encounter| encounter.pairing_index == pairing_index)
                .cloned()
        })
    });
    let suggestions_id = StoredValue::new(format!(
        "round-browser-suggestions-{}",
        common.lifecycle().get_untracked().tournament_id.0
    ));
    let entrant_names = Signal::derive(move || {
        common
            .memberships()
            .get()
            .players
            .values()
            .map(|player| player.username.clone())
            .collect::<Vec<_>>()
    });
    let filtered_items = Memo::new_with_compare(
        move |_| {
            let needle = search.get().trim().to_lowercase();
            let memberships = common.memberships().get();
            swiss
                .rounds()
                .get()
                .get(selected.get())
                .map(|round| swiss_round_items(round, swiss))
                .unwrap_or_default()
                .into_iter()
                .filter(|item| swiss_round_item_matches(&memberships, item, &needle))
                .collect::<Vec<_>>()
        },
        |_, _| true,
    );
    let total = Signal::derive(move || filtered_items.with(Vec::len));
    let pagination = use_fitted_pagination(container, total, 10);
    let page = pagination.page;
    let page_size = pagination.page_size;
    let on_page_change = Callback::new(move |next| page.set(next));
    let on_query_change = Callback::new(move |query| {
        search.set(query);
        page.set(1);
    });
    Effect::new(move |_| {
        let last = swiss.rounds().get().len().saturating_sub(1);
        if selected.get_untracked() > last {
            selected.set(last);
            page.set(1);
        }
    });

    view! {
        <Panel class="min-w-0" body_class="@container">
            <div node_ref=container data-testid="swiss-pairings">
                {move || {
                    let configuration = swiss.configuration().get();
                    let rounds = swiss.rounds().get();
                    let selected_index = selected.get().min(rounds.len().saturating_sub(1));
                    let round = rounds.get(selected_index)?.clone();
                    let (finished, encounter_count) = swiss_round_progress(
                            &rounds,
                            round.round_index,
                        )
                        .unwrap_or((0, round.encounters.len() + round.byes.len()));
                    let configured_rounds = configuration
                        .rounds
                        .resolved_rounds()
                        .map(|rounds| rounds.get() as usize)
                        .unwrap_or(rounds.len());
                    let previous = selected_index.checked_sub(1);
                    let next = (selected_index + 1 < rounds.len()).then_some(selected_index + 1);
                    Some(
                        view! {
                            <div class="flex flex-wrap gap-3 justify-between items-center pb-3 border-b border-black/10 dark:border-white/10">

                                <nav
                                    aria-label=move || {
                                        t_string!(i18n, tournaments.ui.tournament_rounds)
                                            .to_string()
                                    }
                                    class="hidden flex-wrap gap-1 items-center @min-[48rem]:flex"
                                >
                                    <span class="mr-2 text-sm font-semibold">
                                        {t!(i18n, tournaments.ui.round_heading)}
                                    </span>
                                    {rounds
                                        .iter()
                                        .enumerate()
                                        .map(|(index, round)| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class=if index == selected_index {
                                                        "ui-button ui-button-primary ui-button-sm min-w-10"
                                                    } else {
                                                        "ui-button ui-button-secondary ui-button-sm min-w-10"
                                                    }
                                                    aria-current=(index == selected_index).then_some("step")
                                                    on:click=move |_| {
                                                        selected.set(index);
                                                        page.set(1);
                                                    }
                                                >
                                                    {round.round_index + 1}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </nav>
                                <div class="flex gap-2 justify-between items-center w-full @min-[48rem]:hidden">

                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        disabled=previous.is_none()
                                        on:click=move |_| {
                                            if let Some(previous) = previous {
                                                selected.set(previous);
                                                page.set(1);
                                            }
                                        }
                                    >
                                        {move || {
                                            t_string!(i18n, tournaments.ui.previous_round).to_string()
                                        }}
                                    </button>

                                    <span class="text-sm font-semibold tabular-nums">
                                        {format!("{} / {configured_rounds}", round.round_index + 1)}
                                    </span>

                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        disabled=next.is_none()
                                        on:click=move |_| {
                                            if let Some(next) = next {
                                                selected.set(next);
                                                page.set(1);
                                            }
                                        }
                                    >
                                        {move || {
                                            t_string!(i18n, tournaments.ui.next_round).to_string()
                                        }}
                                    </button>
                                </div>
                                <TournamentStandingsControls
                                    page=page.into()
                                    total
                                    page_size
                                    query=search.into()
                                    on_query_change
                                    on_page_change
                                    entrant_names
                                    suggestions_id=suggestions_id.get_value()
                                    search_only=true
                                />
                            </div>
                            <div class="flex flex-wrap gap-y-1 gap-x-4 justify-between items-baseline py-3">
                                <div class="flex flex-wrap gap-y-1 gap-x-3 items-baseline">

                                    <strong class="text-base">
                                        {move || {
                                            t_string!(
                                                i18n, tournaments.view.bracket.stage.round, round = round.round_index + 1
                                            )
                                                .to_string()
                                        }}
                                    </strong>

                                    <span class="text-xs text-gray-500">
                                        {move || {
                                            t_string!(
                                                i18n, tournaments.ui.of_finished, finished = finished, encounter_count = encounter_count
                                            )
                                                .to_string()
                                        }}
                                    </span>
                                </div>

                                <span class="text-xs text-gray-500">
                                    {if matches!(
                                        &configuration.system,
                                        SwissSystem::DoubleSwiss(system)
                                        if system.primary_score
                                            == DoubleSwissPrimaryScore::MatchPoints
                                    ) {
                                        t_string!(
                                            i18n, tournaments.ui.match_points_before_this_round
                                        )
                                            .to_string()
                                    } else {
                                        t_string!(i18n, tournaments.ui.points_before_this_round)
                                            .to_string()
                                    }}
                                </span>
                            </div>
                        },
                    )
                }}
                <div
                    data-page-items
                    class="grid grid-cols-1 gap-2 @min-[64rem]:grid-cols-2 @min-[64rem]:gap-x-4"
                >
                    {move || {
                        let start = page.get().saturating_sub(1).saturating_mul(page_size.get());
                        let visible = filtered_items
                            .get()
                            .into_iter()
                            .skip(start)
                            .take(page_size.get())
                            .collect::<Vec<_>>();
                        if visible.is_empty() {
                            view! {
                                <p class="col-span-full ui-empty-state">
                                    {move || {
                                        t_string!(
                                            i18n, tournaments.ui.no_pairings_match_this_search
                                        )
                                            .to_string()
                                    }}
                                </p>
                            }
                                .into_any()
                        } else {
                            let configuration = swiss.configuration().get();
                            let interactive = is_double_swiss(&configuration);
                            let round_index = swiss
                                .rounds()
                                .get()
                                .get(selected.get())
                                .map(|round| round.round_index)
                                .unwrap_or_default();
                            visible
                                .into_iter()
                                .map(|item| {
                                    view! {
                                        <SwissRoundLedgerItem
                                            common
                                            configuration=configuration.clone()
                                            item
                                            round_index
                                            open_pairing
                                            interactive
                                        />
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    }}
                </div>
                <div
                    class="flex justify-end pt-3"
                    class:hidden=move || total.get() <= page_size.get()
                >
                    <PaginationControls
                        page=page.into()
                        total
                        page_size
                        on_page_change
                        hide_single_page=true
                    />
                </div>
            </div>
        </Panel>
        <For
            each=move || {
                opened_pairing.get().filter(|_| opened_encounter.with(Option::is_some)).into_iter()
            }
            key=|pairing| *pairing
            children=move |_| {
                let point_system = swiss.configuration().get_untracked().game_point_system;
                let series = Memo::new_with_compare(
                    move |_| {
                        let Some(encounter) = opened_encounter.get() else {
                            return Vec::new();
                        };
                        let matchup_slots = encounter
                            .slot_ids
                            .iter()
                            .map(|slot_id| swiss.slots().at_key(*slot_id).into())
                            .collect();
                        let configuration = swiss.configuration().get();
                        let score = matches!(
                            &configuration.system,
                            SwissSystem::DoubleSwiss(system)
                            if system.primary_score == DoubleSwissPrimaryScore::MatchPoints
                        )
                            .then(|| {
                                encounter
                                    .completion
                                    .map_or_else(
                                        || String::from("—"),
                                        |completion| {
                                            format!(
                                                "{} – {}",
                                                swiss_primary_score_text(
                                                    &configuration,
                                                    completion.match_points[0],
                                                ),
                                                swiss_primary_score_text(
                                                    &configuration,
                                                    completion.match_points[1],
                                                ),
                                            )
                                        },
                                    )
                            });
                        vec![
                            TournamentDrawerSeries {
                                label: None,
                                slots: matchup_slots,
                                participants: encounter.participants,
                                score,
                                sets: None,
                                outcome: None,
                            },
                        ]
                    },
                    |_, _| true,
                );
                view! {
                    <TournamentGamesDrawer
                        common
                        series
                        point_system
                        close=close_drawer
                        numbering_format=TournamentFormatState::Swiss(swiss)
                    />
                }
            }
        />
    }
}
