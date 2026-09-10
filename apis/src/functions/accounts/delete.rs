#[cfg(feature = "ssr")]
use crate::websocket::WsHub;
#[cfg(feature = "ssr")]
use db_lib::{models::SoftDeleteReport, DbConn};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use std::sync::Arc;
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[server]
pub async fn delete_account(password: String) -> Result<(), ServerFnError> {
    use crate::functions::{
        auth::{
            identity::uuid,
            logout::logout,
            password::{hash_password, verify_password},
        },
        db::pool,
    };
    use actix_web::web::Data;
    use db_lib::{
        get_conn,
        models::{PushDevice, User},
    };
    let uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_active_by_uuid(&uuid, &mut conn).await?;
    verify_password(&password, &user.password)?;
    let replacement_password = Uuid::new_v4().to_string();
    let replacement_password_hash = hash_password(&replacement_password)?;
    let hub = leptos_actix::extract::<Data<Arc<WsHub>>>().await.ok();

    let report = user
        .soft_delete(&replacement_password_hash, &mut conn)
        .await?;

    if let Some(hub) = hub {
        let hub = hub.get_ref().as_ref();
        hub.revoke_user(user.id);
        if let Err(err) = send_soft_delete_updates(hub, report, user.id, &mut conn).await {
            log::warn!("Failed to send account deletion websocket updates: {err}");
        }
    }

    if let Err(err) = PushDevice::revoke_all_for_user(user.id, &mut conn).await {
        log::warn!(
            "Failed to revoke push devices for soft-deleted user {}: {err}",
            user.id
        );
    }
    logout(None).await?;
    leptos_actix::redirect("/");
    Ok(())
}

#[cfg(feature = "ssr")]
async fn send_soft_delete_updates(
    hub: &WsHub,
    report: SoftDeleteReport,
    deleted_user_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<(), ServerFnError> {
    use crate::{
        common::{ChallengeUpdate, GameReaction, ServerMessage, TournamentUpdate},
        notifications::{game_end_reason_from, GameEndReason},
        websocket::{
            server_handlers::{
                game::{
                    append_fixed_field_commit,
                    append_terminal_game_fanout,
                    TerminalGameFanout,
                },
                tournaments::{
                    append_public_tournament_patches_by_id,
                    arena::{
                        append_arena_player_stats_patches,
                        append_terminal_arena_game_projection,
                    },
                    PublicTournamentSection,
                },
            },
            HandlerOutput,
            InternalServerMessage,
            MessageDestination,
        },
    };
    use db_lib::models::{Challenge, Game, Tournament};
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

    fn resigned_game_reaction(
        game: &Game,
        deleted_user_id: Uuid,
    ) -> Result<Option<GameReaction>, ServerFnError> {
        if game.finished && game.conclusion == Conclusion::Timeout.to_string() {
            return Ok(Some(GameReaction::TimedOut));
        }
        if let Some(game_control) = game.last_game_control().map_err(ServerFnError::new)? {
            return Ok(Some(GameReaction::Control(game_control)));
        }
        Ok(game
            .user_color(deleted_user_id)
            .map(|color| GameReaction::Control(GameControl::Resign(color))))
    }

    let mut output = HandlerOutput::empty();
    for tournament_id in report.deleted_tournament_ids {
        hub.unsubscribe_user_from_tournament_chat(deleted_user_id, &tournament_id);
        hub.invalidate_tournament_members(&tournament_id);
        output.messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Deleted(tournament_id.clone())),
        });
        output.messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(tournament_id)),
        });
    }
    for tournament_id in report.changed_tournament_ids {
        hub.invalidate_tournament_members(&tournament_id);
        output.messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(
                tournament_id.clone(),
            )),
        });
        append_public_tournament_patches_by_id(
            &tournament_id,
            &[
                PublicTournamentSection::Memberships,
                PublicTournamentSection::Lifecycle,
            ],
            &mut output,
            conn,
        )
        .await;
    }
    for tournament_id in report.removed_membership_tournament_ids {
        hub.unsubscribe_user_from_tournament_chat(deleted_user_id, &tournament_id);
        hub.invalidate_tournament_members(&tournament_id);
        output.messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(
                tournament_id.clone(),
            )),
        });
        append_public_tournament_patches_by_id(
            &tournament_id,
            &[PublicTournamentSection::Memberships],
            &mut output,
            conn,
        )
        .await;
    }
    for tournament_id in report.withdrawn_tournament_ids {
        append_public_tournament_patches_by_id(
            &tournament_id,
            &[PublicTournamentSection::Memberships],
            &mut output,
            conn,
        )
        .await;
    }
    for tournament_id in report.arena_paused_tournament_ids {
        match Tournament::find_by_tournament_id(&tournament_id, conn).await {
            Ok(tournament) => {
                append_arena_player_stats_patches(
                    tournament.id,
                    &[deleted_user_id],
                    &mut output,
                    conn,
                )
                .await;
            }
            Err(error) => log::error!(
                "Account deletion committed Arena pause for {tournament_id}, but its patch could not be built: {error}"
            ),
        }
    }
    for commit in report.fixed_field_commits {
        append_fixed_field_commit(Some(commit), &mut output, conn).await;
    }
    for challenge in report.deleted_challenges {
        output
            .messages
            .extend(challenge_removed_messages(challenge));
    }

    for game in &report.tournament_terminal_games {
        if game.arena_ordinal.is_some() {
            append_terminal_arena_game_projection(game, &mut output, conn).await;
        }
        let game_action = if game.conclusion == Conclusion::Timeout.to_string() {
            GameReaction::TimedOut
        } else {
            match game.last_game_control() {
                Ok(Some(control)) => GameReaction::Control(control),
                Ok(None) => GameReaction::Adjudicated,
                Err(error) => {
                    log::error!(
                        "Account deletion game {} has invalid control history: {error}",
                        game.nanoid,
                    );
                    GameReaction::Adjudicated
                }
            }
        };
        let effect_user_id = if game.conclusion == Conclusion::Timeout.to_string() {
            game.current_player_id
        } else {
            deleted_user_id
        };
        let reason = game_end_reason_from(game, GameEndReason::Resignation);
        let effect = TerminalGameFanout {
            game,
            reaction: game_action,
            actor_id: effect_user_id,
            actor_username: None,
            end_reason: Some(reason),
            notification_excluded: Some(deleted_user_id),
            publish_tv: true,
            deleted_row: false,
        };
        append_terminal_game_fanout(
            hub.data.as_ref(),
            std::slice::from_ref(&effect),
            &mut output,
            conn,
        )
        .await;
    }

    for game in &report.resigned_games {
        let game_action = match resigned_game_reaction(game, deleted_user_id) {
            Ok(Some(game_action)) => game_action,
            Ok(None) => {
                log::warn!(
                    "Skipping account deletion update for game {}: no resign or timeout reaction",
                    game.nanoid,
                );
                continue;
            }
            Err(error) => {
                log::error!(
                    "Skipping account deletion update for game {}: {error}",
                    game.nanoid,
                );
                continue;
            }
        };
        let effect_user_id = if game.conclusion == Conclusion::Timeout.to_string() {
            game.current_player_id
        } else {
            deleted_user_id
        };
        let effect = TerminalGameFanout {
            game,
            reaction: game_action,
            actor_id: effect_user_id,
            actor_username: None,
            end_reason: Some(game_end_reason_from(game, GameEndReason::Resignation)),
            notification_excluded: Some(deleted_user_id),
            publish_tv: false,
            deleted_row: false,
        };
        append_terminal_game_fanout(
            hub.data.as_ref(),
            std::slice::from_ref(&effect),
            &mut output,
            conn,
        )
        .await;
    }

    for game in &report.deleted_games {
        let Some(color) = game.user_color(deleted_user_id) else {
            log::warn!(
                "Skipping deleted account game update for game {}: deleted user is not a player",
                game.nanoid,
            );
            continue;
        };
        hub.mark_deleted_game_pending(GameId(game.nanoid.clone()), game.white_id, game.black_id);
        let effect = TerminalGameFanout {
            game,
            reaction: GameReaction::Control(GameControl::Abort(color)),
            actor_id: deleted_user_id,
            actor_username: None,
            end_reason: None,
            notification_excluded: Some(deleted_user_id),
            publish_tv: false,
            deleted_row: true,
        };
        append_terminal_game_fanout(
            hub.data.as_ref(),
            std::slice::from_ref(&effect),
            &mut output,
            conn,
        )
        .await;
    }

    hub.dispatch_handler_output(output).await;

    Ok(())
}
