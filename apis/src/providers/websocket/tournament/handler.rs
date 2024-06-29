use crate::{
    common::TournamentUpdate,
    providers::{
        navigation_controller::NavigationControllerSignal, tournaments::TournamentStateSignal,
        NotificationContext,
    },
};
use leptos::*;
use leptos_router::use_navigate;
use shared_types::ApisId;

pub fn handle_tournament(tournament: TournamentUpdate) {
    match tournament {
        TournamentUpdate::Left(tournament)
        | TournamentUpdate::Created(tournament)
        | TournamentUpdate::Modified(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(vec![*tournament]);
        }
        TournamentUpdate::Declined(tournament) => {
            let mut notifications = expect_context::<NotificationContext>();
            notifications.remove(&ApisId::Tournament(tournament.tournament_id.clone()))
        }
        TournamentUpdate::Joined(tournament) => {
            let mut notifications = expect_context::<NotificationContext>();
            notifications.remove(&ApisId::Tournament(tournament.tournament_id.clone()))
        }
        TournamentUpdate::Invited(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(vec![*tournament.clone()]);
            let mut notifications = expect_context::<NotificationContext>();
            notifications.add(vec![ApisId::Tournament(tournament.tournament_id.clone())])
        }
        TournamentUpdate::Uninvited(tournament) => {
            let mut notifications = expect_context::<NotificationContext>();
            notifications.remove(&ApisId::Tournament(tournament.tournament_id.clone()))
        }
        TournamentUpdate::Tournaments(tournaments) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            let t = tournaments.into_iter().map(|t| *t).collect();
            tournaments_signal.add(t);
        }
        TournamentUpdate::Deleted(nanoid) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            let mut notifications = expect_context::<NotificationContext>();
            let navi = expect_context::<NavigationControllerSignal>();
            notifications.remove(&ApisId::Tournament(nanoid.clone()));
            if let Some(torunament_id) = navi.tournament_signal.get().tournament_id {
                if torunament_id == nanoid {
                    let navigate = use_navigate();
                    navigate("/", Default::default());
                }
            }
            tournaments_signal.remove(nanoid);
        }
        TournamentUpdate::Started(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(vec![*tournament.clone()]);
            // TODO: Inform users the tournament started
        }
    }
}
