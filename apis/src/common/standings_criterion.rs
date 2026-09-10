use crate::i18n::*;
use leptos_i18n::I18nContext;
use shared_types::tournament::{
    round_robin::Criterion as RoundRobinCriterion,
    swiss::Criterion as SwissCriterion,
    TournamentPairingNumberOrder,
};
use std::hash::Hash;

pub trait StandingsCriterionChoice: Clone + Copy + Eq + Hash + Send + Sync + 'static {
    fn name(self, i18n: I18nContext<Locale, I18nKeys>) -> String;
    fn explanation(self, i18n: I18nContext<Locale, I18nKeys>) -> String;
    fn is_seed(self) -> bool;
}

fn seed_name(order: TournamentPairingNumberOrder) -> String {
    // TODO: i18n once copy is approved.
    match order {
        TournamentPairingNumberOrder::Ascending => "Seed: smaller number first",
        TournamentPairingNumberOrder::Descending => "Seed: larger number first",
    }
    .to_string()
}

fn seed_explanation(order: TournamentPairingNumberOrder) -> String {
    // TODO: i18n once copy is approved.
    match order {
        TournamentPairingNumberOrder::Ascending => "Smaller rating seed numbers place first.",
        TournamentPairingNumberOrder::Descending => "Larger rating seed numbers place first.",
    }
    .to_string()
}

fn cut_scores(lowest: u32, highest: u32) -> String {
    // TODO: i18n once copy is approved.
    match (lowest, highest) {
        (0, 0) => String::new(),
        (1, 0) => String::from(", leaving out the lowest score"),
        (low, 0) => format!(", leaving out the {low} lowest scores"),
        (0, 1) => String::from(", leaving out the highest score"),
        (0, high) => format!(", leaving out the {high} highest scores"),
        (low, high) => format!(", leaving out the {low} lowest and {high} highest scores"),
    }
}

impl StandingsCriterionChoice for RoundRobinCriterion {
    fn is_seed(self) -> bool {
        matches!(self, Self::TournamentPairingNumber(_))
    }

    fn name(self, i18n: I18nContext<Locale, I18nKeys>) -> String {
        match self {
            Self::PrimaryScore | Self::GamePoints => {
                t_string!(i18n, tournaments.tiebreakers.names.game_points)
            }
            Self::MatchPoints => t_string!(i18n, tournaments.tiebreakers.names.match_points),
            // TODO: i18n once copy is approved.
            Self::MatchesWon => return String::from("Matches won"),
            Self::TournamentPairingNumber(order) => return seed_name(order),
            Self::Progressive(_) => t_string!(i18n, tournaments.tiebreakers.names.progressive),
            Self::DirectEncounter(_) => {
                t_string!(i18n, tournaments.tiebreakers.names.direct_encounter)
            }
            Self::SonnebornBerger(_) => {
                t_string!(i18n, tournaments.tiebreakers.names.sonneborn_berger)
            }
            Self::Koya(_) => t_string!(i18n, tournaments.tiebreakers.names.koya),
            Self::NumberOfWins => t_string!(i18n, tournaments.tiebreakers.names.score_wins),
            Self::GamesWon => t_string!(i18n, tournaments.tiebreakers.names.games_won),
            Self::GamesWonWithBlack => {
                t_string!(i18n, tournaments.tiebreakers.names.wins_as_black)
            }
        }
        .to_string()
    }

    fn explanation(self, _i18n: I18nContext<Locale, I18nKeys>) -> String {
        // TODO: i18n once copy is approved.
        match self {
            Self::PrimaryScore => String::from("Points used to rank players."),
            Self::GamePoints => String::from("Points earned from individual game results."),
            Self::MatchPoints => String::from("Points earned from the combined result of all games against each opponent. Awarded only when every game has a result."),
            Self::MatchesWon => String::from("Counts completed matches won, including full-match forfeit wins."),
            Self::TournamentPairingNumber(order) => seed_explanation(order),
            Self::Progressive(_) => {
                String::from("Adds your running score after each round, rewarding earlier points.")
            }
            Self::DirectEncounter(_) => String::from("Compares results between the tied players."),
            Self::SonnebornBerger(options) => format!(
                "Weights your results by your opponents’ scores{}.",
                cut_scores(options.cut_lowest, 0)
            ),
            Self::Koya(options) => {
                if options.threshold_numerator == 1 && options.threshold_denominator.get() == 2 {
                    String::from(
                        "Points against opponents who scored at least half the available points.",
                    )
                } else {
                    format!("Points against opponents who scored at least {}/{} of the available points.", options.threshold_numerator, options.threshold_denominator)
                }
            }
            Self::NumberOfWins => {
                String::from("Counts results awarded as wins, including forfeits.")
            }
            Self::GamesWon => String::from("Counts individual games won."),
            Self::GamesWonWithBlack => String::from("Counts games won as Black."),
        }
    }
}

impl StandingsCriterionChoice for SwissCriterion {
    fn is_seed(self) -> bool {
        matches!(self, Self::TournamentPairingNumber(_))
    }

    fn name(self, i18n: I18nContext<Locale, I18nKeys>) -> String {
        match self {
            Self::PrimaryScore => t_string!(i18n, tournaments.tiebreakers.names.game_points),
            Self::GamePoints => t_string!(i18n, tournaments.tiebreakers.names.game_points),
            Self::MatchPoints => t_string!(i18n, tournaments.tiebreakers.names.match_points),
            Self::Buchholz(options) => match (options.cut_lowest, options.cut_highest) {
                (0, 0) => t_string!(i18n, tournaments.tiebreakers.names.buchholz),
                (1, 0) => t_string!(i18n, tournaments.tiebreakers.names.buchholz_cut_1),
                (2, 0) => t_string!(i18n, tournaments.tiebreakers.names.buchholz_cut_2),
                (1, 1) => t_string!(i18n, tournaments.tiebreakers.names.buchholz_median),
                _ => t_string!(i18n, tournaments.tiebreakers.names.buchholz),
            },
            Self::SonnebornBerger(_) => {
                t_string!(i18n, tournaments.tiebreakers.names.sonneborn_berger)
            }
            Self::Progressive(_) => t_string!(i18n, tournaments.tiebreakers.names.progressive),
            Self::TournamentPairingNumber(order) => return seed_name(order),
            Self::DirectEncounter(_) => {
                t_string!(i18n, tournaments.tiebreakers.names.direct_encounter)
            }
            Self::NumberOfWins => t_string!(i18n, tournaments.tiebreakers.names.round_wins),
            Self::GamesWon => t_string!(i18n, tournaments.tiebreakers.names.games_won),
            Self::GamesWonWithBlack => {
                t_string!(i18n, tournaments.tiebreakers.names.wins_as_black)
            }
        }
        .to_string()
    }

    fn explanation(self, _i18n: I18nContext<Locale, I18nKeys>) -> String {
        // TODO: i18n once copy is approved.
        match self {
            Self::PrimaryScore => String::from("Points used to rank players."),
            Self::GamePoints => String::from("Points earned from individual games."),
            Self::MatchPoints => {
                String::from("Points earned from the combined result of each two-game match.")
            }
            Self::Buchholz(options) => format!(
                "Adds your opponents’ scores{}.",
                cut_scores(options.cut_lowest, options.cut_highest)
            ),
            Self::SonnebornBerger(options) => format!(
                "Weights your results by your opponents’ scores{}.",
                cut_scores(options.cut_lowest, 0)
            ),
            Self::Progressive(_) => {
                String::from("Adds your running score after each round, rewarding earlier points.")
            }
            Self::TournamentPairingNumber(order) => seed_explanation(order),
            Self::DirectEncounter(_) => String::from("Compares results between the tied players."),
            Self::NumberOfWins => String::from("Counts rounds awarded as wins."),
            Self::GamesWon => String::from("Counts individual games won."),
            Self::GamesWonWithBlack => String::from("Counts games won as Black."),
        }
    }
}
