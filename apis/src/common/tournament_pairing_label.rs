use crate::i18n::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentStageLabel {
    Final,
    GrandFinal,
    ResetFinal,
    ThirdPlace,
    UpperFinal,
    Semifinals,
    LowerFinal,
    LowerSemifinal,
    RoundOf(usize),
    LowerRound(u32),
}

impl TournamentStageLabel {
    pub fn label(&self, locale: Locale) -> String {
        match self {
            Self::Final => {
                td_string!(locale, tournaments.view.bracket.stage.final_match).to_string()
            }
            Self::GrandFinal => {
                td_string!(locale, tournaments.pairing.stage.grand_final).to_string()
            }
            Self::ResetFinal => {
                td_string!(locale, tournaments.pairing.stage.reset_final).to_string()
            }
            Self::ThirdPlace => {
                td_string!(locale, tournaments.pairing.stage.third_place).to_string()
            }
            Self::UpperFinal => {
                td_string!(locale, tournaments.pairing.stage.upper_final).to_string()
            }
            Self::Semifinals => {
                td_string!(locale, tournaments.pairing.stage.semifinals).to_string()
            }
            Self::LowerFinal => {
                td_string!(locale, tournaments.pairing.stage.lower_final).to_string()
            }
            Self::LowerSemifinal => {
                td_string!(locale, tournaments.pairing.stage.lower_semifinal).to_string()
            }
            Self::RoundOf(count) => {
                td_string!(locale, tournaments.pairing.stage.round_of, count = count).to_string()
            }
            Self::LowerRound(round) => {
                td_string!(locale, tournaments.pairing.stage.lower_round, round = round).to_string()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentPairingLabel {
    NewGame,
    RoundStarts {
        round: u32,
    },
    NewMatch {
        stage: TournamentStageLabel,
    },
    TiebreakSet {
        set: usize,
        stage: TournamentStageLabel,
    },
}

impl TournamentPairingLabel {
    pub fn label(&self, locale: Locale) -> String {
        match self {
            Self::NewGame => td_string!(locale, tournaments.pairing.new_game).to_string(),
            Self::RoundStarts { round } => {
                td_string!(locale, tournaments.pairing.round_starts, round = round).to_string()
            }
            Self::NewMatch { stage } => td_string!(
                locale,
                tournaments.pairing.new_match,
                stage = stage.label(locale)
            )
            .to_string(),
            Self::TiebreakSet { set, stage } => td_string!(
                locale,
                tournaments.pairing.tiebreak_set,
                set = set,
                stage = stage.label(locale)
            )
            .to_string(),
        }
    }

    pub fn description(&self, locale: Locale, opponent: &str) -> String {
        td_string!(
            locale,
            tournaments.pairing.opponent,
            label = self.label(locale),
            opponent = opponent
        )
        .to_string()
    }
}
