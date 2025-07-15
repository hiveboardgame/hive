use std::collections::HashMap;

use crate::{
    common::ScheduleUpdate::{self, *},
    providers::{schedules::SchedulesContext, AuthContext, NotificationContext},
};
use leptos::prelude::{expect_context, Get, Set, Update};

pub fn handle_schedule(schedule_update: ScheduleUpdate) {
    let ctx = expect_context::<SchedulesContext>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();

    match schedule_update {
        Accepted(response) => {
            notifications.schedule_proposals.update(|proposals| {
                proposals.remove(&response.id);
            });

            // Add acceptance notification for the proposer
            if let Some(user) = auth_context.user.get() {
                if response.proposer_id == user.id {
                    notifications.schedule_acceptances.update(|acceptances| {
                        acceptances.insert(response.id);
                    });
                }
            }

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
            if let Some(user) = auth_context.user.get() {
                if response.opponent_id == user.id {
                    notifications.schedule_proposals.update(|proposals| {
                        proposals.insert(response.id);
                    });
                }
            }

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
            notifications.schedule_proposals.update(|proposals| {
                proposals.remove(&response.id);
            });
            notifications.schedule_acceptances.update(|proposals| {
                proposals.remove(&response.id);
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
