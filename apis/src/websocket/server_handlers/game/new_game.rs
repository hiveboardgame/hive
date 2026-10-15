use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage, TournamentPairingLabel},
    notifications::{notify, time_control_label, Event},
    responses::GameResponse,
    websocket::{InternalServerMessage, MessageDestination},
};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) fn new_game_messages(
    game: Arc<GameResponse>,
    accepted_by: Option<Uuid>,
    pairing_label: Option<TournamentPairingLabel>,
) -> Vec<InternalServerMessage> {
    // Every tournament release has an in-app notice. External notifications are
    // grouped at the dispatch boundary, where Arena presence is available.
    let pairing_label = game
        .tournament
        .as_ref()
        .map(|_| pairing_label.unwrap_or(TournamentPairingLabel::NewGame));
    let mut messages = Vec::with_capacity(2);
    for (player, opponent) in [
        (&game.white_player, &game.black_player),
        (&game.black_player, &game.white_player),
    ] {
        let notification = match &game.tournament {
            None if accepted_by != Some(player.uid) => Some(Event::GameStarted {
                recipient: player.uid,
                opponent: opponent.username.clone(),
                game_nanoid: game.game_id.0.clone(),
                time_control: time_control_label(game.speed, game.time_base, game.time_increment),
                speed: game.speed,
            }),
            _ => None,
        };
        if let Some(notification) = notification {
            notify(notification);
        }
        let actor = if accepted_by == Some(opponent.uid) {
            opponent
        } else {
            player
        };
        messages.push(InternalServerMessage {
            destination: MessageDestination::User(player.uid),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_action: GameReaction::New {
                    pairing_label: pairing_label.clone(),
                },
                game: game.clone(),
                game_id: game.game_id.clone(),
                user_id: actor.uid,
                username: actor.username.clone(),
            }))),
        });
    }
    messages
}
