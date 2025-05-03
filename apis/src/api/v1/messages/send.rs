use crate::api::v1::auth::auth::Auth;
use crate::common::{GameUpdate, ServerMessage, ServerResult, GameReaction, GameActionResponse};
use crate::websocket::{ClientActorMessage, InternalServerMessage, MessageDestination, WsServer};
use actix::Addr;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse};
use anyhow::{anyhow, Result};
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use hive_lib::{Piece, Position, State, Turn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_types::GameId;
use shared_types::TimeMode;
use std::str::FromStr;
use crate::responses::GameResponse;

pub async fn send_messages(
    ws_server: Data<Addr<WsServer>>,
    game: &Game,
    user: &User,
    pool: &Data<DbPool>,
    played_turn: Turn,
) -> Result<()> {
    let mut messages = Vec::new();
    let mut conn = get_conn(pool).await?;
    let next_to_move = User::find_by_uuid(&game.current_player_id, &mut conn).await?;
    let games = next_to_move.get_games_with_notifications(&mut conn).await?;
    let mut game_responses = Vec::new();
    for g in games {
        game_responses.push(GameResponse::from_model(&g, &mut conn).await?);
    }
    messages.push(InternalServerMessage {
        destination: MessageDestination::User(game.current_player_id),
        message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
    });
    let response = GameResponse::from_model(&game, &mut conn).await?;
    messages.push(InternalServerMessage {
        destination: MessageDestination::Game(GameId(game.nanoid.clone())),
        message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
            game_id: GameId(game.nanoid.to_owned()),
            game: response.clone(),
            game_action: GameReaction::Turn(played_turn),
            user_id: user.id.to_owned(),
            username: user.username.to_owned(),
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
                from: None,
            };
            ws_server.do_send(cam);
        };
    }
    Ok(())
}
