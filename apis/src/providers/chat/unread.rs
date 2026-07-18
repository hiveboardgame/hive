use super::{Chat, UnreadDisplay};
use crate::{functions::chat::mark_chat_read, providers::AuthIdentity};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{ConversationKey, ConversationUnreadState, GameId, GameThread, TournamentId};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    time::Duration,
};
pub(super) const READ_RECEIPT_FLUSH_DELAY: Duration = Duration::from_millis(250);
const READ_RECEIPT_RETRY_DELAY: Duration = Duration::from_secs(10);
#[cfg(target_arch = "wasm32")]
const READ_RECEIPT_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[cfg(any(target_arch = "wasm32", test))]
async fn complete_before_timeout<Request, Timeout>(
    request: Request,
    timeout: Timeout,
) -> Option<Request::Output>
where
    Request: std::future::Future,
    Timeout: std::future::Future<Output = ()>,
{
    let mut request = Box::pin(request);
    let mut timeout = Box::pin(timeout);
    std::future::poll_fn(move |context| {
        if let std::task::Poll::Ready(output) = request.as_mut().poll(context) {
            return std::task::Poll::Ready(Some(output));
        }
        if timeout.as_mut().poll(context).is_ready() {
            return std::task::Poll::Ready(None);
        }
        std::task::Poll::Pending
    })
    .await
}

async fn mark_chat_read_before_timeout(key: ConversationKey, read_through_id: i64) -> Option<i64> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_timers::future::TimeoutFuture;

        let timeout_ms = u32::try_from(READ_RECEIPT_REQUEST_TIMEOUT.as_millis())
            .expect("read receipt timeout should fit in u32 milliseconds");
        complete_before_timeout(
            mark_chat_read(key, read_through_id),
            TimeoutFuture::new(timeout_ms),
        )
        .await
        .and_then(Result::ok)
    }

    #[cfg(not(target_arch = "wasm32"))]
    mark_chat_read(key, read_through_id).await.ok()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct UnreadSummary {
    total_count: i64,
    latest_message_id: i64,
}

fn update_max_id(current: &mut i64, id: i64) {
    *current = (*current).max(id);
}

#[derive(Clone, Debug, Default)]
pub(super) struct ChannelUnreadState {
    server: Option<ConversationUnreadState>,
    confirmed_read_through: i64,
    pending_read_through: i64,
    local_unread_ids: BTreeSet<i64>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ConversationUnread {
    state: ArcStoredValue<ChannelUnreadState>,
    pub(super) display: ArcRwSignal<UnreadDisplay>,
}

impl ConversationUnread {
    pub(super) fn clear(&self) {
        self.state.set_value(ChannelUnreadState::default());
        self.display.set(UnreadDisplay::default());
    }
}

impl ChannelUnreadState {
    fn read_floor(&self) -> i64 {
        self.confirmed_read_through.max(self.pending_read_through)
    }

    fn server_latest_message_id(&self) -> i64 {
        self.server
            .as_ref()
            .map(|state| state.latest_message_id)
            .unwrap_or(0)
    }

    fn display(&self, key: &ConversationKey, muted: &HashSet<TournamentId>) -> UnreadDisplay {
        if matches!(key, ConversationKey::Tournament(id) if muted.contains(id)) {
            return UnreadDisplay::default();
        }
        let read_floor = self.read_floor();
        let server_count = self
            .server
            .as_ref()
            .filter(|state| state.count > 0 && state.latest_unread_message_id > read_floor)
            .map(|state| state.count)
            .unwrap_or(0);
        let server_latest = self
            .server
            .as_ref()
            .filter(|state| state.count > 0 && state.latest_unread_message_id > read_floor)
            .map(|state| state.latest_unread_message_id)
            .unwrap_or(0);
        let server_latest_message_id = self.server_latest_message_id();
        let mut local_count = 0_i64;
        let mut local_latest = 0;
        for message_id in &self.local_unread_ids {
            if *message_id > read_floor && *message_id > server_latest_message_id {
                local_count = local_count.saturating_add(1);
                local_latest = *message_id;
            }
        }

        UnreadDisplay {
            count: server_count.saturating_add(local_count),
            latest_message_id: server_latest.max(local_latest),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct ReadReceiptCoordinator {
    pub(super) scheduled_read_through: HashMap<ConversationKey, i64>,
    pub(super) in_flight: HashMap<ConversationKey, i64>,
}

impl ReadReceiptCoordinator {
    pub(super) fn clear_channel(&mut self, key: &ConversationKey) {
        self.scheduled_read_through.remove(key);
        self.in_flight.remove(key);
    }

    pub(super) fn record_confirmed_read(&mut self, key: &ConversationKey, read_through_id: i64) {
        if self
            .scheduled_read_through
            .get(key)
            .is_some_and(|scheduled| *scheduled <= read_through_id)
        {
            self.scheduled_read_through.remove(key);
        }
    }

    pub(super) fn schedule_read(&mut self, key: &ConversationKey, read_through_id: i64) -> bool {
        update_max_id(
            self.scheduled_read_through.entry(key.clone()).or_default(),
            read_through_id,
        );
        !self.in_flight.contains_key(key)
    }

    pub(super) fn scheduled_keys(&self) -> Vec<ConversationKey> {
        self.scheduled_read_through.keys().cloned().collect()
    }

    pub(super) fn begin_scheduled_read(&mut self, key: &ConversationKey) -> Option<i64> {
        if self.in_flight.contains_key(key) {
            return None;
        }
        let scheduled = self.scheduled_read_through.remove(key)?;
        self.in_flight.insert(key.clone(), scheduled);
        Some(scheduled)
    }

    pub(super) fn discard_scheduled_read(&mut self, key: &ConversationKey) {
        self.scheduled_read_through.remove(key);
    }

    pub(super) fn finish_in_flight(&mut self, key: &ConversationKey, read_through_id: i64) -> bool {
        if self.in_flight.get(key) != Some(&read_through_id) {
            return false;
        }
        self.in_flight.remove(key);
        true
    }
}
impl Chat {
    pub(super) fn tracks_unread(&self, key: &ConversationKey) -> bool {
        key.tracks_read_receipts()
            && !matches!(
                key,
                ConversationKey::Tournament(tournament_id)
                    if self.tournament_muted_untracked(tournament_id)
            )
    }

    pub(super) fn clear_unread_state(&self, key: &ConversationKey) {
        self.unread_entry(key)
            .state
            .set_value(ChannelUnreadState::default());
        self.read_receipts
            .update_value(|receipts| receipts.clear_channel(key));
        self.sync_unread_display(key);
    }

    pub(super) fn remove_unread_state(&self, key: &ConversationKey) {
        let removed = self
            .unread
            .try_update_value(|registry| registry.remove(key).is_some())
            .unwrap_or(false);
        self.read_receipts
            .update_value(|receipts| receipts.clear_channel(key));
        if removed {
            self.recompute_unread_summary();
        }
    }

    pub(super) fn read_floor_untracked(&self, key: &ConversationKey) -> i64 {
        self.unread_entry(key)
            .state
            .with_value(ChannelUnreadState::read_floor)
    }

    pub(super) fn add_local_unread(&self, key: &ConversationKey, message_id: i64) {
        if message_id <= 0 {
            return;
        }
        self.unread_entry(key).state.update_value(|state| {
            state.local_unread_ids.insert(message_id);
        });
        self.sync_unread_display(key);
    }

    pub(super) fn set_pending_read(&self, key: &ConversationKey, read_through_id: i64) {
        if read_through_id <= 0 {
            return;
        }
        self.unread_entry(key).state.update_value(|state| {
            update_max_id(&mut state.pending_read_through, read_through_id);
        });
        self.sync_unread_display(key);
    }

    fn rollback_pending_read(&self, key: &ConversationKey, read_through_id: i64) {
        self.unread_entry(key).state.update_value(|state| {
            if state.pending_read_through <= read_through_id {
                state.pending_read_through = 0;
            }
        });
        self.sync_unread_display(key);
    }

    pub(crate) fn apply_read_receipt_update(
        &self,
        key: ConversationKey,
        last_read_message_id: i64,
    ) {
        self.record_authoritative_read(&key, last_read_message_id);
    }

    pub(super) fn record_authoritative_read(&self, key: &ConversationKey, read_through_id: i64) {
        if read_through_id <= 0 {
            return;
        }
        self.unread_entry(key).state.update_value(|state| {
            update_max_id(&mut state.confirmed_read_through, read_through_id);
            if state.pending_read_through <= read_through_id {
                state.pending_read_through = 0;
            }
            state.local_unread_ids.retain(|id| *id > read_through_id);
        });
        self.read_receipts.update_value(|receipts| {
            receipts.record_confirmed_read(key, read_through_id);
        });
        self.sync_unread_display(key);
        let read_floor = self.read_floor_untracked(key);
        let current_user_id = self.identity_untracked().and_then(AuthIdentity::user_id);
        if let Some(conversation) = self.conversation_if_exists(key) {
            let cached_unread_count = conversation.signals.messages.with_untracked(|messages| {
                messages
                    .iter()
                    .filter(|message| {
                        message.id > read_floor && Some(message.user_id) != current_user_id
                    })
                    .count() as i64
            });
            conversation
                .history_state()
                .set_unread_anchor(Some(cached_unread_count));
        }
    }

    pub(crate) fn mark_thread_caught_up(&self, key: &ConversationKey, message_id: i64) {
        if !self.is_channel_visible(key)
            || !key.tracks_read_receipts()
            || message_id <= self.read_floor_untracked(key)
        {
            return;
        }
        self.schedule_read_flush(key, message_id);
    }

    pub(super) fn schedule_read_flush(&self, key: &ConversationKey, read_through_id: i64) {
        if self.current_user_id_untracked().is_none()
            || read_through_id <= self.read_floor_untracked(key)
        {
            return;
        }
        let can_flush = self
            .read_receipts
            .try_update_value(|receipts| receipts.schedule_read(key, read_through_id))
            .unwrap_or(false);
        if !can_flush {
            return;
        }
        self.arm_read_receipt_flush();
    }

    pub(super) fn arm_read_receipt_flush(&self) {
        self.arm_read_receipt_flush_after(READ_RECEIPT_FLUSH_DELAY);
    }

    fn arm_read_receipt_flush_after(&self, delay: Duration) {
        let Some(timer) = self.read_receipt_timer.get_value() else {
            return;
        };
        if timer.is_pending.get_untracked() {
            return;
        }
        timer.schedule(delay, ());
    }

    pub(super) fn flush_scheduled_reads(&self) {
        if self.current_user_id_untracked().is_none() {
            return;
        }
        let scheduled_keys = self
            .read_receipts
            .with_value(ReadReceiptCoordinator::scheduled_keys);
        for key in scheduled_keys {
            self.flush_scheduled_read(&key);
        }
    }

    pub(super) fn resume_scheduled_reads(&self) {
        if self.current_user_id_untracked().is_some()
            && !self
                .read_receipts
                .with_value(ReadReceiptCoordinator::scheduled_keys)
                .is_empty()
        {
            self.arm_read_receipt_flush();
        }
    }

    pub(super) fn flush_scheduled_read(&self, key: &ConversationKey) {
        if self.current_user_id_untracked().is_none() {
            return;
        }
        if !self.is_channel_visible(key) {
            self.read_receipts
                .update_value(|receipts| receipts.discard_scheduled_read(key));
            return;
        }
        let read_through_id = self
            .read_receipts
            .try_update_value(|receipts| receipts.begin_scheduled_read(key))
            .flatten();
        if let Some(read_through_id) = read_through_id {
            self.start_read_request(key.clone(), read_through_id);
        }
    }

    pub(super) fn start_read_request(&self, key: ConversationKey, read_through_id: i64) {
        let Some(request_identity) = self.identity_untracked() else {
            return;
        };
        self.set_pending_read(&key, read_through_id);
        let chat = *self;
        let request_session_epoch = self.session_epoch_untracked();
        spawn_local(async move {
            let marked_read_through =
                mark_chat_read_before_timeout(key.clone(), read_through_id).await;
            if chat.identity_untracked() != Some(request_identity)
                || chat.session_epoch_untracked() != request_session_epoch
            {
                return;
            }
            chat.finish_read_request(&key, read_through_id, marked_read_through);
        });
    }

    pub(super) fn finish_read_request(
        &self,
        key: &ConversationKey,
        read_through_id: i64,
        marked_read_through: Option<i64>,
    ) {
        let finished = self
            .read_receipts
            .try_update_value(|receipts| receipts.finish_in_flight(key, read_through_id))
            .unwrap_or(false);
        if !finished {
            return;
        }
        let Some(marked_read_through) = marked_read_through else {
            self.rollback_pending_read(key, read_through_id);
            self.read_receipts.update_value(|receipts| {
                receipts.schedule_read(key, read_through_id);
            });
            self.arm_read_receipt_flush_after(READ_RECEIPT_RETRY_DELAY);
            return;
        };
        self.record_authoritative_read(key, marked_read_through);
        if marked_read_through < read_through_id {
            self.rollback_pending_read(key, read_through_id);
        }
        self.flush_scheduled_read(key);
    }

    pub(crate) fn set_channel_visible(&self, key: &ConversationKey) -> u64 {
        let owner_id = self.next_visible_owner_id.get_value().saturating_add(1);
        self.next_visible_owner_id.set_value(owner_id);
        self.visible_channel
            .set_value(Some(super::VisibleChannelOwner {
                id: owner_id,
                key: key.clone(),
            }));
        owner_id
    }

    pub(crate) fn clear_channel_visible(&self, owner_id: u64) {
        self.visible_channel.update_value(|visible| {
            if visible.as_ref().is_some_and(|owner| owner.id == owner_id) {
                *visible = None;
            }
        });
    }

    pub(super) fn is_channel_visible(&self, key: &ConversationKey) -> bool {
        self.visible_channel
            .with_value(|visible| visible.as_ref().is_some_and(|owner| &owner.key == key))
    }

    pub(super) fn apply_server_unread_states(&self, states: Vec<ConversationUnreadState>) {
        let mut keys = self.unread.with_value(|registry| {
            registry
                .iter()
                .map(|(key, unread)| {
                    unread.state.update_value(|state| state.server = None);
                    key.clone()
                })
                .collect::<HashSet<_>>()
        });
        for server_state in states {
            let key = server_state.key.clone();
            let latest_server_id = server_state.latest_message_id;
            let last_read_message_id = server_state.last_read_message_id;
            self.unread_entry(&key).state.update_value(|state| {
                update_max_id(&mut state.confirmed_read_through, last_read_message_id);
                if state.pending_read_through <= last_read_message_id {
                    state.pending_read_through = 0;
                }
                let read_floor = state.read_floor();
                state
                    .local_unread_ids
                    .retain(|id| *id > latest_server_id && *id > read_floor);
                state.server = Some(server_state);
            });
            self.read_receipts.update_value(|receipts| {
                receipts.record_confirmed_read(&key, last_read_message_id);
            });
            keys.insert(key);
        }
        self.sync_unread_displays(keys.iter());
    }

    pub(super) fn set_history_unread_count(&self, key: &ConversationKey, unread_count: i64) {
        if let Some(conversation) = self.conversation_if_exists(key) {
            conversation
                .history_state()
                .set_unread_anchor(Some(unread_count.max(0)));
        }
    }

    pub(super) fn increment_history_unread_count(&self, key: &ConversationKey) {
        if let Some(conversation) = self.conversation_if_exists(key) {
            conversation.history_state().increment_unread_anchor();
        }
    }

    fn unread_summary_excluding_game(&self, suppressed_game_id: Option<&GameId>) -> UnreadSummary {
        self.unread.with_value(|registry| {
            registry
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        (key, suppressed_game_id),
                        (
                            ConversationKey::Game {
                                game_id,
                                thread: GameThread::Players,
                            },
                            Some(suppressed),
                        ) if game_id == suppressed
                    )
                })
                .fold(UnreadSummary::default(), |mut summary, (_, unread)| {
                    let display = unread.display.get_untracked();
                    summary.total_count = summary.total_count.saturating_add(display.count).max(0);
                    summary.latest_message_id =
                        summary.latest_message_id.max(display.latest_message_id);
                    summary
                })
        })
    }

    fn recompute_unread_summary(&self) {
        let summary = self.unread_summary_excluding_game(None);
        if self.unread_summary.get_untracked() != summary {
            self.unread_summary.set(summary);
        }
    }

    fn sync_unread_display_with_muted(
        &self,
        key: &ConversationKey,
        muted: &HashSet<TournamentId>,
    ) -> bool {
        let unread = self.unread_entry(key);
        let display = unread.state.with_value(|state| state.display(key, muted));
        if unread.display.get_untracked() != display {
            unread.display.set(display);
            true
        } else {
            false
        }
    }

    fn sync_unread_displays<'a>(&self, keys: impl IntoIterator<Item = &'a ConversationKey>) {
        let changed = self
            .preferences
            .with_muted_tournament_ids_untracked(|muted| {
                let mut changed = false;
                for key in keys {
                    changed = self.sync_unread_display_with_muted(key, muted) || changed;
                }
                changed
            });
        if changed {
            self.recompute_unread_summary();
        }
    }

    pub(super) fn sync_unread_display(&self, key: &ConversationKey) {
        let changed = self
            .preferences
            .with_muted_tournament_ids_untracked(|muted| {
                self.sync_unread_display_with_muted(key, muted)
            });
        if changed {
            self.recompute_unread_summary();
        }
    }

    pub(crate) fn total_unread_count_excluding_game(
        &self,
        suppressed_game_id: Option<&GameId>,
    ) -> i64 {
        let summary = self.unread_summary.get();
        if suppressed_game_id.is_none() {
            summary.total_count
        } else {
            self.unread_summary_excluding_game(suppressed_game_id)
                .total_count
        }
    }

    pub(crate) fn latest_unread_message_id_excluding_game(
        &self,
        suppressed_game_id: Option<&GameId>,
    ) -> i64 {
        let summary = self.unread_summary.get();
        if suppressed_game_id.is_none() {
            return summary.latest_message_id;
        }
        self.unread_summary_excluding_game(suppressed_game_id)
            .latest_message_id
    }

    pub(crate) fn unread_count_for_game(&self, game_id: &GameId) -> i64 {
        self.unread_count_for_channel(&ConversationKey::game_players(game_id))
    }

    fn unread_count_for_channel(&self, key: &ConversationKey) -> i64 {
        self.unread(key).get().count
    }

    #[cfg(test)]
    pub(super) fn unread_count_for_channel_untracked(&self, key: &ConversationKey) -> i64 {
        self.unread(key).get_untracked().count
    }
}

#[cfg(test)]
mod tests {
    use super::complete_before_timeout;
    use std::{
        cell::Cell,
        future::{pending, ready, Future},
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    #[test]
    fn timeout_drops_a_stalled_receipt_request() {
        struct DropMarker(Rc<Cell<bool>>);

        impl Drop for DropMarker {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }

        let dropped = Rc::new(Cell::new(false));
        let request_dropped = Rc::clone(&dropped);
        let request = async move {
            let _marker = DropMarker(request_dropped);
            pending::<()>().await;
        };
        let mut race = Box::pin(complete_before_timeout(request, ready(())));
        let mut context = Context::from_waker(Waker::noop());

        assert!(matches!(
            race.as_mut().poll(&mut context),
            Poll::Ready(None),
        ));
        drop(race);
        assert!(dropped.get());
    }
}
