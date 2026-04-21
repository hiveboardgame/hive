use super::{
    challenge::handler::handle_challenge,
    chat::handle::handle_chat,
    game::handle_game,
    oauth::handle::handle_oauth,
    ping::handle::handle_ping,
    schedule::handler::handle_schedule,
    tournament::handler::handle_tournament,
    user_status::handle::handle_user_status,
};
use crate::{
    common::{ServerMessage::*, ServerResult},
    providers::chat::Chat,
};
use leptos::{logging::log, prelude::use_context};
use leptos_router::hooks::use_navigate;
use shared_types::ConversationKey;

fn parse_chat_error_key(field: &str) -> Option<ConversationKey> {
    ConversationKey::from_error_field(field)
}

pub fn handle_response(m: ServerResult) {
    match m {
        ServerResult::Ok(message) => match *message {
            Ping { value, nonce } => handle_ping(nonce, value),
            UserStatus(user_update) => handle_user_status(user_update),
            Game(game_update) => handle_game(*game_update),
            Join(_uuid) => {
                //TODO: Do we do want here
            }
            Challenge(challenge) => handle_challenge(challenge),
            Chat(message) => handle_chat(message),
            RedirectLink(link) => handle_oauth(link),
            Tournament(tournament_update) => handle_tournament(tournament_update),
            Schedule(schedule_update) => handle_schedule(schedule_update),
            todo => {
                log!("Got {todo:?} which is currently still unimplemented");
            }
        },
        ServerResult::Err(e) => {
            if e.status_code == http::StatusCode::UNAUTHORIZED {
                let navigate = use_navigate();
                navigate("/login", Default::default());
            } else if e.field == "chat" || e.field.starts_with("chat:") {
                if let Some(chat) = use_context::<Chat>() {
                    chat.handle_failed_chat_send(parse_chat_error_key(&e.field), e.reason.clone());
                }
            }
            log!("Got error from server: {e}");
        }
    };
}
