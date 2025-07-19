use crate::{
    common::ScheduleUpdate::{self, *},
    providers::{schedules::SchedulesContext, AuthContext, NotificationContext},
    responses::ScheduleResponse,
};
use leptos::prelude::{expect_context, Get, RwSignal, Set, Update};
use shared_types::GameId;
use std::collections::HashMap;
use uuid::Uuid;

pub fn handle_schedule(schedule_update: ScheduleUpdate) {
    let ctx = expect_context::<SchedulesContext>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();

    let update_schedules =
        |signal: RwSignal<_>, response: &ScheduleResponse, reset_agreed: bool| {
            signal.update(|h: &mut HashMap<GameId, HashMap<Uuid, ScheduleResponse>>| {
                let schedules = h.entry(response.game_id.clone()).or_default();
                if reset_agreed {
                    for (_, schedule) in schedules.iter_mut() {
                        schedule.agreed = false;
                    }
                }
                schedules.insert(response.id, response.clone());
            });
        };

    match schedule_update {
        Accepted(response) => {
            notifications.schedule_proposals.update(|p| {
                p.remove(&response.id);
            });

            if let Some(user) = auth_context.user.get() {
                if response.proposer_id == user.id {
                    notifications.schedule_acceptances.update(|a| {
                        a.insert(response.id);
                    });
                }
            }

            update_schedules(ctx.tournament, &response, true);
            update_schedules(ctx.own, &response, true);
        }

        Proposed(response) => {
            if let Some(user) = auth_context.user.get() {
                if response.opponent_id == user.id {
                    notifications.schedule_proposals.update(|p| {
                        p.insert(response.id);
                    });
                }
            }

            update_schedules(ctx.own, &response, false);
        }

        Deleted(response) => {
            notifications.schedule_proposals.update(|p| {
                p.remove(&response.id);
            });
            notifications.schedule_acceptances.update(|a| {
                a.remove(&response.id);
            });

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
