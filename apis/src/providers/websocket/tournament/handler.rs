use crate::{
    common::TournamentUpdate,
    providers::{tournaments::TournamentStateSignal, NotificationContext},
};
use leptos::logging::log;
use leptos::*;
use shared_types::ApisId;

pub fn handle_tournament(tournament: TournamentUpdate) {
    match tournament {
        TournamentUpdate::Left(tournament)
        | TournamentUpdate::Created(tournament)
        | TournamentUpdate::Joined(tournament)
        | TournamentUpdate::Modified(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(vec![tournament]);
        }
        TournamentUpdate::Invited(tournament) => {
            let mut notifications = expect_context::<NotificationContext>();
            log!("Got invited");
            notifications.add(vec![ApisId::Tournament(
                tournament.tournament_id.clone(),
            )])
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
