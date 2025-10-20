use super::{
    chat::handle::handle_chat, oauth::handle::handle_oauth, schedule::handler::handle_schedule,
    tournament::handler::handle_tournament,
};
use crate::common::{ServerMessage::*, ServerResult};
use leptos::logging::log;
use leptos_router::hooks::use_navigate;

pub fn handle_response(m: ServerResult) {
    match m {
        ServerResult::Ok(message) => match *message {
            Chat(message) => handle_chat(message),
            RedirectLink(link) => handle_oauth(link),
            Tournament(tournament_update) => handle_tournament(tournament_update),
            Schedule(schedule_update) => handle_schedule(schedule_update),
            Game(_) | UserStatus(_) | Ping { .. } | Challenge(_) => {
                //Handled in v2
            }
            Error(err) => {
                log!("Got {err} from server");
            }
        },
        ServerResult::Err(e) => {
            if e.status_code == http::StatusCode::UNAUTHORIZED {
                let navegate = use_navigate();
                navegate("/login", Default::default());
            }
            log!("Got error from server: {e}");
        }
    };
}
