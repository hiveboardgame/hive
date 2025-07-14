use crate::common::{
    ChallengeUpdate, GameActionResponse, GameReaction, GameUpdate, ServerMessage, ServerResult,
};
use crate::responses::{ChallengeResponse, GameResponse};
use crate::websocket::{ClientActorMessage, InternalServerMessage, MessageDestination, WsServer};
use actix::Addr;
use actix_web::web::Data;
use anyhow::Result;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use hive_lib::Turn;
use shared_types::{ChallengeId, ChallengeVisibility, GameId, TimeMode};

pub async fn send_messages(
    ws_server: Data<Addr<WsServer>>,
    game: &Game,
    bot: &User,
    pool: &Data<DbPool>,
    played_turn: Turn,
) -> Result<()> {
    let mut messages = Vec::new();
    let mut conn = get_conn(pool).await?;
    let next_to_move = User::find_by_uuid(&game.current_player_id, &mut conn).await?;
    let games = next_to_move.get_games_with_notifications(&mut conn).await?;
    let mut game_responses = Vec::new();
    let user_id = if game.white_id == bot.id {
        game.black_id
    } else {
        game.white_id
    };
    for g in games {
        game_responses.push(GameResponse::from_model(&g, &mut conn).await?);
    }
    messages.push(InternalServerMessage {
        destination: MessageDestination::User(game.current_player_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
    });
    let response = GameResponse::from_model(game, &mut conn).await?;
    messages.push(InternalServerMessage {
        destination: MessageDestination::Game(GameId(game.nanoid.clone())),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
            game_id: GameId(game.nanoid.to_owned()),
            game: response.clone(),
            game_action: GameReaction::Turn(played_turn),
            user_id: bot.id.to_owned(),
            username: bot.username.to_owned(),
        }))),
    });
    // TODO: Just add the few top games and keep them rated
    if response.time_mode == TimeMode::RealTime {
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Game(Box::new(GameUpdate::Tv(response))),
        });
    };
    for message in messages {
        let serialized = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
            let cam = ClientActorMessage {
                destination: message.destination,
                serialized,
                from: Some(user_id),
            };
            ws_server.do_send(cam);
        };
    }
    Ok(())
}

pub async fn send_challenge_messages(
    ws_server: Data<Addr<WsServer>>,
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

    let user_id = if game.white_id == bot.id {
        game.black_id
    } else {
        game.white_id
    };

    messages.push(InternalServerMessage {
        destination: MessageDestination::User(user_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
            game_action: GameReaction::New,
            game: game_response.clone(),
            game_id: game_response.game_id.clone(),
            user_id: bot.id,
            username: bot.username.to_owned(),
        }))),
    });

    // Send all messages
    for message in messages {
        let serialized = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
            let cam = ClientActorMessage {
                destination: message.destination,
                serialized,
                from: Some(user_id),
            };
            ws_server.do_send(cam);
        };
    }
    Ok(())
}

pub async fn send_challenge_creation_message(
    ws_server: Data<Addr<WsServer>>,
    challenge_response: &ChallengeResponse,
    visibility: &ChallengeVisibility,
    opponent_id: Option<uuid::Uuid>,
) -> Result<()> {
    let mut messages = Vec::new();

    match visibility {
        ChallengeVisibility::Public => {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Challenge(ChallengeUpdate::Created(
                    challenge_response.clone(),
                )),
            });
        }
        ChallengeVisibility::Direct => {
            if let Some(opponent_id) = opponent_id {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(opponent_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                        challenge_response.clone(),
                    )),
                });
            }
        }
        ChallengeVisibility::Private => {
            // Do private challenges even make sense for bots?
        }
    }

    // Send all messages
    for message in messages {
        let serialized = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
            let cam = ClientActorMessage {
                destination: message.destination,
                serialized,
                from: Some(challenge_response.challenger.uid),
            };
            ws_server.do_send(cam);
        }
    }

    Ok(())
}
