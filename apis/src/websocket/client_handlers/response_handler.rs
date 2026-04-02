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
use crate::common::{ServerMessage::*, ServerResult};
use crate::providers::{AlertType, AlertsContext};
use leptos::prelude::{use_context, Update};
use leptos::logging::log;
use leptos_router::hooks::use_navigate;

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
            } else {
                // 403 Forbidden (e.g. blocked from DM), 5xx, etc.: show message, do not redirect to login
                let message = if e.reason.is_empty() {
                    format!("{}", e.status_code)
                } else {
                    e.reason.clone()
                };
                if let Some(alerts) = use_context::<AlertsContext>() {
                    alerts.last_alert.update(|v| {
                        *v = Some(AlertType::Error(message));
                    });
                }
            }
            log!("Got error from server: {e}");
        }
    };
}
