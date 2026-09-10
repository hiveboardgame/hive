use super::{Game, Rating, TournamentInvitation};
use crate::{
    db_error::DbError,
    models::{
        tournament_organizer::TournamentOrganizer,
        tournament_user::TournamentUser,
        user::User,
    },
    schema::{
        games,
        tournaments::{self, nanoid as nanoid_field, starts_at as starts_at_column, updated_at},
        tournaments_organizer_invitations,
        tournaments_organizers,
        tournaments_users,
        users,
    },
    tournaments::validate_creation_configuration,
    DbConn,
};
use chrono::prelude::*;
use diesel::{
    deserialize::{FromSql, Result as DeserializeResult},
    dsl::sql,
    pg::{Pg, PgValue},
    prelude::*,
    sql_types::{Jsonb, Text},
    AsExpression,
    FromSqlRow,
};
use diesel_async::RunQueryDsl;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use shared_types::{
    tournament::{
        swiss::RoundConfiguration as SwissRoundConfiguration,
        BotAdmission,
        Config,
        Format,
        FormatConfig,
        MAX_ELIMINATION_SEATS,
        MAX_ROUND_ROBIN_SEATS,
        MAX_SWISS_SEATS,
        MIN_SWISS_START_SEATS,
    },
    Clock,
    GameSpeed,
    TournamentDetails,
    TournamentId,
    TournamentStartSetup,
    TournamentStatus,
};
use std::error::Error as StdError;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tournaments)]
pub struct NewTournament {
    pub nanoid: String,
    pub name: String,
    pub description: Option<String>,
    pub seats: Option<i32>,
    pub min_seats: i32,
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    pub configuration: JsonValue,
    pub finished_at: Option<DateTime<Utc>>,
    pub featured_game_id: Option<Uuid>,
    pub start_setup: Option<JsonValue>,
    pub bracket_order: Option<JsonValue>,
}

impl NewTournament {
    pub fn new(details: TournamentDetails) -> Result<Self, DbError> {
        let TournamentDetails {
            name,
            description,
            seats,
            min_seats,
            invite_only,
            band_upper,
            band_lower,
            starts_at,
            configuration,
        } = details;
        let invalid = |info: &str| DbError::InvalidTournamentDetails {
            info: info.to_owned(),
        };
        if !(4..=50).contains(&name.chars().count()) {
            return Err(invalid("the name must contain between 4 and 50 characters"));
        }
        let description = normalize_tournament_description(description)?;
        let format = configuration.format();
        if format == Format::Arena {
            if seats.is_some() {
                return Err(invalid("Arena tournaments do not have a participant limit"));
            }
            if invite_only {
                return Err(invalid("Arena tournaments cannot be invite-only"));
            }
            if min_seats != 0 {
                return Err(invalid("Arena tournaments do not have a minimum field"));
            }
        }
        let maximum_seats = match format {
            Format::RoundRobin => Some(MAX_ROUND_ROBIN_SEATS),
            Format::Swiss | Format::DoubleSwiss => Some(MAX_SWISS_SEATS),
            Format::SingleElimination | Format::DoubleElimination => Some(MAX_ELIMINATION_SEATS),
            Format::Arena => None,
        };
        if let Some(maximum_seats) = maximum_seats {
            let seats =
                seats.ok_or_else(|| invalid("fixed-field tournaments require a capacity"))?;
            if seats > maximum_seats {
                return Err(invalid(
                    "the participant limit exceeds the format safety cap",
                ));
            }
            if seats <= 0 || min_seats < 0 || min_seats > seats {
                return Err(invalid("invalid tournament seat limits"));
            }
        }
        if let FormatConfig::Swiss(swiss) = &configuration.format {
            if min_seats != MIN_SWISS_START_SEATS {
                return Err(invalid(
                    "a Swiss tournament uses the system-owned five-entrant minimum",
                ));
            }
            if !matches!(swiss.rounds, SwissRoundConfiguration::Automatic { .. }) {
                return Err(invalid(
                    "new Swiss tournaments require automatic round configuration",
                ));
            }
        }
        if starts_at.is_some_and(|starts_at| starts_at <= Utc::now()) {
            return Err(invalid("the scheduled start must be in the future"));
        }
        if format == Format::Arena && starts_at.is_none() {
            return Err(invalid("Arena tournaments require a scheduled start"));
        }

        if format != Format::Arena && min_seats < 2 {
            return Err(invalid(
                "a fixed-field tournament needs at least two entrants",
            ));
        }
        if band_lower
            .zip(band_upper)
            .is_some_and(|(lower, upper)| lower > upper)
        {
            return Err(invalid(
                "the lower rating band exceeds the upper rating band",
            ));
        }
        let participant_count = min_seats as usize;
        validate_creation_configuration(&configuration, participant_count).map_err(|error| {
            invalid(&format!(
                "configuration cannot initialize Tournamint: {error}"
            ))
        })?;
        let persisted_configuration = serde_json::to_value(&configuration).map_err(|error| {
            DbError::InvalidTournamentDetails {
                info: format!("configuration is not persistable: {error}"),
            }
        })?;
        let now = Utc::now();
        Ok(Self {
            nanoid: nanoid!(11),
            name,
            description,
            seats,
            min_seats,
            invite_only,
            band_upper,
            band_lower,
            starts_at,
            started_at: None,
            created_at: now,
            updated_at: now,
            series: None,
            configuration: persisted_configuration,
            finished_at: None,
            featured_game_id: None,
            start_setup: None,
            bracket_order: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(transparent)]
pub struct PersistedConfig(Config);

#[derive(Debug, thiserror::Error)]
#[error("invalid persisted tournament configuration: {0}")]
pub(crate) struct PersistedConfigError(pub(crate) String);

impl FromSql<Jsonb, Pg> for PersistedConfig {
    fn from_sql(value: PgValue<'_>) -> DeserializeResult<Self> {
        let value = <JsonValue as FromSql<Jsonb, Pg>>::from_sql(value)?;
        serde_json::from_value(value).map(Self).map_err(|error| {
            Box::new(PersistedConfigError(error.to_string())) as Box<dyn StdError + Send + Sync>
        })
    }
}

#[derive(Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, Selectable)]
#[diesel(primary_key(id))]
#[diesel(table_name = tournaments)]
pub struct Tournament {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: Option<String>,
    pub seats: Option<i32>,
    pub min_seats: i32,
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
    pub configuration: PersistedConfig,
    pub finished_at: Option<DateTime<Utc>>,
    pub featured_game_id: Option<Uuid>,
    pub start_setup: Option<JsonValue>,
    pub bracket_order: Option<JsonValue>,
}

#[derive(Debug)]
pub struct InvitationCreateOutcome {
    pub tournament: Tournament,
    pub changed: bool,
}

impl Tournament {
    pub fn configuration(&self) -> &Config {
        &self.configuration.0
    }

    pub fn start_setup(&self) -> Option<TournamentStartSetup> {
        self.start_setup.as_ref().map(|value| {
            serde_json::from_value(value.clone()).expect("valid persisted tournament start setup")
        })
    }

    pub fn bracket_order(&self) -> Option<Vec<Uuid>> {
        self.bracket_order.as_ref().map(|value| {
            serde_json::from_value(value.clone()).expect("valid persisted tournament bracket order")
        })
    }

    pub const fn status(&self) -> TournamentStatus {
        if self.finished_at.is_some() {
            TournamentStatus::Finished
        } else if self.started_at.is_some() {
            TournamentStatus::InProgress
        } else {
            TournamentStatus::NotStarted
        }
    }

    pub async fn create(
        user_id: Uuid,
        new_tournament: &NewTournament,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        User::find_active_by_uuid(&user_id, conn).await?;
        if new_tournament.started_at.is_some() || new_tournament.finished_at.is_some() {
            return Err(DbError::InvalidAction {
                info: String::from("Tournament creation cannot bypass the atomic start lifecycle"),
            });
        }
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
        self.ensure_pre_start_changes_open()?;
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
            tournaments::table
                .filter(tournaments::started_at.is_null())
                .filter(tournaments::finished_at.is_null())
                .filter(updated_at.lt(cutoff))
                .filter(starts_at_column.is_null().or(starts_at_column.lt(now))),
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
            let organizer_exists = diesel::select(diesel::dsl::exists(
                tournaments_organizers::table.find((self.id, *user_id)),
            ))
            .get_result(conn)
            .await?;
            if TournamentInvitation::exists(&self.id, user_id, conn).await? || organizer_exists {
                return Ok(());
            }
            return Err(DbError::TournamentInviteOnly);
        }
        Ok(())
    }

    pub(crate) async fn ensure_not_full(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        let Some(capacity) = self.seats else {
            return Ok(());
        };
        if self.number_of_players(conn).await? >= i64::from(capacity) {
            return Err(DbError::TournamentFull);
        }
        Ok(())
    }

    fn ensure_not_started(&self) -> Result<(), DbError> {
        if self.status() != TournamentStatus::NotStarted {
            return Err(DbError::InvalidInput {
                info: format!("Tournament status is {:?}", self.status()),
                error: String::from("Cannot start tournament a second time"),
            });
        }
        Ok(())
    }

    fn ensure_pre_start_changes_open(&self) -> Result<(), DbError> {
        self.ensure_pre_start_changes_open_at(Utc::now())
    }

    fn ensure_pre_start_changes_open_at(&self, observed_at: DateTime<Utc>) -> Result<(), DbError> {
        self.ensure_not_started()?;
        if self
            .start_setup()
            .is_some_and(|setup| setup.active_at(observed_at))
        {
            return Err(DbError::InvalidAction {
                info: String::from(
                    "The roster is locked while the tournament start is being prepared",
                ),
            });
        }
        if self
            .starts_at
            .is_some_and(|scheduled_start| observed_at >= scheduled_start)
        {
            return Err(DbError::InvalidAction {
                info: String::from(
                    "scheduled tournament configuration and signup are closed at starts_at",
                ),
            });
        }
        Ok(())
    }

    fn ensure_arena_does_not_support(&self, action: &str) -> Result<(), DbError> {
        if self.configuration().format() == Format::Arena {
            return Err(DbError::InvalidAction {
                info: format!("Arena tournaments do not support {action}"),
            });
        }
        Ok(())
    }

    fn accepted_membership(
        &self,
        user_id: Uuid,
        accepted_at: DateTime<Utc>,
    ) -> Result<TournamentUser, DbError> {
        if self.configuration().format() == Format::Arena {
            Ok(TournamentUser::accepted_for_arena(
                self.id,
                user_id,
                accepted_at,
            ))
        } else {
            Ok(TournamentUser::accepted_at(self.id, user_id, accepted_at))
        }
    }

    pub(crate) fn ensure_rating_in_band(&self, rating: f64) -> Result<(), DbError> {
        let below = self
            .band_lower
            .is_some_and(|lower| rating < f64::from(lower));
        let above = self
            .band_upper
            .is_some_and(|upper| rating > f64::from(upper));
        if below || above {
            return Err(DbError::InvalidAction {
                info: String::from("Your rating is outside this tournament's rating band"),
            });
        }
        Ok(())
    }

    async fn ensure_user_rating_in_band(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let clock = match &self.configuration().format {
            FormatConfig::RoundRobin(configuration) => configuration.clock,
            FormatConfig::Swiss(configuration) => configuration.clock,
            FormatConfig::Elimination(configuration) => configuration
                .default_plan
                .phases
                .as_slice()
                .first()
                .map(|phase| phase.clock)
                .ok_or_else(|| DbError::InvalidPersistedTournament {
                    reason: String::from("elimination series plan has no opening phase"),
                })?,
            FormatConfig::Arena(configuration) => Clock::Realtime(configuration.game_clock),
        };
        let rating = Rating::for_uuid(user_id, &GameSpeed::from(clock), conn).await?;
        self.ensure_rating_in_band(rating.rating)
    }

    pub(crate) fn ensure_user_bot_admission(&self, user: &User) -> Result<(), DbError> {
        let admitted = match self.configuration().bot_admission {
            BotAdmission::HumansOnly => !user.bot,
            BotAdmission::HumansAndBots => true,
            BotAdmission::BotsOnly => user.bot,
        };
        if admitted {
            Ok(())
        } else {
            Err(DbError::InvalidAction {
                info: String::from("This tournament does not admit this account type"),
            })
        }
    }

    pub async fn ensure_user_is_organizer_or_admin(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let user = User::find_active_by_uuid(user_id, conn).await?;
        let organizer_exists = diesel::select(diesel::dsl::exists(
            tournaments_organizers::table.find((self.id, *user_id)),
        ))
        .get_result(conn)
        .await?;
        if user.admin || organizer_exists {
            return Ok(());
        }
        Err(DbError::Unauthorized)
    }

    pub async fn create_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<InvitationCreateOutcome, DbError> {
        self.ensure_arena_does_not_support("invitations")?;
        self.ensure_pre_start_changes_open()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;
        if TournamentInvitation::exists(&self.id, invitee, conn).await? {
            return Ok(InvitationCreateOutcome {
                tournament: self.clone(),
                changed: false,
            });
        }
        if TournamentUser::contains(self.id, *invitee, conn).await? {
            return Ok(InvitationCreateOutcome {
                tournament: self.clone(),
                changed: false,
            });
        }
        self.ensure_not_full(conn).await?;
        let invitee_user = User::find_active_by_uuid(invitee, conn).await?;
        self.ensure_user_bot_admission(&invitee_user)?;
        self.ensure_user_rating_in_band(invitee, conn).await?;
        let invitation = TournamentInvitation::new(self.id, *invitee);
        invitation.insert(conn).await?;
        Ok(InvitationCreateOutcome {
            tournament: self.clone(),
            changed: true,
        })
    }

    pub async fn retract_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_arena_does_not_support("invitations")?;
        self.ensure_pre_start_changes_open()?;
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;
        User::find_active_by_uuid(invitee, conn).await?;
        if let Some(invitation) =
            TournamentInvitation::find_active_by_ids(&self.id, invitee, conn).await?
        {
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
        self.ensure_arena_does_not_support("invitations")?;
        self.ensure_pre_start_changes_open()?;
        User::find_active_by_uuid(user_id, conn).await?;
        if let Some(invitation) =
            TournamentInvitation::find_active_by_ids(&self.id, user_id, conn).await?
        {
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
        self.ensure_arena_does_not_support("invitations")?;
        let accepted_at = Utc::now();
        self.ensure_pre_start_changes_open_at(accepted_at)?;
        let user = User::find_active_by_uuid(user_id, conn).await?;
        self.ensure_user_bot_admission(&user)?;
        self.ensure_user_rating_in_band(user_id, conn).await?;
        self.ensure_not_full(conn).await?;
        if let Some(invitation) =
            TournamentInvitation::find_active_by_ids(&self.id, user_id, conn).await?
        {
            invitation.delete(conn).await?;
            let tournament_user = self.accepted_membership(*user_id, accepted_at)?;
            tournament_user.insert(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn join(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        let accepted_at = Utc::now();
        self.ensure_pre_start_changes_open_at(accepted_at)?;
        let user = User::find_active_by_uuid(user_id, conn).await?;
        self.ensure_user_bot_admission(&user)?;
        if TournamentUser::contains(self.id, *user_id, conn).await? {
            return Ok(self.clone());
        }
        self.ensure_not_full(conn).await?;
        self.ensure_not_invite_only(user_id, conn).await?;
        self.ensure_user_rating_in_band(user_id, conn).await?;
        if let Some(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await?
        {
            invitation.delete(conn).await?;
        }
        let tournament_user = self.accepted_membership(*user_id, accepted_at)?;
        tournament_user.insert(conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn leave(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        self.ensure_pre_start_changes_open()?;
        User::find_active_by_uuid(user_id, conn).await?;
        TournamentUser::delete(self.id, *user_id, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn update_description(
        &self,
        user_id: &Uuid,
        description: Option<String>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        self.ensure_user_is_organizer_or_admin(user_id, conn)
            .await?;
        let description = normalize_tournament_description(description)?;

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
        self.ensure_arena_does_not_support("kicking participants")?;
        self.ensure_pre_start_changes_open()?;
        self.ensure_user_is_organizer_or_admin(organizer, conn)
            .await?;
        TournamentUser::delete(self.id, *player, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
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

    /// Returns advisory scheduled-start candidates for the requested formats.
    /// Each start attempt must re-lock and check every returned row.
    pub(crate) async fn due_scheduled_start_ids(
        formats: &[Format],
        as_of: DateTime<Utc>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        if limit <= 0 || formats.is_empty() {
            return Ok(Vec::new());
        }
        let candidates = tournaments::table
            .filter(tournaments::started_at.is_null())
            .filter(tournaments::finished_at.is_null())
            .filter(tournaments::starts_at.is_not_null())
            .filter(tournaments::starts_at.le(as_of))
            .filter(
                sql::<Text>("configuration #>> '{format,format}'")
                    .eq_any(formats.iter().map(|format| format.family_tag())),
            )
            .order((tournaments::starts_at.asc(), tournaments::id.asc()))
            .select(tournaments::id)
            .limit(limit)
            .load::<Uuid>(conn)
            .await?;
        Ok(candidates)
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
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn organizers(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentOrganizer::belonging_to(self)
            .inner_join(users::table)
            .filter(users::deleted.eq(false))
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub(crate) fn ensure_organizer_changes_open(&self) -> Result<(), DbError> {
        if self.finished_at.is_some() {
            return Err(DbError::InvalidAction {
                info: String::from("Organizer roles cannot change after the tournament finishes"),
            });
        }
        Ok(())
    }

    pub(crate) async fn has_active_game_for(
        &self,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        diesel::select(diesel::dsl::exists(
            games::table
                .filter(games::tournament_id.eq(self.id))
                .filter(games::finished.eq(false))
                .filter(games::white_id.eq(user_id).or(games::black_id.eq(user_id))),
        ))
        .get_result(conn)
        .await
        .map_err(DbError::from)
    }

    pub async fn games(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(games::tournament_id.eq(self.id))
            .select(Game::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn find(id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(tournaments::table.find(id).first(conn).await?)
    }

    /// Locks the authoritative tournament row by its database identity.
    /// Callers must keep the surrounding transaction open through every
    /// lifecycle, membership, format-fact, and slot write derived from this row.
    pub(crate) async fn find_for_update(id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(tournaments::table.find(id).for_update().first(conn).await?)
    }

    /// Cancels a due scheduled start after the locked fixed-field aggregate is
    /// found to be undersubscribed. No other tournament fact is rewritten.
    pub(crate) async fn clear_due_scheduled_start(
        locked: &Self,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Ok(diesel::update(tournaments::table.find(locked.id))
            .set(tournaments::starts_at.eq::<Option<DateTime<Utc>>>(None))
            .get_result::<Self>(conn)
            .await?)
    }

    /// Persists the locked tournament's transition to `InProgress`.
    pub(crate) async fn persist_started(
        locked: &Self,
        started_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Ok(diesel::update(tournaments::table.find(locked.id))
            .set((
                tournaments::started_at.eq(Some(started_at)),
                updated_at.eq(started_at),
            ))
            .get_result::<Self>(conn)
            .await?)
    }

    pub(crate) async fn persist_configuration(
        locked: &Self,
        configuration: Config,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let configuration =
            serde_json::to_value(configuration).map_err(|error| DbError::InternalError {
                reason: format!("resolved Swiss configuration is not persistable: {error}"),
            })?;
        Ok(diesel::update(tournaments::table.find(locked.id))
            .set(tournaments::configuration.eq(configuration))
            .get_result::<Self>(conn)
            .await?)
    }

    /// Persists the locked tournament's transition to `Finished` without
    /// rewriting its frozen configuration or start timestamp.
    pub(crate) async fn persist_finished(
        locked: &Self,
        finished_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        diesel::delete(
            tournaments_organizer_invitations::table
                .filter(tournaments_organizer_invitations::tournament_id.eq(locked.id)),
        )
        .execute(conn)
        .await?;
        Ok(diesel::update(tournaments::table.find(locked.id))
            .set((
                tournaments::finished_at.eq(Some(finished_at)),
                updated_at.eq(finished_at),
            ))
            .get_result::<Self>(conn)
            .await?)
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

    /// Locks the authoritative tournament row for a pre-start mutation.
    /// Callers must keep the surrounding transaction open through the write.
    pub async fn find_by_tournament_id_for_update(
        tournament_id: &TournamentId,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        let TournamentId(id) = tournament_id;
        Ok(tournaments::table
            .filter(nanoid_field.eq(id))
            .for_update()
            .first(conn)
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

    pub async fn get_live_arenas(conn: &mut DbConn<'_>) -> Result<Vec<Tournament>, DbError> {
        Ok(tournaments::table
            .filter(tournaments::started_at.is_not_null())
            .filter(tournaments::finished_at.is_null())
            .filter(sql::<Text>("configuration #>> '{format,format}'").eq("arena"))
            .order(tournaments::started_at.desc())
            .get_results(conn)
            .await?)
    }
}

pub fn normalize_tournament_description(
    description: Option<String>,
) -> Result<Option<String>, DbError> {
    let description = description
        .map(|description| description.trim().to_owned())
        .filter(|description| !description.is_empty());
    if description
        .as_ref()
        .is_some_and(|description| description.chars().count() > 2000)
    {
        return Err(DbError::InvalidTournamentDetails {
            info: String::from("the description must contain at most 2000 characters"),
        });
    }
    Ok(description)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use shared_types::tournament::{
        arena::{Config as ArenaConfig, MIN_DURATION_SECONDS},
        elimination::{
            Config as EliminationConfig,
            SeriesPlan as EliminationSeriesPlan,
            Topology as EliminationTopology,
        },
        round_robin::Config as RoundRobinConfig,
        swiss::{
            Config as SwissConfig,
            Criterion as SwissCriterion,
            PrimaryScore as DoubleSwissPrimaryScore,
        },
        Clock,
        RealtimeClock,
    };
    use std::num::NonZeroU32;

    fn realtime() -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 3,
        })
    }

    fn details(
        configuration: Config,
        min_seats: i32,
        starts_at: Option<DateTime<Utc>>,
    ) -> TournamentDetails {
        TournamentDetails {
            name: String::from("Typed tournament"),
            description: Some(
                "A typed tournament description that comfortably clears the minimum.".to_owned(),
            ),
            seats: if configuration.format() == Format::Arena {
                None
            } else {
                Some(8)
            },
            min_seats,
            invite_only: false,
            band_upper: Some(2500),
            band_lower: Some(500),
            starts_at,
            configuration,
        }
    }

    #[test]
    fn tournament_description_normalization_is_optional_trimmed_and_bounded() {
        assert_eq!(normalize_tournament_description(None).unwrap(), None);
        assert_eq!(
            normalize_tournament_description(Some(String::from(" \n\t "))).unwrap(),
            None
        );
        assert_eq!(
            normalize_tournament_description(Some(String::from("  concise details  "))).unwrap(),
            Some(String::from("concise details"))
        );
        assert_eq!(
            normalize_tournament_description(Some("x".repeat(2000))).unwrap(),
            Some("x".repeat(2000))
        );
        assert!(normalize_tournament_description(Some("x".repeat(2001))).is_err());
    }

    fn configurable(format: FormatConfig) -> Config {
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format,
        }
    }

    #[test]
    fn typed_creation_accepts_every_public_format_without_runtime_artifacts() {
        let elimination = |topology| {
            FormatConfig::Elimination(EliminationConfig {
                topology,
                default_plan: EliminationSeriesPlan::balanced(realtime()),
                stage_overrides: Vec::new(),
            })
        };
        let mut cases = vec![
            configurable(FormatConfig::RoundRobin(RoundRobinConfig::standard(
                NonZeroU32::new(2).unwrap(),
                realtime(),
            ))),
            configurable(FormatConfig::Swiss(SwissConfig::automatic_swiss(
                0,
                realtime(),
            ))),
            configurable(FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
                0,
                realtime(),
                DoubleSwissPrimaryScore::GamePoints,
            ))),
            configurable(elimination(EliminationTopology::Single { bronze: true })),
            configurable(elimination(EliminationTopology::Double)),
        ];
        cases.push(Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig::new(
                NonZeroU32::new(MIN_DURATION_SECONDS).unwrap(),
                match realtime() {
                    Clock::Realtime(clock) => clock,
                    Clock::Correspondence(_) => unreachable!(),
                },
            )),
        });

        let expected_formats = [
            Format::RoundRobin,
            Format::Swiss,
            Format::DoubleSwiss,
            Format::SingleElimination,
            Format::DoubleElimination,
            Format::Arena,
        ];
        for (configuration, expected_format) in cases.into_iter().zip(expected_formats) {
            let min_seats = match expected_format {
                Format::Arena => 0,
                Format::Swiss | Format::DoubleSwiss => MIN_SWISS_START_SEATS,
                _ => 4,
            };
            let starts_at =
                (expected_format == Format::Arena).then(|| Utc::now() + Duration::hours(1));
            let tournament =
                NewTournament::new(details(configuration.clone(), min_seats, starts_at))
                    .expect("public typed configuration should be creatable");
            assert_eq!(tournament.started_at, None);
            assert_eq!(tournament.finished_at, None);
            assert!(tournament.configuration.get("start_policy").is_none());
            let persisted: Config = serde_json::from_value(tournament.configuration)
                .expect("new tournament configuration should serialize canonically");
            assert_eq!(persisted.format, configuration.format);
        }
    }

    #[test]
    fn public_creation_rejects_resource_and_engine_invalid_configuration() {
        let oversized = configurable(FormatConfig::RoundRobin(RoundRobinConfig::standard(
            NonZeroU32::new(7).unwrap(),
            realtime(),
        )));
        assert!(NewTournament::new(details(oversized, 4, None)).is_err());

        let mut invalid_swiss = SwissConfig::automatic_swiss(0, realtime());
        invalid_swiss.standings = vec![SwissCriterion::MatchPoints];
        assert!(NewTournament::new(details(
            configurable(FormatConfig::Swiss(invalid_swiss)),
            MIN_SWISS_START_SEATS,
            None
        ))
        .is_err());

        let arena = Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig::new(
                NonZeroU32::new(MIN_DURATION_SECONDS).unwrap(),
                match realtime() {
                    Clock::Realtime(clock) => clock,
                    Clock::Correspondence(_) => unreachable!(),
                },
            )),
        };
        assert!(NewTournament::new(details(
            arena.clone(),
            1,
            Some(Utc::now() + Duration::hours(1)),
        ))
        .is_err());
        assert!(NewTournament::new(details(arena.clone(), 0, None)).is_err());
        assert!(
            NewTournament::new(details(arena, 0, Some(Utc::now() - Duration::seconds(1)),))
                .is_err()
        );
    }

    #[test]
    fn swiss_creation_requires_automatic_rounds_and_the_system_minimum() {
        let automatic = configurable(FormatConfig::Swiss(SwissConfig::automatic_swiss(
            0,
            realtime(),
        )));
        assert!(
            NewTournament::new(details(automatic.clone(), MIN_SWISS_START_SEATS - 1, None,))
                .is_err()
        );
        assert!(NewTournament::new(details(automatic, MIN_SWISS_START_SEATS + 1, None,)).is_err());

        let resolved = configurable(FormatConfig::Swiss(SwissConfig::swiss(
            NonZeroU32::new(3).unwrap(),
            realtime(),
        )));
        assert!(NewTournament::new(details(resolved, MIN_SWISS_START_SEATS, None,)).is_err());
    }
}
