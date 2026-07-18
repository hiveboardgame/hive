use crate::{
    common::{
        ChallengeUpdate,
        GameActionResponse,
        GameReaction,
        GameUpdate,
        ServerMessage,
        ServerResult,
    },
    notifications::{
        game_end_reason_from,
        notify,
        notify_game_ended,
        notify_your_turn,
        time_control_label,
        Event,
        GameEndReason,
    },
    responses::{ChallengeResponse, GameResponse},
    websocket::{
        reaction_messages,
        GameFinalize,
        InternalServerMessage,
        MessageDestination,
        WsHub,
    },
};
use actix_web::web::Data;
use anyhow::Result;
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{
    get_conn,
    models::{Game, User},
    DbConn,
    DbPool,
};
use hive_lib::{GameControl, Turn};
use shared_types::{ChallengeId, ChallengeVisibility, GameId, TimeMode};
use std::sync::Arc;

fn get_opponent_id(game: &Game, bot: &User) -> uuid::Uuid {
    if game.white_id == bot.id {
        game.black_id
    } else {
        game.white_id
    }
}

async fn send_messages_batch(hub: &Arc<WsHub>, messages: Vec<InternalServerMessage>) {
    for message in messages {
        let serialized = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
            hub.dispatch(&message.destination, Bytes::from(serialized))
                .await;
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

fn maybe_add_tv_update(
    messages: &mut Vec<InternalServerMessage>,
    hub: &Arc<WsHub>,
    game_response: &GameResponse,
    is_final: bool,
) {
    if game_response.time_mode == TimeMode::RealTime
        && hub.should_send_tv(&game_response.game_id, is_final)
    {
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Game(Box::new(GameUpdate::Tv(game_response.clone()))),
        });
    }
}

pub async fn send_turn_messages(
    hub: Data<Arc<WsHub>>,
    game: &Game,
    bot: &User,
    conn: &mut DbConn<'_>,
    played_turn: Turn,
) -> Result<()> {
    let mut messages = Vec::new();
    let next_to_move = User::find_by_uuid(&game.current_player_id, conn).await?;
    let games = next_to_move.get_games_with_notifications(conn).await?;
    let game_responses = GameResponse::from_games_batch(games, conn).await?;
    messages.push(InternalServerMessage {
        destination: MessageDestination::User(game.current_player_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
    });

    let response = GameResponse::from_model(game, conn).await?;
    let action_response =
        create_game_action_response(response, GameReaction::Turn(played_turn), bot);

    maybe_add_tv_update(
        &mut messages,
        hub.as_ref(),
        &action_response.game,
        game.finished,
    );

    messages.extend(reaction_messages(
        action_response.game_id.clone(),
        game.white_id,
        game.black_id,
        action_response,
    ));

    if game.finished {
        messages.extend(
            GameFinalize {
                game_id: GameId(game.nanoid.clone()),
                white_id: game.white_id,
                black_id: game.black_id,
            }
            .own_game_removed_messages(),
        );
    }

    send_messages_batch(hub.as_ref(), messages).await;

    // Bot path is its own dispatcher — finalize after sending so the human
    // opponent's fanout still reached them.
    if game.finished {
        hub.finalize_game(&GameId(game.nanoid.clone()), game.white_id, game.black_id);
    }

    if game.finished {
        let reason = game_end_reason_from(game, GameEndReason::Move);
        if let Err(e) = notify_game_ended(game, reason, conn).await {
            log::error!("notify game ended {}: {e}", game.nanoid);
        }
    } else {
        notify_your_turn(game, bot.username.clone());
    }
    Ok(())
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
                message: ServerMessage::Challenge(ChallengeUpdate::Created(challenge_clone)),
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
                    message: ServerMessage::Challenge(ChallengeUpdate::Direct(challenge_clone)),
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

pub async fn send_control_messages(
    hub: Data<Arc<WsHub>>,
    game: &Game,
    bot: &User,
    pool: &Data<DbPool>,
    game_control: GameControl,
) -> Result<()> {
    let mut messages = Vec::new();
    let mut conn = get_conn(pool).await?;

    let game_response = GameResponse::from_model(game, &mut conn).await?;
    let action_response =
        create_game_action_response(game_response, GameReaction::Control(game_control), bot);

    maybe_add_tv_update(
        &mut messages,
        hub.as_ref(),
        &action_response.game,
        game.finished,
    );

    // Urgent fanout to the opponent: MessageDestination::Game only reaches
    // already-subscribed sockets, so a human off the game page (e.g. in the
    // lobby) wouldn't otherwise see a bot resign/abort.
    let opponent_id = get_opponent_id(game, bot);
    let opponent = User::find_by_uuid(&opponent_id, &mut conn).await?;
    let opponent_games = opponent.get_games_with_notifications(&mut conn).await?;
    let opponent_responses = GameResponse::from_games_batch(opponent_games, &mut conn).await?;
    messages.push(InternalServerMessage {
        destination: MessageDestination::User(opponent_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Urgent(opponent_responses))),
    });

    messages.extend(reaction_messages(
        action_response.game_id.clone(),
        game.white_id,
        game.black_id,
        action_response,
    ));

    if game.finished {
        messages.extend(
            GameFinalize {
                game_id: GameId(game.nanoid.clone()),
                white_id: game.white_id,
                black_id: game.black_id,
            }
            .own_game_removed_messages(),
        );
    }

    send_messages_batch(hub.as_ref(), messages).await;

    if game.finished {
        hub.finalize_game(&GameId(game.nanoid.clone()), game.white_id, game.black_id);
        let end_reason = match game_control {
            GameControl::Resign(_) => Some(GameEndReason::Resignation),
            GameControl::DrawAccept(_) => Some(GameEndReason::Agreement),
            _ => None,
        };
        if let Some(fallback) = end_reason {
            let reason = game_end_reason_from(game, fallback);
            if let Err(e) = notify_game_ended(game, reason, &mut conn).await {
                log::error!("notify game ended {}: {e}", game.nanoid);
            }
        }
    }
    Ok(())
}
