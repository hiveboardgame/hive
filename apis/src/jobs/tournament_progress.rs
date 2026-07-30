use crate::{
    common::{
        GameActionResponse,
        GameReaction,
        GameUpdate,
        ServerMessage,
        ServerResult,
        TournamentUpdate,
    },
    responses::GameResponse,
    websocket::{InternalServerMessage, MessageDestination, TournamentAudience, WsHub},
};
use actix_web::web::Data;
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{
    get_conn,
    models::{ProgressOutcome, Tournament},
    DbPool,
};
use shared_types::TournamentId;
use std::{sync::Arc, time::Duration};

/// An arena pairs on the tick, so this is also how long a player waits between
/// finishing one game and being offered the next. Every tick replays each
/// automated tournament from its games, so shortening it is not free — this is
/// the knob to turn if arenas feel sluggish or if the job starts costing too
/// much.
const TICK_SECONDS: u64 = 15;

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(TICK_SECONDS));
        loop {
            interval.tick().await;
            let Ok(mut conn) = get_conn(&pool).await else {
                continue;
            };
            let hub = hub.get_ref().clone();

            // Deliberately not wrapped in one transaction: `automatic_progress`
            // opens its own per tournament, so a tournament that has wedged
            // itself is logged and skipped instead of aborting every other
            // automated tournament on the site.
            let progressed = match Tournament::automatic_progress(&mut conn).await {
                Ok(progressed) => progressed,
                Err(error) => {
                    tracing::error!(%error, "automatic tournament progress failed");
                    continue;
                }
            };

            let mut messages = Vec::new();
            for (tournament, outcome) in progressed {
                let tournament_id = TournamentId(tournament.nanoid.clone());

                let games = match outcome {
                    // Nothing to say; the round is still being played.
                    ProgressOutcome::Waiting => continue,
                    ProgressOutcome::ReadyToFinish => {
                        messages.push(InternalServerMessage {
                            destination: MessageDestination::Tournament {
                                tournament_id: tournament_id.clone(),
                                audience: TournamentAudience::Updates,
                            },
                            message: ServerMessage::Tournament(TournamentUpdate::Finished(
                                tournament_id.clone(),
                            )),
                        });
                        continue;
                    }
                    // A fresh round, an arena pairing, or the replay of a match
                    // that could not stand — all of them are new games to play.
                    ProgressOutcome::Advanced(games) | ProgressOutcome::Replays(games) => games,
                };

                messages.push(InternalServerMessage {
                    destination: MessageDestination::Tournament {
                        tournament_id: tournament_id.clone(),
                        audience: TournamentAudience::Updates,
                    },
                    message: ServerMessage::Tournament(TournamentUpdate::StateChanged(
                        tournament_id.clone(),
                    )),
                });

                let game_responses = match GameResponse::from_games_batch(games, &mut conn).await {
                    Ok(responses) => responses,
                    Err(error) => {
                        tracing::error!(
                            tournament = %tournament.nanoid,
                            %error,
                            "could not build responses for newly paired games",
                        );
                        continue;
                    }
                };

                for game in game_responses {
                    for player in [&game.white_player, &game.black_player] {
                        messages.push(InternalServerMessage {
                            destination: MessageDestination::User(player.uid),
                            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                                GameActionResponse {
                                    game_action: GameReaction::New,
                                    game: game.clone(),
                                    game_id: game.game_id.clone(),
                                    user_id: player.uid,
                                    username: player.username.clone(),
                                },
                            ))),
                        });
                    }
                }
            }

            for message in messages {
                let serialized = ServerResult::Ok(Box::new(message.message));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                    hub.dispatch(&message.destination, Bytes::from(serialized))
                        .await;
                }
            }
        }
    });
}
