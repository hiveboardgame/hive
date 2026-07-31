use super::{Game, Schedule, TournamentInvitation};
use crate::{
    db_error::DbError,
    models::{
        tournament_organizer::TournamentOrganizer,
        tournament_user::TournamentUser,
        user::User,
    },
    schema::{
        games::{self, tournament_id as tournament_id_column},
        tournaments::{
            self,
            ends_at,
            name as name_column,
            nanoid as nanoid_field,
            series as series_column,
            started_at,
            starts_at,
            status as status_column,
            updated_at,
        },
        tournaments_organizers,
        tournaments_users,
        users,
    },
    DbConn,
};
use chrono::{prelude::*, TimeDelta};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::GameStatus;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::{
    Conclusion,
    TimeMode,
    TournamentDetails,
    TournamentGameResult,
    TournamentId,
    TournamentMode,
    TournamentSortOrder,
    TournamentStatus,
};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tournaments)]
pub struct NewTournament {
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    pub fully_automated: bool,
    pub third_place_match: bool,
    pub arena_duration_seconds: Option<i32>,
    pub points_win: Option<f64>,
    pub points_draw: Option<f64>,
    pub points_loss: Option<f64>,
    pub points_forfeit_loss: Option<f64>,
    pub points_zero_point_bye: Option<f64>,
    pub points_pairing_allocated_bye: Option<f64>,
}

impl NewTournament {
    pub fn new(details: TournamentDetails) -> Result<Self, DbError> {
        if matches!(details.time_mode, TimeMode::Untimed) {
            return Err(DbError::InvalidInput {
                info: String::from("How did you trigger this?"),
                error: String::from("Cannot create untimed tournament."),
            });
        }

        if details.time_mode == TimeMode::Correspondence && details.round_duration.is_some() {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("Cannot set round duration on correspondence tournaments"),
            });
        }

        if details.seats < details.min_seats {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("Seats is less than minimum number of seats"),
            });
        }

        let mode = TournamentMode::from_str(&details.mode).map_err(|_| {
            DbError::InvalidTournamentDetails {
                info: format!("{} is not a valid tournament mode", details.mode),
            }
        })?;

        // An arena ranks on points, then wins, then fewest games, and a bracket
        // on how far each player got — both are the engine's own orders, fixed
        // and already applied to the standings they return. Neither has a tie
        // left to break, so demanding one here would make them uncreatable.
        if details.tiebreakers.is_empty() && !mode.is_arena() && !mode.is_elimination() {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("No tiebreaker set"),
            });
        }

        // An arena opens on its clock and pairs whoever has turned up, so it has
        // no minimum: if nobody joins, nobody plays, and it ends when the clock
        // does. Every other format pairs a fixed field and needs two.
        if details.min_seats < 2 && !mode.is_arena() {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("A tournament needs at least 2 players"),
            });
        }
        if details.min_seats < 1 {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from("A tournament needs at least one seat"),
            });
        }

        // Only Swiss lets the organizer choose; every other format's round
        // count falls out of the field size, and is written back at start.
        let rounds = if mode.rounds_are_chosen() {
            if details.rounds < 1 {
                return Err(DbError::InvalidTournamentDetails {
                    info: String::from("Number of rounds needs to be >= 1"),
                });
            }
            if details.rounds > 16 {
                return Err(DbError::InvalidTournamentDetails {
                    info: String::from("Number of rounds needs to <= 16"),
                });
            }
            // Checked against the field that can actually start, not the
            // ceiling: `seats` is only an upper bound, and a tournament that
            // starts with `min_seats` players has no more pairings available
            // than that allows.
            if details.rounds >= details.min_seats {
                return Err(DbError::InvalidTournamentDetails {
                    info: String::from("A Swiss tournament needs more players than rounds"),
                });
            }
            details.rounds
        } else {
            0
        };

        // The engine doubles every value so a draw stays a whole number, which
        // is exactly why halves are as fine as a tournament can go.
        if !details.points.is_valid() {
            return Err(DbError::InvalidTournamentDetails {
                info: String::from(
                    "point values must be non-negative and in steps of half a point",
                ),
            });
        }

        // An arena is defined by how long it runs; nothing else bounds it.
        let arena_duration_seconds = if mode.is_arena() {
            match details.arena_duration_seconds {
                Some(seconds) if seconds > 0 => Some(seconds),
                _ => {
                    return Err(DbError::InvalidTournamentDetails {
                        info: String::from("An arena needs a positive duration"),
                    })
                }
            }
        } else {
            None
        };

        Ok(Self {
            nanoid: nanoid!(11),
            name: details.name,
            description: details.description,
            scoring: details.scoring.to_string(),
            tiebreaker: details
                .tiebreakers
                .iter()
                .flatten()
                .map(|t| Some(t.to_string()))
                .collect(),
            seats: details.seats,
            min_seats: details.min_seats,
            rounds,
            invite_only: details.invite_only,
            mode: details.mode,
            time_mode: details.time_mode.to_string(),
            time_base: details.time_base,
            time_increment: details.time_increment,
            band_upper: details.band_upper,
            band_lower: details.band_lower,
            start_mode: details.start_mode.to_string(),
            starts_at: details.starts_at,
            ends_at: None,
            started_at: None,
            round_duration: details.round_duration,
            status: TournamentStatus::NotStarted.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: details.series,
            fully_automated: details.fully_automated,
            third_place_match: details.third_place_match
                && mode == TournamentMode::SingleElimination,
            arena_duration_seconds,
            points_win: details.points.win,
            points_draw: details.points.draw,
            points_loss: details.points.loss,
            points_forfeit_loss: details.points.forfeit_loss,
            points_zero_point_bye: details.points.zero_point_bye,
            points_pairing_allocated_bye: details.points.pairing_allocated_bye,
        })
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = tournaments)]
pub struct Tournament {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub min_seats: i32,
    pub rounds: i32,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_mode: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub round_duration: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    /// Whether the background job advances rounds and finishes this
    /// tournament, rather than an organizer pressing the button.
    pub fully_automated: bool,
    pub third_place_match: bool,
    /// How long an arena runs for. Meaningless, and null, for every other mode.
    pub arena_duration_seconds: Option<i32>,
    /// The point system, in human units. Null means the mode default.
    pub points_win: Option<f64>,
    pub points_draw: Option<f64>,
    pub points_loss: Option<f64>,
    pub points_forfeit_loss: Option<f64>,
    pub points_zero_point_bye: Option<f64>,
    pub points_pairing_allocated_bye: Option<f64>,
}

impl Tournament {
    pub async fn create(
        user_id: Uuid,
        new_tournament: &NewTournament,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        // TODO: create only works when user's rating is RANKABLE
        let tournament: Tournament = diesel::insert_into(tournaments::table)
            .values(new_tournament)
            .get_result(conn)
            .await?;
        let tournament_organizer = TournamentOrganizer::new(tournament.id, user_id);
        diesel::insert_into(tournaments_organizers::table)
            .values(tournament_organizer)
            .execute(conn)
            .await?;
        Ok(tournament)
    }

    pub async fn delete(&mut self, user_id: Uuid, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer_or_admin(&user_id, conn)
            .await?;
        diesel::delete(tournaments::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete_old_and_unstarted(
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<TournamentId>, DbError> {
        use std::time::Duration;
        let cutoff = Utc::now() - Duration::from_secs(60 * 60 * 24 * 60);
        let now = Utc::now();
        let deleted_nanoids = diesel::delete(
            tournaments::table.filter(
                status_column
                    .eq(TournamentStatus::NotStarted.to_string())
                    .and(updated_at.lt(cutoff))
                    .and(starts_at.is_null().or(starts_at.lt(now))),
            ),
        )
        .returning(nanoid_field)
        .get_results::<String>(conn)
        .await?;
        Ok(deleted_nanoids.into_iter().map(TournamentId).collect())
    }

    pub(crate) async fn ensure_not_invite_only(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        if self.invite_only {
            if self
                .invitees(conn)
                .await?
                .iter()
                .any(|invitee| invitee.id == *user_id)
                || self
                    .organizers(conn)
                    .await?
                    .iter()
                    .any(|organizer| organizer.id == *user_id)
            {
                return Ok(());
            }
            return Err(DbError::TournamentInviteOnly);
        }
        Ok(())
    }

    pub(crate) async fn ensure_not_full(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if self.number_of_players(conn).await? == self.seats as i64 {
            return Err(DbError::TournamentFull);
        }
        Ok(())
    }

    fn ensure_not_started(&self) -> Result<(), DbError> {
        if self.status != TournamentStatus::NotStarted.to_string() {
            return Err(DbError::InvalidInput {
                info: format!("Tournament status is {}", self.status),
                error: String::from("Cannot start tournament a second time"),
            });
        }
        Ok(())
    }

    pub(crate) fn ensure_inprogress(&self) -> Result<(), DbError> {
        if self.status != TournamentStatus::InProgress.to_string() {
            return Err(DbError::InvalidInput {
                info: format!("Tournament status is {}", self.status),
                error: String::from("Cannot start tournament a second time"),
            });
        }
        Ok(())
    }

    pub async fn ensure_games_finished(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if self.number_of_games(conn).await? != self.number_of_finished_games(conn).await? {
            return Err(DbError::InvalidAction {
                info: String::from("Not all games have finished"),
            });
        }
        Ok(())
    }

    pub async fn ensure_user_is_organizer_or_admin(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let organizers = self.organizers(conn).await?;
        if organizers.iter().any(|o| o.id == *user_id) || User::is_admin(user_id, conn).await? {
            return Ok(());
        }
        Err(DbError::Unauthorized)
    }

    async fn has_enough_players(&self, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        // An arena is never waiting for a quorum — it runs against a wall clock
        // and admits players for as long as that clock lasts. Holding it back
        // until somebody joins would mean it never opens for them to join.
        if self.mode()?.is_arena() {
            return Ok(true);
        }
        Ok(self.number_of_players(conn).await? >= self.min_seats as i64)
    }

    pub async fn create_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;
        if TournamentInvitation::exists(&self.id, invitee, conn).await? {
            return Ok(self.clone());
        }
        let invitation = TournamentInvitation::new(self.id, *invitee);
        invitation.insert(conn).await?;
        Ok(self.clone())
    }

    pub async fn finish(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_inprogress()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;
        self.ensure_games_finished(conn).await?;
        self.finish_unchecked(conn).await
    }

    /// The `fully_automated` path, driven by the job rather than by an
    /// organizer's click — so there is nobody to authorize against.
    pub async fn finish_automatically(&self, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        self.ensure_inprogress()?;
        self.ensure_games_finished(conn).await?;
        self.finish_unchecked(conn).await
    }

    async fn finish_unchecked(&self, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        let tournament = diesel::update(tournaments::table.find(self.id))
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::Finished.to_string()),
            ))
            .get_result(conn)
            .await?;
        Ok(tournament)
    }

    pub async fn double_forfeit_unstarted_games(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        self.ensure_inprogress()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;

        // A two-game match cannot stand on a drawn result, and a double forfeit
        // is scored as one: this would put every affected pairing into a replay
        // loop rather than clearing it. That is true of Double-Swiss just as
        // much as a bracket. An organizer unsticking one has to adjudicate a
        // winner per game instead.
        if self.mode()?.is_two_game_match() {
            return Err(DbError::InvalidAction {
                info: String::from(
                    "a two-game match needs a decisive result; adjudicate the games instead",
                ),
            });
        }

        let unstarted_game_ids = self
            .game_ids_with_status(GameStatus::NotStarted, conn)
            .await?;

        if unstarted_game_ids.is_empty() {
            return Ok(0);
        }

        let updated_games =
            diesel::update(games::table.filter(games::id.eq_any(&unstarted_game_ids)))
                .set((
                    games::finished.eq(true),
                    games::conclusion.eq(Conclusion::Forfeit.to_string()),
                    games::game_status.eq(GameStatus::Adjudicated.to_string()),
                    games::tournament_game_result
                        .eq(TournamentGameResult::DoubeForfeit.to_string()),
                    games::updated_at.eq(Utc::now()),
                    games::last_interaction.eq(Utc::now()),
                    games::timeout_at.eq(crate::models::game::CLEAR_TIMEOUT_AT),
                    games::finished_at.eq(Utc::now()),
                ))
                .execute(conn)
                .await?;

        Schedule::delete_for_games(&unstarted_game_ids, conn).await?;

        Ok(updated_games)
    }

    pub async fn reset_adjudicated_games(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        self.ensure_inprogress()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;

        // A withdrawal forfeit is adjudicated too, but it is the engine's doing
        // rather than an organizer's and the field still reflects it: clearing
        // one leaves a game nobody will play, and overwrites the very
        // conclusion `restore_withdrawn_games` finds it by, so there would be
        // no way back. Committee stays in — that is what an organizer's own
        // adjudication writes, which is exactly what this undoes.
        let unstarted_game_ids: Vec<Uuid> = games::table
            .filter(tournament_id_column.eq(self.id))
            .filter(games::game_status.eq(GameStatus::Adjudicated.to_string()))
            .filter(games::conclusion.ne(Conclusion::Withdrawal.to_string()))
            .select(games::id)
            .get_results(conn)
            .await?;

        if unstarted_game_ids.is_empty() {
            return Ok(0);
        }

        let updated_games =
            diesel::update(games::table.filter(games::id.eq_any(&unstarted_game_ids)))
                .set((
                    games::finished.eq(false),
                    games::conclusion.eq(Conclusion::Unknown.to_string()),
                    games::game_status.eq(GameStatus::NotStarted.to_string()),
                    games::tournament_game_result.eq(TournamentGameResult::Unknown.to_string()),
                    games::updated_at.eq(Utc::now()),
                    games::last_interaction.eq::<Option<DateTime<Utc>>>(None),
                    games::turn.eq(0),
                    games::timeout_at.eq(crate::models::game::CLEAR_TIMEOUT_AT),
                    games::finished_at.eq::<Option<DateTime<Utc>>>(None),
                ))
                .execute(conn)
                .await?;

        Schedule::delete_for_games(&unstarted_game_ids, conn).await?;

        Ok(updated_games)
    }

    async fn game_ids_with_status(
        &self,
        status: GameStatus,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        Ok(games::table
            .filter(tournament_id_column.eq(self.id))
            .filter(games::game_status.eq(status.to_string()))
            .select(games::id)
            .get_results(conn)
            .await?)
    }

    pub async fn retract_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, invitee, conn).await {
            invitation.delete(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn decline_invitation(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            // Marked, not deleted: an organizer needs to see that somebody said
            // no, which is different from an invitation still sitting unopened.
            invitation.decline(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn accept_invitation(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_not_full(conn).await?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
            let tournament_user = TournamentUser::new(self.id, *user_id);
            tournament_user.insert(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn add_to_series(
        &self,
        series_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(Some(series_id)))
            .get_result(conn)
            .await?)
    }

    pub async fn remove_from_series(&self, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(None::<Uuid>))
            .get_result(conn)
            .await?)
    }

    pub async fn join(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_not_full(conn).await?;
        self.ensure_not_invite_only(user_id, conn).await?;
        let players = self.players(conn).await?;
        if players.len() == self.seats as usize {
            return Ok(self.clone());
        }
        if players.iter().any(|player| player.id == *user_id) {
            return Ok(self.clone());
        }
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
        }
        let tournament_user = TournamentUser::new(self.id, *user_id);
        tournament_user.insert(conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn leave(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        self.ensure_not_started()?;
        TournamentUser::delete(self.id, *user_id, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn update_description(
        &self,
        user_id: &Uuid,
        description: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;

        Ok(diesel::update(tournaments::table.find(self.id))
            .set((
                tournaments::description.eq(description),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn kick(
        &self,
        organizer: &Uuid,
        player: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer_or_admin(organizer, conn)
            .await?;
        TournamentUser::delete(self.id, *player, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn from_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table.find(uuid).first(conn).await?)
    }

    pub async fn find_by_uuid(uuid: Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table.find(uuid).first(conn).await?)
    }

    pub async fn find_by_uuids(
        uuids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        Ok(tournaments::table
            .filter(tournaments::id.eq_any(uuids))
            .load(conn)
            .await?)
    }

    pub async fn from_nanoid(nano: &str, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table
            .filter(nanoid_field.eq(nano))
            .first(conn)
            .await?)
    }

    /// Invitations still outstanding. Somebody who declined is no longer
    /// waiting on anything, so they belong in `declined_invitees` instead.
    pub async fn invitees(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentInvitation::belonging_to(self)
            .inner_join(users::table)
            .filter(crate::schema::tournaments_invitations::declined_at.is_null())
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn declined_invitees(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentInvitation::belonging_to(self)
            .inner_join(users::table)
            .filter(crate::schema::tournaments_invitations::declined_at.is_not_null())
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn players(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentUser::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    /// Players who left mid-event.
    ///
    /// They keep their row and everything they already scored, so nothing else
    /// distinguishes them from a player who simply has no game in the current
    /// round — which is why a view has to be told explicitly.
    pub async fn withdrawn_players(&self, conn: &mut DbConn<'_>) -> Result<Vec<Uuid>, DbError> {
        Ok(tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(self.id))
            .filter(tournaments_users::withdrawn_at.is_not_null())
            .select(tournaments_users::user_id)
            .get_results(conn)
            .await?)
    }

    pub async fn number_of_players(&self, conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(TournamentUser::belonging_to(self)
            .inner_join(users::table)
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn number_of_games(&self, conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(games::table
            .filter(games::tournament_id.eq(self.id))
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn number_of_finished_games(&self, conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(games::table
            .filter(
                games::tournament_id
                    .eq(self.id)
                    .and(games::finished.eq(true)),
            )
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn organizers(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentOrganizer::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn games(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(tournament_id_column.eq(Some(self.id)))
            .get_results(conn)
            .await?)
    }

    pub async fn unfinished_games_for_user_locked(
        &self,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(tournament_id_column.eq(Some(self.id)))
            .filter(games::finished.eq(false))
            .filter(games::white_id.eq(user_id).or(games::black_id.eq(user_id)))
            .order(games::id.asc())
            .for_update()
            .get_results(conn)
            .await?)
    }

    pub async fn start_by_organizer(
        &self,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>, Vec<Uuid>), DbError> {
        self.ensure_user_is_organizer_or_admin(organizer, conn)
            .await?;
        self.start(conn).await
    }

    pub async fn start(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>, Vec<Uuid>), DbError> {
        self.ensure_not_started()?;
        if !self.has_enough_players(conn).await? {
            return Err(DbError::NotEnoughPlayers);
        }
        // Validation could only check `min_seats`; this is the first point the
        // real field is known. A Swiss with more rounds than players runs out
        // of legal pairings partway through, and there is no recovery from that
        // once it has started.
        if self.mode()?.rounds_are_chosen() {
            let players = self.number_of_players(conn).await?;
            if self.rounds as i64 >= players {
                return Err(DbError::InvalidAction {
                    info: format!(
                        "a {}-round Swiss needs more than {players} players",
                        self.rounds
                    ),
                });
            }
        }
        let ends = if let Some(days) = self.round_duration {
            let days = TimeDelta::days(days as i64);
            Some(Utc::now() + days)
        } else {
            None
        };
        let mut deleted_invitees = Vec::new();
        let games = self.create_initial_games(conn).await?;
        let tournament: Tournament = diesel::update(self)
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::InProgress.to_string()),
                started_at.eq(Utc::now()),
                ends_at.eq(ends),
            ))
            .get_result(conn)
            .await?;
        let invitations: Vec<TournamentInvitation> = TournamentInvitation::belonging_to(self)
            .get_results(conn)
            .await?;
        for invitation in invitations.iter() {
            deleted_invitees.push(invitation.invitee_id);
            invitation.delete(conn).await?;
        }
        Ok((tournament, games, deleted_invitees))
    }

    pub async fn find(id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(tournaments::table.find(id).first(conn).await?)
    }

    pub async fn find_by_tournament_id(
        tournament_id: &TournamentId,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        let TournamentId(id) = tournament_id;
        Ok(tournaments::table
            .filter(nanoid_field.eq(id))
            .first(conn)
            .await?)
    }

    pub async fn unstarted(conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        let potential_tournaments: Vec<Tournament> = tournaments::table
            .filter(status_column.eq(TournamentStatus::NotStarted.to_string()))
            .filter(starts_at.le(Utc::now()))
            .get_results(conn)
            .await?;
        let mut tournaments = Vec::new();
        for tournament in potential_tournaments {
            if tournament.has_enough_players(conn).await? {
                tournaments.push(tournament);
            }
        }
        Ok(tournaments)
    }

    /// Starts every tournament whose time has come, each in its own
    /// transaction.
    ///
    /// One that cannot start must not stop the others: a failed statement
    /// poisons the surrounding Postgres transaction, so sharing one would mean
    /// a single bad tournament silently prevents every other scheduled
    /// tournament on the site from ever starting.
    pub async fn automatic_start(
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(Tournament, Vec<Game>, Vec<Uuid>)>, DbError> {
        let mut started_tournaments = Vec::new();
        for tournament in Tournament::unstarted(conn).await? {
            let nanoid = tournament.nanoid.clone();
            let started = conn
                .transaction::<_, DbError, _>(async move |tc| tournament.start(tc).await)
                .await;
            match started {
                Ok(started) => started_tournaments.push(started),
                Err(error) => {
                    tracing::error!(
                        tournament = %nanoid,
                        %error,
                        "could not start this tournament; skipping it",
                    );
                }
            }
        }
        Ok(started_tournaments)
    }

    /// `tournaments.name` is unique, so this lets the form say so before
    /// submitting rather than surfacing a raw constraint violation.
    pub async fn name_exists(name: &str, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        Ok(diesel::select(diesel::dsl::exists(
            tournaments::table.filter(name_column.eq(name)),
        ))
        .get_result(conn)
        .await?)
    }

    pub async fn find_by_tournament_ids(
        tournament_ids: &[TournamentId],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        let nanoids: Vec<&str> = tournament_ids
            .iter()
            .map(|TournamentId(id)| id.as_str())
            .collect();
        Ok(tournaments::table
            .filter(nanoid_field.eq_any(nanoids))
            .get_results(conn)
            .await?)
    }

    pub async fn get_all(
        sort_order: TournamentSortOrder,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        let query = tournaments::table.into_boxed();
        let sorted_query = match sort_order {
            TournamentSortOrder::CreatedAtDesc => query.order(tournaments::created_at.desc()),
            TournamentSortOrder::CreatedAtAsc => query.order(tournaments::created_at.asc()),
            TournamentSortOrder::StartedAtDesc => query.order(tournaments::started_at.desc()),
            TournamentSortOrder::StartedAtAsc => query.order(tournaments::started_at.asc()),
        };
        Ok(sorted_query.get_results(conn).await?)
    }

    pub async fn get_by_status(
        status: TournamentStatus,
        sort_order: TournamentSortOrder,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        let query = tournaments::table
            .filter(status_column.eq(status.to_string()))
            .into_boxed();
        let sorted_query = match sort_order {
            TournamentSortOrder::CreatedAtDesc => query.order(tournaments::created_at.desc()),
            TournamentSortOrder::CreatedAtAsc => query.order(tournaments::created_at.asc()),
            TournamentSortOrder::StartedAtDesc => query.order(tournaments::started_at.desc()),
            TournamentSortOrder::StartedAtAsc => query.order(tournaments::started_at.asc()),
        };
        Ok(sorted_query.get_results(conn).await?)
    }

    pub async fn get_hosting_tournaments(
        user_id: &Uuid,
        sort_order: TournamentSortOrder,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        let mut query = tournaments::table
            .inner_join(tournaments_organizers::table)
            .filter(tournaments_organizers::organizer_id.eq(user_id))
            .select(tournaments::all_columns)
            .order_by(tournaments::status.eq("NotStarted").desc())
            .then_order_by(tournaments::status.eq("InProgress").desc())
            .then_order_by(tournaments::status.eq("Finished").desc())
            .into_boxed();

        query = match sort_order {
            TournamentSortOrder::CreatedAtDesc => {
                query.then_order_by(tournaments::created_at.desc())
            }
            TournamentSortOrder::CreatedAtAsc => query.then_order_by(tournaments::created_at.asc()),
            TournamentSortOrder::StartedAtDesc => {
                query.then_order_by(tournaments::started_at.desc())
            }
            TournamentSortOrder::StartedAtAsc => query.then_order_by(tournaments::started_at.asc()),
        };

        Ok(query.get_results(conn).await?)
    }

    pub async fn get_joined_tournaments(
        user_id: &Uuid,
        sort_order: TournamentSortOrder,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Tournament>, DbError> {
        use crate::schema::tournaments_users;
        let mut query = tournaments::table
            .inner_join(tournaments_users::table)
            .filter(tournaments_users::user_id.eq(user_id))
            .select(tournaments::all_columns)
            .order_by(tournaments::status.eq("NotStarted").desc())
            .then_order_by(tournaments::status.eq("InProgress").desc())
            .then_order_by(tournaments::status.eq("Finished").desc())
            .into_boxed();

        query = match sort_order {
            TournamentSortOrder::CreatedAtDesc => {
                query.then_order_by(tournaments::created_at.desc())
            }
            TournamentSortOrder::CreatedAtAsc => query.then_order_by(tournaments::created_at.asc()),
            TournamentSortOrder::StartedAtDesc => {
                query.then_order_by(tournaments::started_at.desc())
            }
            TournamentSortOrder::StartedAtAsc => query.then_order_by(tournaments::started_at.asc()),
        };

        Ok(query.get_results(conn).await?)
    }
}
