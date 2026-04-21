use crate::{
    common::{ChallengeUpdate, ServerMessage},
    responses::ChallengeResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        REALTIME_DISABLED_MSG,
    },
};
use anyhow::{bail, Result};
use db_lib::{
    get_conn,
    models::{Challenge, NewChallenge, User},
    DbPool,
};
use shared_types::{ChallengeDetails, ChallengeVisibility, TimeMode};
use std::{
    str::FromStr,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
};
use uuid::Uuid;

pub struct CreateHandler {
    details: ChallengeDetails,
    user_id: Uuid,
    pool: DbPool,
    realtime_games_enabled: Arc<AtomicBool>,
}

impl CreateHandler {
    pub async fn new(
        details: ChallengeDetails,
        user_id: Uuid,
        pool: &DbPool,
        realtime_games_enabled: Arc<AtomicBool>,
    ) -> Result<Self> {
        Ok(Self {
            details,
            user_id,
            pool: pool.clone(),
            realtime_games_enabled,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        if self.details.time_mode == TimeMode::RealTime
            && !self.realtime_games_enabled.load(Ordering::Relaxed)
        {
            bail!(REALTIME_DISABLED_MSG);
        }
        let mut conn = get_conn(&self.pool).await?;
        let opponent = match &self.details.opponent {
            Some(username) => Some((User::find_by_username(username, &mut conn).await?).id),
            None => None,
        };

        let new_challenge =
            NewChallenge::new(self.user_id, opponent, &self.details, &mut conn).await?;
        let challenge = Challenge::create(&new_challenge, &mut conn).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        let mut messages = Vec::new();
        match ChallengeVisibility::from_str(&new_challenge.visibility)? {
            ChallengeVisibility::Direct => {
                if let Some(ref opponent) = challenge_response.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(challenge_response.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response,
                        )),
                    });
                }
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(challenge_response.challenger.uid),
                    message: ServerMessage::Challenge(ChallengeUpdate::Direct(challenge_response)),
                });
            }
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Created(challenge_response)),
                });
            }
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    //! Coverage for the realtime-disable gate. The gate runs before
    //! `get_conn`, so these tests use the same trick as
    //! `ws_hub.rs::cache_tests::make_hub`: bb8 builds the pool struct without
    //! opening connections, and anything that bails before the first DB call
    //! is testable without a live database. Negative controls cap the wait
    //! with `tokio::time::timeout` — a timeout (or any error other than the
    //! maintenance message) is the signal that the gate did not fire.
    //!
    //! `AcceptHandler` and `StartHandler` are intentionally not covered here:
    //! both check the gate *after* loading a row from the DB to read its
    //! persisted `time_mode`, so they cannot be exercised against an
    //! unreachable pool without restructuring the handlers.
    use super::*;
    use hive_lib::{ColorChoice, GameType};
    use std::time::Duration;

    fn details_with(time_mode: TimeMode) -> ChallengeDetails {
        let (time_base, time_increment) = match time_mode {
            TimeMode::RealTime => (Some(180), Some(2)),
            TimeMode::Correspondence => (None, Some(86_400)),
            TimeMode::Untimed => (None, None),
        };
        ChallengeDetails {
            rated: false,
            game_type: GameType::Base,
            visibility: ChallengeVisibility::Public,
            opponent: None,
            color_choice: ColorChoice::Random,
            time_mode,
            time_base,
            time_increment,
            band_upper: None,
            band_lower: None,
        }
    }

    async fn unreachable_pool() -> DbPool {
        // bb8 builds the pool struct lazily; no TCP connect happens here.
        // 127.0.0.1:9 is the discard port — any later get_conn either errors
        // fast (connection refused) or hangs until the bb8 connection_timeout
        // (~30s). Negative-control tests cap the wait themselves.
        db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting")
    }

    #[tokio::test]
    async fn realtime_create_bails_when_disabled() {
        let pool = unreachable_pool().await;
        let flag = Arc::new(AtomicBool::new(false));
        let handler = CreateHandler::new(
            details_with(TimeMode::RealTime),
            Uuid::new_v4(),
            &pool,
            flag,
        )
        .await
        .unwrap();
        let err = handler
            .handle()
            .await
            .expect_err("must bail before any DB call");
        assert!(
            err.to_string().contains(REALTIME_DISABLED_MSG),
            "expected maintenance bail, got: {err}"
        );
    }

    #[tokio::test]
    async fn realtime_create_passes_gate_when_enabled() {
        // Flag on: gate must not fire. Execution proceeds to get_conn against
        // the unreachable pool — either we time out, or get a DB error, but
        // it must not be the maintenance message.
        let pool = unreachable_pool().await;
        let flag = Arc::new(AtomicBool::new(true));
        let handler = CreateHandler::new(
            details_with(TimeMode::RealTime),
            Uuid::new_v4(),
            &pool,
            flag,
        )
        .await
        .unwrap();
        match tokio::time::timeout(Duration::from_millis(200), handler.handle()).await {
            Err(_) => {} // timed out in get_conn — gate did not fire
            Ok(Err(err)) => assert!(
                !err.to_string().contains(REALTIME_DISABLED_MSG),
                "gate fired despite flag being enabled: {err}"
            ),
            Ok(Ok(_)) => panic!("unreachable pool should not produce a successful handle()"),
        }
    }

    #[tokio::test]
    async fn correspondence_create_passes_gate_when_disabled() {
        // Gate is RealTime-only: a Correspondence challenge must pass even
        // when the flag is off.
        let pool = unreachable_pool().await;
        let flag = Arc::new(AtomicBool::new(false));
        let handler = CreateHandler::new(
            details_with(TimeMode::Correspondence),
            Uuid::new_v4(),
            &pool,
            flag,
        )
        .await
        .unwrap();
        match tokio::time::timeout(Duration::from_millis(200), handler.handle()).await {
            Err(_) => {} // timed out in get_conn — gate did not fire
            Ok(Err(err)) => assert!(
                !err.to_string().contains(REALTIME_DISABLED_MSG),
                "gate fired for non-RealTime time mode: {err}"
            ),
            Ok(Ok(_)) => panic!("unreachable pool should not produce a successful handle()"),
        }
    }
}
