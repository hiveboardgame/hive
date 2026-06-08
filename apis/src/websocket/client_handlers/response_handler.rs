use super::{
    challenge::handler::{handle_challenge, handle_challenge_snapshot},
    chat::handle::handle_chat,
    game::{handle_game, handle_tv_snapshot, handle_urgent_games_snapshot},
    oauth::handle::handle_oauth,
    ping::handle::handle_ping,
    schedule::handler::{handle_schedule, handle_schedule_notification_snapshot},
    tournament::handler::{handle_tournament, handle_tournament_invitation_snapshot},
    user_status::handle::{handle_user_status, handle_user_status_snapshot},
};
use crate::common::{LobbySnapshot as LobbySnapshotPayload, ServerMessage::*, ServerResult};
use leptos::logging::log;
use leptos_router::hooks::use_navigate;

fn handle_lobby_snapshot(snapshot: LobbySnapshotPayload) {
    handle_tournament_invitation_snapshot(snapshot.tournament_invitations);
    handle_schedule_notification_snapshot(snapshot.schedule_notifications);
    handle_urgent_games_snapshot(snapshot.urgent_games);
    handle_challenge_snapshot(snapshot.challenges);
    handle_tv_snapshot(snapshot.tv_games);
    handle_user_status_snapshot(snapshot.online_users);
}

pub fn handle_response(m: ServerResult) {
    match m {
        ServerResult::Ok(message) => match *message {
            Ping { value, nonce } => handle_ping(nonce, value),
            UserStatus(user_update) => handle_user_status(user_update),
            LobbySnapshot(snapshot) => handle_lobby_snapshot(*snapshot),
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
                let navegate = use_navigate();
                navegate("/login", Default::default());
            }
            log!("Got error from server: {e}");
        }
    };
}
