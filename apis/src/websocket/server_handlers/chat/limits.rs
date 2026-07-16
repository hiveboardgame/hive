use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use uuid::Uuid;

const SUBSCRIPTION_ATTEMPT_LIMIT: u32 = 20;
const SUBSCRIPTION_ATTEMPT_WINDOW: Duration = Duration::from_secs(10);
const USER_SEND_LIMIT: u32 = 30;
const USER_SEND_WINDOW: Duration = Duration::from_secs(10);
const SOCKET_SEND_LIMIT: u32 = 15;
const SOCKET_SEND_WINDOW: Duration = Duration::from_secs(10);
const CLEANUP_INTERVAL: u64 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatLimitError {
    SubscriptionAttempts { retry_after: Duration },
    Send { retry_after: Duration },
}

impl ChatLimitError {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::SubscriptionAttempts { .. } => "Too many chat subscription attempts",
            Self::Send { .. } => "Too many chat messages",
        }
    }

    pub const fn retry_after(self) -> Duration {
        match self {
            Self::SubscriptionAttempts { retry_after } | Self::Send { retry_after } => retry_after,
        }
    }
}

#[derive(Clone, Copy)]
struct Limit {
    attempts: u32,
    window: Duration,
}

#[derive(Clone, Copy)]
struct ChatLimitConfig {
    subscription_attempts: Limit,
    user_sends: Limit,
    socket_sends: Limit,
}

impl Default for ChatLimitConfig {
    fn default() -> Self {
        Self {
            subscription_attempts: Limit {
                attempts: SUBSCRIPTION_ATTEMPT_LIMIT,
                window: SUBSCRIPTION_ATTEMPT_WINDOW,
            },
            user_sends: Limit {
                attempts: USER_SEND_LIMIT,
                window: USER_SEND_WINDOW,
            },
            socket_sends: Limit {
                attempts: SOCKET_SEND_LIMIT,
                window: SOCKET_SEND_WINDOW,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct AttemptWindow {
    started_at: Instant,
    attempts: u32,
}

impl AttemptWindow {
    fn check(&mut self, now: Instant, limit: Limit) -> Result<(), Duration> {
        let elapsed = now.duration_since(self.started_at);
        if elapsed >= limit.window {
            self.started_at = now;
            self.attempts = 0;
        }
        if self.attempts >= limit.attempts {
            return Err(limit.window.saturating_sub(elapsed));
        }
        self.attempts += 1;
        Ok(())
    }
}

#[derive(Default)]
struct ChatLimitState {
    subscription_attempts: HashMap<Uuid, AttemptWindow>,
    user_sends: HashMap<Uuid, AttemptWindow>,
    socket_sends: HashMap<Uuid, AttemptWindow>,
    operations: u64,
}

#[derive(Default)]
pub struct ChatRateLimits {
    config: ChatLimitConfig,
    state: Mutex<ChatLimitState>,
}

impl ChatRateLimits {
    pub fn check_subscription_attempt(&self, socket_id: Uuid) -> Result<(), ChatLimitError> {
        let now = Instant::now();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self::clean_if_due(&mut state, self.config, now);
        let result = Self::check_window(
            &mut state.subscription_attempts,
            socket_id,
            now,
            self.config.subscription_attempts,
        );
        match result {
            Ok(()) => Ok(()),
            Err(retry_after) => Err(ChatLimitError::SubscriptionAttempts { retry_after }),
        }
    }

    pub fn check_send(&self, user_id: Uuid, socket_id: Uuid) -> Result<(), ChatLimitError> {
        let now = Instant::now();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self::clean_if_due(&mut state, self.config, now);
        if let Err(retry_after) = Self::check_window(
            &mut state.socket_sends,
            socket_id,
            now,
            self.config.socket_sends,
        ) {
            return Err(ChatLimitError::Send { retry_after });
        }
        if let Err(retry_after) =
            Self::check_window(&mut state.user_sends, user_id, now, self.config.user_sends)
        {
            return Err(ChatLimitError::Send { retry_after });
        }
        Ok(())
    }

    pub fn remove_socket(&self, socket_id: Uuid) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.subscription_attempts.remove(&socket_id);
        state.socket_sends.remove(&socket_id);
    }

    fn check_window(
        windows: &mut HashMap<Uuid, AttemptWindow>,
        key: Uuid,
        now: Instant,
        limit: Limit,
    ) -> Result<(), Duration> {
        windows
            .entry(key)
            .or_insert(AttemptWindow {
                started_at: now,
                attempts: 0,
            })
            .check(now, limit)
    }

    fn clean_if_due(state: &mut ChatLimitState, config: ChatLimitConfig, now: Instant) {
        state.operations = state.operations.wrapping_add(1);
        if !state.operations.is_multiple_of(CLEANUP_INTERVAL) {
            return;
        }
        state.subscription_attempts.retain(|_, window| {
            now.duration_since(window.started_at) < config.subscription_attempts.window
        });
        state
            .user_sends
            .retain(|_, window| now.duration_since(window.started_at) < config.user_sends.window);
        state
            .socket_sends
            .retain(|_, window| now.duration_since(window.started_at) < config.socket_sends.window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_limits(
        subscription_attempts: u32,
        user_sends: u32,
        socket_sends: u32,
    ) -> ChatRateLimits {
        let window = Duration::from_secs(60);
        ChatRateLimits {
            config: ChatLimitConfig {
                subscription_attempts: Limit {
                    attempts: subscription_attempts,
                    window,
                },
                user_sends: Limit {
                    attempts: user_sends,
                    window,
                },
                socket_sends: Limit {
                    attempts: socket_sends,
                    window,
                },
            },
            state: Mutex::new(ChatLimitState::default()),
        }
    }

    #[test]
    fn subscription_attempts_are_limited_per_socket() {
        let limits = test_limits(2, 10, 10);
        let socket_id = Uuid::new_v4();

        assert_eq!(limits.check_subscription_attempt(socket_id), Ok(()));
        assert_eq!(limits.check_subscription_attempt(socket_id), Ok(()));
        let error = limits
            .check_subscription_attempt(socket_id)
            .expect_err("third attempt is limited");
        assert!(matches!(error, ChatLimitError::SubscriptionAttempts { .. }));
        assert!(error.retry_after() > Duration::ZERO);
        assert!(error.retry_after() <= Duration::from_secs(60));
        assert_eq!(limits.check_subscription_attempt(Uuid::new_v4()), Ok(()));
    }

    #[test]
    fn sends_are_limited_across_a_users_sockets() {
        let limits = test_limits(10, 2, 10);
        let user_id = Uuid::new_v4();

        assert_eq!(limits.check_send(user_id, Uuid::new_v4()), Ok(()));
        assert_eq!(limits.check_send(user_id, Uuid::new_v4()), Ok(()));
        assert!(matches!(
            limits.check_send(user_id, Uuid::new_v4()),
            Err(ChatLimitError::Send { .. })
        ));
    }

    #[test]
    fn per_socket_send_limit_is_an_independent_backstop() {
        let limits = test_limits(10, 10, 2);
        let user_id = Uuid::new_v4();
        let socket_id = Uuid::new_v4();

        assert_eq!(limits.check_send(user_id, socket_id), Ok(()));
        assert_eq!(limits.check_send(user_id, socket_id), Ok(()));
        assert!(matches!(
            limits.check_send(user_id, socket_id),
            Err(ChatLimitError::Send { .. })
        ));
        assert_eq!(limits.check_send(user_id, Uuid::new_v4()), Ok(()));
    }
}
