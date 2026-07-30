use crate::i18n::*;
use leptos_i18n::I18nContext;
use shared_types::Tiebreaker;

/// One sentence on what a tiebreaker measures.
///
/// Shared rather than duplicated: the standings table shows it on the column
/// header, and the create form shows it while an organizer is choosing — and
/// the two must not drift apart.
pub fn tiebreaker_explanation(
    i18n: I18nContext<Locale, I18nKeys>,
    tiebreaker: Tiebreaker,
) -> String {
    match tiebreaker {
        Tiebreaker::RawPoints => t_string!(i18n, tournaments.tiebreakers.raw_points),
        Tiebreaker::HeadToHead => t_string!(i18n, tournaments.tiebreakers.head_to_head),
        Tiebreaker::WinsAsBlack => t_string!(i18n, tournaments.tiebreakers.wins_as_black),
        Tiebreaker::SonnebornBerger => t_string!(i18n, tournaments.tiebreakers.sonneborn_berger),
        Tiebreaker::MatchPoints => t_string!(i18n, tournaments.tiebreakers.match_points),
        Tiebreaker::GamePoints => t_string!(i18n, tournaments.tiebreakers.game_points),
        Tiebreaker::Buchholz => t_string!(i18n, tournaments.tiebreakers.buchholz),
        Tiebreaker::BuchholzCut1 => t_string!(i18n, tournaments.tiebreakers.buchholz_cut_1),
        Tiebreaker::BuchholzCut2 => t_string!(i18n, tournaments.tiebreakers.buchholz_cut_2),
        Tiebreaker::BuchholzMedian => t_string!(i18n, tournaments.tiebreakers.buchholz_median),
        Tiebreaker::BuchholzBuchholz => t_string!(i18n, tournaments.tiebreakers.buchholz_buchholz),
        Tiebreaker::Koya => t_string!(i18n, tournaments.tiebreakers.koya),
        Tiebreaker::ProgressiveScore => t_string!(i18n, tournaments.tiebreakers.progressive_score),
        Tiebreaker::Wins => t_string!(i18n, tournaments.tiebreakers.wins),
        Tiebreaker::RoundsSurvived => t_string!(i18n, tournaments.tiebreakers.rounds_survived),
        Tiebreaker::GamesPlayed => t_string!(i18n, tournaments.tiebreakers.games_played),
        Tiebreaker::Draws => t_string!(i18n, tournaments.tiebreakers.draws),
        Tiebreaker::Losses => t_string!(i18n, tournaments.tiebreakers.losses),
        Tiebreaker::CurrentStreak => t_string!(i18n, tournaments.tiebreakers.current_streak),
        Tiebreaker::BestStreak => t_string!(i18n, tournaments.tiebreakers.best_streak),
        Tiebreaker::Berserks => t_string!(i18n, tournaments.tiebreakers.berserks),
    }
    .to_string()
}
