use crate::{
    common::TournamentUpdate,
    providers::{tournaments::TournamentStateSignal, NotificationContext},
};
use leptos::*;
use shared_types::ApisId;

pub fn handle_tournament(tournament: TournamentUpdate) {
    match tournament {
        TournamentUpdate::Left(tournament)
        | TournamentUpdate::Created(tournament)
        | TournamentUpdate::Modified(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(vec![tournament]);
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
            tournaments_signal.add(vec![tournament.clone()]);
            let mut notifications = expect_context::<NotificationContext>();
            notifications.add(vec![ApisId::Tournament(tournament.tournament_id.clone())])
        }
        TournamentUpdate::Uninvited(tournament) => {
            let mut notifications = expect_context::<NotificationContext>();
            notifications.remove(&ApisId::Tournament(tournament.tournament_id.clone()))
        }
        TournamentUpdate::Tournaments(tournaments) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(tournaments);
        }
        TournamentUpdate::Deleted(nanoid) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.remove(nanoid);
        }
    }
}
