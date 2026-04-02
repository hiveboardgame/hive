use std::sync::Arc;

use super::{
    challenges::handler::ChallengeHandler,
    chat::handler::ChatHandler,
    game::handler::GameActionHandler,
    oauth::handler::OauthHandler,
    schedules::ScheduleHandler,
    tournaments::handler::TournamentHandler,
    user_status::handler::UserStatusHandler,
};
use crate::{
    common::{ClientRequest, GameAction},
    websocket::{
        messages::{AuthError, InternalServerMessage, WsMessage},
        WebsocketData,
    },
};
use db_lib::{
    get_conn,
    helpers::is_blocked,
    models::{Game, Tournament},
    DbPool,
};
use shared_types::{ChatDestination, GameId, SimpleUser};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RequestHandlerError {
    InternalError(#[from] anyhow::Error),
    AuthError(#[from] AuthError),
    /// Operation not allowed (e.g. recipient has blocked the sender for DMs). Use 403, not 401.
    Forbidden(String),
}

impl std::fmt::Display for RequestHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::InternalError(e) => write!(f, "{e}"),
            RequestHandlerError::AuthError(e) => write!(f, "{e}"),
            RequestHandlerError::Forbidden(msg) => write!(f, "{msg}"),
        }
    }
}
pub struct RequestHandler {
    command: ClientRequest,
    data: Arc<WebsocketData>,
    received_from: actix::Recipient<WsMessage>, // This is the socket the message was received over
    pool: DbPool,
    user_id: Uuid,
    username: String,
    authed: bool,
    admin: bool,
}
type Result<T> = std::result::Result<T, RequestHandlerError>;
impl RequestHandler {
    pub fn new(
        command: ClientRequest,
        data: Arc<WebsocketData>,
        sender_addr: actix::Recipient<WsMessage>,
        user: SimpleUser,
        pool: DbPool,
    ) -> Self {
        Self {
            received_from: sender_addr,
            command,
            data,
            pool,
            user_id: user.user_id,
            username: user.username,
            authed: user.authed,
            admin: user.admin,
        }
    }

    fn ensure_auth(&self) -> Result<()> {
        if !self.authed {
            Err(AuthError::Unauthorized)?
        }
        Ok(())
    }

    fn ensure_admin(&self) -> Result<()> {
        if !self.admin {
            Err(AuthError::Unauthorized)?
        }
        Ok(())
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.command.clone() {
            ClientRequest::LinkDiscord => OauthHandler::new(self.user_id).handle().await?,
            ClientRequest::Chat(message_container) => {
                self.ensure_auth()?;
                if self.user_id != message_container.message.user_id {
                    Err(AuthError::Unauthorized)?
                }
                if message_container.destination == ChatDestination::Global {
                    self.ensure_admin()?;
                }
                if let ChatDestination::TournamentLobby(tournament_id) = &message_container.destination
                {
                    let pool = self.pool.clone();
                    let user_id = self.user_id;
                    let nanoid = tournament_id.0.clone();
                    let check = async move {
                        let mut conn = get_conn(&pool).await.map_err(|_| AuthError::Unauthorized)?;
                        let tournament = Tournament::from_nanoid(&nanoid, &mut conn)
                            .await
                            .map_err(|_| AuthError::Unauthorized)?;
                        let is_player = tournament
                            .players(&mut conn)
                            .await
                            .map(|p| p.iter().any(|u| u.id == user_id))
                            .unwrap_or(false);
                        let is_organizer = tournament
                            .organizers(&mut conn)
                            .await
                            .map(|o| o.iter().any(|u| u.id == user_id))
                            .unwrap_or(false);
                        if !is_player && !is_organizer {
                            Err(AuthError::Unauthorized)
                        } else {
                            Ok(())
                        }
                    };
                    check.await?;
                }
                if let ChatDestination::GamePlayers(game_id, ..) = &message_container.destination {
                    let pool = self.pool.clone();
                    let user_id = self.user_id;
                    let nanoid = game_id.0.clone();
                    let check = async move {
                        let mut conn = get_conn(&pool).await.map_err(|_| AuthError::Unauthorized)?;
                        let game = Game::find_by_game_id(&GameId(nanoid), &mut conn)
                        .await
                        .map_err(|_| AuthError::Unauthorized)?;
                        if user_id != game.white_id && user_id != game.black_id {
                            Err(AuthError::Unauthorized)
                        } else {
                            Ok(())
                        }
                    };
                    check.await?;
                }
                if let ChatDestination::GameSpectators(game_id, ..) = &message_container.destination {
                    let pool = self.pool.clone();
                    let user_id = self.user_id;
                    let nanoid = game_id.0.clone();
                    let check = async move {
                        let mut conn = get_conn(&pool).await.map_err(|_| AuthError::Unauthorized)?;
                        let game = Game::find_by_game_id(&GameId(nanoid), &mut conn)
                            .await
                            .map_err(|_| AuthError::Unauthorized)?;
                        if user_id == game.white_id || user_id == game.black_id {
                            Err(AuthError::Unauthorized)
                        } else {
                            Ok(())
                        }
                    };
                    check.await?;
                }
                // Recipient has blocked sender: do not deliver or persist (DM only). Return 403 so client shows message, not redirect to login.
                if let ChatDestination::User((recipient_id, _)) = &message_container.destination {
                    let blocked = {
                        let mut conn = get_conn(&self.pool).await.map_err(|_| AuthError::Unauthorized)?;
                        is_blocked(&mut conn, *recipient_id, self.user_id)
                            .await
                            .map_err(|_| AuthError::Unauthorized)?
                    };
                    if blocked {
                        return Err(RequestHandlerError::Forbidden(
                            "You cannot send messages to this user".to_string(),
                        ));
                    }
                }
                ChatHandler::new(message_container, self.data.clone(), self.pool.clone()).handle()
            }
            ClientRequest::Tournament(tournament_action) => {
                TournamentHandler::new(
                    tournament_action,
                    &self.username,
                    self.user_id,
                    self.data.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Pong(nonce) => {
                self.data.pings.update(self.user_id, nonce);
                vec![]
            }
            ClientRequest::Game {
                action: game_action,
                game_id,
            } => {
                match game_action {
                    GameAction::Turn(_) | GameAction::Control(_) => self.ensure_auth()?,
                    _ => {}
                };
                GameActionHandler::new(
                    &game_id,
                    game_action,
                    self.received_from.clone(),
                    (&self.username, self.user_id),
                    self.data.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Challenge(challenge_action) => {
                self.ensure_auth()?;
                ChallengeHandler::new(challenge_action, &self.username, self.user_id, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
            ClientRequest::Away => UserStatusHandler::new().await?.handle().await?,
            ClientRequest::Schedule(action) => {
                match action {
                    crate::common::ScheduleAction::TournamentPublic(_) => {}
                    _ => self.ensure_auth()?,
                }
                ScheduleHandler::new(self.user_id, action, &self.pool)
                    .await?
                    .handle()
                    .await?
            }
        };
        Ok(messages)
    }
}
