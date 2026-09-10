use shared_types::tournament_view::EliminationPlayerResultResponse;

use shared_types::tournament::elimination::Stage;

fn exit_stage_label(stage: Stage) -> String {
    // TODO: i18n once copy is approved.
    match stage {
        Stage::SingleRound { round_index } => format!("Round {}", round_index + 1),
        Stage::SingleFinal => String::from("Final"),
        Stage::Bronze => String::from("Third-place match"),
        Stage::WinnersRound { round_index } => format!("Upper round {}", round_index + 1),
        Stage::LosersMinor { .. } | Stage::LosersMajor { .. } => format!(
            "Lower round {}",
            stage
                .lower_round_ordinal()
                .expect("lower-bracket stage has a native ordinal")
                + 1
        ),
        Stage::GrandFinal => String::from("Final"),
        Stage::Reset => String::from("Second final"),
    }
}

pub(crate) fn result_label(result: EliminationPlayerResultResponse) -> Option<String> {
    // TODO: i18n once copy is approved.
    match (result.placement, result.exit_stage) {
        (Some(1), _) => Some(String::from("Champion")),
        (Some(2), _) => Some(String::from("Second place")),
        (Some(3), _) => Some(String::from("Third place")),
        (Some(4), Some(Stage::Bronze)) => Some(String::from("Fourth place")),
        (Some(place), Some(stage)) => Some(format!("#{place} · {}", exit_stage_label(stage))),
        (Some(place), None) => Some(format!("#{place}")),
        (None, None) => None,
        (None, Some(_)) => unreachable!("exit stage and placement become terminal together"),
    }
}

pub(crate) fn overview_result_label(result: EliminationPlayerResultResponse) -> Option<String> {
    // TODO: i18n once copy is approved.
    match (result.placement, result.exit_stage) {
        (Some(1), _) => Some(String::from("Champion")),
        (Some(2), _) => Some(String::from("Second place")),
        (Some(3), _) => Some(String::from("Third place")),
        (Some(4), Some(Stage::Bronze)) => Some(String::from("Fourth place")),
        (Some(_), Some(stage)) => Some(exit_stage_label(stage)),
        (Some(_), None) | (None, None) => None,
        (None, Some(_)) => unreachable!("exit stage and placement become terminal together"),
    }
}
