use crate::providers::{
    ArenaStateStoreFields,
    EliminationStateStoreFields,
    RoundRobinStateStoreFields,
    SwissStateStoreFields,
    TournamentCommonStoreFields,
    TournamentFormatStore,
    TournamentState,
};
use leptos::prelude::*;
use shared_types::{
    tournament::{AdjudicatedSideResult, GameOutcome, PlayedGameOutcome, Resolution},
    tournament_view::SlotResponse,
    GameSpeed,
    SwissProgress,
};

#[derive(Clone, Copy, Default)]
struct OutcomeStatistics {
    games_recorded: u32,
    white_wins: u32,
    black_wins: u32,
    draws: u32,
    double_forfeits: u32,
    other_adjudicated: u32,
    missing_outcomes: u32,
}

impl OutcomeStatistics {
    fn record_finished_game(&mut self, outcome: Option<GameOutcome>) {
        self.games_recorded += 1;
        match outcome {
            Some(GameOutcome::Played(PlayedGameOutcome::WhiteWin)) => self.white_wins += 1,
            Some(GameOutcome::Played(PlayedGameOutcome::BlackWin)) => self.black_wins += 1,
            Some(GameOutcome::Played(PlayedGameOutcome::Draw)) => self.draws += 1,
            Some(GameOutcome::Adjudicated(outcome)) => match (outcome.white(), outcome.black()) {
                (AdjudicatedSideResult::ForfeitWin, _) => self.white_wins += 1,
                (_, AdjudicatedSideResult::ForfeitWin) => self.black_wins += 1,
                (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw) => self.draws += 1,
                (AdjudicatedSideResult::DoubleForfeit, AdjudicatedSideResult::DoubleForfeit) => {
                    self.double_forfeits += 1
                }
                _ => self.other_adjudicated += 1,
            },
            None => self.missing_outcomes += 1,
        }
    }
}

#[derive(Clone, Copy)]
enum CompletionDetail {
    BerserkRate { berserks: u32, player_seats: u32 },
    ByeRate { byes: u32, player_rounds: u32 },
    WalkoverRate { walkovers: u32, resolved_slots: u32 },
    SwissEndedEarly,
}

struct TournamentCompletionStatistics {
    outcomes: OutcomeStatistics,
    average_rating: Option<u64>,
    details: Vec<CompletionDetail>,
}

fn slot_outcomes<'a>(slots: impl Iterator<Item = &'a SlotResponse>) -> OutcomeStatistics {
    let mut outcomes = OutcomeStatistics::default();
    for slot in slots.filter(|slot| slot.game.as_ref().is_some_and(|game| game.finished)) {
        outcomes.record_finished_game(slot.outcome);
    }
    outcomes
}

fn rating_speed(format: TournamentFormatStore) -> Option<GameSpeed> {
    let clock = match format {
        TournamentFormatStore::Arena(arena) => Some(shared_types::Clock::Realtime(
            arena.configuration().get().game_clock,
        )),
        TournamentFormatStore::RoundRobin(round_robin) => {
            Some(round_robin.configuration().get().clock)
        }
        TournamentFormatStore::Swiss(swiss) => Some(swiss.configuration().get().clock),
        TournamentFormatStore::Elimination(elimination) => elimination
            .configuration()
            .get()
            .default_plan
            .phases
            .first()
            .map(|phase| phase.clock),
    };
    clock.map(GameSpeed::from)
}

fn average_participant_rating(tournament: TournamentState) -> Option<u64> {
    let speed = rating_speed(tournament.format)?;
    let memberships = tournament.common.memberships().get();
    if memberships.players.is_empty() {
        return None;
    }
    let total = memberships
        .players
        .values()
        .map(|player| player.rating_for_speed(&speed))
        .sum::<u64>();
    Some(total / memberships.players.len() as u64)
}

fn completion_statistics(tournament: TournamentState) -> TournamentCompletionStatistics {
    let average_rating = average_participant_rating(tournament);
    let (outcomes, details) = match tournament.format {
        TournamentFormatStore::Arena(arena) => {
            let games = arena.games().get();
            let mut outcomes = OutcomeStatistics::default();
            let mut berserks = 0;
            for game in games.values().filter(|game| game.game.finished) {
                outcomes.record_finished_game(game.outcome);
                berserks += game.game.berserked.into_iter().map(u32::from).sum::<u32>();
            }
            let player_seats = outcomes.games_recorded.saturating_mul(2);
            (
                outcomes,
                vec![CompletionDetail::BerserkRate {
                    berserks,
                    player_seats,
                }],
            )
        }
        TournamentFormatStore::RoundRobin(round_robin) => {
            let slots = round_robin.slots().get();
            (slot_outcomes(slots.values()), Vec::new())
        }
        TournamentFormatStore::Swiss(swiss) => {
            let slots = swiss.slots().get();
            let rounds = swiss.rounds().get();
            let byes = rounds.iter().map(|round| round.byes.len() as u32).sum();
            let player_rounds = rounds
                .iter()
                .map(|round| (round.encounters.len() * 2 + round.byes.len()) as u32)
                .sum();
            let mut details = vec![CompletionDetail::ByeRate {
                byes,
                player_rounds,
            }];
            if swiss.progress().get() == SwissProgress::PairingExhausted {
                details.push(CompletionDetail::SwissEndedEarly);
            }
            (slot_outcomes(slots.values()), details)
        }
        TournamentFormatStore::Elimination(elimination) => {
            let slots = elimination.slots().get();
            let resolved_slots = slots
                .values()
                .filter(|slot| slot.resolution.is_some())
                .count() as u32;
            let walkovers = slots
                .values()
                .filter(|slot| matches!(slot.resolution, Some(Resolution::Withdrawal(_))))
                .count() as u32;
            (
                slot_outcomes(slots.values()),
                vec![CompletionDetail::WalkoverRate {
                    walkovers,
                    resolved_slots,
                }],
            )
        }
    };
    TournamentCompletionStatistics {
        outcomes,
        average_rating,
        details,
    }
}

fn percentage(numerator: u32, denominator: u32) -> String {
    if denominator == 0 {
        return String::from("0%");
    }
    format!(
        "{}%",
        (u64::from(numerator) * 100 + u64::from(denominator) / 2) / u64::from(denominator)
    )
}

#[component]
fn CompletionRow(label: String, value: String) -> impl IntoView {
    view! {
        <div class="contents">
            <dt class="text-gray-600 dark:text-gray-300">{label}</dt>
            <dd class="font-semibold tabular-nums text-right">{value}</dd>
        </div>
    }
}

#[component]
pub fn TournamentCompletionSummary(tournament: TournamentState) -> impl IntoView {
    view! {
        <div class="space-y-4" data-testid="tournament-completion-summary">
            // TODO: i18n once copy is approved.
            <h2 class="text-lg font-bold">"Tournament complete"</h2>
            {move || {
                let statistics = completion_statistics(tournament);
                let outcomes = statistics.outcomes;
                view! {
                    <dl class="grid gap-y-1 gap-x-4 text-sm grid-cols-[minmax(0,1fr)_auto]">
                        // TODO: i18n once copy is approved.
                        <CompletionRow
                            label=String::from("Games recorded")
                            value=outcomes.games_recorded.to_string()
                        />
                        {statistics
                            .average_rating
                            .map(|rating| {
                                view! {
                                    // TODO: i18n once copy is approved.
                                    <CompletionRow
                                        label=String::from("Average participant rating")
                                        value=rating.to_string()
                                    />
                                }
                            })}
                        // TODO: i18n once copy is approved.
                        <CompletionRow
                            label=String::from("White wins")
                            value=percentage(outcomes.white_wins, outcomes.games_recorded)
                        />
                        // TODO: i18n once copy is approved.
                        <CompletionRow
                            label=String::from("Black wins")
                            value=percentage(outcomes.black_wins, outcomes.games_recorded)
                        />
                        // TODO: i18n once copy is approved.
                        <CompletionRow
                            label=String::from("Draws")
                            value=percentage(outcomes.draws, outcomes.games_recorded)
                        />
                        // TODO: i18n once copy is approved.
                        <CompletionRow
                            label=String::from("Double forfeits")
                            value=percentage(outcomes.double_forfeits, outcomes.games_recorded)
                        />
                        {(outcomes.other_adjudicated > 0)
                            .then(|| {
                                view! {
                                    // TODO: i18n once copy is approved.
                                    <CompletionRow
                                        label=String::from("Other adjudicated outcomes")
                                        value=percentage(
                                            outcomes.other_adjudicated,
                                            outcomes.games_recorded,
                                        )
                                    />
                                }
                            })}
                        {(outcomes.missing_outcomes > 0)
                            .then(|| {
                                view! {
                                    // TODO: i18n once copy is approved.
                                    <CompletionRow
                                        label=String::from("Outcome unavailable")
                                        value=percentage(
                                            outcomes.missing_outcomes,
                                            outcomes.games_recorded,
                                        )
                                    />
                                }
                            })}
                        {statistics
                            .details
                            .into_iter()
                            .map(|detail| match detail {
                                CompletionDetail::BerserkRate { berserks, player_seats } => {
                                    view! {
                                        // TODO: i18n once copy is approved.
                                        <CompletionRow
                                            label=String::from("Berserk rate")
                                            value=percentage(berserks, player_seats)
                                        />
                                    }
                                        .into_any()
                                }
                                CompletionDetail::ByeRate { byes, player_rounds } => {
                                    view! {
                                        // TODO: i18n once copy is approved.
                                        <CompletionRow
                                            label=String::from("Bye rate")
                                            value=percentage(byes, player_rounds)
                                        />
                                    }
                                        .into_any()
                                }
                                CompletionDetail::WalkoverRate { walkovers, resolved_slots } => {
                                    view! {
                                        // TODO: i18n once copy is approved.
                                        <CompletionRow
                                            label=String::from("Walkover rate")
                                            value=percentage(walkovers, resolved_slots)
                                        />
                                    }
                                        .into_any()
                                }
                                CompletionDetail::SwissEndedEarly => {
                                    view! {
                                        // TODO: i18n once copy is approved.
                                        <CompletionRow
                                            label=String::from("Ended early")
                                            value=String::from("No legal non-rematch pairing remained")
                                        />
                                    }
                                        .into_any()
                                }
                            })
                            .collect_view()}
                    </dl>
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::tournament::AdjudicatedGameOutcome;

    #[test]
    fn every_recorded_outcome_contributes_to_the_completion_breakdown() {
        let mut statistics = OutcomeStatistics::default();
        for outcome in [
            PlayedGameOutcome::WhiteWin,
            PlayedGameOutcome::BlackWin,
            PlayedGameOutcome::Draw,
        ] {
            statistics.record_finished_game(Some(GameOutcome::Played(outcome)));
        }
        for (white, black) in [
            (
                AdjudicatedSideResult::ForfeitWin,
                AdjudicatedSideResult::ForfeitLoss,
            ),
            (
                AdjudicatedSideResult::ForfeitLoss,
                AdjudicatedSideResult::ForfeitWin,
            ),
            (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw),
            (
                AdjudicatedSideResult::DoubleForfeit,
                AdjudicatedSideResult::DoubleForfeit,
            ),
            (
                AdjudicatedSideResult::Draw,
                AdjudicatedSideResult::ForfeitLoss,
            ),
        ] {
            statistics.record_finished_game(Some(GameOutcome::Adjudicated(
                AdjudicatedGameOutcome::new(white, black).unwrap(),
            )));
        }
        statistics.record_finished_game(None);
        assert_eq!(statistics.games_recorded, 9);
        assert_eq!(
            (
                statistics.white_wins,
                statistics.black_wins,
                statistics.draws
            ),
            (2, 2, 2)
        );
        assert_eq!(
            (
                statistics.double_forfeits,
                statistics.other_adjudicated,
                statistics.missing_outcomes
            ),
            (1, 1, 1)
        );
        assert_eq!(
            statistics.games_recorded,
            statistics.white_wins
                + statistics.black_wins
                + statistics.draws
                + statistics.double_forfeits
                + statistics.other_adjudicated
                + statistics.missing_outcomes
        );
    }
}
