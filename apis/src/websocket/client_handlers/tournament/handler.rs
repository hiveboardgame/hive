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

fn path_is_or_descendant(current_path: &str, root: &str) -> bool {
    current_path == root
        || current_path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub fn handle_tournament(tournament: TournamentUpdate) {
    let notify_update = expect_context::<UpdateNotifier>().tournament_update;
    let notifications = expect_context::<NotificationContext>();
    let chat = expect_context::<Chat>();

    //TODO: @ion when creating a tournament get navigated to it
    match tournament {
        TournamentUpdate::Left(tournament_id) => {
            chat.clear_tournament_thread(&tournament_id);
            chat.refresh_inbox_and_catalog();
            let location = use_location();
            let current_path = location.pathname.get_untracked();
            let message_path = format!("/message/tournament/{tournament_id}");
            if path_is_or_descendant(&current_path, &message_path) {
                use_navigate()("/message", Default::default());
            }
        }
        TournamentUpdate::StateChanged(tournament_id) => {
            notify_update.set(tournament_id);
        }
        TournamentUpdate::Adjudicated(tournament_id) => {
            notify_update.set(tournament_id);
        }
        TournamentUpdate::Created(tournament_id) => {
            notify_update.set(tournament_id);
        }
        TournamentUpdate::Declined(tournament_id) | TournamentUpdate::Uninvited(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
        }
        TournamentUpdate::Joined(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
            chat.refresh_inbox_and_catalog();
        }
        TournamentUpdate::Invited(tournament_id) => {
            notifications.tournament_invitation_insert(tournament_id);
        }
        TournamentUpdate::Deleted(t_id) => {
            notify_update.set(t_id.clone());
            notifications.tournament_invitation_remove(&t_id);
            chat.clear_tournament_thread(&t_id);
            chat.request_catalog_refresh();
            let location = use_location();
            let current_path = location.pathname.get_untracked();
            let tournament_path = format!("/tournament/{t_id}");
            let message_path = format!("/message/tournament/{t_id}");

            let navigate = use_navigate();
            if path_is_or_descendant(&current_path, &message_path) {
                navigate("/message", Default::default());
            } else if path_is_or_descendant(&current_path, &tournament_path) {
                navigate("/tournaments/", Default::default());
            }
        }
        TournamentUpdate::Started(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_invitation_remove(&tournament_id);
            notifications.tournament_started.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
        TournamentUpdate::Finished(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_finished.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
    }
}
