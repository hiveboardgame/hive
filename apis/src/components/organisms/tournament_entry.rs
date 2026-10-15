use super::tournament_explanations::stage_plan_label;
use crate::{
    common::{compact_realtime_clock, tournament_path_matches},
    components::molecules::{modal::Modal, time_row::format_compact_duration},
    i18n::*,
    providers::{
        ArenaStateStoreFields,
        AuthContext,
        AuthIdentity,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatState,
    },
};
use leptos::{html::Dialog, prelude::*};
use leptos_router::hooks::use_location;
use reactive_stores::Store;
use shared_types::{
    clock::CorrespondenceClock,
    tournament::{
        elimination::{ClinchPolicy, SeriesPlan, SetLimit, Topology},
        swiss::{RoundConfiguration, System as SwissSystem},
        FormatConfig,
        ReleasePolicy,
        MIN_SWISS_START_SEATS,
    },
    Clock,
    TournamentStatus,
};

fn configuration(format: TournamentFormatState) -> FormatConfig {
    match format {
        TournamentFormatState::Arena(state) => FormatConfig::Arena(state.configuration().get()),
        TournamentFormatState::RoundRobin(state) => {
            FormatConfig::RoundRobin(state.configuration().get())
        }
        TournamentFormatState::Swiss(state) => FormatConfig::Swiss(state.configuration().get()),
        TournamentFormatState::Elimination(state) => {
            FormatConfig::Elimination(state.configuration().get())
        }
    }
}

fn duration_text(seconds: u32) -> String {
    if seconds >= 86400 && seconds.is_multiple_of(86400) {
        let days = seconds / 86400;
        t_string!(use_i18n(), tournaments.ui.day_count, count = days).to_string()
    } else {
        format_compact_duration(seconds)
    }
}

fn clock_text(clock: Clock) -> String {
    let i18n = use_i18n();
    match clock {
        Clock::Realtime(clock) => compact_realtime_clock(clock),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => t_string!(
            i18n,
            game.time_mode.correspondence.duration_move,
            duration = duration_text(seconds_per_move.get())
        )
        .to_string(),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => t_string!(
            i18n,
            tournaments.ui.time_per_player,
            value1 = duration_text(seconds_each.get())
        )
        .to_string(),
    }
}

fn swiss_round_summary(
    rounds: RoundConfiguration,
    players: usize,
    capacity: Option<i32>,
) -> Vec<String> {
    let i18n = use_i18n();
    if let Some(rounds) = rounds.resolved_rounds() {
        return vec![t_string!(i18n, tournaments.ui.rounds, high = rounds).to_string()];
    }
    let minimum = MIN_SWISS_START_SEATS as usize;
    let resolve = |count| {
        rounds
            .resolve(count)
            .and_then(|rounds| rounds.resolved_rounds())
            .map(|rounds| rounds.get())
    };
    let lower = resolve(minimum);
    let upper = capacity
        .and_then(|count| usize::try_from(count).ok())
        .and_then(resolve);
    let mut lines = vec![match (lower, upper) {
        (Some(low), Some(high)) if low == high => {
            t_string!(i18n, tournaments.ui.rounds, high = high).to_string()
        }
        (Some(low), Some(high)) => t_string!(
            i18n,
            tournaments.ui.rounds_depending_on_how_many_players_join,
            low = low,
            high = high
        )
        .to_string(),
        _ => t_string!(
            i18n,
            tournaments
                .ui
                .rounds_based_on_the_number_of_players_at_the_start
        )
        .to_string(),
    }];
    if lower != upper {
        if players >= minimum {
            if let Some(estimate) = resolve(players) {
                lines.push(
                    t_string!(
                        i18n,
                        tournaments.ui.with_the_current_players_rounds,
                        players = players,
                        estimate = estimate
                    )
                    .to_string(),
                );
            }
        }
        lines.push(
            t_string!(
                i18n,
                tournaments
                    .ui
                    .final_round_count_is_set_when_the_tournament_starts
            )
            .to_string(),
        );
    }
    lines
}

fn series_summary(plan: &SeriesPlan) -> Vec<String> {
    let i18n = use_i18n();
    plan.phases
        .iter()
        .enumerate()
        .map(|(index, phase)| {
            let prefix = if index == 0 {
                t_string!(i18n, tournaments.ui.each_match).to_string()
            } else {
                t_string!(i18n, tournaments.ui.if_still_tied).to_string()
            };
            let sets = match phase.set_limit {
                SetLimit::AtMost(sets) if sets.get() == 1 => {
                    t_string!(i18n, tournaments.ui.one_set).to_string()
                }
                SetLimit::AtMost(sets) => {
                    t_string!(i18n, tournaments.ui.up_to_sets, sets = sets).to_string()
                }
                SetLimit::UntilDecisive => {
                    t_string!(i18n, tournaments.ui.sets_until_the_tie_is_broken).to_string()
                }
            };
            let ending = match phase.clinch {
                ClinchPolicy::PlayAll => {
                    t_string!(i18n, tournaments.ui.all_games_in_a_set_are_played).to_string()
                }
                ClinchPolicy::EarlyClinch => t_string!(
                    i18n,
                    tournaments
                        .ui
                        .remaining_games_are_skipped_once_a_player_has_won_the_set
                )
                .to_string(),
            };
            let games =
                t_string!(i18n, tournaments.ui.game_count, count = phase.games_per_set).to_string();
            t_string!(
                i18n,
                tournaments.ui.per_set_time_control,
                prefix = prefix,
                games = games,
                sets = sets,
                ending = ending,
                value1 = clock_text(phase.clock)
            )
            .to_string()
        })
        .collect()
}

#[component]
pub fn TournamentEntrySummary(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
    #[prop(optional)] joining: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let identity = expect_context::<AuthContext>().identity;
    let lines = move || {
        let lifecycle = common.lifecycle().get();
        let players = common.memberships().read().players.len();

        match configuration(format) {
            FormatConfig::Swiss(config) => {
                let mut lines = swiss_round_summary(config.rounds, players, lifecycle.seats);
                let double = matches!(config.system, SwissSystem::DoubleSwiss(_));
                lines.push(if double {
                    t_string!(i18n, tournaments.ui.two_games_per_opponent).to_string()
                } else {
                    t_string!(i18n, tournaments.ui.one_game_per_opponent).to_string()
                });
                lines.push(
                    t_string!(
                        i18n,
                        tournaments.ui.time_control,
                        value1 = clock_text(config.clock)
                    )
                    .to_string(),
                );
                lines.push(
                    t_string!(
                        i18n,
                        tournaments.ui.new_pairings_each_round_based_on_standings
                    )
                    .to_string(),
                );
                if double {
                    lines.push(match config.double_swiss_release_policy {
                        ReleasePolicy::FullyUnlocked => {
                            t_string!(i18n, tournaments.ui.both_games_are_available_together)
                                .to_string()
                        }
                        ReleasePolicy::SequentialPerMatchup => t_string!(
                            i18n,
                            tournaments
                                .ui
                                .the_second_game_starts_after_the_first_finishes
                        )
                        .to_string(),
                    });
                }
                lines
            }
            FormatConfig::RoundRobin(config) => {
                let games = config.rules.repeats as usize;
                let joined = identity
                    .get()
                    .and_then(AuthIdentity::user_id)
                    .is_some_and(|id| common.memberships().read().players.contains_key(&id));
                let field = players + usize::from(joining && !joined);
                let opponents = field.saturating_sub(1);
                let total = opponents * games;
                let total_text = if joining {
                    t_string!(i18n, tournaments.ui.you_would_play, count = total).to_string()
                } else {
                    t_string!(i18n, tournaments.ui.per_player, count = total).to_string()
                };
                let mut lines = vec![if opponents == 0 {
                    t_string!(
                        i18n,
                        tournaments.ui.against_each_opponent_who_joins,
                        count = games
                    )
                    .to_string()
                } else if games == 1 || opponents == 1 {
                    t_string!(
                        i18n,
                        tournaments.ui.against,
                        total_text = total_text,
                        count = opponents
                    )
                    .to_string()
                } else {
                    t_string!(
                        i18n,
                        tournaments.ui.against_each_of_opponents,
                        total_text = total_text,
                        games = games,
                        opponents = opponents
                    )
                    .to_string()
                }];
                if lifecycle.status == TournamentStatus::NotStarted {
                    let basis = if joining {
                        t_string!(
                            i18n,
                            tournaments.ui.based_on_players_including_you,
                            count = field
                        )
                        .to_string()
                    } else {
                        t_string!(
                            i18n,
                            tournaments.ui.based_on_the_current_players,
                            count = field
                        )
                        .to_string()
                    };
                    let capacity = lifecycle
                        .seats
                        .and_then(|seats| usize::try_from(seats).ok());
                    lines.push(match capacity {
                        Some(seats) if seats > field => {
                            let maximum = seats.saturating_sub(1) * games;
                            t_string!(
                                i18n,
                                tournaments.ui.up_to_per_player_if_all_places_fill,
                                basis = basis,
                                count = maximum,
                                seats = seats
                            )
                            .to_string()
                        }
                        _ => basis,
                    });
                }
                lines.push(
                    t_string!(
                        i18n,
                        tournaments.ui.time_control,
                        value1 = clock_text(config.clock)
                    )
                    .to_string(),
                );
                lines.push(match config.release_policy {
                    ReleasePolicy::FullyUnlocked => {
                        t_string!(i18n, tournaments.creation.faq.release_all).to_string()
                    }
                    ReleasePolicy::SequentialPerMatchup => {
                        t_string!(i18n, tournaments.ui.sequential_opponent_games_help).to_string()
                    }
                });
                lines
            }
            FormatConfig::Elimination(config) => {
                let mut lines = vec![match config.topology {
                    Topology::Single { bronze: false } => t_string!(
                        i18n,
                        tournaments
                            .ui
                            .single_elimination_one_match_loss_eliminates_you
                    )
                    .to_string(),
                    Topology::Single { bronze: true } => t_string!(
                        i18n,
                        tournaments
                            .ui
                            .single_elimination_with_a_third_place_match_for_semifinal_losers
                    )
                    .to_string(),
                    Topology::Double => t_string!(
                        i18n,
                        tournaments
                            .ui
                            .double_elimination_two_match_losses_eliminate_you
                    )
                    .to_string(),
                }];
                lines.push(
                    t_string!(
                        i18n,
                        tournaments
                            .ui
                            .win_a_set_to_win_the_match_tied_sets_are_replayed
                    )
                    .to_string(),
                );
                lines.extend(series_summary(&config.default_plan));
                for stage in &config.stage_overrides {
                    lines.push(format!(
                        "{} — {}",
                        stage_plan_label(stage.stage),
                        series_summary(&stage.plan).join(" ")
                    ));
                }
                if config.topology == Topology::Double {
                    lines.push(t_string!(i18n, tournaments.ui.a_second_final_is_played_if_the_lower_bracket_winner_wins_the_first).to_string());
                }
                lines
            }
            FormatConfig::Arena(config) => vec![
                t_string!(
                    i18n,
                    tournaments.ui.duration_time_control,
                    value1 = format_compact_duration(config.duration_seconds.get()),
                    value2 = clock_text(Clock::Realtime(config.game_clock))
                )
                .to_string(),
                t_string!(
                    i18n,
                    tournaments
                        .ui
                        .you_ll_keep_getting_new_opponents_while_the_arena_is_running
                )
                .to_string(),
                t_string!(i18n, tournaments.ui.arena_leave_after_start_help).to_string(),
            ],
        }
    };
    view! {
        <ul class="pl-5 space-y-2 text-sm list-disc">
            {move || lines().into_iter().map(|line| view! { <li>{line}</li> }).collect_view()}
        </ul>
    }
}

#[component]
pub fn TournamentJoinButton(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
    #[prop(into)] can_join: Signal<bool>,
    #[prop(into)] label: Signal<String>,
    on_join: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let dialog = NodeRef::<Dialog>::new();
    let auth = expect_context::<AuthContext>();
    let identity = auth.identity;
    let opened_by = RwSignal::new(None);
    let tournament_id = StoredValue::new(common.lifecycle().get_untracked().tournament_id);
    let pathname = use_location().pathname;
    let route_active = move || {
        tournament_path_matches(&pathname.get(), &tournament_id.get_value())
            && common.lifecycle().get().tournament_id == tournament_id.get_value()
    };
    let can_confirm = move || {
        route_active()
            && can_join.get()
            && opened_by.get().is_some()
            && opened_by.get() == identity.get().and_then(AuthIdentity::user_id)
    };
    Effect::new(move |_| {
        if !route_active()
            || !can_join.get()
            || opened_by.get() != identity.get().and_then(AuthIdentity::user_id)
        {
            if let Some(dialog) = dialog.get() {
                dialog.close();
            }
        }
    });
    view! {
        <button
            type="button"
            class="ui-button ui-button-primary ui-button-sm"
            prop:disabled=move || !can_join.get()
            on:click=move |_| {
                if route_active() && can_join.get_untracked() {
                    opened_by.set(identity.get_untracked().and_then(AuthIdentity::user_id));
                    if let Some(dialog) = dialog.get() {
                        let _ = dialog.show_modal();
                    }
                }
            }
        >
            {move || label.get()}
        </button>

        <Modal
            dialog_el=dialog
            aria_label=move || t_string!(i18n, tournaments.ui.join_tournament).to_string()
        >
            <div class="p-4 space-y-4 max-w-lg">
                <h2 class="text-lg font-bold">
                    {move || {
                        t_string!(i18n, tournaments.ui.join, value1 = common.lifecycle().get().name)
                            .to_string()
                    }}
                </h2>
                <TournamentEntrySummary common format joining=true />
                <div class="flex gap-2 justify-end">
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            if let Some(dialog) = dialog.get() {
                                dialog.close();
                            }
                        }
                    >
                        {t!(i18n, common.cancel)}
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        prop:disabled=move || !can_confirm()
                        on:click=move |_| {
                            if can_confirm() {
                                if let Some(dialog) = dialog.get() {
                                    dialog.close();
                                }
                                on_join.run(());
                            }
                        }
                    >
                        {t!(i18n, tournaments.ui.join_tournament)}
                    </button>
                </div>
            </div>
        </Modal>
    }
}
