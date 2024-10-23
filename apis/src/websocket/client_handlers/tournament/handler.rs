use crate::{
    common::TournamentUpdate,
    providers::{
        navigation_controller::NavigationControllerSignal, tournaments::TournamentStateContext,
        NotificationContext,
    },
};
use leptos::*;
use leptos_router::use_navigate;

pub fn handle_tournament(tournament: TournamentUpdate) {
    match tournament {
        TournamentUpdate::Left(tournament)
        | TournamentUpdate::Created(tournament)
        | TournamentUpdate::Modified(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateContext>();
            tournaments_signal.add_full(vec![*tournament]);
        }
        TournamentUpdate::Declined(tournament) => {
            let notifications = expect_context::<NotificationContext>();
            notifications.tournament_invitations.update(|invitations| {
                invitations.remove(&tournament.tournament_id);
            });
        }
        TournamentUpdate::Joined(tournament) => {
            let notifications = expect_context::<NotificationContext>();
            notifications.tournament_invitations.update(|invitations| {
                invitations.remove(&tournament.tournament_id);
            });
        }
        TournamentUpdate::Invited(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateContext>();
            tournaments_signal.add_full(vec![*tournament.clone()]);
            let notifications = expect_context::<NotificationContext>();
            notifications.tournament_invitations.update(|invitations| {
                invitations.insert(tournament.tournament_id.clone());
            });
        }
        TournamentUpdate::Uninvited(tournament) => {
            let notifications = expect_context::<NotificationContext>();
            notifications.tournament_invitations.update(|invitations| {
                invitations.remove(&tournament.tournament_id);
            });
        }
        TournamentUpdate::Tournaments(tournaments) => {
            let mut tournaments_signal = expect_context::<TournamentStateContext>();
            let t = tournaments.into_iter().map(|t| *t).collect();
            tournaments_signal.add_full(t);
        }
        TournamentUpdate::AbstractTournaments(tournaments) => {
            let mut tournaments_signal = expect_context::<TournamentStateContext>();
            tournaments_signal.add_abstract(tournaments);
        }
        TournamentUpdate::Deleted(t_id) => {
            let mut tournaments_signal = expect_context::<TournamentStateContext>();
            let notifications = expect_context::<NotificationContext>();
            let navi = expect_context::<NavigationControllerSignal>();
            notifications.tournament_invitations.update(|t| {
                t.remove(&t_id);
            });
            if let Some(torunament_id) = navi.tournament_signal.get().tournament_id {
                if torunament_id == t_id {
                    let navigate = use_navigate();
                    navigate("/", Default::default());
                }
            }
            tournaments_signal.remove(t_id);
        }
        TournamentUpdate::Started(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateContext>();
            tournaments_signal.add_full(vec![*tournament.clone()]);
            let notifications = expect_context::<NotificationContext>();
            notifications.tournament_invitations.update(|invitations| {
                invitations.remove(&tournament.tournament_id);
            });
            notifications.tournament_started.update(|tournaments| {
                tournaments.insert(tournament.tournament_id.clone());
            });
            // TODO: Inform users the tournament started
        }
    }
}
