use leptos::prelude::*;

#[server]
pub async fn delete_account(password: String) -> Result<(), ServerFnError> {
    use crate::{
        functions::{
            auth::{
                identity::uuid,
                logout::logout,
                password::{hash_password, verify_password},
            },
            db::pool,
        },
        websocket::WsHub,
    };
    use actix_web::web::Data;
    use db_lib::{get_conn, models::User};
    let uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_active_by_uuid(&uuid, &mut conn).await?;
    verify_password(&password, &user.password)?;
    let replacement_password = uuid::Uuid::new_v4().to_string();
    let replacement_password_hash = hash_password(&replacement_password)?;

    let report = user
        .soft_delete(&replacement_password_hash, &mut conn)
        .await?;

    if let Ok(hub) = leptos_actix::extract::<Data<std::sync::Arc<WsHub>>>().await {
        let hub = hub.get_ref().as_ref();
        hub.revoke_user(user.id);
        if let Err(err) = send_soft_delete_updates(hub, report, user.id, &mut conn).await {
            log::warn!("Failed to send account deletion websocket updates: {err}");
        }
    }

    logout().await?;
    leptos_actix::redirect("/");
    Ok(())
}

#[cfg(feature = "ssr")]
async fn send_soft_delete_updates(
    hub: &crate::websocket::WsHub,
    report: db_lib::models::SoftDeleteReport,
    deleted_user_id: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<(), leptos::prelude::ServerFnError> {
    use crate::{
        common::{ChallengeUpdate, GameActionResponse, GameReaction, ServerMessage, ServerResult},
        responses::GameResponse,
        websocket::{reaction_messages, GameFinalize, InternalServerMessage, MessageDestination},
    };
    use bytes::Bytes;
    use codee::{binary::MsgpackSerdeCodec, Encoder};
    use db_lib::models::{Challenge, Game};
    use hive_lib::GameControl;
    use shared_types::{ChallengeId, ChallengeVisibility, Conclusion, GameId};

    fn challenge_removed_messages(challenge: Challenge) -> Vec<InternalServerMessage> {
        let challenge_id = ChallengeId(challenge.nanoid);
        let visibility = match challenge.visibility.parse::<ChallengeVisibility>() {
            Ok(visibility) => visibility,
            Err(_) => return Vec::new(),
        };
        match visibility {
            ChallengeVisibility::Public => {
                vec![InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_id)),
                }]
            }
            ChallengeVisibility::Private => {
                vec![InternalServerMessage {
                    destination: MessageDestination::User(challenge.challenger_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_id)),
                }]
            }
            ChallengeVisibility::Direct => challenge
                .opponent_id
                .map(|opponent_id| {
                    vec![
                        InternalServerMessage {
                            destination: MessageDestination::User(opponent_id),
                            message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                                challenge_id.clone(),
                            )),
                        },
                        InternalServerMessage {
                            destination: MessageDestination::User(challenge.challenger_id),
                            message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                                challenge_id,
                            )),
                        },
                    ]
                })
                .unwrap_or_default(),
        }
    }

    async fn add_game_update(
        hub: &crate::websocket::WsHub,
        messages: &mut Vec<InternalServerMessage>,
        game: Game,
        game_action: GameReaction,
        deleted_row: bool,
        deleted_user_id: uuid::Uuid,
        conn: &mut db_lib::DbConn<'_>,
    ) -> Result<GameFinalize, leptos::prelude::ServerFnError> {
        let game_id = GameId(game.nanoid.clone());
        let game_response = GameResponse::from_model(&game, conn)
            .await
            .map_err(leptos::prelude::ServerFnError::new)?;
        if deleted_row {
            hub.mark_deleted_game_pending(game_id.clone(), game.white_id, game.black_id);
        }
        messages.extend(reaction_messages(
            game_id.clone(),
            game.white_id,
            game.black_id,
            GameActionResponse {
                game_action,
                game: game_response,
                game_id: game_id.clone(),
                user_id: deleted_user_id,
                username: String::from("Deleted user"),
            },
        ));
        let finalize = GameFinalize {
            game_id,
            white_id: game.white_id,
            black_id: game.black_id,
        };
        messages.extend(finalize.own_game_removed_messages());
        Ok(finalize)
    }

    fn resigned_game_reaction(game: &Game, deleted_user_id: uuid::Uuid) -> Option<GameReaction> {
        if game.finished && game.conclusion == Conclusion::Timeout.to_string() {
            return Some(GameReaction::TimedOut);
        }
        if let Some(game_control) = game.last_game_control() {
            return Some(GameReaction::Control(game_control));
        }
        game.user_color(deleted_user_id)
            .map(|color| GameReaction::Control(GameControl::Resign(color)))
    }

    let mut messages = Vec::new();
    for challenge in report.deleted_challenges {
        messages.extend(challenge_removed_messages(challenge));
    }

    let mut finalizations = Vec::new();

    for game in report.resigned_games {
        let Some(game_action) = resigned_game_reaction(&game, deleted_user_id) else {
            log::warn!(
                "Skipping account deletion update for game {}: no resign or timeout reaction",
                game.nanoid
            );
            continue;
        };
        match add_game_update(
            hub,
            &mut messages,
            game,
            game_action,
            false,
            deleted_user_id,
            conn,
        )
        .await
        {
            Ok(finalize) => finalizations.push(finalize),
            Err(err) => log::warn!("Skipping account deletion game update: {err}"),
        }
    }

    for game in report.deleted_games {
        let Some(color) = game.user_color(deleted_user_id) else {
            log::warn!(
                "Skipping deleted account game update for game {}: deleted user is not a player",
                game.nanoid
            );
            continue;
        };
        match add_game_update(
            hub,
            &mut messages,
            game,
            GameReaction::Control(GameControl::Abort(color)),
            true,
            deleted_user_id,
            conn,
        )
        .await
        {
            Ok(finalize) => finalizations.push(finalize),
            Err(err) => log::warn!("Skipping deleted account game update: {err}"),
        }
    }

    for message in messages {
        let serialized = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
            hub.dispatch(&message.destination, Bytes::from(serialized), None)
                .await;
        }
    }

    for finalize in finalizations {
        hub.finalize_game(&finalize.game_id, finalize.white_id, finalize.black_id);
    }

    Ok(())
}
