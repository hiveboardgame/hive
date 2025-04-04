use crate::{
    common::TournamentUpdate,
    providers::{NotificationContext, UpdateNotifier},
};
use leptos::prelude::*;

pub fn handle_tournament(tournament: TournamentUpdate) {
    let notify_update = expect_context::<UpdateNotifier>().tournament_update;
    let notifications = expect_context::<NotificationContext>();

    match tournament {
        TournamentUpdate::Left(tournament_id)
        | TournamentUpdate::Created(tournament_id)
        | TournamentUpdate::Adjudicated(tournament_id)
        | TournamentUpdate::Modified(tournament_id) => {
            notify_update.set(tournament_id);
        }
        TournamentUpdate::Declined(tournament_id)
        | TournamentUpdate::Joined(tournament_id)
        | TournamentUpdate::Uninvited(tournament_id) => {
            notifications.tournament_invitations.update(|invitations| {
                invitations.remove(&tournament_id);
            });
        }
        TournamentUpdate::Invited(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_invitations.update(|invitations| {
                invitations.insert(tournament_id.clone());
            });
        }
        TournamentUpdate::Deleted(t_id) => {
            notify_update.set(t_id.clone());
            notifications.tournament_invitations.update(|t| {
                t.remove(&t_id);
            });
        }
        TournamentUpdate::Started(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_invitations.update(|invitations| {
                invitations.remove(&tournament_id);
            });
            notifications.tournament_started.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
            // TODO: Inform users the tournament started
        }
        TournamentUpdate::Finished(tournament_id) => {
            notify_update.set(tournament_id.clone());
            notifications.tournament_finished.update(|tournaments| {
                tournaments.insert(tournament_id.clone());
            });
        }
    }
}
