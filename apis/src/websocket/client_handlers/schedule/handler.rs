use std::collections::HashMap;

use crate::{
    common::ScheduleUpdate::{self, *},
    providers::schedules::SchedulesContext,
};
use leptos::{expect_context, SignalSet, SignalUpdate};

pub fn handle_schedule(schedule_update: ScheduleUpdate) {
    let ctx = expect_context::<SchedulesContext>();

    match schedule_update {
        Accepted(response) => {
            ctx.tournament.update(|h| {
                if let Some(schedules) = h.get_mut(&response.game_id) {
                    for (_, schedule) in schedules.iter_mut() {
                        schedule.agreed = false;
                    }
                    schedules.insert(response.id, response.clone());
                } else {
                    let mut new_schedules = HashMap::new();
                    new_schedules.insert(response.id, response.clone());
                    h.insert(response.game_id.clone(), new_schedules);
                }
            });
            ctx.own.update(|h| {
                if let Some(schedules) = h.get_mut(&response.game_id) {
                    for (_, schedule) in schedules.iter_mut() {
                        schedule.agreed = false;
                    }
                    schedules.insert(response.id, response);
                }
            });
        }
        Proposed(response) => {
            ctx.own.update(|h| {
                if let Some(schedules) = h.get_mut(&response.game_id) {
                    schedules.insert(response.id, response);
                } else {
                    let mut new_schedules = HashMap::new();
                    new_schedules.insert(response.id, response.clone());
                    h.insert(response.game_id, new_schedules);
                }
            });
        }
        Deleted(response) => {
            ctx.tournament.update(|h| {
                if let Some(schedules) = h.get_mut(&response.game_id) {
                    schedules.remove(&response.id);
                }
            });
            ctx.own.update(|h| {
                if let Some(schedules) = h.get_mut(&response.game_id) {
                    schedules.remove(&response.id);
                }
            });
        }
        TournamentSchedules(schedules) => ctx.tournament.set(schedules),
        OwnTournamentSchedules(schedules) => ctx.own.set(schedules),
    }
}
