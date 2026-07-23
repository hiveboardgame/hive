use std::sync::Arc;

use db_lib::{
    get_conn,
    models::{NotificationPreferences, PushDevice, User},
    DbPool,
};
use tokio::sync::mpsc;

use super::{
    channel::{parse_channels, Channel},
    event::Event,
    NotifyOutcome,
    PendingNotifications,
    PushBackends,
    PushTelemetry,
};
use crate::{i18n::*, websocket::busybee::Busybee};
use chrono::{Duration as ChronoDuration, Utc};
use shared_types::NotificationCategory;
use std::{future::Future, pin::Pin, time::Duration};
use uuid::Uuid;

const PARK_WINDOW: Duration = Duration::from_secs(5);

const QUEUE_CAPACITY: usize = 1024;

const SWEEP_INTERVAL: Duration = Duration::from_secs(300);

pub struct Notifier {
    tx: mpsc::Sender<Event>,
    telemetry: Arc<PushTelemetry>,
}

impl Notifier {
    pub fn spawn(
        pool: DbPool,
        push: PushBackends,
        pending: Arc<PendingNotifications>,
        telemetry: Arc<PushTelemetry>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Event>(QUEUE_CAPACITY);
        let push = Arc::new(push);
        tokio::spawn(dispatcher_loop(
            rx,
            pool,
            push,
            pending.clone(),
            telemetry.clone(),
        ));
        tokio::spawn(sweep_loop(pending));
        Self { tx, telemetry }
    }

    pub fn enqueue(&self, event: Event) {
        self.telemetry.received();
        if let Err(err) = self.tx.try_send(event) {
            self.telemetry.dropped_queue_full();
            log::warn!("notification dispatcher: enqueue failed: {err}");
        }
    }
}

async fn sweep_loop(pending: Arc<PendingNotifications>) {
    let mut interval = tokio::time::interval(SWEEP_INTERVAL);
    loop {
        interval.tick().await;
        pending.sweep();
        super::sweep_game_ended_dedup();
    }
}

async fn dispatcher_loop(
    mut rx: mpsc::Receiver<Event>,
    pool: DbPool,
    push: Arc<PushBackends>,
    pending: Arc<PendingNotifications>,
    telemetry: Arc<PushTelemetry>,
) {
    while let Some(event) = rx.recv().await {
        let pool = pool.clone();
        let push = push.clone();
        let pending = pending.clone();
        let telemetry = telemetry.clone();
        tokio::spawn(async move {
            handle_event(event, pool, push, pending, telemetry).await;
        });
    }
}

async fn handle_event(
    event: Event,
    pool: DbPool,
    push: Arc<PushBackends>,
    pending: Arc<PendingNotifications>,
    telemetry: Arc<PushTelemetry>,
) {
    let recipient = event.recipient();

    if matches!(event, Event::TestPush { .. }) {
        telemetry.test_pushes();
        let locale = fetch_locale(recipient, &pool).await;
        send_push(&event, recipient, push, pool, &telemetry, locale, None).await;
        return;
    }

    let (channels, locale) = {
        let mut conn = match get_conn(&pool).await {
            Ok(c) => c,
            Err(err) => {
                telemetry.prefs_db_error();
                log::warn!("notify: get_conn failed for {recipient}: {err}");
                return;
            }
        };
        let prefs = match NotificationPreferences::find_for_user(recipient, &mut conn).await {
            Ok(p) => p,
            Err(err) => {
                telemetry.prefs_db_error();
                log::warn!("notify: find prefs for {recipient} failed: {err}");
                return;
            }
        };
        let channels = channels_for(&prefs, event.category());
        let locale = match User::find_by_uuid(&recipient, &mut conn).await {
            Ok(user) => parse_locale(user.lang.as_deref().unwrap_or("")),
            Err(_) => Locale::default(),
        };
        (channels, locale)
    };
    if channels.is_empty() {
        telemetry.suppressed_prefs();
        return;
    }

    match event.ack_key() {
        None => {
            send_channels(
                &event, recipient, &channels, &push, &pool, &telemetry, locale,
            )
            .await
        }
        Some(key) => {
            telemetry.ack_eligible();
            let Some(token) = pending.register(recipient, key.clone()) else {
                telemetry.ack_suppressed();
                return;
            };
            tokio::select! {
                _ = tokio::time::sleep(PARK_WINDOW) => {
                    telemetry.ack_fired();
                    send_channels(&event, recipient, &channels, &push, &pool, &telemetry, locale).await;
                }
                _ = token.cancelled() => { telemetry.ack_suppressed(); }
            }
            pending.clear(recipient, &key, &token);
        }
    }
}

fn channels_for(prefs: &NotificationPreferences, category: NotificationCategory) -> Vec<Channel> {
    let raw = match category {
        NotificationCategory::YourTurn => &prefs.your_turn,
        NotificationCategory::Challenges => &prefs.challenges,
        NotificationCategory::GameEnded => &prefs.game_ended,
        NotificationCategory::Tournament => &prefs.tournament,
        NotificationCategory::Schedules => &prefs.schedules,
        NotificationCategory::Dms => &prefs.dms,
        NotificationCategory::GeneralChat => &prefs.general_chat,
    };
    parse_channels(raw)
}

fn parse_locale(code: &str) -> Locale {
    Locale::get_all()
        .iter()
        .find(|l| l.to_string() == code)
        .copied()
        .unwrap_or_default()
}

async fn fetch_locale(recipient: Uuid, pool: &DbPool) -> Locale {
    let Ok(mut conn) = get_conn(pool).await else {
        return Locale::default();
    };
    match User::find_by_uuid(&recipient, &mut conn).await {
        Ok(user) => parse_locale(user.lang.as_deref().unwrap_or("")),
        Err(_) => Locale::default(),
    }
}

async fn send_channels<'a>(
    event: &'a Event,
    recipient: Uuid,
    channels: &'a [Channel],
    push: &'a Arc<PushBackends>,
    pool: &'a DbPool,
    telemetry: &'a Arc<PushTelemetry>,
    locale: Locale,
) {
    let mut sends: Vec<Pin<Box<dyn Future<Output = ()> + Send + 'a>>> = Vec::new();
    for channel in channels {
        match channel {
            Channel::Push => sends.push(Box::pin(send_push(
                event,
                recipient,
                Arc::clone(push),
                pool.clone(),
                telemetry,
                locale,
                Some(event.category()),
            ))),
            Channel::Discord if event.suppresses_discord() => {}
            Channel::Discord => sends.push(Box::pin(send_discord(event, recipient))),
            Channel::Email => {
                let (subject, _body) = event.render_email();
                log::warn!("notify: email channel not implemented (would send '{subject}')");
            }
        }
    }
    futures_util::future::join_all(sends).await;
}

const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(30);

async fn delete_dead_token(pool: &DbPool, platform: &str, device_token: &str) {
    if let Ok(mut conn) = get_conn(pool).await {
        let _ = PushDevice::delete_dead_token(platform, device_token, &mut conn).await;
    }
}

async fn send_push(
    event: &Event,
    recipient: uuid::Uuid,
    push: Arc<PushBackends>,
    pool: DbPool,
    telemetry: &Arc<PushTelemetry>,
    locale: Locale,
    recheck_category: Option<NotificationCategory>,
) {
    let payload = event.render_push(locale);

    let devices = {
        let mut conn = match get_conn(&pool).await {
            Ok(c) => c,
            Err(err) => {
                telemetry.device_db_error();
                log::warn!("notify: get_conn for push devices failed: {err}");
                return;
            }
        };
        match PushDevice::find_for_user(recipient, &mut conn).await {
            Ok(d) => d,
            Err(err) => {
                telemetry.device_db_error();
                log::warn!("notify: find push_devices for {recipient} failed: {err}");
                return;
            }
        }
    };
    if devices.is_empty() {
        telemetry.no_device();
        return;
    }

    for device in devices {
        let outcome = push.send(&device, &payload).await;
        match outcome {
            NotifyOutcome::Delivered => {
                telemetry.delivered();
                if Utc::now() - device.last_seen_at > ChronoDuration::days(1) {
                    if let Ok(mut conn) = get_conn(&pool).await {
                        let _ = PushDevice::touch(device.id, &mut conn).await;
                    }
                }
            }
            NotifyOutcome::Retryable => {
                telemetry.retryable();
                log::debug!(
                    "notify: push retryable for device {}, scheduling one retry",
                    device.id
                );
                schedule_retry(
                    Arc::clone(&push),
                    pool.clone(),
                    device.clone(),
                    payload.clone(),
                    Arc::clone(telemetry),
                    recipient,
                    recheck_category,
                );
            }
            NotifyOutcome::TokenDead => {
                telemetry.token_dead();
                log::warn!(
                    "notify: dead token reported for device {} ({}), deleting",
                    device.id,
                    device.platform
                );
                delete_dead_token(&pool, &device.platform, &device.device_token).await;
            }
            NotifyOutcome::Failed(reason) => {
                telemetry.failed();
                log::warn!(
                    "notify: push failed for device {} ({}): {reason}",
                    device.id,
                    device.platform
                );
            }
        }
    }
}

fn schedule_retry(
    push: Arc<PushBackends>,
    pool: DbPool,
    device: PushDevice,
    payload: super::payload::Push,
    telemetry: Arc<PushTelemetry>,
    recipient: Uuid,
    recheck_category: Option<NotificationCategory>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(RETRY_DELAY).await;
        let device_id = device.id;
        let platform = device.platform.clone();
        match get_conn(&pool).await {
            // Re-check ownership, not just liveness: the same browser endpoint can
            // be re-registered by another account between send and retry (upsert
            // keeps the row id while moving user_id), so a stale retry must not
            // deliver the original recipient's notification to the new owner.
            Ok(mut conn) => match PushDevice::is_active_for_user(device_id, recipient, &mut conn)
                .await
            {
                Ok(true) => {
                    if let Some(category) = recheck_category {
                        match NotificationPreferences::find_for_user(recipient, &mut conn).await {
                            Ok(prefs) => {
                                if !channels_for(&prefs, category).contains(&Channel::Push) {
                                    telemetry.retry_gave_up();
                                    log::debug!(
                                        "notify: retry skipped, push disabled in prefs for {recipient}"
                                    );
                                    return;
                                }
                            }
                            Err(err) => {
                                telemetry.prefs_db_error();
                                log::warn!(
                                    "notify: retry prefs check for {recipient} failed: {err}"
                                );
                                return;
                            }
                        }
                    }
                }
                Ok(false) => {
                    telemetry.retry_gave_up();
                    log::debug!("notify: retry skipped, device {device_id} no longer active");
                    return;
                }
                Err(err) => {
                    telemetry.device_db_error();
                    log::warn!("notify: retry is_active check for {device_id} failed: {err}");
                    return;
                }
            },
            Err(err) => {
                telemetry.device_db_error();
                log::warn!("notify: retry get_conn for {device_id} failed: {err}");
                return;
            }
        }
        let outcome = push.send(&device, &payload).await;
        match outcome {
            NotifyOutcome::Delivered => {
                telemetry.retry_delivered();
                log::debug!("notify: retry delivered for device {device_id}");
            }
            NotifyOutcome::Retryable => {
                telemetry.retry_gave_up();
                log::warn!("notify: retry still retryable for device {device_id}, dropping");
            }
            NotifyOutcome::TokenDead => {
                telemetry.retry_gave_up();
                log::warn!(
                    "notify: retry confirmed dead token for device {device_id} ({platform}), deleting"
                );
                delete_dead_token(&pool, &platform, &device.device_token).await;
            }
            NotifyOutcome::Failed(reason) => {
                telemetry.retry_gave_up();
                log::warn!("notify: retry failed for device {device_id} ({platform}): {reason}");
            }
        }
    });
}

async fn send_discord(event: &Event, recipient: uuid::Uuid) {
    let msg = event.render_discord();
    if let Err(err) = Busybee::msg(recipient, msg).await {
        log::warn!("notify: discord send for {recipient} failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs_with(general_chat: Vec<Option<String>>) -> NotificationPreferences {
        NotificationPreferences {
            user_id: Uuid::nil(),
            your_turn: vec![],
            challenges: vec![],
            game_ended: vec![],
            tournament: vec![],
            schedules: vec![],
            general_chat,
            dms: vec![],
        }
    }

    #[test]
    fn channels_for_general_chat_reads_general_chat_column() {
        let prefs = prefs_with(vec![Some("push".to_string())]);
        assert_eq!(
            channels_for(&prefs, NotificationCategory::GeneralChat),
            vec![Channel::Push]
        );
    }

    #[test]
    fn channels_for_general_chat_empty_by_default() {
        let prefs = prefs_with(vec![]);
        assert!(channels_for(&prefs, NotificationCategory::GeneralChat).is_empty());
    }
}
