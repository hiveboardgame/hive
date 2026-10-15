use super::super::schedule::handler::purge_tournament_schedules;
use crate::{
    common::TournamentUpdate,
    providers::{chat::Chat, ActiveTournamentState, AlertType, AlertsContext, NotificationContext},
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

fn deleted_tournament_redirect(
    current_path: &str,
    tournament_id: &TournamentId,
) -> Option<&'static str> {
    let tournament_path = format!("/tournament/{tournament_id}");
    let message_path = format!("/message/tournament/{tournament_id}");
    if path_is_or_descendant(current_path, &message_path) {
        Some("/message")
    } else if path_is_or_descendant(current_path, &tournament_path) {
        Some("/tournaments/")
    } else {
        None
    }
}

fn is_tournament_creation_path(current_path: &str) -> bool {
    path_is_or_descendant(current_path, "/tournaments/create")
}

pub fn handle_tournament(tournament: TournamentUpdate) {
    let notifications = expect_context::<NotificationContext>();
    let chat = expect_context::<Chat>();

    match tournament {
        TournamentUpdate::Created { .. }
        | TournamentUpdate::CreationFailed
        | TournamentUpdate::InvitationFailed { .. } => {}
        TournamentUpdate::Left {
            tournament_id,
            retained_chat_access,
        } => {
            if !retained_chat_access {
                chat.clear_tournament_thread(&tournament_id);
            }
            chat.refresh_inbox_and_catalog();
            let location = use_location();
            let current_path = location.pathname.get_untracked();
            let message_path = format!("/message/tournament/{tournament_id}");
            if !retained_chat_access && path_is_or_descendant(&current_path, &message_path) {
                use_navigate()("/message", Default::default());
            }
        }
        TournamentUpdate::Patch {
            tournament_id,
            patch,
        } => {
            expect_context::<ActiveTournamentState>().apply_patch(&tournament_id, *patch);
        }
        TournamentUpdate::SlotsClosed {
            tournament_id: _,
            count,
        } => {
            expect_context::<AlertsContext>()
                .last_alert
                .set(Some(AlertType::Notification(
                    // TODO: i18n once copy is approved.
                    if count == 1 {
                        String::from("Recorded a double forfeit.")
                    } else {
                        format!("Recorded {count} double forfeits.")
                    },
                )));
        }
        TournamentUpdate::CreationSucceeded(tournament_id) => {
            let location = use_location();
            if is_tournament_creation_path(&location.pathname.get_untracked()) {
                use_navigate()(&format!("/tournament/{tournament_id}"), Default::default());
            }
        }
        TournamentUpdate::Declined(tournament_id) | TournamentUpdate::Uninvited(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
        }
        TournamentUpdate::Joined(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
            chat.refresh_inbox_and_catalog();
        }
        TournamentUpdate::OrganizerInvited(tournament_id) => {
            notifications.organizer_invitation_insert(tournament_id);
        }
        TournamentUpdate::OrganizerUninvited(tournament_id)
        | TournamentUpdate::OrganizerInvitationsClosed(tournament_id) => {
            notifications.organizer_invitation_remove(&tournament_id);
        }
        TournamentUpdate::OrganizerJoined(tournament_id) => {
            notifications.organizer_invitation_remove(&tournament_id);
            chat.refresh_inbox_and_catalog();
        }
        TournamentUpdate::OrganizerLeft(tournament_id, still_member) => {
            if !still_member {
                chat.clear_tournament_thread(&tournament_id);
                let location = use_location();
                let message_path = format!("/message/tournament/{tournament_id}");
                if path_is_or_descendant(&location.pathname.get_untracked(), &message_path) {
                    use_navigate()("/message", Default::default());
                }
            }
            chat.refresh_inbox_and_catalog();
        }
        TournamentUpdate::Invited(tournament_id) => {
            notifications.tournament_invitation_insert(tournament_id);
        }
        TournamentUpdate::Deleted(t_id) => {
            notifications.tournament_pairings.update(|pairings| {
                pairings.retain(|_, notice| notice.tournament_id != t_id);
            });
            purge_tournament_schedules(&t_id);
            notifications.tournament_invitation_remove(&t_id);
            notifications.organizer_invitation_remove(&t_id);
            chat.clear_tournament_thread(&t_id);
            chat.request_catalog_refresh();
            let location = use_location();
            let current_path = location.pathname.get_untracked();
            if let Some(destination) = deleted_tournament_redirect(&current_path, &t_id) {
                use_navigate()(destination, Default::default());
            }
        }
        TournamentUpdate::StartDeferred(_) => {
            // TODO: i18n once copy is approved.
            expect_context::<AlertsContext>().last_alert.set(Some(AlertType::Notification(
                String::from("Not enough entrants to start. The tournament now uses manual start; reschedule it or start it later."),
            )));
        }
        TournamentUpdate::Started(tournament_id) => {
            notifications.tournament_invitation_remove(&tournament_id);
            notifications.tournament_started.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
        TournamentUpdate::Finished(tournament_id) => {
            notifications.tournament_pairings.update(|pairings| {
                pairings.retain(|_, notice| notice.tournament_id != tournament_id);
            });
            notifications.organizer_invitation_remove(&tournament_id);
            purge_tournament_schedules(&tournament_id);
            notifications.tournament_finished.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{deleted_tournament_redirect, is_tournament_creation_path};
    use shared_types::TournamentId;

    #[test]
    fn authoritative_delete_event_redirects_only_affected_tournament_routes() {
        let deleted = TournamentId(String::from("deleted"));

        assert_eq!(
            deleted_tournament_redirect("/tournament/deleted", &deleted),
            Some("/tournaments/")
        );
        assert_eq!(
            deleted_tournament_redirect("/tournament/deleted/games", &deleted),
            Some("/tournaments/")
        );
        assert_eq!(
            deleted_tournament_redirect("/message/tournament/deleted", &deleted),
            Some("/message")
        );
        assert_eq!(
            deleted_tournament_redirect("/tournament/other", &deleted),
            None
        );
        assert_eq!(deleted_tournament_redirect("/tournaments/", &deleted), None);
    }

    #[test]
    fn creation_acknowledgements_redirect_only_from_creation_routes() {
        assert!(is_tournament_creation_path("/tournaments/create"));
        assert!(is_tournament_creation_path("/tournaments/create/arena"));
        assert!(is_tournament_creation_path("/tournaments/create/swiss"));
        assert!(!is_tournament_creation_path("/tournaments"));
        assert!(!is_tournament_creation_path("/tournaments/created"));
    }
}
