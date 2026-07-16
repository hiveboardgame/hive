pub mod abandon;
pub mod adjudicate_result;
pub mod bulk_adjudicate;
pub mod create;
pub mod delete;
pub mod finish;
pub mod handler;
pub mod invitation_accept;
pub mod invitation_create;
pub mod invitation_decline;
pub mod invitation_retract;
pub mod join;
pub mod kick;
pub mod leave;
pub mod progress_to_next_round;
pub mod start;

use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use shared_types::TournamentId;
use uuid::Uuid;

fn membership_removed_messages(
    tournament_id: TournamentId,
    user_id: Uuid,
) -> Vec<InternalServerMessage> {
    vec![
        InternalServerMessage {
            destination: MessageDestination::User(user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Left(tournament_id.clone())),
        },
        InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::StateChanged(tournament_id)),
        },
    ]
}
