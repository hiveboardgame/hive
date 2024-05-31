use crate::{common::TournamentUpdate, providers::tournaments::TournamentStateSignal};
use leptos::*;

pub fn handle_tournament(tournament: TournamentUpdate) {
    match tournament {
        TournamentUpdate::Created(tournament) | TournamentUpdate::Joined(tournament) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(vec![tournament]);
        }
        TournamentUpdate::Tournaments(tournaments) => {
            let mut tournaments_signal = expect_context::<TournamentStateSignal>();
            tournaments_signal.add(tournaments);
        }
        _ => unimplemented!(),
    }
}
