use hive_lib::{Color, GameStatus};
use uuid::Uuid;

use super::state::TournamentState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WithdrawalObligation {
    Terminal {
        slot_id: Uuid,
        game_id: Uuid,
    },
    Played {
        slot_id: Uuid,
        game_id: Uuid,
        color: Color,
    },
    Unstarted {
        slot_id: Uuid,
        game_id: Uuid,
    },
    Unreleased,
}

/// Normalizes current fixed-field obligations without changing format facts.
/// A `PlayedResignation` executor must normalize the returned game:
/// an already-expired clock can settle before the withdrawal does.
/// Swiss bye/sit-out decisions and elimination withdrawal remain in their
/// format adapters. Arena has no withdrawal operation.
pub(crate) fn withdrawal_obligations(
    state: &TournamentState,
    player: Uuid,
) -> Vec<WithdrawalObligation> {
    let games_by_slot = state.games_by_slot();
    let mut actions = Vec::new();
    for slot in &state.slots {
        if slot.white != player && slot.black != player {
            continue;
        }
        if slot.resolution.is_some() {
            continue;
        }
        match games_by_slot.get(&slot.id).copied() {
            None => actions.push(WithdrawalObligation::Unreleased),
            Some(game) => {
                if game.finished {
                    actions.push(WithdrawalObligation::Terminal {
                        slot_id: slot.id,
                        game_id: game.id,
                    });
                } else if game.turn > 0 || game.game_status == GameStatus::InProgress.to_string() {
                    actions.push(WithdrawalObligation::Played {
                        slot_id: slot.id,
                        game_id: game.id,
                        color: if slot.white == player {
                            Color::White
                        } else {
                            Color::Black
                        },
                    });
                } else {
                    actions.push(WithdrawalObligation::Unstarted {
                        slot_id: slot.id,
                        game_id: game.id,
                    });
                }
            }
        }
    }
    actions
}
