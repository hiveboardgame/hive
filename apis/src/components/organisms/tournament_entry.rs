use super::tournament_explanations::stage_plan_label;
use crate::{
    common::{compact_realtime_clock, tournament_path_matches},
    components::molecules::{modal::Modal, time_row::format_compact_duration},
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
    // TODO: i18n once copy is approved.
    if seconds >= 86400 && seconds.is_multiple_of(86400) {
        let days = seconds / 86400;
        format!("{days} {}", if days == 1 { "day" } else { "days" })
    } else {
        format_compact_duration(seconds)
    }
}

fn clock_text(clock: Clock) -> String {
    // TODO: i18n once copy is approved.
    match clock {
        Clock::Realtime(clock) => compact_realtime_clock(clock),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => {
            format!("{} per move", duration_text(seconds_per_move.get()))
        }
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => {
            format!("{} per player", duration_text(seconds_each.get()))
        }
    }
}

fn swiss_round_summary(
    rounds: RoundConfiguration,
    players: usize,
    capacity: Option<i32>,
) -> Vec<String> {
    // TODO: i18n once copy is approved.
    if let Some(rounds) = rounds.resolved_rounds() {
        return vec![format!("Rounds: {rounds}.")];
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
        (Some(low), Some(high)) if low == high => format!("Rounds: {high}."),
        (Some(low), Some(high)) => {
            format!("Rounds: {low}–{high}, depending on how many players join.")
        }
        _ => String::from("Rounds: based on the number of players at the start."),
    }];
    if lower != upper {
        if players >= minimum {
            if let Some(estimate) = resolve(players) {
                lines.push(format!(
                    "With the current {players} players: {estimate} rounds."
                ));
            }
        }
        lines.push(String::from(
            "Final round count is set when the tournament starts.",
        ));
    }
    lines
}

fn series_summary(plan: &SeriesPlan) -> Vec<String> {
    plan.phases
        .iter()
        .enumerate()
        .map(|(index, phase)| {
            // TODO: i18n once copy is approved.
            let prefix = if index == 0 {
                "Each match"
            } else {
                "If still tied"
            };
            let sets = match phase.set_limit {
                SetLimit::AtMost(sets) if sets.get() == 1 => String::from("1 set"),
                SetLimit::AtMost(sets) => format!("up to {sets} sets"),
                SetLimit::UntilDecisive => String::from("sets until the tie is broken"),
            };
            let ending = match phase.clinch {
                ClinchPolicy::PlayAll => "All games in a set are played.",
                ClinchPolicy::EarlyClinch => {
                    "Remaining games are skipped once a player has won the set."
                }
            };
            let games = if phase.games_per_set == 1 {
                String::from("1 game")
            } else {
                format!("{} games", phase.games_per_set)
            };
            format!(
                "{prefix}: {games} per set, {sets}. {ending} Time control: {}.",
                clock_text(phase.clock)
            )
        })
        .collect()
}

#[component]
pub fn TournamentEntrySummary(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
    #[prop(optional)] joining: bool,
) -> impl IntoView {
    let identity = expect_context::<AuthContext>().identity;
    let lines = move || {
        let lifecycle = common.lifecycle().get();
        let players = common.memberships().read().players.len();
        // TODO: i18n once copy is approved.
        match configuration(format) {
            FormatConfig::Swiss(config) => {
                let mut lines = swiss_round_summary(config.rounds, players, lifecycle.seats);
                let double = matches!(config.system, SwissSystem::DoubleSwiss(_));
                lines.push(if double { "2 games per opponent, one with each color." } else { "1 game per opponent." }.into());
                lines.push(format!("Time control: {}.", clock_text(config.clock)));
                lines.push(String::from("New pairings each round, based on standings."));
                if double {
                    lines.push(match config.double_swiss_release_policy {
                        ReleasePolicy::FullyUnlocked => "Both games are available together.",
                        ReleasePolicy::SequentialPerMatchup => "The second game starts after the first finishes.",
                    }.into());
                }
                lines
            }
            FormatConfig::RoundRobin(config) => {
                let games = config.rules.repeats as usize;
                let joined = identity.get().and_then(AuthIdentity::user_id)
                    .is_some_and(|id| common.memberships().read().players.contains_key(&id));
                let field = players + usize::from(joining && !joined);
                let opponents = field.saturating_sub(1);
                let total = opponents * games;
                let game_word = if total == 1 { "game" } else { "games" };
                let opponent_word = if opponents == 1 { "opponent" } else { "opponents" };
                let total_text = if joining {
                    format!("You would play {total} {game_word}")
                } else {
                    format!("{total} {game_word} per player")
                };
                let mut lines = vec![if opponents == 0 {
                    format!("{games} {} against each opponent who joins.", if games == 1 { "game" } else { "games" })
                } else if games == 1 || opponents == 1 {
                    format!("{total_text} against {opponents} {opponent_word}.")
                } else {
                    format!("{total_text}: {games} against each of {opponents} opponents.")
                }];
                if lifecycle.status == TournamentStatus::NotStarted {
                    let basis = if joining {
                        format!("Based on {field} players, including you.")
                    } else {
                        format!("Based on the current {field} players.")
                    };
                    let capacity = lifecycle.seats.and_then(|seats| usize::try_from(seats).ok());
                    lines.push(match capacity {
                        Some(seats) if seats > field => {
                            let maximum = seats.saturating_sub(1) * games;
                            format!("{basis} Up to {maximum} {} per player if all {seats} places fill.", if maximum == 1 { "game" } else { "games" })
                        }
                        _ => basis,
                    });
                }
                lines.push(format!("Time control: {}.", clock_text(config.clock)));
                lines.push(match config.release_policy {
                    ReleasePolicy::FullyUnlocked => "All games are available together.",
                    ReleasePolicy::SequentialPerMatchup => "Against each opponent, the next game starts after the previous one finishes.",
                }.into());
                lines
            }
            FormatConfig::Elimination(config) => {
                let mut lines = vec![match config.topology {
                    Topology::Single { bronze: false } => "Single elimination: one match loss eliminates you.",
                    Topology::Single { bronze: true } => "Single elimination, with a third-place match for semifinal losers.",
                    Topology::Double => "Double elimination: two match losses eliminate you.",
                }.into()];
                lines.push(String::from("Win a set to win the match. Tied sets are replayed."));
                lines.extend(series_summary(&config.default_plan));
                for stage in &config.stage_overrides {
                    lines.push(format!("{} — {}", stage_plan_label(stage.stage), series_summary(&stage.plan).join(" ")));
                }
                if config.topology == Topology::Double {
                    lines.push(String::from("A second final is played if the lower-bracket winner wins the first."));
                }
                lines
            }
            FormatConfig::Arena(config) => vec![
                format!("Duration: {}. Time control: {}.", format_compact_duration(config.duration_seconds.get()), clock_text(Clock::Realtime(config.game_clock))),
                String::from("You'll keep getting new opponents while the arena is running."),
                String::from("Once it starts, you can't leave. Pause to stop new pairings; your current game continues."),
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
        // TODO: i18n once copy is approved.
        <Modal dialog_el=dialog aria_label="Join tournament">
            <div class="p-4 space-y-4 max-w-lg">
                <h2 class="text-lg font-bold">
                    {move || format!("Join {}?", common.lifecycle().get().name)}
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
                        "Cancel"
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
                        "Join tournament"
                    </button>
                </div>
            </div>
        </Modal>
    }
}
