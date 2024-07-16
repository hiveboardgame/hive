use crate::{
    common::ScheduleUpdate::{self, *},
    providers::schedules::SchedulesContext,
    responses::ScheduleResponse,
};
use leptos::{expect_context, SignalSet, SignalUpdate};

pub fn handle_schedule(schedule_update: ScheduleUpdate) {
    let ctx = expect_context::<SchedulesContext>();

    match schedule_update {
        Accepted(response) => {
            ctx.tournament.update(|v: &mut Vec<ScheduleResponse>| {
                v.iter_mut().for_each(|k| {
                    if k.game_id == response.game_id {
                        k.start_t = response.start_t;
                        k.agreed = true;
                    }
                });
            });
            ctx.own.update(|v| {
                if let Some(v) = v.get_mut(&response.game_id) {
                    v.iter_mut().for_each(|s| s.agreed = response.id == s.id)
                }
            });
        }
        Proposed(response) => {
            ctx.own.update(|v| {
                if let Some(schedules) = v.get_mut(&response.game_id) {
                    schedules.retain(|s| s.id != response.id);
                    schedules.push(response);
                }
            });
        }
        Deleted(response) => {
            ctx.tournament.update(|v: &mut Vec<ScheduleResponse>| {
                v.iter_mut().for_each(|k| {
                    if k.game_id == response.game_id {
                        k.agreed = false;
                    }
                });
            });
            ctx.own.update(|v| {
                if let Some(s) = v.get_mut(&response.game_id) {
                    s.retain(|s| s.id != response.id)
                }
            });
        }
        TournamentSchedules(schedules) => ctx.tournament.set(schedules),
        OwnTournamentSchedules(schedules) => ctx.own.set(schedules),
    }
}
