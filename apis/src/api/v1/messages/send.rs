use crate::{
    common::{ChallengeUpdate, GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    notifications::{notify, time_control_label, Event},
    responses::{ChallengeResponse, GameResponse},
    websocket::{InternalServerMessage, MessageDestination, WsHub},
};
use actix_web::web::Data;
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use shared_types::{ChallengeId, ChallengeVisibility};
use std::sync::Arc;

fn get_opponent_id(game: &Game, bot: &User) -> uuid::Uuid {
    if game.white_id == bot.id {
        game.black_id
    } else {
        game.white_id
    }
}

pub(crate) async fn send_messages_batch(hub: &Arc<WsHub>, messages: Vec<InternalServerMessage>) {
    for message in messages {
        if let Err(error) = hub.dispatch_message(message).await {
            log::error!("encode committed bot effect: {error}");
        }
    }
}

fn create_game_action_response(
    game_response: GameResponse,
    action: GameReaction,
    bot: &User,
) -> GameActionResponse {
    GameActionResponse {
        game_id: game_response.game_id.clone(),
        game: game_response,
        game_action: action,
        user_id: bot.id,
        username: bot.username.clone(),
    }
}

pub async fn send_challenge_messages(
    hub: Data<Arc<WsHub>>,
    deleted_challenges: Vec<ChallengeId>,
    game: &Game,
    bot: &User,
    pool: &Data<DbPool>,
) -> Result<()> {
    let mut messages = Vec::new();
    let mut conn = get_conn(pool).await?;

    // Add challenge deletion messages
    for challenge_id in deleted_challenges {
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_id)),
        });
    }

    // Add game creation message
    let game_response = GameResponse::from_model(game, &mut conn).await?;
    let user_id = get_opponent_id(game, bot);
    let action_response = create_game_action_response(game_response, GameReaction::New, bot);

    messages.push(InternalServerMessage {
        destination: MessageDestination::User(user_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(action_response))),
    });
    send_messages_batch(hub.as_ref(), messages).await;
    Ok(())
}

pub async fn send_challenge_creation_message(
    hub: Data<Arc<WsHub>>,
    challenge_response: &ChallengeResponse,
    visibility: &ChallengeVisibility,
    opponent_id: Option<uuid::Uuid>,
) -> Result<()> {
    let mut messages = Vec::new();
    let challenge_clone = challenge_response.clone();

    match visibility {
        ChallengeVisibility::Public => {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Challenge(ChallengeUpdate::Created(Box::new(
                    challenge_clone,
                ))),
            });
        }
        ChallengeVisibility::Direct => {
            if let Some(opponent_id) = opponent_id {
                notify(Event::ChallengeReceived {
                    recipient: opponent_id,
                    challenger: challenge_response.challenger.username.clone(),
                    challenge_nanoid: challenge_response.challenge_id.0.clone(),
                    time_control: time_control_label(
                        challenge_response.speed,
                        challenge_response.time_base,
                        challenge_response.time_increment,
                    ),
                    rated: challenge_response.rated,
                });
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(opponent_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Direct(Box::new(
                        challenge_clone,
                    ))),
                });
            }
        }
        ChallengeVisibility::Private => {
            // Do private challenges even make sense for bots?
        }
    }

    send_messages_batch(hub.as_ref(), messages).await;
    Ok(())
}

pub async fn send_challenge_removed_messages(
    hub: Data<Arc<WsHub>>,
    challenges: Vec<ChallengeResponse>,
) {
    let mut messages = Vec::new();

    for challenge in challenges {
        match challenge.visibility {
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        challenge.challenge_id,
                    )),
                });
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(challenge.challenger.uid),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        challenge.challenge_id,
                    )),
                });
            }
            ChallengeVisibility::Direct => {
                if let Some(opponent) = challenge.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge.challenge_id.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(challenge.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge.challenge_id,
                        )),
                    });
                }
            }
        }
    }

    send_messages_batch(hub.as_ref(), messages).await;
}
