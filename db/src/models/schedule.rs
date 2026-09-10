use crate::{
    db_error::DbError,
    models::TournamentSlot,
    schema::{games, schedule_offers, tournament_slots, tournaments},
    DbConn,
};
use chrono::{DateTime, Duration, Utc};
use diesel::{prelude::*, SelectableHelper};
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::GameStatus;
use shared_types::{Clock, GameStart, ScheduleOfferStatus};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = schedule_offers)]
struct NewScheduleOffer {
    id: Uuid,
    tournament_id: Uuid,
    tournament_slot_id: Uuid,
    proposer_id: Uuid,
    candidate_times: Vec<Option<DateTime<Utc>>>,
    status: String,
    selected_time: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    resolved_by: Option<Uuid>,
    notified: bool,
}

#[derive(Queryable, Identifiable, Clone, Debug, Selectable)]
#[diesel(table_name = schedule_offers)]
pub struct ScheduleOffer {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub tournament_slot_id: Uuid,
    pub proposer_id: Uuid,
    pub candidate_times: Vec<Option<DateTime<Utc>>>,
    pub status: String,
    pub selected_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub notified: bool,
}

impl ScheduleOffer {
    pub async fn propose(
        user_id: Uuid,
        tournament_id: Uuid,
        tournament_slot_id: Uuid,
        candidate_times: Vec<DateTime<Utc>>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        let candidate_times = canonical_candidates(candidate_times)?;
        conn.transaction::<_, DbError, _>(async move |tc| {
            Self::propose_locked(
                user_id,
                tournament_id,
                tournament_slot_id,
                candidate_times,
                tc,
            )
            .await
        })
        .await
    }

    async fn propose_locked(
        user_id: Uuid,
        tournament_id: Uuid,
        tournament_slot_id: Uuid,
        candidate_times: Vec<DateTime<Utc>>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Self::lock_schedulable_slot(user_id, tournament_id, tournament_slot_id, conn).await?;
        let now = Utc::now();
        if candidate_times.iter().any(|candidate| *candidate <= now) {
            return Err(DbError::InvalidAction {
                info: String::from("schedule candidates must be in the future"),
            });
        }

        let mut changed = diesel::update(
            schedule_offers::table
                .filter(schedule_offers::tournament_id.eq(tournament_id))
                .filter(schedule_offers::tournament_slot_id.eq(tournament_slot_id))
                .filter(schedule_offers::status.eq(ScheduleOfferStatus::Pending.as_str())),
        )
        .set((
            schedule_offers::status.eq(ScheduleOfferStatus::Superseded.as_str()),
            schedule_offers::resolved_at.eq(Some(now)),
            schedule_offers::resolved_by.eq(Some(user_id)),
        ))
        .returning(Self::as_returning())
        .get_results(conn)
        .await?;

        let offer = diesel::insert_into(schedule_offers::table)
            .values(NewScheduleOffer {
                id: Uuid::new_v4(),
                tournament_id,
                tournament_slot_id,
                proposer_id: user_id,
                candidate_times: candidate_times.into_iter().map(Some).collect(),
                status: ScheduleOfferStatus::Pending.as_str().to_owned(),
                selected_time: None,
                created_at: now,
                resolved_at: None,
                resolved_by: None,
                notified: false,
            })
            .returning(Self::as_returning())
            .get_result(conn)
            .await?;
        changed.push(offer);
        Ok(changed)
    }

    pub async fn accept(
        offer_id: Uuid,
        user_id: Uuid,
        selected_time: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        conn.transaction::<_, DbError, _>(async move |tc| {
            Self::accept_locked(offer_id, user_id, selected_time, tc).await
        })
        .await
    }

    async fn accept_locked(
        offer_id: Uuid,
        user_id: Uuid,
        selected_time: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let (tournament_id, tournament_slot_id) = Self::identity(offer_id, conn).await?;
        let slot =
            Self::lock_schedulable_slot(user_id, tournament_id, tournament_slot_id, conn).await?;
        let offer = Self::find_for_update(offer_id, conn).await?;
        if offer.status()? != ScheduleOfferStatus::Pending {
            return Err(inactive_offer());
        }
        if offer.tournament_id != tournament_id || offer.tournament_slot_id != tournament_slot_id {
            return Err(DbError::SerializationConflict);
        }
        if offer.proposer_id == user_id
            || Self::opponent_id(offer.proposer_id, slot.white, slot.black) != user_id
        {
            return Err(DbError::Unauthorized);
        }
        let candidates = offer.candidates()?;
        if !candidates.contains(&selected_time) {
            return Err(DbError::InvalidAction {
                info: String::from("the selected time is not part of this offer"),
            });
        }
        let now = Utc::now();
        if selected_time <= now {
            return Err(DbError::InvalidAction {
                info: String::from("the selected schedule time is no longer in the future"),
            });
        }

        diesel::update(
            schedule_offers::table
                .filter(schedule_offers::tournament_id.eq(tournament_id))
                .filter(schedule_offers::tournament_slot_id.eq(tournament_slot_id))
                .filter(schedule_offers::status.eq(ScheduleOfferStatus::Accepted.as_str()))
                .filter(schedule_offers::notified.eq(false)),
        )
        .set(schedule_offers::notified.eq(true))
        .execute(conn)
        .await?;
        let accepted = diesel::update(schedule_offers::table.find(offer_id))
            .set((
                schedule_offers::status.eq(ScheduleOfferStatus::Accepted.as_str()),
                schedule_offers::selected_time.eq(Some(selected_time)),
                schedule_offers::resolved_at.eq(Some(now)),
                schedule_offers::resolved_by.eq(Some(user_id)),
                schedule_offers::notified.eq(false),
            ))
            .returning(Self::as_returning())
            .get_result(conn)
            .await?;
        diesel::update(tournament_slots::table.find(tournament_slot_id))
            .set(tournament_slots::scheduled_at.eq(Some(selected_time)))
            .execute(conn)
            .await?;
        Ok(accepted)
    }

    pub async fn decline(
        offer_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Self::resolve_pending(
            offer_id,
            user_id,
            ScheduleOfferStatus::Declined,
            false,
            conn,
        )
        .await
    }

    pub async fn withdraw(
        offer_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Self::resolve_pending(
            offer_id,
            user_id,
            ScheduleOfferStatus::Withdrawn,
            true,
            conn,
        )
        .await
    }

    async fn resolve_pending(
        offer_id: Uuid,
        user_id: Uuid,
        next_status: ScheduleOfferStatus,
        proposer_action: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        conn.transaction::<_, DbError, _>(async move |tc| {
            Self::resolve_pending_locked(offer_id, user_id, next_status, proposer_action, tc).await
        })
        .await
    }

    async fn resolve_pending_locked(
        offer_id: Uuid,
        user_id: Uuid,
        next_status: ScheduleOfferStatus,
        proposer_action: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let (tournament_id, tournament_slot_id) = Self::identity(offer_id, conn).await?;
        let slot =
            Self::lock_schedulable_slot(user_id, tournament_id, tournament_slot_id, conn).await?;
        let offer = Self::find_for_update(offer_id, conn).await?;
        if offer.status()? != ScheduleOfferStatus::Pending {
            return Err(inactive_offer());
        }
        let opponent_id = Self::opponent_id(offer.proposer_id, slot.white, slot.black);
        let allowed = if proposer_action {
            offer.proposer_id == user_id
        } else {
            opponent_id == user_id
        };
        if !allowed {
            return Err(DbError::Unauthorized);
        }
        Ok(diesel::update(schedule_offers::table.find(offer_id))
            .set((
                schedule_offers::status.eq(next_status.as_str()),
                schedule_offers::resolved_at.eq(Some(Utc::now())),
                schedule_offers::resolved_by.eq(Some(user_id)),
            ))
            .returning(Self::as_returning())
            .get_result(conn)
            .await?)
    }

    pub async fn close_pending_for_slot(
        tournament_id: Uuid,
        tournament_slot_id: Uuid,
        resolved_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        diesel::update(tournament_slots::table.find(tournament_slot_id))
            .set(tournament_slots::scheduled_at.eq(Option::<DateTime<Utc>>::None))
            .execute(conn)
            .await?;
        Ok(diesel::update(
            schedule_offers::table
                .filter(schedule_offers::tournament_id.eq(tournament_id))
                .filter(schedule_offers::tournament_slot_id.eq(tournament_slot_id))
                .filter(schedule_offers::status.eq(ScheduleOfferStatus::Pending.as_str())),
        )
        .set((
            schedule_offers::status.eq(ScheduleOfferStatus::Cancelled.as_str()),
            schedule_offers::resolved_at.eq(Some(resolved_at)),
        ))
        .returning(Self::as_returning())
        .get_results(conn)
        .await?)
    }

    pub async fn purge_tournament(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::delete(
            schedule_offers::table.filter(schedule_offers::tournament_id.eq(tournament_id)),
        )
        .execute(conn)
        .await?;
        diesel::update(
            tournament_slots::table.filter(tournament_slots::tournament_id.eq(tournament_id)),
        )
        .set((
            tournament_slots::scheduled_at.eq(Option::<DateTime<Utc>>::None),
            tournament_slots::deadline_at.eq(Option::<DateTime<Utc>>::None),
        ))
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn find_for_tournament(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(schedule_offers::table
            .filter(schedule_offers::tournament_id.eq(tournament_id))
            .order((schedule_offers::created_at, schedule_offers::id))
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    pub async fn find_user_notifications(
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        let now = Utc::now();
        let offers = schedule_offers::table
            .inner_join(tournaments::table.on(schedule_offers::tournament_id.eq(tournaments::id)))
            .inner_join(
                tournament_slots::table.on(schedule_offers::tournament_id
                    .eq(tournament_slots::tournament_id)
                    .and(schedule_offers::tournament_slot_id.eq(tournament_slots::id))),
            )
            .filter(tournaments::started_at.is_not_null())
            .filter(tournaments::finished_at.is_null())
            .filter(
                tournament_slots::white_id
                    .eq(user_id)
                    .or(tournament_slots::black_id.eq(user_id)),
            )
            .filter(
                schedule_offers::status
                    .eq(ScheduleOfferStatus::Pending.as_str())
                    .and(schedule_offers::proposer_id.ne(user_id))
                    .or(schedule_offers::status
                        .eq(ScheduleOfferStatus::Accepted.as_str())
                        .and(schedule_offers::proposer_id.eq(user_id))
                        .and(schedule_offers::notified.eq(false))),
            )
            .select(Self::as_select())
            .load::<Self>(conn)
            .await?;
        Ok(offers
            .into_iter()
            .filter(|offer| {
                offer.status().is_ok_and(|status| match status {
                    ScheduleOfferStatus::Pending => offer
                        .candidates()
                        .is_ok_and(|candidates| candidates.into_iter().any(|time| time > now)),
                    ScheduleOfferStatus::Accepted => true,
                    _ => false,
                })
            })
            .collect())
    }

    pub async fn mark_notified(
        offer_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let rows_affected = diesel::update(schedule_offers::table.find(offer_id))
            .filter(schedule_offers::proposer_id.eq(user_id))
            .filter(schedule_offers::status.eq(ScheduleOfferStatus::Accepted.as_str()))
            .filter(schedule_offers::notified.eq(false))
            .set(schedule_offers::notified.eq(true))
            .execute(conn)
            .await?;
        if rows_affected == 0 {
            return Err(DbError::Unauthorized);
        }
        Ok(())
    }

    pub async fn get_upcoming_agreed_games(
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(Uuid, DateTime<Utc>)>, DbError> {
        let now = Utc::now();
        let one_week_later = now + Duration::weeks(1);
        Ok(tournament_slots::table
            .inner_join(
                games::table.on(tournament_slots::tournament_id
                    .eq(games::tournament_id.assume_not_null())
                    .and(tournament_slots::id.eq(games::tournament_slot_id.assume_not_null()))),
            )
            .filter(tournament_slots::resolution.is_null())
            .filter(tournament_slots::scheduled_at.between(now, one_week_later))
            .filter(games::finished.eq(false))
            .filter(games::turn.eq(0))
            .filter(games::game_status.eq(GameStatus::NotStarted.to_string()))
            .filter(games::game_start.eq(GameStart::Ready.to_string()))
            .select((games::id, tournament_slots::scheduled_at.assume_not_null()))
            .load(conn)
            .await?)
    }

    pub fn status(&self) -> Result<ScheduleOfferStatus, DbError> {
        ScheduleOfferStatus::try_from(self.status.as_str()).map_err(invalid_offer)
    }

    pub fn candidates(&self) -> Result<Vec<DateTime<Utc>>, DbError> {
        let candidates = self
            .candidate_times
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid_offer("candidate array contains null"))?;
        Ok(candidates)
    }

    pub fn opponent_id(proposer_id: Uuid, white_id: Uuid, black_id: Uuid) -> Uuid {
        if proposer_id == white_id {
            black_id
        } else {
            white_id
        }
    }

    async fn identity(offer_id: Uuid, conn: &mut DbConn<'_>) -> Result<(Uuid, Uuid), DbError> {
        Ok(schedule_offers::table
            .find(offer_id)
            .select((
                schedule_offers::tournament_id,
                schedule_offers::tournament_slot_id,
            ))
            .first(conn)
            .await?)
    }

    async fn find_for_update(offer_id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(schedule_offers::table
            .find(offer_id)
            .for_update()
            .select(Self::as_select())
            .first(conn)
            .await?)
    }

    async fn lock_schedulable_slot(
        user_id: Uuid,
        tournament_id: Uuid,
        tournament_slot_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<TournamentSlot, DbError> {
        let slot = TournamentSlot::find_for_update(tournament_id, tournament_slot_id, conn).await?;
        if user_id != slot.white && user_id != slot.black {
            return Err(DbError::Unauthorized);
        }
        if !matches!(slot.clock, Clock::Realtime(_)) {
            return Err(DbError::InvalidAction {
                info: String::from("only realtime tournament Slots can be scheduled"),
            });
        }
        if slot.resolution.is_some() {
            return Err(DbError::InvalidAction {
                info: String::from("a terminal tournament Slot cannot be scheduled"),
            });
        }
        Ok(slot)
    }
}

fn canonical_candidates(
    mut candidate_times: Vec<DateTime<Utc>>,
) -> Result<Vec<DateTime<Utc>>, DbError> {
    if !(1..=3).contains(&candidate_times.len()) {
        return Err(DbError::InvalidInput {
            info: String::from("a schedule offer must contain one to three times"),
            error: String::new(),
        });
    }
    candidate_times.sort_unstable();
    let original_len = candidate_times.len();
    candidate_times.dedup();
    if candidate_times.len() != original_len {
        return Err(DbError::InvalidInput {
            info: String::from("a schedule offer cannot contain duplicate times"),
            error: String::new(),
        });
    }
    Ok(candidate_times)
}

fn inactive_offer() -> DbError {
    DbError::InvalidAction {
        info: String::from("the schedule offer is no longer pending"),
    }
}

fn invalid_offer(reason: impl Display) -> DbError {
    DbError::InvalidPersistedTournament {
        reason: format!("invalid persisted schedule offer: {reason}"),
    }
}
