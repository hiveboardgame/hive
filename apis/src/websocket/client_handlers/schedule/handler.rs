use crate::{
    common::ScheduleUpdate,
    providers::{
        AuthContext,
        AuthIdentity,
        NotificationContext,
        ScheduleNotificationKind,
        SchedulesContext,
    },
    responses::ScheduleResponse,
};
use chrono::Utc;
use leptos::prelude::{expect_context, Get, With};
use shared_types::ScheduleOfferStatus;
use std::collections::HashSet;

pub fn handle_schedule_notification_snapshot(snapshot: Vec<ScheduleResponse>) {
    let schedules = expect_context::<SchedulesContext>();
    let notifications = expect_context::<NotificationContext>();
    let auth = expect_context::<AuthContext>();
    let expired = schedules.notification_snapshot_apply(snapshot);
    for offer_id in expired {
        notifications.schedule_notification_remove(offer_id);
    }
    let (proposal_ids, acceptance_ids) = auth
        .identity
        .get()
        .and_then(AuthIdentity::user_id)
        .map_or_else(
            || (HashSet::new(), HashSet::new()),
            |user_id| schedules.notification_ids_for_user(user_id),
        );
    notifications.schedule_notifications_snapshot_apply(proposal_ids, acceptance_ids);
}

pub fn handle_schedule(update: ScheduleUpdate) {
    let schedules = expect_context::<SchedulesContext>();
    let notifications = expect_context::<NotificationContext>();
    let auth = expect_context::<AuthContext>();
    match update {
        ScheduleUpdate::Changed(responses) => {
            for response in responses {
                schedules.schedule_update(&response);
                apply_notification_update(schedules, notifications, auth.identity.get(), &response);
            }
        }
        ScheduleUpdate::TournamentPurged(tournament_id) => {
            let removed = schedules.notification_schedules.with(|offers| {
                offers
                    .values()
                    .filter(|offer| offer.tournament_id == tournament_id)
                    .map(|offer| offer.id)
                    .collect::<Vec<_>>()
            });
            schedules.purge_tournament(&tournament_id);
            for offer_id in removed {
                notifications.schedule_notification_remove(offer_id);
            }
        }
    }
}

fn apply_notification_update(
    schedules: SchedulesContext,
    notifications: NotificationContext,
    identity: Option<AuthIdentity>,
    response: &ScheduleResponse,
) {
    let Some(user_id) = identity.and_then(AuthIdentity::user_id) else {
        return;
    };
    match response.status {
        ScheduleOfferStatus::Pending
            if response.opponent_id == user_id && response.has_future_candidate(Utc::now()) =>
        {
            schedules.notification_schedule_update(response);
            notifications
                .schedule_notification_insert(ScheduleNotificationKind::Proposal, response.id);
        }
        ScheduleOfferStatus::Accepted if response.proposer_id == user_id && !response.notified => {
            schedules.notification_schedule_update(response);
            notifications
                .schedule_notification_insert(ScheduleNotificationKind::Acceptance, response.id);
        }
        _ => {
            schedules.notification_schedule_delete(response.id);
            notifications.schedule_notification_remove(response.id);
        }
    }
}
