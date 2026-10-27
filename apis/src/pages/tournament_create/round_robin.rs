use super::common::{
    clock_signal,
    release_policy_from_token,
    release_policy_token,
    validate_configuration,
    ClockFields,
    CommonDraft,
    CreationDetails,
    CreationShell,
    FieldDraft,
    FieldSizeFields,
};
use crate::{
    components::{
        atoms::input_slider::InputSliderWithCallback,
        molecules::{panel::Panel, tiebreaker_picker::TiebreakerPicker},
    },
    i18n::*,
    providers::ChallengeParams,
};
use leptos::prelude::*;
use reactive_stores::Store;
use shared_types::tournament::{
    round_robin::{
        creation_tiebreakers as round_robin_creation_tiebreakers,
        Config as RoundRobinConfig,
        Criterion as RoundRobinCriterion,
        DirectEncounterOptions as RoundRobinDirectEncounterOptions,
        PrimaryScore as RoundRobinPrimaryScore,
        SonnebornBergerOptions as RoundRobinSonnebornBergerOptions,
    },
    BotAdmission,
    Clock,
    Config,
    DirectEncounterForfeitPolicy,
    Format,
    FormatConfig,
    ReleasePolicy,
    RepeatedEncounterPolicy,
    MAX_ROUND_ROBIN_SEATS,
};
use std::num::NonZeroU32;

#[derive(Clone, Copy)]
struct RoundRobinDraft {
    field: FieldDraft,
    repeats: RwSignal<i32>,
    primary_score: RwSignal<RoundRobinPrimaryScore>,
    release_policy: RwSignal<ReleasePolicy>,
    tiebreakers: RwSignal<Vec<RoundRobinCriterion>>,
}

impl RoundRobinDraft {
    fn new() -> Self {
        Self {
            field: FieldDraft::new(),
            repeats: RwSignal::new(2),
            primary_score: RwSignal::new(RoundRobinPrimaryScore::GamePoints),
            release_policy: RwSignal::new(ReleasePolicy::FullyUnlocked),
            tiebreakers: RwSignal::new(round_robin_default_criteria()),
        }
    }

    fn retain_tiebreakers(self) {
        let available = round_robin_creation_tiebreakers(
            NonZeroU32::new(self.repeats.get_untracked() as u32).expect("positive repeats"),
            self.primary_score.get_untracked(),
        );
        self.tiebreakers
            .update(|criteria| criteria.retain(|criterion| available.contains(criterion)));
    }

    fn set_repeats(self, repeats: i32) {
        batch(|| {
            self.repeats.set(repeats);
            if repeats % 2 != 0 {
                self.primary_score.set(RoundRobinPrimaryScore::GamePoints);
            }
            self.retain_tiebreakers();
        });
    }

    fn set_primary_score(self, primary: RoundRobinPrimaryScore) {
        batch(|| {
            self.primary_score.set(primary);
            self.retain_tiebreakers();
        });
    }

    fn configuration(self, clock: Clock) -> Result<Config, String> {
        let repeats = u32::try_from(self.repeats.get())
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or_else(|| String::from("Invalid repeat count"))?;
        validate_configuration(round_robin_creation_configuration(
            repeats,
            self.primary_score.get(),
            self.release_policy.get(),
            &self.tiebreakers.get(),
            clock,
        ))
    }
}

fn round_robin_direct_encounter() -> RoundRobinCriterion {
    RoundRobinCriterion::DirectEncounter(RoundRobinDirectEncounterOptions {
        forfeits: DirectEncounterForfeitPolicy::IncludeAsScored,
        repeated_encounters: RepeatedEncounterPolicy::Average,
    })
}

fn round_robin_default_criteria() -> Vec<RoundRobinCriterion> {
    vec![
        round_robin_direct_encounter(),
        RoundRobinCriterion::SonnebornBerger(RoundRobinSonnebornBergerOptions { cut_lowest: 0 }),
        RoundRobinCriterion::GamesWonWithBlack,
    ]
}

fn round_robin_standings(tiebreakers: &[RoundRobinCriterion]) -> Vec<RoundRobinCriterion> {
    std::iter::once(RoundRobinCriterion::PrimaryScore)
        .chain(tiebreakers.iter().copied())
        .collect()
}

fn round_robin_creation_configuration(
    repeats: NonZeroU32,
    primary_score: RoundRobinPrimaryScore,
    release_policy: ReleasePolicy,
    tiebreakers: &[RoundRobinCriterion],
    clock: Clock,
) -> Config {
    let mut round_robin = RoundRobinConfig::standard(repeats, clock);
    round_robin.release_policy = release_policy;
    round_robin.rules.primary_score = primary_score;
    round_robin.rules.standings.criteria = round_robin_standings(tiebreakers);
    Config {
        bot_admission: BotAdmission::HumansOnly,
        format: FormatConfig::RoundRobin(round_robin),
    }
}

fn round_robin_dimensions(player_count: i32, repeats: i32) -> (i32, i32) {
    let player_count = player_count.max(2);
    let repeats = repeats.max(1);
    let rounds_per_pass = if player_count % 2 == 0 {
        player_count - 1
    } else {
        player_count
    };
    let games_per_pass = player_count * (player_count - 1) / 2;
    (rounds_per_pass * repeats, games_per_pass * repeats)
}

fn round_robin_size_summary(min_players: i32, max_players: i32, repeats: i32) -> String {
    let i18n = use_i18n();
    let (minimum_rounds, minimum_games) = round_robin_dimensions(min_players, repeats);
    let (maximum_rounds, maximum_games) = round_robin_dimensions(max_players, repeats);
    let rounds = if minimum_rounds != maximum_rounds {
        t_string!(
            i18n,
            tournaments.creation.rounds_range,
            minimum = minimum_rounds,
            maximum = maximum_rounds
        )
        .to_string()
    } else if minimum_rounds == 1 {
        t_string!(i18n, tournaments.creation.one_round).to_string()
    } else {
        t_string!(
            i18n,
            tournaments.creation.rounds_count,
            count = minimum_rounds
        )
        .to_string()
    };
    let games = if minimum_games != maximum_games {
        t_string!(
            i18n,
            tournaments.creation.games_range,
            minimum = minimum_games,
            maximum = maximum_games
        )
        .to_string()
    } else if minimum_games == 1 {
        t_string!(i18n, tournaments.creation.one_game).to_string()
    } else {
        t_string!(
            i18n,
            tournaments.creation.games_count,
            count = minimum_games
        )
        .to_string()
    };
    if min_players == max_players {
        t_string!(
            i18n,
            tournaments.creation.round_robin_size,
            players = min_players,
            rounds = rounds,
            games = games
        )
        .to_string()
    } else {
        t_string!(
            i18n,
            tournaments.creation.round_robin_size_range,
            minimum = min_players,
            maximum = max_players,
            rounds = rounds,
            games = games
        )
        .to_string()
    }
}

#[component]
pub(super) fn RoundRobinCreationForm() -> impl IntoView {
    let i18n = use_i18n();
    let common = CommonDraft::new(true, true);
    let draft = RoundRobinDraft::new();
    let params = Store::new(ChallengeParams::default());
    let clock = clock_signal(params);
    let round_robin_has_matches = Signal::derive(move || draft.repeats.get() % 2 == 0);
    let round_robin_available_tiebreakers = Signal::derive(move || {
        round_robin_creation_tiebreakers(
            NonZeroU32::new(draft.repeats.get() as u32).expect("positive repeats"),
            draft.primary_score.get(),
        )
    });
    view! {
        <CreationShell
            draft=common
            format=Signal::derive(|| Format::RoundRobin)
            clock
            configuration=Callback::new(move |()| draft.configuration(clock.get_untracked()))
            field=Signal::derive(move || (
                Some(draft.field.maximum.get()),
                draft.field.minimum.get(),
            ))
            valid=Signal::derive(move || draft.field.valid(MAX_ROUND_ROBIN_SEATS))
            participant_summary=Signal::derive(move || draft.field.summary())
        >
            <CreationDetails draft=common>
                <FieldSizeFields field=draft.field maximum=MAX_ROUND_ROBIN_SEATS />
            </CreationDetails>
            <ClockFields params />
            <Panel
                title=move || t_string!(i18n, tournaments.creation.rules).to_string()
                body_class="space-y-5"
            >
                <div class="space-y-3 ui-setting-group">
                    <div class="flex gap-3 justify-between items-center">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.games_per_opponent)}
                        </span>
                        <span class="font-bold text-gray-900 dark:text-gray-100">
                            {draft.repeats}
                        </span>
                    </div>
                    <InputSliderWithCallback
                        signal=Signal::derive(move || draft.repeats.get())
                        callback=Callback::new(move |repeats| draft.set_repeats(repeats))
                        name="Round Robin repeats"
                        min=1
                        max=6
                        step=1
                    />
                    <p class="text-sm text-gray-700 dark:text-gray-300">
                        {move || round_robin_size_summary(
                            draft.field.minimum.get(),
                            draft.field.maximum.get(),
                            draft.repeats.get(),
                        )}
                    </p>
                </div>
                <div class="space-y-4">
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">{t!(i18n, tournaments.creation.scoring)}</span>
                        <select
                            class="ui-field-select"
                            prop:value=move || match draft.primary_score.get() {
                                RoundRobinPrimaryScore::GamePoints => "game_points",
                                RoundRobinPrimaryScore::MatchPoints => "match_points",
                            }
                            on:change=move |event| {
                                let primary = if event_target_value(&event) == "match_points"
                                    && round_robin_has_matches.get()
                                {
                                    RoundRobinPrimaryScore::MatchPoints
                                } else {
                                    RoundRobinPrimaryScore::GamePoints
                                };
                                draft.set_primary_score(primary);
                            }
                        >
                            <option value="game_points">
                                {t!(i18n, tournaments.tiebreakers.names.game_points)}
                            </option>
                            <option
                                value="match_points"
                                disabled=move || !round_robin_has_matches.get()
                            >
                                {t!(i18n, tournaments.tiebreakers.names.match_points)}
                            </option>
                        </select>
                        <p class="ui-field-helper">
                            {move || {
                                if round_robin_has_matches.get() {
                                    t_string!(
                                        i18n, tournaments.creation.round_robin_match_help, games = draft.repeats.get()
                                    )
                                        .to_string()
                                } else {
                                    t_string!(i18n, tournaments.creation.match_points_requirement)
                                        .to_string()
                                }
                            }}
                        </p>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.game_availability)}
                        </span>
                        <select
                            class="ui-field-select"
                            prop:value=move || { release_policy_token(draft.release_policy.get()) }
                            on:change=move |event| {
                                if let Some(policy) = release_policy_from_token(
                                    &event_target_value(&event),
                                ) {
                                    draft.release_policy.set(policy);
                                }
                            }
                        >
                            <option value="fully_unlocked">
                                {t!(i18n, tournaments.creation.all_games_together)}
                            </option>
                            <option value="sequential_per_matchup">
                                {t!(i18n, tournaments.creation.one_game_per_opponent)}
                            </option>
                        </select>
                    </label>
                    <TiebreakerPicker
                        selected=draft.tiebreakers
                        available=round_robin_available_tiebreakers
                    />
                    <p class="ui-field-helper">
                        {t!(i18n, tournaments.creation.primary_score_tiebreakers)}
                    </p>
                </div>
            </Panel>
        </CreationShell>
    }
}
