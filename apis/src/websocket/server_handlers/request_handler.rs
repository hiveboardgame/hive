use super::challenges::handler::ChallengeHandler;
use super::chat::handler::ChatHandler;
use super::game::handler::GameActionHandler;
use super::oauth::handler::OauthHandler;
use super::pong::handler::PongHandler;
use super::schedules::ScheduleHandler;
use super::tournaments::handler::TournamentHandler;
use super::user_status::handler::UserStatusHandler;
use crate::common::{ClientRequest, GameAction};
use crate::websocket::chat::Chats;
use crate::websocket::lag_tracking::{Lags, Pings};
use crate::websocket::messages::AuthError;
use crate::websocket::messages::InternalServerMessage;
use crate::websocket::messages::WsMessage;
use crate::websocket::tournament_game_start::TournamentGameStart;
use db_lib::DbPool;
use shared_types::{ChatDestination, SimpleUser};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RequestHandlerError {
    InternalError(#[from] anyhow::Error),
    AuthError(#[from] AuthError),
}

impl std::fmt::Display for RequestHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::InternalError(e) => write!(f,"{}", e),
            RequestHandlerError::AuthError(e) => write!(f, "{}", e),
        }
    }
}
pub struct RequestHandler {
    command: ClientRequest,
    chat_storage: actix_web::web::Data<Chats>,
    game_start: actix_web::web::Data<TournamentGameStart>,
    lags: actix_web::web::Data<Lags>,
    pings: actix_web::web::Data<Pings>,
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
        chat_storage: actix_web::web::Data<Chats>,
        game_start: actix_web::web::Data<TournamentGameStart>,
        pings: actix_web::web::Data<Pings>,
        lags: actix_web::web::Data<Lags>,
        sender_addr: actix::Recipient<WsMessage>,
        user: SimpleUser,
        pool: DbPool,
    ) -> Self {
        Self {
            received_from: sender_addr,
            command,
            chat_storage,
            game_start,
            pings,
            lags,
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
                ChatHandler::new(message_container, self.chat_storage.clone()).handle()
            }
            ClientRequest::Tournament(tournament_action) => {
                TournamentHandler::new(
                    tournament_action,
                    &self.username,
                    self.user_id,
                    self.chat_storage.clone(),
                    &self.pool,
                )
                .await?
                .handle()
                .await?
            }
            ClientRequest::Pong(nonce) => {
                PongHandler::new(self.user_id, nonce, self.pings.clone()).handle();
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
                    (&self.username, self.user_id),
                    self.received_from.clone(),
                    self.chat_storage.clone(),
                    self.game_start.clone(),
                    self.pings.clone(),
                    self.lags.clone(),
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
