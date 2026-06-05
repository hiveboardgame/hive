use crate::{
    common::ScheduleUpdate::{self, *},
    providers::{
        schedules::SchedulesContext,
        AuthContext,
        NotificationContext,
        ScheduleNotificationKind,
    },
    responses::ScheduleResponse,
};
use leptos::prelude::{expect_context, Get, RwSignal, Set, Update};
use shared_types::GameId;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

type ScheduleMap = HashMap<GameId, HashMap<Uuid, ScheduleResponse>>;

fn update_schedules(
    signal: RwSignal<ScheduleMap>,
    response: &ScheduleResponse,
    reset_agreed: bool,
) {
    signal.update(|schedules| {
        let game_schedules = schedules.entry(response.game_id.clone()).or_default();
        if reset_agreed {
            for schedule in game_schedules.values_mut() {
                schedule.agreed = false;
            }
        }
        game_schedules.insert(response.id, response.clone());
    });
}

fn remove_schedule(signal: RwSignal<ScheduleMap>, response: &ScheduleResponse) {
    signal.update(|h| {
        if let Some(schedules) = h.get_mut(&response.game_id) {
            schedules.remove(&response.id);
        }
    });
}

pub fn handle_schedule_notification_snapshot(snapshot: Vec<ScheduleResponse>) {
    let ctx = expect_context::<SchedulesContext>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();
    let Some(user) = auth_context.user.get() else {
        notifications.schedule_notifications_snapshot_apply(HashSet::new(), HashSet::new());
        ctx.clear_own_resync_dirty();
        return;
    };

    let mut proposal_ids = HashSet::new();
    let mut acceptance_ids = HashSet::new();
    for response in snapshot {
        if response.opponent_id == user.id {
            if !ctx.is_own_schedule_resync_dirty(response.id) {
                proposal_ids.insert(response.id);
                update_schedules(ctx.own, &response, false);
            }
        } else if response.proposer_id == user.id
            && !ctx.is_own_acceptance_snapshot_dirty(&response)
        {
            acceptance_ids.insert(response.id);
            update_schedules(ctx.own, &response, true);
        }
    }

    notifications.schedule_notifications_snapshot_apply(proposal_ids, acceptance_ids);
    ctx.clear_own_resync_dirty();
}

pub fn handle_schedule(schedule_update: ScheduleUpdate) {
    let ctx = expect_context::<SchedulesContext>();
    let notifications = expect_context::<NotificationContext>();
    let auth_context = expect_context::<AuthContext>();

    match schedule_update {
        Accepted(response) => {
            ctx.mark_own_schedule_dirty(response.id);
            ctx.mark_own_game_dirty(&response.game_id);
            notifications.schedule_notification_remove(response.id);

            if let Some(user) = auth_context.user.get() {
                if response.proposer_id == user.id {
                    notifications.schedule_notification_insert(
                        ScheduleNotificationKind::Acceptance,
                        response.id,
                    );
                }
            }

            update_schedules(ctx.tournament, &response, true);
            update_schedules(ctx.own, &response, true);
        }

        Proposed(response) => {
            ctx.mark_own_schedule_dirty(response.id);
            if let Some(user) = auth_context.user.get() {
                if response.opponent_id == user.id {
                    notifications.schedule_notification_insert(
                        ScheduleNotificationKind::Proposal,
                        response.id,
                    );
                }
            }

            update_schedules(ctx.own, &response, false);
        }

        Deleted(response) => {
            ctx.mark_own_schedule_dirty(response.id);
            notifications.schedule_notification_remove(response.id);

            remove_schedule(ctx.tournament, &response);
            remove_schedule(ctx.own, &response);
        }

        TournamentSchedules(schedules) => ctx.tournament.set(schedules),
        OwnTournamentSchedules(schedules) => ctx.own.set(schedules),
    }
}
