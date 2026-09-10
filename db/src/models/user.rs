use super::rating::Rating;
use crate::{
    db_error::DbError,
    helpers::run_serializable,
    models::{
        Challenge,
        DeadlineSettlement,
        Game,
        GameUser,
        NewRating,
        NotificationPreferences,
        Tournament,
        TournamentSlot,
        TournamentUser,
    },
    schema::{
        challenges,
        games::{self, current_player_id, finished, game_status},
        ratings::{self, rating},
        tournaments,
        tournaments_invitations,
        tournaments_organizer_invitations,
        tournaments_organizers,
        tournaments_users,
        users::{
            self,
            dsl::{
                deleted as deleted_field,
                email as email_field,
                normalized_username,
                password as password_field,
                updated_at,
                username as username_field,
                users as users_table,
            },
            lang,
            takeback,
        },
    },
    tournaments::{
        fixed_field::{self, supports_format, withdrawal},
        settle_arena_game,
        FixedFieldCommit,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::{exists, sql},
    query_dsl::BelongingToDsl,
    select,
    sql_types::BigInt,
    BoolExpressionMethods,
    ExpressionMethods,
    Identifiable,
    Insertable,
    OptionalExtension,
    PgTextExpressionMethods,
    QueryDsl,
    Queryable,
    Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use hive_lib::{GameControl, GameResult, GameStatus};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use shared_types::{
    tournament::{arena::PairingIntent, Format},
    Conclusion,
    GameId,
    GameSpeed,
    LeaderboardKind,
    Takeback,
    TournamentId,
    RANKABLE_DEVIATION,
    RESERVED_USERNAMES,
};
use std::collections::HashSet;
use uuid::Uuid;

const MAX_USERNAME_LENGTH: usize = 20;
const MIN_USERNAME_LENGTH: usize = 2;
const VALID_USERNAME_CHARS: &str = "-_";
const DELETED_USERNAME_PREFIX: &str = "deleted_user_";

lazy_static! {
    static ref EMAIL_RE: Regex = Regex::new(r"^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$").unwrap();
}

fn valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || VALID_USERNAME_CHARS.contains(c)
}

fn is_reserved_deleted_username(username: &str) -> bool {
    username
        .to_ascii_lowercase()
        .starts_with(DELETED_USERNAME_PREFIX)
}

fn validate_email(email: &str) -> Result<(), DbError> {
    if !EMAIL_RE.is_match(email) {
        let reason = format!("invalid e-mail address: {email:?}");
        return Err(DbError::InvalidInput {
            info: String::from("E-mail address is invalid"),
            error: reason,
        });
    }
    Ok(())
}

fn validate_username(username: &str) -> Result<(), DbError> {
    if !username.chars().all(valid_username_char) {
        let reason = format!("invalid username characters: {username:?}");
        return Err(DbError::InvalidInput {
            info: String::from("Username has invalid characters"),
            error: reason,
        });
    }
    if username.len() > MAX_USERNAME_LENGTH {
        let reason = format!("username must be <= {MAX_USERNAME_LENGTH} chars");
        return Err(DbError::InvalidInput {
            info: String::from("Username is too long."),
            error: reason,
        });
    }
    if username.len() < MIN_USERNAME_LENGTH {
        let reason = format!("username must be >= {MAX_USERNAME_LENGTH} chars");
        return Err(DbError::InvalidInput {
            info: String::from("Username is too short."),
            error: reason,
        });
    }
    if RESERVED_USERNAMES.contains(&username.to_lowercase().as_str()) {
        return Err(DbError::InvalidInput {
            info: String::from("Pick another username."),
            error: "Username is not allowed.".to_string(),
        });
    }
    if is_reserved_deleted_username(username) {
        return Err(DbError::InvalidInput {
            info: String::from("Pick another username."),
            error: format!("Username cannot start with {DELETED_USERNAME_PREFIX}."),
        });
    }
    Ok(())
}

fn stable_unique_uuids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}

struct SoftDeleteTournamentSnapshot {
    related_tournament_ids: Vec<Uuid>,
    in_progress_membership_ids: Vec<Uuid>,
    in_progress_participants: Vec<(Uuid, Uuid, DateTime<Utc>)>,
}

impl SoftDeleteTournamentSnapshot {
    async fn load(user_id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        let mut related_tournament_ids: Vec<Uuid> = tournaments_invitations::table
            .filter(tournaments_invitations::invitee_id.eq(user_id))
            .select(tournaments_invitations::tournament_id)
            .load(conn)
            .await?;
        related_tournament_ids.extend(
            tournaments_users::table
                .filter(tournaments_users::user_id.eq(user_id))
                .select(tournaments_users::tournament_id)
                .load::<Uuid>(conn)
                .await?,
        );
        related_tournament_ids.extend(
            tournaments_organizers::table
                .filter(tournaments_organizers::organizer_id.eq(user_id))
                .select(tournaments_organizers::tournament_id)
                .load::<Uuid>(conn)
                .await?,
        );
        related_tournament_ids.extend(
            tournaments_organizer_invitations::table
                .filter(tournaments_organizer_invitations::invitee_id.eq(user_id))
                .select(tournaments_organizer_invitations::tournament_id)
                .load::<Uuid>(conn)
                .await?,
        );
        related_tournament_ids.extend(
            tournaments::table
                .filter(
                    sql::<diesel::sql_types::Bool>("start_setup->>'owner_id' = ")
                        .bind::<diesel::sql_types::Text, _>(user_id.to_string()),
                )
                .select(tournaments::id)
                .load::<Uuid>(conn)
                .await?,
        );
        let related_tournament_ids = stable_unique_uuids(&related_tournament_ids);

        let in_progress_membership_ids: Vec<Uuid> = tournaments_users::table
            .inner_join(tournaments::table)
            .filter(tournaments_users::user_id.eq(user_id))
            .filter(tournaments_users::withdrawn_at.is_null())
            .filter(tournaments::started_at.is_not_null())
            .filter(tournaments::finished_at.is_null())
            .order(tournaments_users::tournament_id.asc())
            .select(tournaments_users::tournament_id)
            .load(conn)
            .await?;
        let in_progress_membership_ids = stable_unique_uuids(&in_progress_membership_ids);
        let in_progress_participants = if related_tournament_ids.is_empty() {
            Vec::new()
        } else {
            tournaments_users::table
                .inner_join(tournaments::table)
                .filter(tournaments_users::tournament_id.eq_any(&related_tournament_ids))
                .filter(tournaments::started_at.is_not_null())
                .filter(tournaments::finished_at.is_null())
                .order((
                    tournaments_users::tournament_id.asc(),
                    tournaments_users::user_id.asc(),
                ))
                .select((
                    tournaments_users::tournament_id,
                    tournaments_users::user_id,
                    tournaments_users::accepted_at,
                ))
                .load(conn)
                .await?
        };
        Ok(Self {
            related_tournament_ids,
            in_progress_membership_ids,
            in_progress_participants,
        })
    }

    fn withdrawal_instant(&self, effective_at: DateTime<Utc>) -> DateTime<Utc> {
        self.in_progress_participants
            .iter()
            .map(|(_, _, accepted_at)| *accepted_at)
            .max()
            .map(|accepted_at| effective_at.max(accepted_at))
            .unwrap_or(effective_at)
    }
}

async fn lock_tournaments_for_update(
    tournament_ids: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<Vec<Tournament>, DbError> {
    if tournament_ids.is_empty() {
        return Ok(Vec::new());
    }
    let locked: Vec<Tournament> = tournaments::table
        .filter(tournaments::id.eq_any(tournament_ids))
        .order(tournaments::id.asc())
        .select(Tournament::as_select())
        .for_update()
        .load(conn)
        .await?;
    Ok(locked)
}

fn active_tournament_deletion_unavailable() -> DbError {
    DbError::InvalidAction {
        info: String::from(
            "account deletion is unavailable while tournament withdrawal facts cannot be recorded",
        ),
    }
}

#[derive(Debug, Default)]
pub struct SoftDeleteReport {
    pub deleted_games: Vec<Game>,
    pub resigned_games: Vec<Game>,
    pub deleted_challenges: Vec<Challenge>,
    pub deleted_tournament_ids: Vec<TournamentId>,
    pub changed_tournament_ids: Vec<TournamentId>,
    pub removed_membership_tournament_ids: Vec<TournamentId>,
    pub withdrawn_tournament_ids: Vec<TournamentId>,
    pub arena_paused_tournament_ids: Vec<TournamentId>,
    pub tournament_terminal_games: Vec<Game>,
    pub fixed_field_commits: Vec<FixedFieldCommit>,
}

impl SoftDeleteReport {
    fn record_fixed_field_commit(&mut self, effects: FixedFieldCommit) {
        if let Some(current) = self
            .fixed_field_commits
            .iter_mut()
            .find(|current| current.tournament_id == effects.tournament_id)
        {
            current.merge(effects);
        } else {
            self.fixed_field_commits.push(effects);
        }
    }
}

#[derive(Insertable, Debug)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub normalized_username: String,
    pub patreon: bool,
    pub bot: bool,
}

impl NewUser {
    pub fn new(username: &str, hashed_password: &str, email: &str) -> Result<Self, DbError> {
        validate_email(email)?;
        validate_username(username)?;
        Ok(Self {
            username: username.to_owned(),
            password: hashed_password.to_owned(),
            email: email.to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            normalized_username: username.to_lowercase(),
            patreon: false,
            bot: false,
        })
    }
}

#[derive(Queryable, Identifiable, Serialize, Selectable, Deserialize, Debug, Clone)]
#[diesel(primary_key(id))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub normalized_username: String,
    pub patreon: bool,
    pub admin: bool,
    pub takeback: String,
    pub bot: bool,
    pub deleted: bool,
    pub lang: Option<String>,
    pub email_verified: bool,
    pub pending_email: Option<String>,
}

struct RankingGate {
    min_played: i64,
    max_deviation: f64,
}

impl RankingGate {
    fn for_kind(kind: LeaderboardKind) -> Self {
        if kind.is_bots() {
            Self {
                min_played: 1,
                max_deviation: f64::MAX,
            }
        } else {
            Self {
                min_played: 0,
                max_deviation: RANKABLE_DEVIATION,
            }
        }
    }
}

impl User {
    fn assign_ranks(rows: Vec<(User, Rating)>) -> Vec<(User, Rating, i64)> {
        let mut last_rating: Option<f64> = None;
        let mut last_rank = 0_i64;

        rows.into_iter()
            .enumerate()
            .map(|(idx, (user, rating_row))| {
                let position = idx as i64 + 1;
                if last_rating != Some(rating_row.rating) {
                    last_rating = Some(rating_row.rating);
                    last_rank = position;
                }
                (user, rating_row, last_rank)
            })
            .collect()
    }

    fn deleted_identity(user_id: Uuid) -> (String, String) {
        let username = format!("{DELETED_USERNAME_PREFIX}{user_id}");
        let email = format!("{username}@deleted.invalid");
        (username, email)
    }

    /// Refuses missing or soft-deleted accounts without using account rows as
    /// cross-feature locks.
    pub(crate) async fn ensure_active_ids(
        user_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        Self::active_bot_ids(user_ids, conn).await.map(|_| ())
    }

    /// Validates an active account set and returns the bot subset without
    /// taking account-row locks. Tournament release uses the subset to start a
    /// Ready bot-versus-bot game without waiting for an impossible human
    /// handshake.
    pub(crate) async fn active_bot_ids(
        user_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<HashSet<Uuid>, DbError> {
        let user_ids = stable_unique_uuids(user_ids);
        if user_ids.is_empty() {
            return Ok(HashSet::new());
        }
        let active_users = users_table
            .filter(users::id.eq_any(&user_ids))
            .filter(deleted_field.eq(false))
            .select((users::id, users::bot))
            .load::<(Uuid, bool)>(conn)
            .await?;
        if active_users.len() != user_ids.len() {
            return Err(DbError::InvalidAction {
                info: String::from("Tournament mutation requires active users"),
            });
        }
        Ok(active_users
            .into_iter()
            .filter_map(|(user_id, bot)| bot.then_some(user_id))
            .collect())
    }

    /// Existing tournament entrants may be anonymized after the roster was
    /// frozen. Only active bots can auto-ready a game; a deleted bot waits for
    /// the organizer just like any other retained deleted entrant.
    pub(crate) async fn tournament_bot_ids(
        user_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<HashSet<Uuid>, DbError> {
        let user_ids = stable_unique_uuids(user_ids);
        let entrants = users_table
            .filter(users::id.eq_any(&user_ids))
            .select((users::id, users::bot, users::deleted))
            .load::<(Uuid, bool, bool)>(conn)
            .await?;
        if entrants.len() != user_ids.len() {
            return Err(DbError::InvalidAction {
                info: String::from("Tournament entrant account is missing"),
            });
        }
        Ok(entrants
            .into_iter()
            .filter_map(|(id, bot, deleted)| (bot && !deleted).then_some(id))
            .collect())
    }

    pub async fn create(new_user: NewUser, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        let user: User = diesel::insert_into(users::table)
            .values(new_user)
            .get_result(conn)
            .await?;
        for game_speed in GameSpeed::all_rated().into_iter() {
            diesel::insert_into(ratings::table)
                .values(NewRating::for_uuid(&user.id, game_speed))
                .execute(conn)
                .await?;
        }
        NotificationPreferences::create_for_user(user.id, conn).await?;
        Ok(user)
    }

    pub async fn edit(
        &self,
        new_password: &str,
        new_email: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<User, DbError> {
        Ok(match (new_password.is_empty(), new_email.is_empty()) {
            (true, true) => users_table.find(&self.id).first(conn).await?,
            (true, false) => {
                diesel::update(self)
                    .set((email_field.eq(new_email), updated_at.eq(Utc::now())))
                    .get_result(conn)
                    .await?
            }
            (false, true) => {
                diesel::update(self)
                    .set((password_field.eq(new_password), updated_at.eq(Utc::now())))
                    .get_result(conn)
                    .await?
            }
            (false, false) => {
                diesel::update(self)
                    .set((
                        password_field.eq(new_password),
                        email_field.eq(new_email),
                        updated_at.eq(Utc::now()),
                    ))
                    .get_result(conn)
                    .await?
            }
        })
    }

    pub async fn set_takeback(&self, tb: Takeback, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        let tb = tb.to_string();
        diesel::update(self)
            .set(takeback.eq(tb.to_string()))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn set_lang(&self, new_lang: &str, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(self)
            .set(lang.eq(new_lang))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn find_by_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table.find(uuid).first(conn).await?)
    }

    pub async fn find_active_by_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table
            .find(uuid)
            .filter(deleted_field.eq(false))
            .first(conn)
            .await?)
    }

    pub async fn find_by_uuids(
        uuids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<User>, DbError> {
        Ok(users_table
            .filter(users::id.eq_any(uuids))
            .load(conn)
            .await?)
    }

    pub async fn find_by_username(username: &str, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table
            .filter(normalized_username.eq(username.to_lowercase()))
            .filter(deleted_field.eq(false))
            .first(conn)
            .await?)
    }

    /// Resolves a direct-message route, including soft-deleted accounts whose
    /// tombstone username is still present in the messages catalog.
    pub async fn find_dm_route_user_by_username(
        username: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<(Uuid, String, bool)>, DbError> {
        Ok(users_table
            .filter(normalized_username.eq(username.to_lowercase()))
            .select((users::id, username_field, deleted_field))
            .first(conn)
            .await
            .optional()?)
    }

    pub async fn search_usernames(
        pattern: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<User>, DbError> {
        if pattern.is_empty() {
            return Ok(vec![]);
        }
        Ok(users_table
            .filter(normalized_username.ilike(format!("%{pattern}%")))
            .filter(deleted_field.eq(false))
            .load(conn)
            .await?)
    }

    pub async fn username_exists(username: &str, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        Ok(select(exists(
            users_table.filter(normalized_username.eq(username.to_lowercase())),
        ))
        .get_result(conn)
        .await?)
    }

    pub async fn uuid_exists(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        Ok(select(exists(users_table.find(uuid)))
            .get_result(conn)
            .await?)
    }

    pub async fn find_by_email(email: &str, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table
            .filter(email_field.eq(email.to_lowercase()))
            .filter(deleted_field.eq(false))
            .first(conn)
            .await?)
    }

    pub async fn find_for_login(login: &str, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        let user_result = Self::find_by_email(login, conn).await;
        let user = if let Ok(user) = user_result {
            user
        } else {
            Self::find_by_username(login, conn).await?
        };
        Ok(user)
    }

    pub async fn soft_delete(
        &self,
        replacement_password_hash: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<SoftDeleteReport, DbError> {
        let user_id = self.id;
        let replacement_password_hash = replacement_password_hash.to_owned();
        run_serializable(conn, move |tc| {
            let replacement_password_hash = replacement_password_hash.clone();
            Box::pin(async move {
                User::find_active_by_uuid(&user_id, tc).await?;
                let current = SoftDeleteTournamentSnapshot::load(user_id, tc).await?;
                let locked_tournaments =
                    lock_tournaments_for_update(&current.related_tournament_ids, tc).await?;
                let in_progress_membership_ids = current
                    .in_progress_membership_ids
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>();
                let in_progress_tournaments = locked_tournaments
                    .iter()
                    .filter(|tournament| in_progress_membership_ids.contains(&tournament.id))
                    .cloned()
                    .collect::<Vec<_>>();

                let mut report = SoftDeleteReport::default();

                for tournament in &in_progress_tournaments {
                    let configuration = tournament.configuration();
                    if configuration.format() != Format::Arena
                        && !supports_format(configuration.format())
                    {
                        return Err(active_tournament_deletion_unavailable());
                    }
                }

                let mut initial_fixed_field_slot_ids = Vec::new();
                for tournament in &in_progress_tournaments {
                    if tournament.configuration().format() != Format::Arena {
                        initial_fixed_field_slot_ids
                            .extend(fixed_field::reconciliation_slot_ids(tournament.id, tc).await?);
                    }
                }
                let initial_fixed_field_slot_ids =
                    stable_unique_uuids(&initial_fixed_field_slot_ids);
                TournamentSlot::find_across_tournaments_by_ids_for_update(
                    &initial_fixed_field_slot_ids,
                    tc,
                )
                .await?;

                let mut reconciled_in_progress_tournaments = Vec::new();
                for tournament in in_progress_tournaments {
                    if tournament.configuration().format() == Format::Arena {
                        reconciled_in_progress_tournaments.push(tournament);
                        continue;
                    }
                    let (tournament, commit) =
                        fixed_field::reconcile_locked(tournament, tc).await?;
                    if let Some(commit) = commit {
                        report.record_fixed_field_commit(commit);
                    }
                    if tournament.finished_at.is_none() {
                        reconciled_in_progress_tournaments.push(tournament);
                    }
                }
                let in_progress_tournaments = reconciled_in_progress_tournaments;

                let mut fixed_field_withdrawals = Vec::new();
                for tournament in &in_progress_tournaments {
                    if tournament.configuration().format() != Format::Arena {
                        fixed_field_withdrawals.push((
                            TournamentId(tournament.nanoid.clone()),
                            withdrawal::prepare(tournament.clone(), user_id, tc).await?,
                        ));
                    }
                }
                let fixed_field_slot_ids = stable_unique_uuids(
                    &fixed_field_withdrawals
                        .iter()
                        .flat_map(|(_, prepared)| prepared.slot_ids().iter().copied())
                        .collect::<Vec<_>>(),
                );
                let locked_slots = TournamentSlot::find_across_tournaments_by_ids_for_update(
                    &fixed_field_slot_ids,
                    tc,
                )
                .await?;

                let candidate_unfinished_game_ids: Vec<Uuid> = games::table
                    .filter(games::finished.eq(false))
                    .filter(games::tournament_slot_id.is_null())
                    .filter(games::white_id.eq(user_id).or(games::black_id.eq(user_id)))
                    .order(games::id.asc())
                    .select(games::id)
                    .load(tc)
                    .await?;
                let mut affected_game_ids = candidate_unfinished_game_ids;
                affected_game_ids.extend(
                    fixed_field_withdrawals
                        .iter()
                        .flat_map(|(_, prepared)| prepared.game_ids().iter().copied()),
                );
                let affected_game_ids = stable_unique_uuids(&affected_game_ids);
                let locked_games = Game::find_by_ids_for_update(&affected_game_ids, tc).await?;
                // A timeout can release a successor before withdrawal normalization
                // completes, so its opponent's rating may not belong to an existing game yet.
                let fixed_field_tournament_ids = fixed_field_withdrawals
                    .iter()
                    .map(|(_, prepared)| prepared.tournament_id())
                    .collect::<HashSet<_>>();
                let mut rating_user_ids = current
                    .in_progress_participants
                    .iter()
                    .filter(|(tournament_id, _, _)| {
                        fixed_field_tournament_ids.contains(tournament_id)
                    })
                    .map(|(_, participant_id, _)| *participant_id)
                    .collect::<Vec<_>>();
                rating_user_ids.extend(
                    locked_games
                        .iter()
                        .flat_map(|game| [game.white_id, game.black_id]),
                );
                let rating_user_ids = stable_unique_uuids(&rating_user_ids);
                if !rating_user_ids.is_empty() {
                    ratings::table
                        .filter(ratings::user_uid.eq_any(&rating_user_ids))
                        .order((ratings::speed.asc(), ratings::user_uid.asc()))
                        .for_update()
                        .select(ratings::id)
                        .load::<i32>(tc)
                        .await?;
                }

                let effective_at = Utc::now();
                let withdrawn_at = current.withdrawal_instant(effective_at);

                let arena_unfinished_games = locked_games
                    .iter()
                    .filter_map(|game| {
                        if !game.finished
                            && game.tournament_slot_id.is_none()
                            && game.arena_ordinal.is_some()
                        {
                            game.user_color(user_id).map(|color| (game.clone(), color))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let ordinary_games = locked_games
                    .iter()
                    .filter_map(|game| {
                        if !game.finished && game.tournament_id.is_none() {
                            game.user_color(user_id).map(|color| (game.clone(), color))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                for tournament in in_progress_tournaments
                    .iter()
                    .filter(|tournament| tournament.configuration().format() == Format::Arena)
                {
                    TournamentUser::persist_arena_pairing_state(
                        tournament.id,
                        user_id,
                        PairingIntent::Paused,
                        None,
                        tc,
                    )
                    .await?;
                    report
                        .arena_paused_tournament_ids
                        .push(TournamentId(tournament.nanoid.clone()));
                }
                for (tournament_nanoid, prepared) in fixed_field_withdrawals {
                    let outcome =
                        withdrawal::apply(prepared, &locked_slots, &locked_games, withdrawn_at, tc)
                            .await?;
                    if outcome.applied {
                        report
                            .withdrawn_tournament_ids
                            .push(tournament_nanoid.clone());
                    }
                    report
                        .tournament_terminal_games
                        .extend(outcome.terminal_games);
                    if let Some(commit) = outcome.commit {
                        report.record_fixed_field_commit(commit);
                    }
                }

                for tournament in &locked_tournaments {
                    for (game, color) in arena_unfinished_games
                        .iter()
                        .filter(|(game, _)| game.tournament_id == Some(tournament.id))
                    {
                        let (terminal, terminal_at) =
                            match game.settle_deadline(withdrawn_at, tc).await? {
                                DeadlineSettlement::Terminal {
                                    game, terminal_at, ..
                                } => (game, terminal_at),
                                DeadlineSettlement::Active(game) => (
                                    game.finish_game_control(
                                        GameControl::Resign(*color),
                                        GameResult::Winner(color.opposite_color()),
                                        Conclusion::Resigned,
                                        withdrawn_at,
                                        tc,
                                    )
                                    .await?,
                                    withdrawn_at,
                                ),
                            };
                        settle_arena_game(tournament.id, &terminal, terminal_at, withdrawn_at, tc)
                            .await?;
                        report.tournament_terminal_games.push(terminal);
                    }
                }

                let deleted_challenges: Vec<Challenge> = challenges::table
                    .filter(
                        challenges::challenger_id
                            .eq(user_id)
                            .or(challenges::opponent_id.eq(user_id)),
                    )
                    .order(challenges::id.asc())
                    .for_update()
                    .load(tc)
                    .await?;
                let deleted_challenge_ids = deleted_challenges
                    .iter()
                    .map(|challenge| challenge.id)
                    .collect::<Vec<_>>();
                report.deleted_challenges = deleted_challenges;
                if !deleted_challenge_ids.is_empty() {
                    diesel::delete(
                        challenges::table.filter(challenges::id.eq_any(deleted_challenge_ids)),
                    )
                    .execute(tc)
                    .await?;
                }

                diesel::delete(
                    tournaments_invitations::table
                        .filter(tournaments_invitations::invitee_id.eq(user_id)),
                )
                .execute(tc)
                .await?;

                let not_started_organized_tournaments: Vec<(Uuid, String)> =
                    tournaments_organizers::table
                        .inner_join(tournaments::table)
                        .filter(tournaments_organizers::organizer_id.eq(user_id))
                        .filter(tournaments::started_at.is_null())
                        .filter(tournaments::finished_at.is_null())
                        .select((tournaments_organizers::tournament_id, tournaments::nanoid))
                        .load(tc)
                        .await?;
                let mut exclusively_organized = Vec::new();
                for (id, nanoid) in not_started_organized_tournaments {
                    let another_organizer = select(exists(
                        tournaments_organizers::table
                            .inner_join(users::table)
                            .filter(tournaments_organizers::tournament_id.eq(id))
                            .filter(tournaments_organizers::organizer_id.ne(user_id))
                            .filter(users::deleted.eq(false)),
                    ))
                    .get_result::<bool>(tc)
                    .await?;
                    if !another_organizer {
                        exclusively_organized.push((id, nanoid));
                    }
                }
                let not_started_organized_tournaments = exclusively_organized;
                let not_started_organized_tournament_ids = not_started_organized_tournaments
                    .iter()
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                report.deleted_tournament_ids = not_started_organized_tournaments
                    .into_iter()
                    .map(|(_, nanoid)| TournamentId(nanoid))
                    .collect();
                if !not_started_organized_tournament_ids.is_empty() {
                    diesel::delete(
                        tournaments::table
                            .filter(tournaments::id.eq_any(not_started_organized_tournament_ids)),
                    )
                    .execute(tc)
                    .await?;
                }

                diesel::delete(
                    tournaments_organizer_invitations::table
                        .filter(tournaments_organizer_invitations::invitee_id.eq(user_id)),
                )
                .execute(tc)
                .await?;
                let retained_rosters = locked_tournaments
                    .iter()
                    .filter(|tournament| {
                        tournament
                            .start_setup()
                            .is_some_and(|setup| setup.active_at(withdrawn_at))
                    })
                    .map(|tournament| tournament.id)
                    .collect::<Vec<_>>();
                for tournament in &locked_tournaments {
                    if report
                        .deleted_tournament_ids
                        .iter()
                        .any(|id| id.0 == tournament.nanoid)
                    {
                        continue;
                    }
                    report
                        .changed_tournament_ids
                        .push(TournamentId(tournament.nanoid.clone()));
                    if tournament
                        .start_setup()
                        .is_some_and(|setup| setup.owner_id == user_id)
                    {
                        diesel::update(tournaments::table.find(tournament.id))
                            .set(tournaments::start_setup.eq(None::<serde_json::Value>))
                            .execute(tc)
                            .await?;
                    }
                }

                let not_started_tournaments: Vec<(Uuid, String)> = tournaments_users::table
                    .inner_join(tournaments::table)
                    .filter(tournaments_users::user_id.eq(user_id))
                    .filter(tournaments::started_at.is_null())
                    .filter(tournaments::finished_at.is_null())
                    .filter(tournaments::id.ne_all(&retained_rosters))
                    .select((tournaments_users::tournament_id, tournaments::nanoid))
                    .load(tc)
                    .await?;
                let not_started_tournament_ids = not_started_tournaments
                    .iter()
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                report.removed_membership_tournament_ids = not_started_tournaments
                    .into_iter()
                    .map(|(_, nanoid)| TournamentId(nanoid))
                    .collect();
                if !not_started_tournament_ids.is_empty() {
                    diesel::delete(
                        tournaments_users::table
                            .filter(tournaments_users::user_id.eq(user_id))
                            .filter(
                                tournaments_users::tournament_id.eq_any(not_started_tournament_ids),
                            ),
                    )
                    .execute(tc)
                    .await?;
                }

                for (game, color) in ordinary_games {
                    let checked = match game.settle_deadline(withdrawn_at, tc).await? {
                        DeadlineSettlement::Active(game)
                        | DeadlineSettlement::Terminal { game, .. } => game,
                    };
                    if checked.finished {
                        report.resigned_games.push(checked);
                    } else if checked.game_status == GameStatus::NotStarted.to_string() {
                        let mut deleted_game = checked.clone();
                        deleted_game.finished = true;
                        checked.delete(tc).await?;
                        report.deleted_games.push(deleted_game);
                    } else {
                        let game_control = GameControl::Resign(color);
                        let resigned_game = checked
                            .finish_game_control(
                                game_control,
                                GameResult::Winner(color.opposite_color()),
                                Conclusion::Resigned,
                                withdrawn_at,
                                tc,
                            )
                            .await?;
                        report.resigned_games.push(resigned_game);
                    }
                }

                let (deleted_username, deleted_email) = Self::deleted_identity(user_id);
                diesel::update(users_table.find(user_id))
                    .set((
                        username_field.eq(&deleted_username),
                        normalized_username.eq(&deleted_username),
                        email_field.eq(&deleted_email),
                        password_field.eq(&replacement_password_hash),
                        users::admin.eq(false),
                        deleted_field.eq(true),
                        updated_at.eq(effective_at),
                    ))
                    .execute(tc)
                    .await?;

                Ok(report)
            })
        })
        .await
    }

    pub async fn get_ongoing_games(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        Ok(GameUser::belonging_to(self)
            .inner_join(games::table)
            .select(Game::as_select())
            .filter(finished.eq(false))
            .get_results(conn)
            .await?)
    }

    pub async fn get_games_with_notifications(
        &self,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Game>, DbError> {
        Ok(GameUser::belonging_to(self)
            .inner_join(games::table)
            .select(Game::as_select())
            .filter(current_player_id.eq(self.id))
            .filter(finished.eq(false))
            .filter(
                game_status
                    .ne("NotStarted")
                    .or(games::tournament_slot_id.is_null()),
            )
            .get_results(conn)
            .await?)
    }

    pub async fn get_urgent_nanoids(&self, conn: &mut DbConn<'_>) -> Result<Vec<GameId>, DbError> {
        Ok(self
            .get_games_with_notifications(conn)
            .await?
            .into_iter()
            .map(|game| GameId(game.nanoid))
            .collect())
    }

    pub async fn get_top_users(
        kind: LeaderboardKind,
        game_speed: &GameSpeed,
        maybe_user: Option<Uuid>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(User, Rating, i64)>, DbError> {
        let speed = game_speed.to_string();
        let gate = RankingGate::for_kind(kind);

        let mut top = Self::assign_ranks(
            users::table
                .inner_join(ratings::table)
                .filter(users::deleted.eq(false))
                .filter(users::bot.eq(kind.is_bots()))
                .filter(ratings::speed.eq(speed.clone()))
                .filter(ratings::played.ge(gate.min_played))
                .filter(ratings::deviation.le(gate.max_deviation))
                .select((User::as_select(), Rating::as_select()))
                .order_by(rating.desc())
                .then_order_by(ratings::played.desc())
                .then_order_by(users::id.asc())
                .limit(limit)
                .load::<(User, Rating)>(conn)
                .await?,
        );

        let Some(user_id) = maybe_user else {
            return Ok(top);
        };

        if top.iter().any(|(user, _, _)| user.id == user_id) {
            return Ok(top);
        }

        let viewer_row = match users::table
            .inner_join(ratings::table)
            .filter(users::deleted.eq(false))
            .filter(users::bot.eq(kind.is_bots()))
            .filter(ratings::speed.eq(speed))
            .filter(ratings::played.ge(gate.min_played))
            .filter(ratings::deviation.le(gate.max_deviation))
            .select((
                User::as_select(),
                Rating::as_select(),
                sql::<BigInt>("RANK() OVER (ORDER BY ratings.rating DESC)"),
            ))
            .order_by(ratings::rating.desc())
            .load::<(User, Rating, i64)>(conn)
            .await
        {
            Ok(rows) => match rows.into_iter().find(|(user, _, _)| user.id == user_id) {
                Some(row) => row,
                None => return Ok(top),
            },
            Err(_) => return Ok(top),
        };

        top.push(viewer_row);

        Ok(top)
    }
}

#[cfg(test)]
mod tests {
    use super::stable_unique_uuids;
    use uuid::Uuid;

    #[test]
    fn stable_unique_uuids_orders_and_deduplicates_identifiers() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let third = Uuid::from_u128(3);

        assert_eq!(
            stable_unique_uuids(&[third, first, second, first]),
            vec![first, second, third]
        );
    }
}
