use std::sync::Arc;

use super::chat::handler::ChatHandler;
use super::user_status::handler::UserStatusHandler;
use crate::common::ClientRequest;
use crate::websocket::messages::AuthError;
use crate::websocket::messages::InternalServerMessage;
use crate::websocket::WebsocketData;
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
            RequestHandlerError::InternalError(e) => write!(f, "{e}"),
            RequestHandlerError::AuthError(e) => write!(f, "{e}"),
        }
    }
}
pub struct RequestHandler {
    command: ClientRequest,
    data: Arc<WebsocketData>,
    user_id: Uuid,
    authed: bool,
    admin: bool,
}
type Result<T> = std::result::Result<T, RequestHandlerError>;
impl RequestHandler {
    pub fn new(command: ClientRequest, data: Arc<WebsocketData>, user: SimpleUser) -> Self {
        Self {
            command,
            data,
            user_id: user.user_id,
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
           
            ClientRequest::Chat(message_container) => {
                self.ensure_auth()?;
                if self.user_id != message_container.message.user_id {
                    Err(AuthError::Unauthorized)?
                }
                if message_container.destination == ChatDestination::Global {
                    self.ensure_admin()?;
                }
                ChatHandler::new(message_container, self.data.clone()).handle()
            }
            
            ClientRequest::Away => UserStatusHandler::new().await?.handle().await?,
            ClientRequest::Pong(_)
            | ClientRequest::Game { .. }
            | ClientRequest::Challenge(_)
            | ClientRequest::Schedule(_)
            | ClientRequest::LinkDiscord
            | ClientRequest::Tournament(_) => {
                //Handled in v2
                vec![]
            }
        };
        Ok(messages)
    }
}
