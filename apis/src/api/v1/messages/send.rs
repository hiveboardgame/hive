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
use hive_lib::{GameControl, Turn};
use shared_types::{ChallengeId, ChallengeVisibility, TimeMode};

fn get_opponent_id(game: &Game, bot: &User) -> uuid::Uuid {
    if game.white_id == bot.id {
        game.black_id
    } else {
        game.white_id
    }
}

fn send_messages_batch(
    ws_server: &Addr<WsServer>,
    messages: Vec<InternalServerMessage>,
    from_user_id: Option<uuid::Uuid>,
) {
    for message in messages {
        let serialized = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
            let cam = ClientActorMessage {
                destination: message.destination,
                serialized,
                from: from_user_id,
            };
            ws_server.do_send(cam);
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

fn maybe_add_tv_update(messages: &mut Vec<InternalServerMessage>, game_response: &GameResponse) {
    if game_response.time_mode == TimeMode::RealTime {
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Game(Box::new(GameUpdate::Tv(game_response.clone()))),
        });
    }
}

pub async fn send_turn_messages(
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
    let user_id = get_opponent_id(game, bot);
    
    for g in games {
        game_responses.push(GameResponse::from_model(&g, &mut conn).await?);
    }
    
    messages.push(InternalServerMessage {
        destination: MessageDestination::User(game.current_player_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
    });
    
    let response = GameResponse::from_model(game, &mut conn).await?;
    let action_response = create_game_action_response(response, GameReaction::Turn(played_turn), bot);
    let game_id = action_response.game_id.clone();
    
    maybe_add_tv_update(&mut messages, &action_response.game);
    
    messages.push(InternalServerMessage {
        destination: MessageDestination::Game(game_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(action_response))),
    });
    
    send_messages_batch(&ws_server, messages, Some(user_id));
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
    let user_id = get_opponent_id(game, bot);
    let action_response = create_game_action_response(game_response, GameReaction::New, bot);

    messages.push(InternalServerMessage {
        destination: MessageDestination::User(user_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(action_response))),
    });

    send_messages_batch(&ws_server, messages, Some(user_id));
    Ok(())
}

pub async fn send_challenge_creation_message(
    ws_server: Data<Addr<WsServer>>,
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

    send_messages_batch(&ws_server, messages, Some(challenge_response.challenger.uid));
    Ok(())
}

pub async fn send_control_messages(
    ws_server: Data<Addr<WsServer>>,
    game: &Game,
    bot: &User,
    pool: &Data<DbPool>,
    game_control: GameControl,
) -> Result<()> {
    let mut messages = Vec::new();
    let mut conn = get_conn(pool).await?;
    
    let opponent_id = get_opponent_id(game, bot);
    let game_response = GameResponse::from_model(game, &mut conn).await?;
    let action_response = create_game_action_response(game_response, GameReaction::Control(game_control), bot);
    let game_id = action_response.game_id.clone();

    maybe_add_tv_update(&mut messages, &action_response.game);

    messages.push(InternalServerMessage {
        destination: MessageDestination::Game(game_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(action_response))),
    });
    
    send_messages_batch(&ws_server, messages, Some(opponent_id));
    Ok(())
}
