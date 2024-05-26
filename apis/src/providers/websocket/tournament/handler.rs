use crate::{common::TournamentUpdate, providers::tournaments::TournamentStateSignal};
use leptos::*;

pub fn handle_tournament(tournament: TournamentUpdate) {
    match tournament {
        TournamentUpdate::Created(tournament) | TournamentUpdate::Joined(tournament) => {
            let mut tournaments = expect_context::<TournamentStateSignal>();
            tournaments.add(vec![tournament]);
        }
        _ => unimplemented!(),
    }
}
