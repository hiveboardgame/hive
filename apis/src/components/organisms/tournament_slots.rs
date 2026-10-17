use shared_types::tournament_view::SlotResponse;
pub(crate) mod participant_schedule;

pub use participant_schedule::{
    ParticipantScheduleIndex,
    ParticipantScheduleLayout,
    ParticipantScheduleOpponent,
};

use crate::{
    common::{
        format_local_datetime as format_schedule_time,
        tournament_game_number,
        unfinished_tournament_game_label,
        ScheduleAction,
        TournamentAction,
        TournamentAdjudicationIntent,
    },
    components::{
        atoms::{date_time_picker::DateTimePicker, message_button::MessageButton},
        molecules::modal::Modal,
        organisms::tournament_closeout::TournamentCloseout,
    },
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        schedules::ScheduleMap,
        ApiRequestsProvider,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatState,
        TournamentScheduleState,
        TournamentState,
    },
    responses::{ScheduleResponse, TournamentMemberships},
};
use chrono::{DateTime, Duration, Local, Timelike, Utc};
use hive_lib::{Color, GameStatus};
use leptos::{html::Dialog, prelude::*};
use leptos_i18n::I18nContext;
use leptos_router::{
    components::{Outlet, A},
    hooks::{use_location, use_navigate},
    NavigateOptions,
};
use reactive_stores::ArcField;
use shared_types::{
    tournament::{
        elimination::Stage,
        AdjudicatedSideResult,
        Format,
        FormatConfig,
        GameOutcome,
        PlayedGameOutcome,
        Resolution,
        SlotKey,
        SwissLeg,
    },
    Clock,
    ScheduleOfferStatus,
    SlotAdminAction,
    TournamentGameResult,
    TournamentId,
    SCHEDULED_GAME_GRACE,
};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) fn outcome_label(i18n: I18nContext<Locale, I18nKeys>, outcome: GameOutcome) -> String {
    match outcome {
        GameOutcome::Played(PlayedGameOutcome::WhiteWin) => {
            t_string!(i18n, tournaments.view.slots.outcome.played_white_win).to_string()
        }
        GameOutcome::Played(PlayedGameOutcome::Draw) => {
            t_string!(i18n, tournaments.view.slots.outcome.played_draw).to_string()
        }
        GameOutcome::Played(PlayedGameOutcome::BlackWin) => {
            t_string!(i18n, tournaments.view.slots.outcome.played_black_win).to_string()
        }
        GameOutcome::Adjudicated(outcome) => match (outcome.white(), outcome.black()) {
            (AdjudicatedSideResult::ForfeitWin, AdjudicatedSideResult::ForfeitLoss) => {
                t_string!(i18n, tournaments.view.slots.outcome.white_forfeit_win).to_string()
            }
            (AdjudicatedSideResult::ForfeitLoss, AdjudicatedSideResult::ForfeitWin) => {
                t_string!(i18n, tournaments.view.slots.outcome.black_forfeit_win).to_string()
            }
            (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw) => {
                t_string!(i18n, tournaments.view.slots.outcome.adjudicated_draw).to_string()
            }
            (AdjudicatedSideResult::Draw, AdjudicatedSideResult::ForfeitLoss) => t_string!(
                i18n,
                tournaments.view.slots.outcome.white_draw_black_forfeit
            )
            .to_string(),
            (AdjudicatedSideResult::ForfeitLoss, AdjudicatedSideResult::Draw) => t_string!(
                i18n,
                tournaments.view.slots.outcome.white_forfeit_black_draw
            )
            .to_string(),
            (AdjudicatedSideResult::DoubleForfeit, AdjudicatedSideResult::DoubleForfeit) => {
                t_string!(i18n, tournaments.view.slots.outcome.double_forfeit).to_string()
            }
            _ => unreachable!("validated adjudicated outcome"),
        },
    }
}

pub(crate) fn stage_text(i18n: I18nContext<Locale, I18nKeys>, stage: Stage) -> String {
    match stage {
        Stage::SingleRound { round_index } => t_string!(
            i18n,
            tournaments.view.bracket.stage.round,
            round = round_index + 1,
        )
        .to_string(),
        Stage::SingleFinal => {
            t_string!(i18n, tournaments.view.bracket.stage.final_match).to_string()
        }
        Stage::Bronze => t_string!(i18n, tournaments.pairing.stage.third_place).to_string(),
        Stage::WinnersRound { round_index } => t_string!(
            i18n,
            tournaments.view.bracket.stage.winners_round,
            round = round_index + 1,
        )
        .to_string(),
        Stage::LosersMinor { .. } => t_string!(
            i18n,
            tournaments.view.bracket.stage.losers_minor,
            round = stage
                .lower_round_ordinal()
                .expect("lower-bracket stage has a native ordinal")
                + 1,
        )
        .to_string(),
        Stage::LosersMajor { .. } => t_string!(
            i18n,
            tournaments.view.bracket.stage.losers_major,
            round = stage
                .lower_round_ordinal()
                .expect("lower-bracket stage has a native ordinal")
                + 1,
        )
        .to_string(),
        Stage::GrandFinal => t_string!(i18n, tournaments.pairing.stage.grand_final).to_string(),
        Stage::Reset => t_string!(i18n, tournaments.pairing.stage.reset_final).to_string(),
    }
}

pub(crate) fn slot_card_label(
    i18n: I18nContext<Locale, I18nKeys>,
    tournament: TournamentState,
    slot: &SlotResponse,
) -> Option<String> {
    match (slot.key, tournament.format) {
        (SlotKey::RoundRobin { .. }, TournamentFormatState::RoundRobin(format)) => {
            let repeats = format.configuration().get().rules.repeats;
            format.rounds().get().iter().find_map(|round| {
                round
                    .slots
                    .iter()
                    .any(|round_slot| round_slot.slot_id == slot.id)
                    .then(|| {
                        match tournament_game_number(tournament.format, slot, round.pass_index + 1)
                        {
                            Some(number) if repeats > 1 => t_string!(
                                i18n,
                                tournaments.ui.game_of,
                                number = (number),
                                value1 = (repeats)
                            )
                            .to_string(),
                            Some(number) => {
                                t_string!(i18n, tournaments.schedule_context.game, game = (number))
                                    .to_string()
                            }
                            None => unfinished_tournament_game_label(i18n.get_locale(), slot)
                                .to_string(),
                        }
                    })
            })
        }
        (SlotKey::Swiss { slot: native_slot }, TournamentFormatState::Swiss(format)) => {
            let kind = FormatConfig::Swiss(format.configuration().get()).kind();
            format.rounds().get().iter().find_map(|round| {
                round.encounters.iter().find_map(|encounter| {
                    encounter.slot_ids.contains(&slot.id).then(|| {
                        let base = t_string!(
                            i18n,
                            tournaments.schedule_context.round_board,
                            round = (round.round_index + 1),
                            board = (encounter.pairing_index + 1)
                        )
                        .to_string();
                        if kind == Format::DoubleSwiss {
                            let scheduled = if native_slot.leg == SwissLeg::Second {
                                2
                            } else {
                                1
                            };
                            match tournament_game_number(tournament.format, slot, scheduled) {
                                Some(number) => t_string!(
                                    i18n,
                                    tournaments.view.slots.base_game_number_of,
                                    base = (base),
                                    number = (number)
                                )
                                .to_string(),
                                None => {
                                    format!(
                                        "{base} · {}",
                                        unfinished_tournament_game_label(i18n.get_locale(), slot)
                                    )
                                }
                            }
                        } else {
                            base
                        }
                    })
                })
            })
        }
        (
            SlotKey::Elimination {
                node: node_id,
                slot: _,
            },
            TournamentFormatState::Elimination(format),
        ) => {
            let nodes = format.nodes().get();
            let node = nodes.iter().find(|node| node.node_id == node_id)?;
            let mut parts = vec![stage_text(i18n, node.stage)];
            if nodes
                .iter()
                .filter(|candidate| candidate.stage == node.stage)
                .count()
                > 1
            {
                parts.push(
                    t_string!(
                        i18n,
                        tournaments.view.slots.match_value1,
                        value1 = (node.stage_ordinal + 1)
                    )
                    .to_string(),
                );
            }
            if let Some(series) = &node.series {
                for (set_index, set) in series.sets.iter().enumerate() {
                    if let Some(game_index) =
                        set.slots.iter().position(|entry| entry.slot_id == slot.id)
                    {
                        parts.push(
                            t_string!(
                                i18n,
                                tournaments.view.slots.set_value1_game_value2_of_value3,
                                value1 = (set_index + 1),
                                value2 = (game_index + 1),
                                value3 = (set.slots.len())
                            )
                            .to_string(),
                        );
                        break;
                    }
                }
            }
            Some(parts.join(" · "))
        }
        _ => None,
    }
}

pub(crate) fn slot_ordering(
    tournament: TournamentState,
) -> impl Fn(&SlotResponse) -> (u8, usize, usize, usize, usize) {
    let round_robin = match tournament.format {
        TournamentFormatState::RoundRobin(format) => format.rounds().with(|rounds| {
            rounds
                .iter()
                .flat_map(|round| {
                    round.slots.iter().map(move |slot| {
                        (
                            slot.slot_id,
                            (
                                0,
                                round.pass_index,
                                round.round_index,
                                slot.board_index,
                                round.pass_index,
                            ),
                        )
                    })
                })
                .collect::<HashMap<_, _>>()
        }),
        _ => HashMap::new(),
    };
    let elimination = match tournament.format {
        TournamentFormatState::Elimination(format) => format.nodes().with(|nodes| {
            nodes
                .iter()
                .map(|node| (node.node_id, (node.wave_index, node.stage_ordinal)))
                .collect::<HashMap<_, _>>()
        }),
        _ => HashMap::new(),
    };
    move |slot| match (slot.key, tournament.format) {
        (SlotKey::RoundRobin { .. }, TournamentFormatState::RoundRobin(_)) => round_robin
            .get(&slot.id)
            .copied()
            .unwrap_or((0, usize::MAX, usize::MAX, usize::MAX, usize::MAX)),
        (SlotKey::Swiss { slot: native_slot }, TournamentFormatState::Swiss(_)) => (
            1,
            native_slot.round_index as usize,
            native_slot.pairing_index as usize,
            match native_slot.leg {
                SwissLeg::Single | SwissLeg::First => 0,
                SwissLeg::Second => 1,
            },
            0,
        ),
        (
            SlotKey::Elimination {
                node: node_id,
                slot: native_slot,
            },
            TournamentFormatState::Elimination(_),
        ) => elimination
            .get(&node_id)
            .map(|&(wave_index, stage_ordinal)| {
                (
                    2,
                    wave_index,
                    stage_ordinal,
                    node_id.value(),
                    native_slot.value() as usize,
                )
            })
            .unwrap_or((2, usize::MAX, usize::MAX, usize::MAX, usize::MAX)),
        _ => (u8::MAX, usize::MAX, usize::MAX, usize::MAX, usize::MAX),
    }
}

pub(super) fn fixed_slot_fields(tournament: TournamentState) -> Vec<ArcField<SlotResponse>> {
    let slot_ids = match tournament.format {
        TournamentFormatState::Arena(_) => return Vec::new(),
        TournamentFormatState::RoundRobin(format) => format
            .rounds()
            .get()
            .into_iter()
            .flat_map(|round| round.slots.into_iter().map(|slot| slot.slot_id))
            .collect::<Vec<_>>(),
        TournamentFormatState::Swiss(format) => format
            .rounds()
            .get()
            .into_iter()
            .flat_map(|round| round.encounters)
            .flat_map(|encounter| encounter.slot_ids)
            .collect(),
        TournamentFormatState::Elimination(format) => format
            .nodes()
            .get()
            .into_iter()
            .filter_map(|node| node.series)
            .flat_map(|series| series.sets)
            .flat_map(|set| set.slots.into_iter().map(|slot| slot.slot_id))
            .collect(),
    };
    slot_ids
        .into_iter()
        .filter_map(|slot_id| tournament.format.slot(slot_id))
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SlotOffers {
    pub(crate) latest_pending: Option<ScheduleResponse>,
    pub(crate) latest: Option<ScheduleResponse>,
}

pub(crate) fn offers_by_slot(
    schedules: &ScheduleMap,
    tournament_id: &TournamentId,
) -> HashMap<Uuid, SlotOffers> {
    let mut index = HashMap::<Uuid, SlotOffers>::new();
    for offer in schedules
        .values()
        .filter(|offer| offer.tournament_id == *tournament_id)
    {
        let slot = index.entry(offer.slot_id).or_default();
        if slot
            .latest
            .as_ref()
            .is_none_or(|latest| (offer.created_at, offer.id) > (latest.created_at, latest.id))
        {
            slot.latest = Some(offer.clone());
        }
        if offer.status == ScheduleOfferStatus::Pending
            && slot.latest_pending.as_ref().is_none_or(|pending| {
                (offer.created_at, offer.id) > (pending.created_at, pending.id)
            })
        {
            slot.latest_pending = Some(offer.clone());
        }
    }
    index
}

pub(crate) fn schedule_is_available(slot: &SlotResponse, viewer_id: Uuid) -> bool {
    slot.participants.contains(&viewer_id)
        && matches!(slot.clock, Clock::Realtime(_))
        && slot.resolution.is_none()
        && slot
            .game
            .as_ref()
            .is_none_or(|game| !game.finished && game.status == GameStatus::NotStarted)
}

#[derive(Clone)]
struct AdjudicationReview {
    slot_id: Uuid,
    expected_resolution: Option<Resolution>,
    result: TournamentGameResult,
    names: [String; 2],
    slot: SlotResponse,
    pathname: String,
}

#[component]
fn SlotAdjudicationControls(
    slot: SlotResponse,
    white_name: String,
    black_name: String,
) -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<OrganizerContext>();
    let can_set_result = slot.available_admin_actions.iter().any(|action| {
        matches!(
            action,
            SlotAdminAction::RecordResult | SlotAdminAction::ReplaceResult
        )
    });
    let can_white_win = can_set_result
        && slot
            .available_results
            .contains(&TournamentGameResult::Winner(Color::White));
    let can_draw = can_set_result && slot.available_results.contains(&TournamentGameResult::Draw);
    let can_black_win = can_set_result
        && slot
            .available_results
            .contains(&TournamentGameResult::Winner(Color::Black));
    let can_double_forfeit = can_set_result
        && slot
            .available_results
            .contains(&TournamentGameResult::DoubleForfeit);
    let can_clear_result = slot
        .available_admin_actions
        .contains(&SlotAdminAction::ClearResult);
    let review = AdjudicationReview {
        slot_id: slot.id,
        expected_resolution: slot.resolution,
        result: TournamentGameResult::Unknown,
        names: [white_name, black_name],
        slot: slot.clone(),
        pathname: context.pathname.get_untracked(),
    };
    let apply = Callback::new(move |result: TournamentGameResult| {
        if current_manage_route(&context).is_none() {
            return;
        }
        context.adjudication_review.set(Some(AdjudicationReview {
            result,
            ..review.clone()
        }));
        if let Some(dialog) = context.adjudication_dialog.get() {
            let _ = dialog.show_modal();
        }
    });
    view! {
        <div class="space-y-1.5">
            <p class="text-sm font-bold">{t!(i18n, tournaments.view.slots.record_result)}</p>
            <div class="flex flex-wrap gap-1">
                <Show when=move || can_white_win>
                    <button
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            apply.run(TournamentGameResult::Winner(Color::White));
                        }
                    >
                        "1-0"
                    </button>
                </Show>
                <Show when=move || can_draw>
                    <button
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            apply.run(TournamentGameResult::Draw);
                        }
                    >
                        "½-½"
                    </button>
                </Show>
                <Show when=move || can_black_win>
                    <button
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            apply.run(TournamentGameResult::Winner(Color::Black));
                        }
                    >
                        "0-1"
                    </button>
                </Show>
                <Show when=move || can_double_forfeit>
                    <button
                        class="ui-button ui-button-danger ui-button-sm"
                        on:click=move |_| {
                            apply.run(TournamentGameResult::DoubleForfeit);
                        }
                    >
                        "0-0"
                    </button>
                </Show>
                <Show when=move || can_clear_result>
                    <button
                        class="ui-button ui-button-danger ui-button-sm"
                        on:click=move |_| {
                            apply.run(TournamentGameResult::Unknown);
                        }
                    >
                        <span title=move || {
                            t_string!(i18n, tournaments.view.slots.clear).to_string()
                        }>{t!(i18n, tournaments.view.slots.clear_result)}</span>
                    </button>
                </Show>
            </div>
            <p class="text-xs text-gray-600 dark:text-gray-400">
                {t!(i18n, tournaments.view.slots.double_forfeit_help)}
            </p>
        </div>
    }
}

#[component]
fn AdjudicationDialog() -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<OrganizerContext>();
    let api = expect_context::<ApiRequestsProvider>();
    let dialog_el = context.adjudication_dialog;
    let reviewed = context.adjudication_review;
    let cancel = Callback::new(move |()| {
        reviewed.set(None);
        if let Some(dialog) = dialog_el.get() {
            dialog.close();
        }
    });
    Effect::new(move |_| {
        if reviewed.with(|review| {
            review
                .as_ref()
                .is_some_and(|review| review.pathname != context.pathname.get())
        }) {
            cancel.run(());
        }
    });
    let confirm = move |_| {
        let Some(review) = reviewed.get_untracked() else {
            return;
        };
        cancel.run(());
        if current_manage_route(&context).is_none()
            || context.pathname.get_untracked() != review.pathname
        {
            return;
        }
        let Some(current) = context
            .tournament
            .format
            .slot(review.slot_id)
            .and_then(|slot| slot.try_get_untracked())
        else {
            return;
        };
        let required = if review.result == TournamentGameResult::Unknown {
            SlotAdminAction::ClearResult
        } else if review.expected_resolution.is_some() {
            SlotAdminAction::ReplaceResult
        } else {
            SlotAdminAction::RecordResult
        };
        if current.resolution != review.expected_resolution
            || !current.available_admin_actions.contains(&required)
            || (review.result != TournamentGameResult::Unknown
                && !current.available_results.contains(&review.result))
        {
            return;
        }
        let advance =
            review.expected_resolution.is_none() && review.result != TournamentGameResult::Unknown;
        if api.0.get().tournament(TournamentAction::AdjudicateResult(
            context.tournament_id.get_value(),
            TournamentAdjudicationIntent {
                slot_id: review.slot_id,
                expected_resolution: review.expected_resolution,
                result: review.result,
            },
        )) && advance
        {
            context
                .advance_after_result
                .set(Some((review.slot_id, review.pathname)));
        }
    };
    view! {
        <Modal
            dialog_el
            aria_label=move || t_string!(i18n, tournaments.view.slots.confirm_result).to_string()
            on_close=cancel
        >
            {move || {
                reviewed
                    .get()
                    .map(|review| {
                        let clearing = review.result == TournamentGameResult::Unknown;
                        let dangerous = clearing
                            || review.result == TournamentGameResult::DoubleForfeit;
                        let [white, black] = &review.names;
                        let result = match review.result {
                            TournamentGameResult::Winner(Color::White) => {
                                t_string!(
                                    i18n, tournaments.view.slots.white_wins_result, white = (white)
                                )
                                    .to_string()
                            }
                            TournamentGameResult::Winner(Color::Black) => {
                                t_string!(
                                    i18n, tournaments.view.slots.black_wins_result, black = (black)
                                )
                                    .to_string()
                            }
                            TournamentGameResult::Draw => {
                                t_string!(i18n, tournaments.view.slots.draw).to_string()
                            }
                            TournamentGameResult::DoubleForfeit => {
                                t_string!(i18n, tournaments.view.slots.double_forfeit_result)
                                    .to_string()
                            }
                            TournamentGameResult::Unknown => {
                                t_string!(
                                    i18n, tournaments.view.slots.remove_the_recorded_result_and_reopen_this_game
                                )
                                    .to_string()
                            }
                        };
                        let matchup_names = review.names.clone();
                        let identity_slot = review.slot.clone();
                        view! {
                            <div class="px-4 pb-4 space-y-4 w-[min(90vw,28rem)]">
                                <h2 class="text-xl font-bold">
                                    {move || {
                                        if clearing {
                                            t_string!(i18n, tournaments.view.slots.clear_result_prompt)
                                                .to_string()
                                        } else {
                                            t_string!(
                                                i18n, tournaments.view.slots.record_result_confirmation
                                            )
                                                .to_string()
                                        }
                                    }}
                                </h2>
                                <div class="space-y-1">
                                    <p class="font-semibold">
                                        {move || organizer_matchup_label(
                                            &matchup_names[0],
                                            &matchup_names[1],
                                        )}
                                    </p>
                                    {move || {
                                        organizer_match_identity(
                                                i18n,
                                                context.tournament,
                                                &identity_slot,
                                            )
                                            .map(|identity| {
                                                view! { <p class="text-sm text-gray-500">{identity}</p> }
                                            })
                                    }}
                                    <p class="font-semibold">{result}</p>
                                    {review
                                        .expected_resolution
                                        .is_some()
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {move || {
                                                        t_string!(
                                                            i18n, tournaments.view.slots.previous_result_value1, value1 = organizer_result_label(i18n, &review.slot)
                                                        )
                                                            .to_string()
                                                    }}
                                                </p>
                                            }
                                        })}
                                </div>
                                <p class="text-sm text-gray-600 dark:text-gray-300">
                                    {move || {
                                        if clearing {
                                            t_string!(
                                                i18n, tournaments.view.slots.the_game_will_need_a_new_result
                                            )
                                                .to_string()
                                        } else {
                                            t_string!(
                                                i18n, tournaments.view.slots.adjudication_progress_warning
                                            )
                                                .to_string()
                                        }
                                    }}
                                </p>
                                <div class="flex gap-2 justify-end">
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        on:click=move |_| cancel.run(())
                                    >
                                        {t!(i18n, common.cancel)}
                                    </button>
                                    <button
                                        type="button"
                                        class=if dangerous {
                                            "ui-button ui-button-danger ui-button-sm"
                                        } else {
                                            "ui-button ui-button-primary ui-button-sm"
                                        }
                                        on:click=confirm
                                    >
                                        {move || {
                                            if clearing {
                                                t_string!(i18n, tournaments.view.slots.clear_result)
                                                    .to_string()
                                            } else {
                                                t_string!(i18n, tournaments.view.slots.record_result)
                                                    .to_string()
                                            }
                                        }}
                                    </button>
                                </div>
                            </div>
                        }
                    })
            }}
        </Modal>
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum DeadlineTarget {
    AllReleased,
    MissingPlayBy,
    Slot(Uuid),
}

fn initial_deadline() -> DateTime<Utc> {
    (Local::now() + Duration::days(7))
        .with_hour(23)
        .and_then(|time| time.with_minute(59))
        .and_then(|time| time.with_second(0))
        .and_then(|time| time.with_nanosecond(0))
        .unwrap_or_else(|| Local::now() + Duration::days(7))
        .to_utc()
}

fn is_released_unstarted(slot: &SlotResponse) -> bool {
    slot.resolution.is_none()
        && slot
            .game
            .as_ref()
            .is_some_and(|game| !game.finished && game.status == GameStatus::NotStarted)
        && slot
            .available_admin_actions
            .contains(&SlotAdminAction::SetDeadline)
}

#[component]
fn DeadlineEditorBody(
    tournament: TournamentState,
    target: DeadlineTarget,
    close_editor: Callback<()>,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let route_context = expect_context::<OrganizerContext>();
    let i18n = use_i18n();
    let scope = RwSignal::new(target);
    let selected_slot = match target {
        DeadlineTarget::Slot(slot_id) => tournament
            .format
            .slot(slot_id)
            .and_then(|slot| slot.try_get_untracked()),
        _ => None,
    };
    let previous_deadline = selected_slot.as_ref().and_then(|slot| slot.deadline_at);
    let selected_matchup = Memo::new(move |_| {
        selected_slot.as_ref().map(|slot| {
            let names = organizer_names(&tournament.common.memberships().get_untracked(), slot);
            let identity = slot_card_label(i18n, tournament, slot).unwrap_or_default();
            t_string!(
                i18n,
                tournaments.view.slots.value1_vs_value2_value3,
                value1 = (names[0].clone()),
                value2 = (names[1].clone()),
                value3 = (if identity.is_empty() {
                    String::new()
                } else {
                    format!(" · {identity}")
                })
            )
            .to_string()
        })
    });
    let deadline = RwSignal::new(previous_deadline.unwrap_or_else(initial_deadline));
    let draft_valid = RwSignal::new(false);
    let input_error = RwSignal::new(false);
    let min = Local::now() + Duration::minutes(5);
    let max = min + Duration::weeks(52);
    let local_time_ready = RwSignal::new(false);
    Effect::new(move |_| local_time_ready.set(true));
    let local_time_label = Memo::new(move |_| {
        if local_time_ready.get() {
            t_string!(
                i18n,
                tournaments
                    .view
                    .slots
                    .play_by_your_local_time_currently_value1,
                value1 = Local::now().format("UTC%:z")
            )
            .to_string()
        } else {
            t_string!(i18n, tournaments.view.slots.play_by_your_local_time).to_string()
        }
    });
    let apply = Callback::new(move |deadline_at: Option<DateTime<Utc>>| {
        if deadline_at.is_some()
            && (!draft_valid.get_untracked()
                || deadline_at.is_some_and(|time| time <= Utc::now())
                || (matches!(target, DeadlineTarget::Slot(_)) && deadline_at == previous_deadline))
        {
            return;
        }
        if current_manage_route(&route_context).is_none() {
            return;
        }
        let Some(lifecycle) = tournament.common.lifecycle().try_get_untracked() else {
            return;
        };
        let slot_ids = untrack(|| fixed_slot_fields(tournament))
            .into_iter()
            .filter_map(|slot| slot.try_get_untracked())
            .filter(|slot| match scope.get_untracked() {
                DeadlineTarget::AllReleased => is_released_unstarted(slot),
                DeadlineTarget::MissingPlayBy => {
                    is_released_unstarted(slot) && slot.deadline_at.is_none()
                }
                DeadlineTarget::Slot(slot_id) => {
                    slot.id == slot_id
                        && slot.resolution.is_none()
                        && slot
                            .available_admin_actions
                            .contains(&SlotAdminAction::SetDeadline)
                }
            })
            .map(|slot| slot.id)
            .collect::<Vec<_>>();
        let tournament_id = lifecycle.tournament_id;
        if slot_ids.is_empty() {
            input_error.set(true);
            return;
        }
        input_error.set(false);
        api.get().schedule_action(ScheduleAction::SetDeadline {
            tournament_id,
            slot_ids,
            deadline_at,
        });
        let _ = close_editor.try_run(());
    });
    view! {
        <div class="px-4 pb-5 mx-auto space-y-4 w-[min(94vw,32rem)]">
            <div>
                <h2 class="text-xl font-bold">
                    {t!(i18n, tournaments.view.slots.set_play_by_time)}
                </h2>
                {move || {
                    selected_matchup
                        .get()
                        .map(|matchup| {
                            view! { <p class="text-sm font-semibold break-words">{matchup}</p> }
                        })
                }}
                {previous_deadline
                    .map(|time| {
                        view! {
                            <p class="text-sm text-gray-600 dark:text-gray-300">
                                {move || {
                                    t_string!(
                                        i18n, tournaments.view.slots.previous_value1, value1 = (format_schedule_time(i18n.get_locale(), time))
                                    )
                                        .to_string()
                                }}
                            </p>
                        }
                    })}
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    {t!(i18n, tournaments.view.slots.deadline_help)}
                </p>
            </div>
            <Show when=move || !matches!(target, DeadlineTarget::Slot(_))>
                <label class="block">
                    <span class="ui-field-label">{t!(i18n, tournaments.view.slots.apply_to)}</span>
                    <select
                        class="w-full ui-field-select"
                        on:change=move |event| {
                            if current_manage_route(&route_context).is_none() {
                                return;
                            }
                            scope
                                .set(
                                    match event_target_value(&event).as_str() {
                                        "missing" => DeadlineTarget::MissingPlayBy,
                                        _ => DeadlineTarget::AllReleased,
                                    },
                                );
                        }
                    >
                        <option value="all">
                            {t!(i18n, tournaments.view.slots.all_available_unstarted_games)}
                        </option>
                        <option value="missing">
                            {t!(
                                i18n, tournaments.view.slots.available_unstarted_games_without_a_play_by_time
                            )}
                        </option>
                    </select>
                </label>
            </Show>
            <DateTimePicker
                input_id=String::from("schedule-deadline").to_string()
                text=move || {
                    current_manage_route(&route_context)
                        .and_then(|_| local_time_label.try_get())
                        .unwrap_or_default()
                }
                min
                max
                draft_valid
                value=deadline.get_untracked().with_timezone(&Local)
                success_callback=Callback::from(move |time: DateTime<Utc>| {
                    if current_manage_route(&route_context).is_some() {
                        deadline.set(time);
                        input_error.set(false);
                    }
                })
            />
            <Show when=move || input_error.get()>
                <p class="ui-field-error">
                    {t!(
                        i18n, tournaments.view.slots.no_available_unstarted_games_are_in_this_scope
                    )}
                </p>
            </Show>
            <div class="flex flex-wrap gap-2 justify-end">
                <Show when=move || scope.get() != DeadlineTarget::MissingPlayBy>
                    <button
                        type="button"
                        class="ui-button ui-button-danger ui-button-sm"
                        on:click=move |_| {
                            let _ = apply.try_run(None);
                        }
                    >
                        {t!(i18n, tournaments.view.slots.clear_play_by_times)}
                    </button>
                </Show>
                <button
                    type="button"
                    class="ui-button ui-button-primary ui-button-md"
                    prop:disabled=move || {
                        !draft_valid.get()
                            || (matches!(target, DeadlineTarget::Slot(_))
                                && Some(deadline.get()) == previous_deadline)
                    }
                    on:click=move |_| {
                        let Some(deadline) = deadline.try_get_untracked() else {
                            return;
                        };
                        let _ = apply.try_run(Some(deadline));
                    }
                >
                    {t!(i18n, tournaments.view.slots.set_play_by)}
                </button>
            </div>
        </div>
    }
}

#[component]
fn DeadlineEditorDialog(
    tournament: TournamentState,
    target: RwSignal<Option<DeadlineTarget>>,
    dialog_el: NodeRef<Dialog>,
) -> impl IntoView {
    let i18n = use_i18n();
    let route_context = expect_context::<OrganizerContext>();
    let close_editor = Callback::new(move |_: ()| {
        if current_manage_route(&route_context).is_none() {
            return;
        }
        if let Some(dialog) = dialog_el.try_get().flatten() {
            dialog.close();
        }
        target.set(None);
    });
    view! {
        <Modal
            dialog_el
            aria_label=move || t_string!(i18n, tournaments.view.slots.set_play_by_time).to_string()
            on_close=close_editor
        >
            <For
                each=move || {
                    current_manage_route(&route_context).and_then(|_| target.try_get().flatten())
                }
                key=|target| *target
                let:target
            >
                <DeadlineEditorBody tournament target close_editor />
            </For>
        </Modal>
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrganizerLifecycle {
    Unstarted,
    Finished,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum OrganizerView {
    All,
    NeedsAttention,
    NoAttempt,
    ProposalPending,
    LastProposalEnded,
    TimeAgreed,
    MissingPlayBy,
    PlayBySoon,
    PlayByPassed,
    Finished,
}

impl OrganizerView {
    const ALL: [Self; 10] = [
        Self::All,
        Self::NeedsAttention,
        Self::NoAttempt,
        Self::ProposalPending,
        Self::LastProposalEnded,
        Self::TimeAgreed,
        Self::MissingPlayBy,
        Self::PlayBySoon,
        Self::PlayByPassed,
        Self::Finished,
    ];
}

fn organizer_view_value(view: OrganizerView) -> &'static str {
    match view {
        OrganizerView::All => "all",
        OrganizerView::NeedsAttention => "needs-attention",
        OrganizerView::NoAttempt => "no-attempt",
        OrganizerView::ProposalPending => "proposal-pending",
        OrganizerView::LastProposalEnded => "last-proposal-ended",
        OrganizerView::TimeAgreed => "time-agreed",
        OrganizerView::MissingPlayBy => "missing-play-by",
        OrganizerView::PlayBySoon => "play-by-soon",
        OrganizerView::PlayByPassed => "play-by-passed",
        OrganizerView::Finished => "finished",
    }
}

fn organizer_view_from_value(value: &str) -> OrganizerView {
    match value {
        "all" => OrganizerView::All,
        "no-attempt" => OrganizerView::NoAttempt,
        "proposal-pending" => OrganizerView::ProposalPending,
        "last-proposal-ended" => OrganizerView::LastProposalEnded,
        "time-agreed" => OrganizerView::TimeAgreed,
        "missing-play-by" => OrganizerView::MissingPlayBy,
        "play-by-soon" => OrganizerView::PlayBySoon,
        "play-by-passed" => OrganizerView::PlayByPassed,
        "finished" => OrganizerView::Finished,
        _ => OrganizerView::NeedsAttention,
    }
}

fn organizer_view_label(view: OrganizerView) -> String {
    let i18n = use_i18n();
    match view {
        OrganizerView::All => t_string!(i18n, common.all).to_string(),
        OrganizerView::NeedsAttention => {
            t_string!(i18n, tournaments.view.slots.needs_attention).to_string()
        }
        OrganizerView::NoAttempt => t_string!(i18n, tournaments.view.slots.no_attempt).to_string(),
        OrganizerView::ProposalPending => {
            t_string!(i18n, tournaments.view.slots.proposal_pending).to_string()
        }
        OrganizerView::LastProposalEnded => {
            t_string!(i18n, tournaments.view.slots.last_proposal_ended).to_string()
        }
        OrganizerView::TimeAgreed => {
            t_string!(i18n, tournaments.view.slots.time_agreed).to_string()
        }
        OrganizerView::MissingPlayBy => t_string!(i18n, tournaments.view.slots.missing).to_string(),
        OrganizerView::PlayBySoon => t_string!(i18n, tournaments.view.slots.due_soon).to_string(),
        OrganizerView::PlayByPassed => t_string!(i18n, tournaments.view.slots.passed).to_string(),
        OrganizerView::Finished => t_string!(i18n, common.finished).to_string(),
    }
}

fn organizer_select_label(view: OrganizerView) -> String {
    let i18n = use_i18n();
    match view {
        OrganizerView::All => t_string!(i18n, common.all).to_string(),
        OrganizerView::NeedsAttention => {
            t_string!(i18n, tournaments.view.slots.scheduling_needs_attention).to_string()
        }
        OrganizerView::NoAttempt => {
            t_string!(i18n, tournaments.view.slots.scheduling_no_attempt).to_string()
        }
        OrganizerView::ProposalPending => {
            t_string!(i18n, tournaments.view.slots.scheduling_proposal_pending).to_string()
        }
        OrganizerView::LastProposalEnded => {
            t_string!(i18n, tournaments.view.slots.scheduling_last_proposal_ended).to_string()
        }
        OrganizerView::TimeAgreed => {
            t_string!(i18n, tournaments.view.slots.scheduling_time_agreed).to_string()
        }
        OrganizerView::MissingPlayBy => {
            t_string!(i18n, tournaments.view.slots.play_by_missing).to_string()
        }
        OrganizerView::PlayBySoon => {
            t_string!(i18n, tournaments.view.slots.play_by_due_soon).to_string()
        }
        OrganizerView::PlayByPassed => {
            t_string!(i18n, tournaments.view.slots.play_by_passed).to_string()
        }
        OrganizerView::Finished => t_string!(i18n, common.finished).to_string(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrganizerTone {
    Urgent,
    Attention,
    Pending,
    Active,
    Finished,
    Neutral,
}

fn organizer_status_class(tone: OrganizerTone) -> &'static str {
    match tone {
        OrganizerTone::Urgent => "text-ladybug-red",
        OrganizerTone::Attention => "text-orange-700 dark:text-orange-300",
        OrganizerTone::Pending => "text-blue-700 dark:text-blue-300",
        OrganizerTone::Active => "text-teal-700 dark:text-teal-300",
        OrganizerTone::Finished => "text-gray-700 dark:text-gray-200",
        OrganizerTone::Neutral => "text-gray-600 dark:text-gray-300",
    }
}

fn organizer_marker_class(tone: OrganizerTone) -> &'static str {
    match tone {
        OrganizerTone::Urgent => "bg-ladybug-red",
        OrganizerTone::Attention => "bg-orange-500",
        OrganizerTone::Pending => "bg-blue-600",
        OrganizerTone::Active => "bg-pillbug-teal",
        OrganizerTone::Finished | OrganizerTone::Neutral => "bg-gray-400 dark:bg-gray-500",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OrganizerRowState {
    lifecycle: OrganizerLifecycle,
    pending: Option<ScheduleResponse>,
    latest_offer: Option<ScheduleResponse>,
    scheduling_view: Option<OrganizerView>,
    proposal_pending: bool,
    no_attempt: bool,
    last_proposal_ended: bool,
    expired_pending: bool,
    missing_play_by: bool,
    play_by_soon: bool,
    play_by_passed: bool,
    attention_rank: u8,
    priority_rank: u8,
    sort_at: DateTime<Utc>,
    label: String,
    supporting: Option<String>,
    tone: OrganizerTone,
}

fn organizer_lifecycle(slot: &SlotResponse) -> Option<OrganizerLifecycle> {
    if slot.resolution.is_some() {
        return Some(OrganizerLifecycle::Finished);
    }
    match slot.game.as_ref().map(|game| &game.status) {
        None | Some(GameStatus::NotStarted) => Some(OrganizerLifecycle::Unstarted),
        Some(GameStatus::InProgress | GameStatus::Finished(_) | GameStatus::Adjudicated) => None,
    }
}

fn organizer_lifecycle_label(lifecycle: OrganizerLifecycle) -> String {
    let i18n = use_i18n();
    match lifecycle {
        OrganizerLifecycle::Unstarted => t_string!(i18n, common.not_started).to_string(),
        OrganizerLifecycle::Finished => t_string!(i18n, common.finished).to_string(),
    }
}

fn organizer_result_label(i18n: I18nContext<Locale, I18nKeys>, slot: &SlotResponse) -> String {
    match slot.resolution {
        Some(Resolution::Result(outcome)) => outcome_label(i18n, outcome),
        Some(Resolution::Withdrawal(GameOutcome::Played(PlayedGameOutcome::WhiteWin))) => {
            t_string!(i18n, tournaments.view.slots.white_won_by_withdrawal).to_string()
        }
        Some(Resolution::Withdrawal(GameOutcome::Played(PlayedGameOutcome::BlackWin))) => {
            t_string!(i18n, tournaments.view.slots.black_won_by_withdrawal).to_string()
        }
        Some(Resolution::Withdrawal(GameOutcome::Played(PlayedGameOutcome::Draw))) => {
            t_string!(i18n, tournaments.view.slots.draw_withdrawal).to_string()
        }
        Some(Resolution::Withdrawal(GameOutcome::Adjudicated(outcome))) => {
            match (outcome.white(), outcome.black()) {
                (AdjudicatedSideResult::ForfeitWin, AdjudicatedSideResult::ForfeitLoss) => {
                    t_string!(i18n, tournaments.view.slots.white_won_by_withdrawal).to_string()
                }
                (AdjudicatedSideResult::ForfeitLoss, AdjudicatedSideResult::ForfeitWin) => {
                    t_string!(i18n, tournaments.view.slots.black_won_by_withdrawal).to_string()
                }
                (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw) => {
                    t_string!(i18n, tournaments.view.slots.draw_withdrawal).to_string()
                }
                (AdjudicatedSideResult::Draw, AdjudicatedSideResult::ForfeitLoss) => {
                    t_string!(i18n, tournaments.view.slots.white_draw_black_withdrew).to_string()
                }
                (AdjudicatedSideResult::ForfeitLoss, AdjudicatedSideResult::Draw) => {
                    t_string!(i18n, tournaments.view.slots.black_draw_white_withdrew).to_string()
                }
                (AdjudicatedSideResult::DoubleForfeit, AdjudicatedSideResult::DoubleForfeit) => {
                    t_string!(i18n, tournaments.view.slots.double_forfeit_withdrawal).to_string()
                }
                _ => unreachable!("validated adjudicated outcome"),
            }
        }
        Some(Resolution::Clinched) => {
            t_string!(i18n, tournaments.ui.not_played_match_already_decided).to_string()
        }
        Some(Resolution::CancelledByWithdrawal) => {
            t_string!(i18n, tournaments.ui.not_played_withdrawal).to_string()
        }
        None => t_string!(i18n, common.finished).to_string(),
    }
}

fn organizer_elapsed_label(time: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let i18n = use_i18n();
    let minutes = (now - time).num_minutes().max(0);
    if minutes < 60 {
        t_string!(
            i18n,
            tournaments.view.slots.minutes_min,
            minutes = (minutes)
        )
        .to_string()
    } else if minutes < 24 * 60 {
        t_string!(
            i18n,
            tournaments.view.slots.value1_hr,
            value1 = (minutes / 60)
        )
        .to_string()
    } else {
        t_string!(
            i18n,
            tournaments.view.slots.value1_days,
            value1 = (minutes / (24 * 60))
        )
        .to_string()
    }
}

fn organizer_row_state(
    i18n: I18nContext<Locale, I18nKeys>,
    slot: &SlotResponse,
    schedule: Option<&SlotOffers>,
    schedules_ready: bool,
    now: DateTime<Utc>,
) -> Option<OrganizerRowState> {
    let lifecycle = organizer_lifecycle(slot)?;
    let scheduling_relevant =
        lifecycle == OrganizerLifecycle::Unstarted && matches!(slot.clock, Clock::Realtime(_));
    let pending = scheduling_relevant
        .then(|| schedule.and_then(|slot| slot.latest_pending.clone()))
        .flatten();
    let proposal_pending = pending
        .as_ref()
        .is_some_and(|offer| offer.has_future_candidate(now));
    let expired_pending = pending.is_some() && !proposal_pending;
    let has_history = schedules_ready
        && scheduling_relevant
        && schedule.is_some_and(|slot| slot.latest.is_some());
    let no_attempt =
        schedules_ready && scheduling_relevant && slot.scheduled_at.is_none() && !has_history;
    let last_proposal_ended = has_history && pending.is_none() && slot.scheduled_at.is_none();
    let unstarted = lifecycle == OrganizerLifecycle::Unstarted;
    let missing_play_by = unstarted && slot.deadline_at.is_none();
    let play_by_passed = unstarted && slot.deadline_at.is_some_and(|deadline| deadline <= now);
    let play_by_soon = unstarted
        && slot
            .deadline_at
            .is_some_and(|deadline| deadline > now && deadline <= now + Duration::days(2));
    let missed_agreement = schedules_ready
        && scheduling_relevant
        && !proposal_pending
        && slot
            .scheduled_at
            .is_some_and(|time| time + SCHEDULED_GAME_GRACE <= now);
    let expired_pending_without_agreement =
        schedules_ready && scheduling_relevant && expired_pending && slot.scheduled_at.is_none();
    let needs_time_due_soon = schedules_ready
        && scheduling_relevant
        && slot.scheduled_at.is_none()
        && !proposal_pending
        && play_by_soon;
    let (needs_attention, attention_rank, attention_at) =
        if schedules_ready && scheduling_relevant && play_by_passed {
            (
                true,
                0,
                slot.deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            )
        } else if missed_agreement {
            (
                true,
                1,
                slot.scheduled_at
                    .map(|time| time + SCHEDULED_GAME_GRACE)
                    .unwrap_or(DateTime::<Utc>::MAX_UTC),
            )
        } else if expired_pending_without_agreement {
            (
                true,
                2,
                pending
                    .as_ref()
                    .and_then(|offer| offer.candidate_times.iter().max().copied())
                    .unwrap_or(DateTime::<Utc>::MAX_UTC),
            )
        } else if needs_time_due_soon {
            (
                true,
                3,
                slot.deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            )
        } else {
            (false, u8::MAX, DateTime::<Utc>::MAX_UTC)
        };
    let scheduling_view = if !schedules_ready || !scheduling_relevant {
        None
    } else if needs_attention {
        Some(OrganizerView::NeedsAttention)
    } else if proposal_pending {
        Some(OrganizerView::ProposalPending)
    } else if slot.scheduled_at.is_some() {
        Some(OrganizerView::TimeAgreed)
    } else if no_attempt {
        Some(OrganizerView::NoAttempt)
    } else if last_proposal_ended {
        Some(OrganizerView::LastProposalEnded)
    } else {
        None
    };

    let scheduled_at = slot.scheduled_at;
    let deadline_at = slot.deadline_at;
    let local_time = |time| format_schedule_time(i18n.get_locale(), time);
    let play_by_support = || {
        deadline_at.map(|time| {
            t_string!(
                i18n,
                tournaments.view.slots.play_by_value1,
                value1 = (local_time(time))
            )
            .to_string()
        })
    };
    let (priority_rank, sort_at, label, supporting, tone) = match lifecycle {
        OrganizerLifecycle::Finished => (
            9,
            slot.resolved_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            organizer_result_label(i18n, slot),
            slot.resolved_at.map(|time| {
                t_string!(
                    i18n,
                    tournaments.view.slots.finished_value1,
                    value1 = (local_time(time))
                )
                .to_string()
            }),
            OrganizerTone::Finished,
        ),
        OrganizerLifecycle::Unstarted if play_by_passed => (
            0,
            deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            t_string!(i18n, tournaments.view.slots.play_by_passed).to_string(),
            play_by_support(),
            OrganizerTone::Urgent,
        ),
        OrganizerLifecycle::Unstarted if missed_agreement => (
            1,
            scheduled_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            t_string!(
                i18n,
                tournaments.view.slots.start_missed_by_value1,
                value1 = (organizer_elapsed_label(
                    scheduled_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
                    now,
                ))
            )
            .to_string(),
            scheduled_at.map(|time| {
                t_string!(
                    i18n,
                    tournaments.view.slots.agreed_value1,
                    value1 = (local_time(time))
                )
                .to_string()
            }),
            OrganizerTone::Attention,
        ),
        OrganizerLifecycle::Unstarted if expired_pending_without_agreement => (
            2,
            attention_at,
            t_string!(i18n, tournaments.view.slots.proposed_times_passed).to_string(),
            pending
                .as_ref()
                .and_then(|offer| {
                    offer
                        .candidate_times
                        .iter()
                        .max()
                        .map(|time| (offer, *time))
                })
                .map(|(offer, time)| {
                    t_string!(
                        i18n,
                        notifications.push.schedule_propose_body,
                        proposer = (offer.proposer_username.clone()),
                        when = (local_time(time))
                    )
                    .to_string()
                }),
            OrganizerTone::Attention,
        ),
        OrganizerLifecycle::Unstarted if needs_time_due_soon => (
            3,
            deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            if no_attempt {
                t_string!(i18n, tournaments.view.slots.no_scheduling_attempt).to_string()
            } else {
                t_string!(i18n, tournaments.view.slots.last_proposal_ended).to_string()
            },
            play_by_support(),
            OrganizerTone::Urgent,
        ),
        OrganizerLifecycle::Unstarted if proposal_pending => (
            5,
            pending
                .as_ref()
                .and_then(|offer| {
                    offer
                        .candidate_times
                        .iter()
                        .filter(|candidate| **candidate > now)
                        .min()
                        .copied()
                })
                .unwrap_or(DateTime::<Utc>::MAX_UTC),
            t_string!(i18n, tournaments.view.slots.proposal_pending).to_string(),
            pending.as_ref().map(|offer| {
                t_string!(
                    i18n,
                    tournaments.view.slots.value1_proposed_waiting_for_value2,
                    value1 = (offer.proposer_username.clone()),
                    value2 = (offer.opponent_username.clone())
                )
                .to_string()
            }),
            OrganizerTone::Pending,
        ),
        OrganizerLifecycle::Unstarted if scheduled_at.is_some() => {
            let scheduled_at = scheduled_at.unwrap_or(DateTime::<Utc>::MAX_UTC);
            let starting = scheduled_at <= now + Duration::minutes(15)
                && scheduled_at + SCHEDULED_GAME_GRACE > now;
            (
                if starting { 4 } else { 6 },
                scheduled_at,
                if starting {
                    t_string!(i18n, tournaments.view.slots.starting_now).to_string()
                } else {
                    t_string!(i18n, tournaments.view.slots.time_agreed).to_string()
                },
                Some(local_time(scheduled_at)),
                OrganizerTone::Active,
            )
        }
        OrganizerLifecycle::Unstarted if no_attempt => (
            7,
            deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            t_string!(i18n, tournaments.view.slots.no_scheduling_attempt).to_string(),
            play_by_support(),
            OrganizerTone::Neutral,
        ),
        OrganizerLifecycle::Unstarted if last_proposal_ended => (
            8,
            deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            t_string!(i18n, tournaments.view.slots.last_proposal_ended).to_string(),
            play_by_support(),
            OrganizerTone::Neutral,
        ),
        OrganizerLifecycle::Unstarted => (
            8,
            deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC),
            t_string!(i18n, common.not_started).to_string(),
            play_by_support(),
            OrganizerTone::Neutral,
        ),
    };

    Some(OrganizerRowState {
        lifecycle,
        pending,
        latest_offer: schedule.and_then(|offers| offers.latest.clone()),
        scheduling_view,
        proposal_pending,
        no_attempt,
        last_proposal_ended,
        expired_pending,
        missing_play_by,
        play_by_soon,
        play_by_passed,
        attention_rank,
        priority_rank,
        sort_at,
        label,
        supporting,
        tone,
    })
}

fn organizer_view_matches(view: OrganizerView, state: &OrganizerRowState) -> bool {
    match view {
        OrganizerView::All => state.lifecycle == OrganizerLifecycle::Unstarted,
        OrganizerView::NeedsAttention
        | OrganizerView::NoAttempt
        | OrganizerView::ProposalPending
        | OrganizerView::LastProposalEnded
        | OrganizerView::TimeAgreed => state.scheduling_view == Some(view),
        OrganizerView::MissingPlayBy => state.missing_play_by,
        OrganizerView::PlayBySoon => state.play_by_soon,
        OrganizerView::PlayByPassed => state.play_by_passed,
        OrganizerView::Finished => state.lifecycle == OrganizerLifecycle::Finished,
    }
}

fn organizer_slot_fields(tournament: TournamentState) -> Vec<ArcField<SlotResponse>> {
    let fields = match tournament.format {
        TournamentFormatState::Arena(_) => Vec::new(),
        TournamentFormatState::RoundRobin(_) | TournamentFormatState::Elimination(_) => {
            fixed_slot_fields(tournament)
        }
        TournamentFormatState::Swiss(format) => format
            .rounds()
            .get()
            .last()
            .into_iter()
            .flat_map(|round| &round.encounters)
            .flat_map(|encounter| encounter.slot_ids.iter().copied())
            .filter_map(|slot_id| tournament.format.slot(slot_id))
            .collect(),
    };
    fields
        .into_iter()
        .filter(|slot| {
            slot.with(|slot| {
                slot.resolution.is_some()
                    || slot
                        .game
                        .as_ref()
                        .is_none_or(|game| !game.finished && game.status == GameStatus::NotStarted)
            })
        })
        .collect()
}

fn organizer_match_identity(
    i18n: I18nContext<Locale, I18nKeys>,
    tournament: TournamentState,
    slot: &SlotResponse,
) -> Option<String> {
    match (slot.key, tournament.format) {
        (SlotKey::RoundRobin { .. }, TournamentFormatState::RoundRobin(format)) => {
            let repeats = format.configuration().get().rules.repeats as usize;
            (repeats > 1)
                .then(|| slot_card_label(i18n, tournament, slot))
                .flatten()
        }
        (SlotKey::Swiss { slot: native_slot }, TournamentFormatState::Swiss(format)) => {
            let kind = FormatConfig::Swiss(format.configuration().get()).kind();
            format.rounds().get().iter().find_map(|round| {
                round.encounters.iter().find_map(|encounter| {
                    encounter.slot_ids.contains(&slot.id).then(|| {
                        let board = t_string!(
                            i18n,
                            tournaments.view.slots.board_value1,
                            value1 = (encounter.pairing_index + 1)
                        )
                        .to_string();
                        if kind == Format::DoubleSwiss {
                            let scheduled = if native_slot.leg == SwissLeg::Second {
                                2
                            } else {
                                1
                            };
                            match tournament_game_number(tournament.format, slot, scheduled) {
                                Some(number) => t_string!(
                                    i18n,
                                    tournaments.view.slots.base_game_number_of,
                                    base = (board),
                                    number = (number)
                                )
                                .to_string(),
                                None => {
                                    format!(
                                        "{board} · {}",
                                        unfinished_tournament_game_label(
                                            use_i18n().get_locale(),
                                            slot
                                        )
                                    )
                                }
                            }
                        } else {
                            board
                        }
                    })
                })
            })
        }
        (SlotKey::Elimination { .. }, TournamentFormatState::Elimination(_)) => {
            slot_card_label(i18n, tournament, slot)
        }
        _ => None,
    }
}

fn same_matchup(left: &SlotResponse, right: &SlotResponse) -> bool {
    match (left.key, right.key) {
        (SlotKey::RoundRobin { .. }, SlotKey::RoundRobin { .. }) => left
            .participants
            .iter()
            .all(|player| right.participants.contains(player)),
        (SlotKey::Swiss { slot: left }, SlotKey::Swiss { slot: right }) => {
            left.round_index == right.round_index && left.pairing_index == right.pairing_index
        }
        (SlotKey::Elimination { node: left, .. }, SlotKey::Elimination { node: right, .. }) => {
            left == right
        }
        _ => false,
    }
}

fn organizer_names(memberships: &TournamentMemberships, slot: &SlotResponse) -> [String; 2] {
    let i18n = use_i18n();
    [
        memberships
            .players
            .get(&slot.white())
            .map(|user| user.username.clone())
            .unwrap_or_else(|| t_string!(i18n, common.white).to_string()),
        memberships
            .players
            .get(&slot.black())
            .map(|user| user.username.clone())
            .unwrap_or_else(|| t_string!(i18n, common.black).to_string()),
    ]
}

fn organizer_matchup_label(white: &str, black: &str) -> String {
    let i18n = use_i18n();
    t_string!(
        i18n,
        common.versus_players,
        white = (white),
        black = (black)
    )
    .to_string()
}

fn organizer_view_slots(
    slots: &[ArcField<SlotResponse>],
    states: &HashMap<Uuid, OrganizerRowState>,
    tournament: TournamentState,
    memberships: &TournamentMemberships,
    view: OrganizerView,
    query: &str,
) -> Vec<Uuid> {
    let query = query.trim().to_lowercase();
    let slot_order = slot_ordering(tournament);
    let mut visible = slots
        .iter()
        .filter_map(|slot| {
            let slot = slot.try_get()?;
            let state = states.get(&slot.id)?;
            if !organizer_view_matches(view, state) {
                return None;
            }
            let names = organizer_names(memberships, &slot);
            if !query.is_empty()
                && !names[0].to_lowercase().contains(&query)
                && !names[1].to_lowercase().contains(&query)
            {
                return None;
            }
            Some((
                slot.clone(),
                state.clone(),
                names[0].to_lowercase(),
                names[1].to_lowercase(),
                slot_order(&slot),
            ))
        })
        .collect::<Vec<_>>();
    visible.sort_by(|left, right| {
        let order = match view {
            OrganizerView::All => left.4.cmp(&right.4),
            OrganizerView::NeedsAttention => left
                .1
                .attention_rank
                .cmp(&right.1.attention_rank)
                .then_with(|| left.1.sort_at.cmp(&right.1.sort_at)),
            OrganizerView::MissingPlayBy => left
                .1
                .priority_rank
                .cmp(&right.1.priority_rank)
                .then_with(|| left.1.sort_at.cmp(&right.1.sort_at)),
            OrganizerView::Finished => right.0.resolved_at.cmp(&left.0.resolved_at),
            OrganizerView::NoAttempt | OrganizerView::LastProposalEnded => left
                .0
                .deadline_at
                .unwrap_or(DateTime::<Utc>::MAX_UTC)
                .cmp(&right.0.deadline_at.unwrap_or(DateTime::<Utc>::MAX_UTC)),
            OrganizerView::ProposalPending => left.1.sort_at.cmp(&right.1.sort_at),
            OrganizerView::TimeAgreed => left.0.scheduled_at.cmp(&right.0.scheduled_at),
            OrganizerView::PlayBySoon | OrganizerView::PlayByPassed => {
                left.0.deadline_at.cmp(&right.0.deadline_at)
            }
        };
        order
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.cmp(&right.4))
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    visible
        .into_iter()
        .map(|(slot, _, _, _, _)| slot.id)
        .collect()
}

fn organizer_progress_label(tournament: TournamentState) -> String {
    let i18n = use_i18n();
    match tournament.format {
        TournamentFormatState::Swiss(format) => format.rounds().get().last().map_or_else(
            || t_string!(i18n, tournaments.view.slots.no_current_round).to_string(),
            |round| {
                let slots = round
                    .encounters
                    .iter()
                    .flat_map(|encounter| &encounter.slot_ids)
                    .filter_map(|slot_id| tournament.format.slot(*slot_id))
                    .collect::<Vec<_>>();
                let finished = slots
                    .iter()
                    .filter(|slot| slot.try_get().is_some_and(|slot| slot.resolution.is_some()))
                    .count();
                t_string!(
                    i18n,
                    tournaments
                        .view
                        .slots
                        .round_value1_finished_of_value2_matches_resolved,
                    value1 = (round.round_index + 1),
                    finished = (finished),
                    value2 = (slots.len())
                )
                .to_string()
            },
        ),
        _ => {
            let slots = fixed_slot_fields(tournament);
            let finished = slots
                .iter()
                .filter(|slot| slot.try_get().is_some_and(|slot| slot.resolution.is_some()))
                .count();
            t_string!(
                i18n,
                tournaments.view.slots.finished_of_value1_matches_resolved,
                finished = (finished),
                value1 = (slots.len())
            )
            .to_string()
        }
    }
}

fn organizer_row_states(
    i18n: I18nContext<Locale, I18nKeys>,
    slots: &[ArcField<SlotResponse>],
    slot_offers: &HashMap<Uuid, SlotOffers>,
    schedules_ready: bool,
    now: DateTime<Utc>,
) -> HashMap<Uuid, OrganizerRowState> {
    slots
        .iter()
        .filter_map(|slot| {
            let slot = slot.try_get()?;
            organizer_row_state(i18n, &slot, slot_offers.get(&slot.id), schedules_ready, now)
                .map(|state| (slot.id, state))
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManageRoute {
    Index,
    Match(Uuid),
    Invalid,
}

fn manage_route(pathname: &str, tournament_id: &TournamentId) -> Option<ManageRoute> {
    let root = format!("/tournament/{}/manage", tournament_id.0);
    let suffix = pathname.strip_prefix(&root)?;
    if suffix.is_empty() || suffix == "/" {
        return Some(ManageRoute::Index);
    }
    let Some(slot_id) = suffix.strip_prefix("/matches/") else {
        return Some(ManageRoute::Invalid);
    };
    let slot_id = slot_id.trim_end_matches('/');
    Some(
        (!slot_id.is_empty() && !slot_id.contains('/'))
            .then(|| slot_id.parse().ok())
            .flatten()
            .map_or(ManageRoute::Invalid, ManageRoute::Match),
    )
}

#[derive(Clone, Copy)]
struct OrganizerContext {
    tournament_id: StoredValue<TournamentId>,
    pathname: Memo<String>,
    manage_root: StoredValue<String>,
    tournament: TournamentState,
    schedules_ready: Signal<bool>,
    slots: Signal<Vec<ArcField<SlotResponse>>>,
    row_states: Memo<HashMap<Uuid, OrganizerRowState>>,
    now: Signal<DateTime<Utc>>,
    view: RwSignal<OrganizerView>,
    query: RwSignal<String>,
    view_counts: Memo<Vec<(OrganizerView, usize)>>,
    visible_slot_ids: Memo<Vec<Uuid>>,
    deadline_target: RwSignal<Option<DeadlineTarget>>,
    deadline_dialog: NodeRef<Dialog>,
    adjudication_review: RwSignal<Option<AdjudicationReview>>,
    adjudication_dialog: NodeRef<Dialog>,
    advance_after_result: RwSignal<Option<(Uuid, String)>>,
}

fn current_manage_route(context: &OrganizerContext) -> Option<ManageRoute> {
    let pathname = context.pathname.try_get()?;
    let tournament_id = context.tournament_id.try_get_value()?;
    let route = manage_route(&pathname, &tournament_id)?;
    (context
        .tournament
        .common
        .lifecycle()
        .try_get()
        .map(|lifecycle| lifecycle.tournament_id == tournament_id)
        == Some(true))
    .then_some(route)
}

#[component]
fn OrganizerViewLink(view: OrganizerView) -> impl IntoView {
    let context = expect_context::<OrganizerContext>();
    let href = context.manage_root.get_value();
    let count = move || {
        current_manage_route(&context)?;
        context.view_counts.with(|counts| {
            counts
                .iter()
                .find_map(|(candidate, count)| (*candidate == view).then_some(*count))
        })
    };
    view! {
        <A
            href=href
            scroll=false
            attr:aria-current=move || {
                if current_manage_route(&context).is_some() && context.view.get() == view {
                    "page"
                } else {
                    "false"
                }
            }
            attr:class=move || {
                if current_manage_route(&context).is_some() && context.view.get() == view {
                    "grid grid-cols-[minmax(0,1fr)_auto] gap-2 items-center px-3 py-2.5 border-l-2 border-pillbug-teal bg-pillbug-teal/10 font-semibold no-link-style"
                } else {
                    "grid grid-cols-[minmax(0,1fr)_auto] gap-2 items-center px-3 py-2.5 border-l-2 border-transparent text-gray-800 hover:bg-blue-light/70 dark:text-gray-100 dark:hover:bg-pillbug-teal/15 no-link-style"
                }
            }
            on:click=move |_| {
                if current_manage_route(&context).is_some() {
                    context.view.set(view);
                }
            }
        >
            <span class="text-sm truncate">{move || organizer_view_label(view)}</span>
            <span class="px-1.5 text-xs text-center text-gray-600 rounded dark:text-gray-300 min-w-6 bg-black/5 dark:bg-white/10">
                {count}
            </span>
        </A>
    }
}

#[component]
fn OrganizerViews() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <aside class="hidden overflow-hidden flex-col min-w-0 min-h-0 border-r border-black/10 bg-odd-light/90 tournament-three:flex dark:border-white/10 dark:bg-surface-muted">
            <header class="py-2.5 px-3 border-b shrink-0 border-black/10 dark:border-white/10">
                <h3 class="font-bold">{t!(i18n, tournaments.view.slots.views)}</h3>
            </header>
            <nav class="overflow-y-auto flex-1 min-h-0">
                <div class="py-1 border-b border-black/10 dark:border-white/10">
                    <OrganizerViewLink view=OrganizerView::All />
                </div>
                <div class="py-1">
                    <p class="px-3 pt-1 pb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                        {t!(i18n, common.scheduling)}
                    </p>
                    <OrganizerViewLink view=OrganizerView::NeedsAttention />
                    <OrganizerViewLink view=OrganizerView::NoAttempt />
                    <OrganizerViewLink view=OrganizerView::ProposalPending />
                    <OrganizerViewLink view=OrganizerView::LastProposalEnded />
                    <OrganizerViewLink view=OrganizerView::TimeAgreed />
                </div>
                <div class="py-1 border-t border-black/10 dark:border-white/10">
                    <p class="px-3 pt-1 pb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                        {t!(i18n, common.play_by)}
                    </p>
                    <OrganizerViewLink view=OrganizerView::MissingPlayBy />
                    <OrganizerViewLink view=OrganizerView::PlayBySoon />
                    <OrganizerViewLink view=OrganizerView::PlayByPassed />
                </div>
                <div class="py-1 border-t border-black/10 dark:border-white/10">
                    <OrganizerViewLink view=OrganizerView::Finished />
                </div>
            </nav>
        </aside>
    }
}

#[component]
fn OrganizerMatchLink(slot_id: Uuid) -> impl IntoView {
    let context = expect_context::<OrganizerContext>();
    let i18n = use_i18n();
    let slot = Memo::new(move |_| {
        context
            .tournament
            .format
            .slot(slot_id)
            .and_then(|slot| slot.try_get())
    });
    let state = Memo::new(move |_| {
        context
            .row_states
            .with(|states| states.get(&slot_id).cloned())
    });
    let identity = Memo::new(move |_| {
        slot.with(|slot| {
            slot.as_ref()
                .and_then(|slot| organizer_match_identity(i18n, context.tournament, slot))
        })
    });
    let names = Memo::new(move |_| {
        slot.with(|slot| {
            slot.as_ref().map(|slot| {
                context
                    .tournament
                    .common
                    .memberships()
                    .with(|memberships| organizer_names(memberships, slot))
            })
        })
    });
    let selected = Memo::new(move |_| {
        let route = current_manage_route(&context)?;
        Some(
            route == ManageRoute::Match(slot_id)
                || (route == ManageRoute::Index
                    && context
                        .visible_slot_ids
                        .with(|slot_ids| slot_ids.first().copied() == Some(slot_id))),
        )
    });
    view! {
        {move || {
            let selected = selected.get()?;
            let state = state.get()?;
            let identity = identity.get();
            let names = names.get()?;
            let href = format!(
                "{value1}/matches/{slot_id}",
                value1 = (context.manage_root.get_value()),
                slot_id = (slot_id),
            )
                .to_string();
            let tone = organizer_status_class(state.tone);
            let marker = organizer_marker_class(state.tone);
            Some(
                view! {
                    <A
                        href=href
                        scroll=false
                        attr:aria-current=selected.then_some("page")
                        attr:class=if selected {
                            "grid grid-cols-[0.65rem_minmax(0,1fr)_auto] gap-2 items-start py-2 px-3 border-l-2 border-pillbug-teal bg-pillbug-teal/10 no-link-style"
                        } else {
                            "grid grid-cols-[0.65rem_minmax(0,1fr)_auto] gap-2 items-start py-2 px-3 border-l-2 border-transparent text-gray-800 odd:bg-odd-light even:bg-even-light hover:bg-blue-light/70 dark:text-gray-100 dark:odd:bg-surface-row-odd dark:even:bg-surface-row-even dark:hover:bg-pillbug-teal/15 no-link-style"
                        }
                    >
                        <span
                            class=format!("mt-1.5 block size-2 rounded-full {marker}")
                            aria-hidden="true"
                        ></span>
                        <span class="min-w-0">
                            <span class="flex gap-2 justify-between items-baseline min-w-0">
                                {identity
                                    .map(|identity| {
                                        view! {
                                            <span class="min-w-0 text-xs font-semibold text-gray-500 dark:text-gray-400 truncate">
                                                {identity}
                                            </span>
                                        }
                                    })}
                                <span class=format!(
                                    "text-xs font-semibold truncate shrink-0 {tone}",
                                )>{state.label}</span>
                            </span>
                            <span class="block text-sm font-bold truncate">
                                {{
                                    let names = names.clone();
                                    move || organizer_matchup_label(&names[0], &names[1])
                                }}
                            </span>
                            {state
                                .supporting
                                .map(|supporting| {
                                    view! {
                                        <span class="block text-xs text-gray-600 dark:text-gray-300 truncate">
                                            {supporting}
                                        </span>
                                    }
                                })}
                        </span>
                        <span class="pt-3 text-xl leading-none text-gray-500" aria-hidden="true">
                            "›"
                        </span>
                    </A>
                },
            )
        }}
    }
}

#[component]
fn OrganizerMatchList() -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<OrganizerContext>();
    let filter_navigate = use_navigate();
    let search_navigate = use_navigate();
    view! {
        <section class=move || {
            let at_root = current_manage_route(&context) == Some(ManageRoute::Index);
            format!(
                "flex-1 min-w-0 min-h-0 overflow-hidden flex-col border-black/10 bg-even-light/95 tournament-two:!flex tournament-two:border-r dark:border-white/10 dark:bg-surface-panel {}",
                if at_root { "flex" } else { "hidden" },
            )
        }>
            <header class="p-3 space-y-2 border-b shrink-0 border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-raised">
                <div class="flex gap-2 justify-between items-center">
                    <h3 class="font-bold">{t!(i18n, tournaments.calendar.title)}</h3>
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        on:click=move |_| {
                            if current_manage_route(&context).is_none() {
                                return;
                            }
                            context.deadline_target.set(Some(DeadlineTarget::AllReleased));
                            if let Some(dialog) = context.deadline_dialog.try_get().flatten() {
                                let _ = dialog.show_modal();
                            }
                        }
                    >
                        {t!(i18n, tournaments.view.slots.set_play_by)}
                    </button>
                </div>
                <select
                    class="w-full ui-field-select tournament-three:hidden"
                    prop:value=move || {
                        current_manage_route(&context)
                            .map(|_| organizer_view_value(context.view.get()))
                            .unwrap_or_default()
                    }
                    on:change=move |event| {
                        let Some(route) = current_manage_route(&context) else {
                            return;
                        };
                        if route != ManageRoute::Index {
                            filter_navigate(
                                &context.manage_root.get_value(),
                                NavigateOptions::default(),
                            );
                        }
                        context.view.set(organizer_view_from_value(&event_target_value(&event)));
                    }
                >
                    <For each=move || OrganizerView::ALL key=|view| *view let:view>
                        <option value=organizer_view_value(
                            view,
                        )>
                            {move || {
                                if current_manage_route(&context).is_none() {
                                    return String::new();
                                }
                                let count = context
                                    .view_counts
                                    .with(|counts| {
                                        counts
                                            .iter()
                                            .find_map(|(candidate, count)| {
                                                (*candidate == view).then_some(*count)
                                            })
                                            .unwrap_or_default()
                                    });
                                format!("{} {count}", organizer_select_label(view))
                            }}
                        </option>
                    </For>
                </select>
                <label class="block">
                    <span class="sr-only">{t!(i18n, common.search_players)}</span>
                    <input
                        type="search"
                        class="w-full ui-field-input"
                        placeholder=move || t_string!(i18n, common.search_players).to_string()
                        prop:value=move || {
                            current_manage_route(&context)
                                .map(|_| context.query.get())
                                .unwrap_or_default()
                        }
                        on:input=move |event| {
                            let Some(route) = current_manage_route(&context) else {
                                return;
                            };
                            if route != ManageRoute::Index {
                                search_navigate(
                                    &context.manage_root.get_value(),
                                    NavigateOptions::default(),
                                );
                            }
                            context.query.set(event_target_value(&event));
                        }
                    />
                </label>
                <Show when=move || {
                    current_manage_route(&context).is_some() && !context.schedules_ready.get()
                }>
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        {t!(i18n, tournaments.view.slots.loading_scheduling_activity)}
                    </p>
                </Show>
            </header>
            <div class="overflow-y-auto flex-1 min-h-0 divide-y divide-black/10 dark:divide-white/10">
                <Show
                    when=move || {
                        current_manage_route(&context).is_some()
                            && !context.visible_slot_ids.with(Vec::is_empty)
                    }
                    fallback=move || {
                        view! {
                            <p class="p-5 text-sm text-center text-gray-600 dark:text-gray-300">
                                {t!(i18n, tournaments.view.slots.no_matches_in_this_view)}
                            </p>
                        }
                    }
                >
                    <For
                        each=move || {
                            current_manage_route(&context)
                                .map(|_| context.visible_slot_ids.get())
                                .unwrap_or_default()
                        }
                        key=|slot_id| *slot_id
                        let:slot_id
                    >
                        <OrganizerMatchLink slot_id />
                    </For>
                </Show>
            </div>
        </section>
    }
}

#[component]
fn OrganizerEmptyDetail(#[prop(into)] message: TextProp, show_mobile_back: bool) -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<OrganizerContext>();
    let manage_root = context.manage_root.get_value();
    view! {
        <div class="flex flex-col justify-center items-center p-6 min-h-full text-center">
            <Show when=move || show_mobile_back>
                <A
                    href=manage_root.clone()
                    scroll=false
                    attr:class="self-start mb-4 text-sm font-semibold no-link-style text-pillbug-teal tournament-two:hidden"
                >
                    {t!(i18n, tournaments.view.slots.all_matches)}
                </A>
            </Show>
            <p class="text-sm text-gray-600 dark:text-gray-300">{move || message.get()}</p>
        </div>
    }
}

#[component]
fn OrganizerMatchDetail(slot_id: Uuid, mobile_back: bool) -> impl IntoView {
    let context = expect_context::<OrganizerContext>();
    let i18n = use_i18n();
    let slot = Memo::new(move |_| {
        context
            .tournament
            .format
            .slot(slot_id)
            .and_then(|slot| slot.try_get())
    });
    let state = Memo::new(move |_| {
        context
            .row_states
            .with(|states| states.get(&slot_id).cloned())
    });
    let names = Memo::new(move |_| {
        slot.with(|slot| {
            slot.as_ref()
                .map(|slot| organizer_names(&context.tournament.common.memberships().get(), slot))
        })
    });
    view! {
        {move || {
            current_manage_route(&context)?;
            let manage_root = context.manage_root.get_value();
            let slot = slot.get()?;
            let state = state.get()?;
            let identity = organizer_match_identity(i18n, context.tournament, &slot);
            let names = names.get()?;
            let game_href = slot.game.as_ref().map(|game| format!("/game/{}", game.game_id.0));
            let can_set_deadline = slot
                .available_admin_actions
                .contains(&SlotAdminAction::SetDeadline);
            let has_result_action = slot
                .available_admin_actions
                .iter()
                .any(|action| {
                    matches!(
                        action,
                        SlotAdminAction::RecordResult
                        | SlotAdminAction::ReplaceResult
                        | SlotAdminAction::ClearResult
                    )
                });
            let admin = has_result_action
                .then(|| {
                    SlotAdjudicationControls(SlotAdjudicationControlsProps {
                        slot: slot.clone(),
                        white_name: names[0].clone(),
                        black_name: names[1].clone(),
                    })
                });
            let status_class = organizer_status_class(state.tone);
            let pending = state.pending.clone();
            let proposal_pending = state.proposal_pending;
            let expired_pending = state.expired_pending;
            let scheduling_relevant = state.lifecycle == OrganizerLifecycle::Unstarted
                && matches!(slot.clock, Clock::Realtime(_));
            let result = (state.lifecycle == OrganizerLifecycle::Finished)
                .then(|| organizer_result_label(i18n, &slot));
            Some(
                view! {
                    <div class="flex flex-col min-h-full">
                        <header class="py-3 px-3 border-b sm:px-4 shrink-0 border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-raised">
                            <Show when=move || mobile_back>
                                <A
                                    href=manage_root.clone()
                                    scroll=false
                                    attr:class="inline-flex mb-2 text-sm font-semibold no-link-style text-pillbug-teal tournament-two:hidden"
                                >
                                    {t!(i18n, tournaments.view.slots.all_matches)}
                                </A>
                            </Show>
                            {identity
                                .map(|identity| {
                                    view! {
                                        <p class="text-xs font-semibold text-gray-500 dark:text-gray-400">
                                            {identity}
                                        </p>
                                    }
                                })}
                            <h3 class="text-lg font-bold break-words">
                                {{
                                    let names = names.clone();
                                    move || organizer_matchup_label(&names[0], &names[1])
                                }}
                            </h3>
                            <p class=format!(
                                "mt-1 text-sm font-semibold {status_class}",
                            )>{state.label.clone()}</p>
                        </header>
                        <div class="flex-1">
                            <section class="grid gap-3 items-center py-3 px-3 border-b sm:px-4 grid-cols-[minmax(0,1fr)_auto] border-black/10 dark:border-white/10">
                                <div>
                                    <p class="text-xs font-bold text-gray-500 uppercase dark:text-gray-400">
                                        {t!(i18n, common.game)}
                                    </p>
                                    <p class="text-sm font-semibold">
                                        {move || organizer_lifecycle_label(state.lifecycle)}
                                    </p>
                                </div>
                                {game_href
                                    .map(|href| {
                                        view! {
                                            <a
                                                class="ui-button ui-button-secondary ui-button-sm"
                                                href=href
                                            >
                                                {t!(i18n, common.open_game)}
                                            </a>
                                        }
                                    })}
                            </section>
                            <section class="py-3 px-3 space-y-2 border-b sm:px-4 border-black/10 dark:border-white/10">
                                <p class="text-xs font-bold text-gray-500 uppercase dark:text-gray-400">
                                    {t!(i18n, common.scheduling)}
                                </p>
                                {if state.lifecycle != OrganizerLifecycle::Unstarted {
                                    view! {
                                        <p class="text-sm">
                                            {t!(i18n, tournaments.view.slots.scheduling_closed)}
                                        </p>
                                    }
                                        .into_any()
                                } else if !scheduling_relevant {
                                    view! {
                                        <p class="text-sm">
                                            {t!(
                                                i18n, tournaments.view.slots.scheduling_is_not_needed_for_this_game
                                            )}
                                        </p>
                                    }
                                        .into_any()
                                } else if let Some(time) = slot.scheduled_at {
                                    view! {
                                        <p class="text-sm">
                                            <span class="font-semibold">
                                                {t!(i18n, tournaments.view.slots.agreed)}
                                            </span>
                                            {format_schedule_time(i18n.get_locale(), time)}
                                        </p>
                                    }
                                        .into_any()
                                } else if proposal_pending || expired_pending {
                                    ().into_any()
                                } else if state.no_attempt {
                                    view! {
                                        <p class="text-sm">
                                            {t!(i18n, tournaments.view.slots.no_proposal_yet)}
                                        </p>
                                    }
                                        .into_any()
                                } else if state.last_proposal_ended {
                                    view! {
                                        <p class="text-sm">
                                            {t!(
                                                i18n, tournaments.view.slots.last_proposal_ended_without_an_agreement
                                            )}
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <p class="text-sm">
                                            {t!(
                                                i18n, tournaments.view.slots.no_current_scheduling_information
                                            )}
                                        </p>
                                    }
                                        .into_any()
                                }}
                                {state
                                    .latest_offer
                                    .as_ref()
                                    .filter(|_| pending.is_none())
                                    .map(|offer| {
                                        let attribution = if offer.status
                                            == ScheduleOfferStatus::Accepted
                                        {
                                            t_string!(
                                                i18n, tournaments.view.slots.proposed_by_value1_accepted_by_value2, value1 = (offer.proposer_username.clone()), value2 = (offer.opponent_username.clone())
                                            )
                                                .to_string()
                                        } else {
                                            t_string!(
                                                i18n, tournaments.view.slots.last_proposal_by_value1, value1 = (offer.proposer_username.clone())
                                            )
                                                .to_string()
                                        };
                                        view! {
                                            <p class="text-sm text-gray-600 dark:text-gray-300">
                                                {attribution}
                                            </p>
                                        }
                                    })}
                                {pending
                                    .map(|offer| {
                                        let heading = if proposal_pending {
                                            t_string!(
                                                i18n, tournaments.view.slots.value1_proposed_waiting_for_value2, value1 = (offer.proposer_username.clone()), value2 = (offer.opponent_username.clone())
                                            )
                                                .to_string()
                                        } else {
                                            t_string!(
                                                i18n, tournaments.view.slots.value1_proposed_all_times_have_passed, value1 = (offer.proposer_username.clone())
                                            )
                                                .to_string()
                                        };
                                        let heading_class = if proposal_pending {
                                            "text-sm font-semibold text-blue-700 dark:text-blue-300"
                                        } else {
                                            "text-sm font-semibold text-orange-700 dark:text-orange-300"
                                        };
                                        view! {
                                            <div class="space-y-1">
                                                <p class=heading_class>{heading}</p>
                                                <ul class="space-y-0.5 text-xs text-gray-600 dark:text-gray-300">
                                                    {offer
                                                        .candidate_times
                                                        .into_iter()
                                                        .map(|time| {
                                                            view! {
                                                                <li class=move || {
                                                                    if expired_pending || time <= context.now.get() {
                                                                        "text-ladybug-red"
                                                                    } else {
                                                                        ""
                                                                    }
                                                                }>{format_schedule_time(i18n.get_locale(), time)}</li>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </ul>
                                            </div>
                                        }
                                    })}
                            </section>
                            <section class="grid gap-3 items-center py-3 px-3 border-b sm:px-4 grid-cols-[minmax(0,1fr)_auto] border-black/10 dark:border-white/10">
                                <div>
                                    <p class="text-xs font-bold text-gray-500 uppercase dark:text-gray-400">
                                        {t!(i18n, common.play_by)}
                                    </p>
                                    <p class="text-sm font-semibold">
                                        {slot
                                            .deadline_at
                                            .map(|time| format_schedule_time(i18n.get_locale(), time))
                                            .unwrap_or_else(|| {
                                                t_string!(i18n, tournaments.view.slots.not_set).to_string()
                                            })}
                                    </p>
                                </div>
                                <Show when=move || can_set_deadline>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        on:click=move |_| {
                                            if current_manage_route(&context).is_none() {
                                                return;
                                            }
                                            context
                                                .deadline_target
                                                .set(Some(DeadlineTarget::Slot(slot_id)));
                                            if let Some(dialog) = context
                                                .deadline_dialog
                                                .try_get()
                                                .flatten()
                                            {
                                                let _ = dialog.show_modal();
                                            }
                                        }
                                    >
                                        {move || {
                                            if slot.deadline_at.is_some() {
                                                t_string!(i18n, common.change).to_string()
                                            } else {
                                                t_string!(i18n, common.set).to_string()
                                            }
                                        }}
                                    </button>
                                </Show>
                            </section>
                            {result
                                .map(|result| {
                                    view! {
                                        <section class="py-3 px-3 border-b sm:px-4 border-black/10 dark:border-white/10">
                                            <p class="text-xs font-bold text-gray-500 uppercase dark:text-gray-400">
                                                {t!(i18n, common.result)}
                                            </p>
                                            <p class="text-sm font-semibold">{result}</p>
                                        </section>
                                    }
                                })}
                            {admin
                                .map(|controls| {
                                    view! {
                                        <section class="py-3 px-3 border-b sm:px-4 border-black/10 dark:border-white/10">
                                            <h4 class="mb-2 text-sm font-bold">
                                                {t!(i18n, tournaments.view.slots.organizer_actions)}
                                            </h4>
                                            {controls}
                                        </section>
                                    }
                                })}
                        </div>
                        <footer class="grid gap-2 p-3 mt-auto border-t sm:p-4 border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-raised">
                            <div class="flex flex-wrap gap-2 justify-between items-center">
                                <span class="text-sm font-semibold truncate">
                                    {names[0].clone()}
                                </span>
                                <MessageButton username=names[0].clone() />
                            </div>
                            <div class="flex flex-wrap gap-2 justify-between items-center">
                                <span class="text-sm font-semibold truncate">
                                    {names[1].clone()}
                                </span>
                                <MessageButton username=names[1].clone() />
                            </div>
                        </footer>
                    </div>
                },
            )
        }}
    }
}

#[component]
pub fn TournamentOrganizerIndex() -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<OrganizerContext>();
    view! {
        {move || {
            (current_manage_route(&context) == Some(ManageRoute::Index))
                .then(|| {
                    context
                        .visible_slot_ids
                        .with(|slot_ids| slot_ids.first().copied())
                        .map(|slot_id| {
                            view! { <OrganizerMatchDetail slot_id mobile_back=false /> }.into_any()
                        })
                        .unwrap_or_else(|| {
                            view! {
                                <OrganizerEmptyDetail
                                    message=move || {
                                        t_string!(
                                            i18n, tournaments.view.slots.no_matches_in_this_view
                                        )
                                            .to_string()
                                    }
                                    show_mobile_back=false
                                />
                            }
                                .into_any()
                        })
                })
        }}
    }
}

#[component]
pub fn TournamentOrganizerMatch() -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<OrganizerContext>();
    let selected_match = Memo::new(move |_| {
        let ManageRoute::Match(slot_id) = current_manage_route(&context)? else {
            return None;
        };
        Some(
            context
                .row_states
                .with(|states| states.contains_key(&slot_id))
                .then_some(slot_id),
        )
    });
    view! {
        {move || {
            Some(
                selected_match
                    .get()?
                    .map(|slot_id| {
                        view! { <OrganizerMatchDetail slot_id mobile_back=true /> }.into_any()
                    })
                    .unwrap_or_else(|| {
                        view! {
                            <OrganizerEmptyDetail
                                message=move || {
                                    t_string!(i18n, tournaments.view.slots.match_unavailable)
                                        .to_string()
                                }
                                show_mobile_back=true
                            />
                        }
                            .into_any()
                    }),
            )
        }}
    }
}

#[component]
pub fn TournamentOrganizerLayout(
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    schedules_ready: Signal<bool>,
    organizer: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let expected_tournament_id = tournament.tournament_id();
    let pathname = location.pathname;
    let manage_root = format!("/tournament/{}/manage", expected_tournament_id.0);
    let slots = Signal::derive(move || organizer_slot_fields(tournament));
    let schedule_tournament_id = expected_tournament_id.clone();
    let slot_offers = Memo::new(move |_| {
        schedules.with(|schedules| offers_by_slot(schedules, &schedule_tournament_id))
    });
    let now = use_ticking_now();
    let row_states = Memo::new(move |_| {
        slots.with(|slots| {
            slot_offers.with(|slot_offers| {
                organizer_row_states(i18n, slots, slot_offers, schedules_ready.get(), now.get())
            })
        })
    });
    let organizer_view = RwSignal::new(OrganizerView::NeedsAttention);
    let query = RwSignal::new(String::new());
    let view_counts = Memo::new(move |_| {
        row_states.with(|states| {
            OrganizerView::ALL
                .into_iter()
                .map(|candidate| {
                    let count = states
                        .values()
                        .filter(|state| organizer_view_matches(candidate, state))
                        .count();
                    (candidate, count)
                })
                .collect()
        })
    });
    let visible_slot_ids = Memo::new(move |_| {
        let memberships = tournament.common.memberships().get();
        slots.with(|slots| {
            row_states.with(|states| {
                organizer_view_slots(
                    slots,
                    states,
                    tournament,
                    &memberships,
                    organizer_view.get(),
                    &query.get(),
                )
            })
        })
    });
    let deadline_target = RwSignal::new(None::<DeadlineTarget>);
    let deadline_dialog = NodeRef::<Dialog>::new();

    let context = OrganizerContext {
        tournament_id: StoredValue::new(expected_tournament_id),
        pathname,
        manage_root: StoredValue::new(manage_root),
        tournament,
        schedules_ready,
        slots,
        row_states,
        now,
        view: organizer_view,
        query,
        view_counts,
        visible_slot_ids,
        deadline_target,
        deadline_dialog,
        adjudication_review: RwSignal::new(None),
        adjudication_dialog: NodeRef::new(),
        advance_after_result: RwSignal::new(None),
    };
    provide_context(context);

    let navigate_next = use_navigate();
    Effect::new(move |_| {
        let Some((slot_id, pathname)) = context.advance_after_result.get() else {
            return;
        };
        if current_manage_route(&context).is_none() || context.pathname.get() != pathname {
            context.advance_after_result.set(None);
            return;
        }
        let Some(completed) = tournament
            .format
            .slot(slot_id)
            .and_then(|slot| slot.try_get())
        else {
            return;
        };
        if completed.resolution.is_none() {
            return;
        }
        let ordering = slot_ordering(tournament);
        let next = fixed_slot_fields(tournament)
            .into_iter()
            .filter_map(|slot| slot.try_get())
            .filter(|slot| {
                slot.resolution.is_none()
                    && same_matchup(&completed, slot)
                    && ordering(slot) > ordering(&completed)
            })
            .min_by_key(|slot| ordering(slot));
        if let Some(next) = next {
            let Some(game) = &next.game else {
                return;
            };
            if game.status == GameStatus::NotStarted && !game.finished {
                context.view.set(OrganizerView::All);
                navigate_next(
                    &format!(
                        "{value1}/matches/{value2}",
                        value1 = (context.manage_root.get_value()),
                        value2 = (next.id)
                    )
                    .to_string(),
                    NavigateOptions {
                        scroll: false,
                        ..Default::default()
                    },
                );
            }
        }
        context.advance_after_result.set(None);
    });

    let initialized = RwSignal::new(false);
    Effect::new(move |_| {
        if initialized.get_untracked()
            || !context.schedules_ready.get()
            || context.slots.with(Vec::is_empty)
        {
            return;
        }
        let initial_view = context.view_counts.with(|counts| {
            [
                OrganizerView::NeedsAttention,
                OrganizerView::All,
                OrganizerView::Finished,
            ]
            .into_iter()
            .find(|candidate| {
                counts
                    .iter()
                    .any(|(view, count)| view == candidate && *count > 0)
            })
        });
        if let Some(initial_view) = initial_view {
            context.view.set(initial_view);
        }
        initialized.set(true);
    });

    Effect::new(move |_| {
        if !context.schedules_ready.get() {
            return;
        }
        let Some(ManageRoute::Match(slot_id)) = current_manage_route(&context) else {
            return;
        };
        let Some(state) = context
            .row_states
            .with(|states| states.get(&slot_id).cloned())
        else {
            return;
        };
        if context
            .visible_slot_ids
            .with(|slot_ids| slot_ids.contains(&slot_id))
        {
            return;
        }
        if !context.query.get_untracked().is_empty() {
            context.query.set(String::new());
        }
        if !organizer_view_matches(context.view.get_untracked(), &state) {
            let target_view = if state.lifecycle == OrganizerLifecycle::Finished {
                OrganizerView::Finished
            } else if let Some(scheduling_view) = state.scheduling_view {
                scheduling_view
            } else if state.missing_play_by {
                OrganizerView::MissingPlayBy
            } else if state.play_by_soon {
                OrganizerView::PlayBySoon
            } else if state.play_by_passed {
                OrganizerView::PlayByPassed
            } else {
                return;
            };
            context.view.set(target_view);
        }
    });

    let dialog_tournament = context.tournament;
    let dialog_target = context.deadline_target;
    view! {
        <div class="space-y-3">
            <header class="flex flex-wrap gap-3 justify-between items-center px-0.5">
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    {move || {
                        current_manage_route(&context)
                            .map(|_| { organizer_progress_label(context.tournament) })
                    }}
                </p>
                <TournamentCloseout tournament organizer />
            </header>
            <div class="flex overflow-hidden flex-col rounded-lg border shadow-sm h-[calc(100dvh-19rem)] min-h-[28rem] border-black/10 bg-even-light/95 tournament-two:grid tournament-two:h-[calc(100dvh-16.5rem)] tournament-two:min-h-[32rem] tournament-two:grid-cols-[minmax(18rem,22rem)_minmax(0,1fr)] tournament-two:grid-rows-[minmax(0,1fr)] tournament-three:grid-cols-[13rem_minmax(20rem,1fr)_minmax(18rem,24rem)] dark:border-white/10 dark:bg-surface-panel">
                <OrganizerViews />
                <OrganizerMatchList />
                <main class=move || {
                    let at_root = current_manage_route(&context) == Some(ManageRoute::Index);
                    format!(
                        "flex-1 min-w-0 min-h-0 overflow-y-auto flex-col bg-even-light/95 tournament-two:!flex dark:bg-surface-panel {}",
                        if at_root { "hidden" } else { "flex" },
                    )
                }>
                    <Outlet />
                </main>
            </div>
        </div>
        <AdjudicationDialog />
        <DeadlineEditorDialog
            tournament=dialog_tournament
            target=dialog_target
            dialog_el=deadline_dialog
        />
    }
}
