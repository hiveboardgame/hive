use crate::i18n::*;
use shared_types::tournament_view::EliminationPlayerResultResponse;

use shared_types::tournament::elimination::Stage;

fn exit_stage_label(locale: Locale, stage: Stage) -> String {
    match stage {
        Stage::SingleRound { round_index } => td_string!(
            locale,
            tournaments.view.bracket.stage.round,
            round = round_index + 1
        )
        .to_string(),
        Stage::SingleFinal => {
            td_string!(locale, tournaments.view.bracket.stage.final_match).to_string()
        }
        Stage::Bronze => td_string!(locale, tournaments.ui.third_place_match_title).to_string(),
        Stage::WinnersRound { round_index } => {
            td_string!(locale, tournaments.ui.upper_round, value1 = round_index + 1).to_string()
        }
        Stage::LosersMinor { .. } | Stage::LosersMajor { .. } => td_string!(
            locale,
            tournaments.pairing.stage.lower_round,
            round = stage
                .lower_round_ordinal()
                .expect("lower-bracket stage has a native ordinal")
                + 1
        )
        .to_string(),
        Stage::GrandFinal => td_string!(locale, tournaments.pairing.stage.grand_final).to_string(),
        Stage::Reset => td_string!(locale, tournaments.pairing.stage.reset_final).to_string(),
    }
}

pub(crate) fn result_label(
    locale: Locale,
    result: EliminationPlayerResultResponse,
) -> Option<String> {
    match (result.placement, result.exit_stage) {
        (Some(1), _) => Some(td_string!(locale, tournaments.ui.champion_title).to_string()),
        (Some(2), _) => Some(td_string!(locale, tournaments.ui.second_place).to_string()),
        (Some(3), _) => Some(td_string!(locale, tournaments.pairing.stage.third_place).to_string()),
        (Some(4), Some(Stage::Bronze)) => {
            Some(td_string!(locale, tournaments.ui.fourth_place).to_string())
        }
        (Some(place), Some(stage)) => {
            Some(format!("#{place} · {}", exit_stage_label(locale, stage)))
        }
        (Some(place), None) => Some(format!("#{place}")),
        (None, None) => None,
        (None, Some(_)) => unreachable!("exit stage and placement become terminal together"),
    }
}

pub(crate) fn overview_result_label(
    locale: Locale,
    result: EliminationPlayerResultResponse,
) -> Option<String> {
    match (result.placement, result.exit_stage) {
        (Some(1), _) => Some(td_string!(locale, tournaments.ui.champion_title).to_string()),
        (Some(2), _) => Some(td_string!(locale, tournaments.ui.second_place).to_string()),
        (Some(3), _) => Some(td_string!(locale, tournaments.pairing.stage.third_place).to_string()),
        (Some(4), Some(Stage::Bronze)) => {
            Some(td_string!(locale, tournaments.ui.fourth_place).to_string())
        }
        (Some(_), Some(stage)) => Some(exit_stage_label(locale, stage)),
        (Some(_), None) | (None, None) => None,
        (None, Some(_)) => unreachable!("exit stage and placement become terminal together"),
    }
}
