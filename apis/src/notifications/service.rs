use std::sync::Arc;

use db_lib::{
    get_conn,
    models::{NotificationPreferences, PushDevice},
    DbPool,
};
use tokio::sync::mpsc;

use super::{
    channel::{parse_channels, Channel},
    event::{Event, PrefField},
    NotifyOutcome, PushBackends,
};
use crate::websocket::busybee::Busybee;

/// Bounded queue: trigger sites must never block on push delivery. A full
/// queue means we're seeing more notification events per second than the
/// dispatcher can drain — at that point dropping (with a warn) is correct
/// because the user has bigger problems than a missed notification.
const QUEUE_CAPACITY: usize = 1024;

/// Unified notification dispatcher.
///
/// Trigger sites call [`crate::notifications::notify`] which forwards an
/// [`Event`] to this service over an mpsc channel. The background task does
/// all the I/O — DB lookups for prefs + devices, FCM HTTP, Busybee — so the
/// trigger site returns immediately and game flow never blocks on
/// notification delivery.
pub struct Notifier {
    tx: mpsc::Sender<Event>,
}

impl Notifier {
    pub fn spawn(pool: DbPool, push: PushBackends) -> Self {
        let (tx, rx) = mpsc::channel::<Event>(QUEUE_CAPACITY);
        let push = Arc::new(push);
        tokio::spawn(dispatcher_loop(rx, pool, push));
        Self { tx }
    }

    pub fn enqueue(&self, event: Event) {
        if let Err(err) = self.tx.try_send(event) {
            log::warn!("notification dispatcher: enqueue failed: {err}");
        }
    }
}

async fn dispatcher_loop(
    mut rx: mpsc::Receiver<Event>,
    pool: DbPool,
    push: Arc<PushBackends>,
) {
    while let Some(event) = rx.recv().await {
        let pool = pool.clone();
        let push = push.clone();
        // Per-event spawn so a slow channel send (FCM auth refresh, Busybee
        // timeout) doesn't block the next event. Loss-on-panic is fine —
        // the next event creates a fresh task.
        tokio::spawn(async move {
            handle_event(event, pool, push).await;
        });
    }
}

async fn handle_event(event: Event, pool: DbPool, push: Arc<PushBackends>) {
    let recipient = event.recipient();

    let mut conn = match get_conn(&pool).await {
        Ok(c) => c,
        Err(err) => {
            log::warn!("notify: get_conn failed for {recipient}: {err}");
            return;
        }
    };

    let prefs = match NotificationPreferences::find_for_user(recipient, &mut conn).await {
        Ok(p) => p,
        Err(err) => {
            log::warn!("notify: find prefs for {recipient} failed: {err}");
            return;
        }
    };

    let raw = match event.pref_field() {
        PrefField::YourTurn => &prefs.your_turn,
        PrefField::Challenges => &prefs.challenges,
        PrefField::GameEnded => &prefs.game_ended,
        PrefField::Tournament => &prefs.tournament,
        PrefField::Dms => &prefs.dms,
    };
    let channels = parse_channels(raw);
    if channels.is_empty() {
        return;
    }

    for channel in channels {
        match channel {
            Channel::Push => {
                // No server-side foreground gate: Android's FCM delivers
                // notifications with a `notification` block directly when
                // the app is backgrounded or killed, and routes through
                // the plugin's FirebaseMessagingService when foregrounded
                // (where the plugin can decide whether to render). The
                // server can't reliably distinguish "WS connected" from
                // "user actually looking at the screen" — backgrounded
                // apps keep their sockets alive — so we delegate to the
                // OS layer instead of trying to second-guess it. A user
                // who finds the redundancy annoying when active on
                // another device can silence push in settings.
                send_push(&event, recipient, &push, &pool).await;
            }
            Channel::Discord => send_discord(&event, recipient).await,
            Channel::Email => {
                // Stub: shape locked in via Event::render_email, backend
                // not wired yet. Render to a no-op so the omission is
                // visible in logs at WARN.
                let (subject, _body) = event.render_email();
                log::warn!("notify: email channel not implemented (would send '{subject}')");
            }
        }
    }
}

async fn send_push(event: &Event, recipient: uuid::Uuid, push: &PushBackends, pool: &DbPool) {
    let payload = event.render_push();

    let mut conn = match get_conn(pool).await {
        Ok(c) => c,
        Err(err) => {
            log::warn!("notify: get_conn for push devices failed: {err}");
            return;
        }
    };
    let devices = match PushDevice::find_for_user(recipient, &mut conn).await {
        Ok(d) => d,
        Err(err) => {
            log::warn!("notify: find push_devices for {recipient} failed: {err}");
            return;
        }
    };
    if devices.is_empty() {
        return;
    }

    for device in devices {
        // Skip APNs entirely during the iOS-paused phase. The push_devices
        // row may exist for users who installed an iOS build, but server
        // dispatch isn't implemented and we don't want a Failed log line
        // for every move. See memory entry `ios_push_paused`.
        if device.platform == "apns" {
            continue;
        }

        let outcome = push
            .send(&device.platform, &device.device_token, &payload)
            .await;
        match outcome {
            NotifyOutcome::Delivered => {}
            NotifyOutcome::Retryable => {
                log::warn!(
                    "notify: push retryable for device {} (no retry job yet)",
                    device.id
                );
            }
            NotifyOutcome::TokenDead => {
                log::warn!(
                    "notify: dead token reported for device {} ({}), deleting",
                    device.id,
                    device.platform
                );
                let _ = PushDevice::delete_dead_token(
                    &device.platform,
                    &device.device_token,
                    &mut conn,
                )
                .await;
            }
            NotifyOutcome::Failed(reason) => {
                log::warn!(
                    "notify: push failed for device {} ({}): {reason}",
                    device.id,
                    device.platform
                );
            }
        }
    }
}

async fn send_discord(event: &Event, recipient: uuid::Uuid) {
    let msg = event.render_discord();
    if let Err(err) = Busybee::msg(recipient, msg).await {
        log::warn!("notify: discord send for {recipient} failed: {err}");
    }
}
