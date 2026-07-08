use crate::{
    common::TournamentUpdate,
    providers::{chat::Chat, NotificationContext, UpdateNotifier},
};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use shared_types::TournamentId;

pub fn handle_tournament_invitation_snapshot(invitations: Vec<TournamentId>) {
    let notifications = expect_context::<NotificationContext>();
    notifications.tournament_invitations_snapshot_apply(invitations);
}

pub fn handle_tournament(tournament: TournamentUpdate) {
    let notify_update = expect_context::<UpdateNotifier>().tournament_update;
    let notifications = expect_context::<NotificationContext>();
    let chat = expect_context::<Chat>();

    //TODO: @ion when creating a tournament get navigated to it
    match tournament {
        TournamentUpdate::Left(tournament_id) | TournamentUpdate::Modified(tournament_id) => {
            notify_update.set(tournament_id.clone());
            refresh_chat_hub_if_tracking(chat, &tournament_id);
        }
        TournamentUpdate::Adjudicated(tournament_id) => {
            notify_update.set(tournament_id);
            chat.refresh_messages_hub();
        }
        TournamentUpdate::Created(tournament_id) => {
            notify_update.set(tournament_id);
        }
        TournamentUpdate::Declined(tournament_id) | TournamentUpdate::Uninvited(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
            refresh_chat_hub_if_tracking(chat, &tournament_id);
        }
        TournamentUpdate::Joined(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
            chat.refresh_messages_hub();
        }
        TournamentUpdate::Invited(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_invitation_insert(tournament_id);
        }
        TournamentUpdate::Deleted(t_id) => {
            notify_update.set(t_id.clone());
            notifications.tournament_invitation_remove(&t_id);
            refresh_chat_hub_if_tracking(chat, &t_id);
            let location = use_location();
            let current_path = location.pathname.get();
            let tournament_path = format!("/tournament/{t_id}");

            if current_path.starts_with(&tournament_path) {
                let navigate = use_navigate();
                navigate("/tournaments/", Default::default());
            }
        }
        TournamentUpdate::Started(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_invitation_remove(&tournament_id);
            refresh_chat_hub_if_tracking(chat, &tournament_id);
            notifications.tournament_started.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
        TournamentUpdate::Finished(tournament_id) => {
            notify_update.set(tournament_id.clone());
            refresh_chat_hub_if_tracking(chat, &tournament_id);
            notifications.tournament_finished.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
    }
}

fn refresh_chat_hub_if_tracking(chat: Chat, tournament_id: &TournamentId) {
    let tracking = chat.messages_hub_data.with_untracked(|hub| {
        hub.as_ref().is_some_and(|hub| {
            hub.tournaments
                .iter()
                .any(|channel| &channel.tournament_id == tournament_id)
        })
    });
    if tracking {
        chat.refresh_messages_hub();
    }
}
