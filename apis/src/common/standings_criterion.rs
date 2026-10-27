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

fn seed_name(i18n: I18nContext<Locale, I18nKeys>, order: TournamentPairingNumberOrder) -> String {
    match order {
        TournamentPairingNumberOrder::Ascending => {
            t_string!(i18n, tournaments.tiebreakers.names.seed_ascending).to_string()
        }
        TournamentPairingNumberOrder::Descending => {
            t_string!(i18n, tournaments.tiebreakers.names.seed_descending).to_string()
        }
    }
    .to_string()
}

fn seed_explanation(
    i18n: I18nContext<Locale, I18nKeys>,
    order: TournamentPairingNumberOrder,
) -> String {
    match order {
        TournamentPairingNumberOrder::Ascending => {
            t_string!(i18n, tournaments.tiebreakers.seed_ascending).to_string()
        }
        TournamentPairingNumberOrder::Descending => {
            t_string!(i18n, tournaments.tiebreakers.seed_descending).to_string()
        }
    }
    .to_string()
}

fn cut_scores(i18n: I18nContext<Locale, I18nKeys>, lowest: u32, highest: u32) -> String {
    match (lowest, highest) {
        (0, 0) => String::new(),
        (1, 0) => t_string!(i18n, tournaments.tiebreakers.cut_lowest).to_string(),
        (low, 0) => {
            t_string!(i18n, tournaments.tiebreakers.cut_lowest_many, count = low).to_string()
        }
        (0, 1) => t_string!(i18n, tournaments.tiebreakers.cut_highest).to_string(),
        (0, high) => {
            t_string!(i18n, tournaments.tiebreakers.cut_highest_many, count = high).to_string()
        }
        (low, high) => t_string!(
            i18n,
            tournaments.tiebreakers.cut_both,
            low = low,
            high = high
        )
        .to_string(),
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

            Self::MatchesWon => {
                return t_string!(i18n, tournaments.tiebreakers.names.matches_won).to_string()
            }
            Self::TournamentPairingNumber(order) => return seed_name(i18n, order),
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

    fn explanation(self, i18n: I18nContext<Locale, I18nKeys>) -> String {
        match self {
            Self::PrimaryScore => {
                t_string!(i18n, tournaments.tiebreakers.primary_score).to_string()
            }
            Self::GamePoints => {
                t_string!(i18n, tournaments.tiebreakers.individual_games).to_string()
            }
            Self::MatchPoints => {
                t_string!(i18n, tournaments.tiebreakers.round_robin_match_points).to_string()
            }
            Self::MatchesWon => t_string!(i18n, tournaments.tiebreakers.matches_won).to_string(),
            Self::TournamentPairingNumber(order) => seed_explanation(i18n, order),
            Self::Progressive(_) => {
                t_string!(i18n, tournaments.tiebreakers.progressive).to_string()
            }
            Self::DirectEncounter(_) => {
                t_string!(i18n, tournaments.tiebreakers.direct_encounter).to_string()
            }
            Self::SonnebornBerger(options) => t_string!(
                i18n,
                tournaments.tiebreakers.sonneborn_berger,
                cut = cut_scores(i18n, options.cut_lowest, 0)
            )
            .to_string(),
            Self::Koya(options) => {
                if options.threshold_numerator == 1 && options.threshold_denominator.get() == 2 {
                    t_string!(i18n, tournaments.tiebreakers.koya_half).to_string()
                } else {
                    t_string!(
                        i18n,
                        tournaments.tiebreakers.koya_threshold,
                        numerator = options.threshold_numerator,
                        denominator = options.threshold_denominator.get()
                    )
                    .to_string()
                }
            }
            Self::NumberOfWins => t_string!(i18n, tournaments.tiebreakers.score_wins).to_string(),
            Self::GamesWon => t_string!(i18n, tournaments.tiebreakers.games_won).to_string(),
            Self::GamesWonWithBlack => {
                t_string!(i18n, tournaments.tiebreakers.wins_as_black).to_string()
            }
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
            Self::TournamentPairingNumber(order) => return seed_name(i18n, order),
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

    fn explanation(self, i18n: I18nContext<Locale, I18nKeys>) -> String {
        match self {
            Self::PrimaryScore => {
                t_string!(i18n, tournaments.tiebreakers.primary_score).to_string()
            }
            Self::GamePoints => {
                t_string!(i18n, tournaments.tiebreakers.individual_games).to_string()
            }
            Self::MatchPoints => {
                t_string!(i18n, tournaments.tiebreakers.swiss_match_points).to_string()
            }
            Self::Buchholz(options) => t_string!(
                i18n,
                tournaments.tiebreakers.buchholz,
                cut = cut_scores(i18n, options.cut_lowest, options.cut_highest)
            )
            .to_string(),
            Self::SonnebornBerger(options) => t_string!(
                i18n,
                tournaments.tiebreakers.sonneborn_berger,
                cut = cut_scores(i18n, options.cut_lowest, 0)
            )
            .to_string(),
            Self::Progressive(_) => {
                t_string!(i18n, tournaments.tiebreakers.progressive).to_string()
            }
            Self::TournamentPairingNumber(order) => seed_explanation(i18n, order),
            Self::DirectEncounter(_) => {
                t_string!(i18n, tournaments.tiebreakers.direct_encounter).to_string()
            }
            Self::NumberOfWins => t_string!(i18n, tournaments.tiebreakers.round_wins).to_string(),
            Self::GamesWon => t_string!(i18n, tournaments.tiebreakers.games_won).to_string(),
            Self::GamesWonWithBlack => {
                t_string!(i18n, tournaments.tiebreakers.wins_as_black).to_string()
            }
        }
    }
}
