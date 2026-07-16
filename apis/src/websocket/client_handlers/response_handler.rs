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
use crate::{
    common::{
        ExternalServerError,
        LobbySnapshot as LobbySnapshotPayload,
        ServerMessage::*,
        ServerResult,
        SubscriptionAttempt,
        UserSettingsUpdate,
    },
    providers::chat::Chat,
};
use leptos::{logging::log, prelude::use_context};
use leptos_router::hooks::use_navigate;
use shared_types::ConversationKey;

fn handle_lobby_snapshot(snapshot: LobbySnapshotPayload) {
    handle_tournament_invitation_snapshot(snapshot.tournament_invitations);
    handle_schedule_notification_snapshot(snapshot.schedule_notifications);
    handle_urgent_games_snapshot(snapshot.urgent_games);
    handle_challenge_snapshot(snapshot.challenges);
    handle_tv_snapshot(snapshot.tv_games);
    handle_user_status_snapshot(snapshot.online_users);
}

fn handle_user_settings(update: UserSettingsUpdate) {
    let Some(chat) = use_context::<Chat>() else {
        return;
    };
    match update {
        UserSettingsUpdate::BlockedUser { user_id, blocked } => {
            chat.apply_blocked_user_update(user_id, blocked);
        }
        UserSettingsUpdate::TournamentChatMuted {
            tournament_id,
            muted,
        } => {
            chat.set_tournament_muted_authoritative(&tournament_id, muted);
        }
    }
}

fn handle_chat_read(key: ConversationKey, last_read_message_id: i64) {
    if let Some(chat) = use_context::<Chat>() {
        chat.apply_read_receipt_update(key, last_read_message_id);
    }
}

fn handle_chat_subscription_ready(acknowledgement: SubscriptionAttempt) {
    if let Some(chat) = use_context::<Chat>() {
        chat.confirm_subscription(acknowledgement);
    }
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
            ChatRead {
                key,
                last_read_message_id,
            } => handle_chat_read(key, last_read_message_id),
            ChatSubscribed(acknowledgement) => handle_chat_subscription_ready(acknowledgement),
            RedirectLink(link) => handle_oauth(link),
            Tournament(tournament_update) => handle_tournament(tournament_update),
            UserSettings(update) => handle_user_settings(update),
            Schedule(schedule_update) => handle_schedule(schedule_update),
            todo => {
                log!("Got {todo:?} which is currently still unimplemented");
            }
        },
        ServerResult::Err(e) => {
            log!("Got error from server: {e}");
            match e {
                ExternalServerError::Unauthorized { .. } => {
                    let navigate = use_navigate();
                    navigate("/login", Default::default());
                }
                ExternalServerError::ChatSubscribe { attempt, error } => {
                    if let Some(chat) = use_context::<Chat>() {
                        chat.fail_subscription(attempt, error);
                    }
                }
                ExternalServerError::ChatSend {
                    key,
                    client_id,
                    error,
                } => {
                    if let Some(chat) = use_context::<Chat>() {
                        chat.handle_failed_chat_send(key, client_id, error.into());
                    }
                }
                ExternalServerError::Request { .. } => {}
            }
        }
    };
}
