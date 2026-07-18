use super::rating::Rating;
use crate::{
    db_error::DbError,
    models::{Challenge, Game, GameUser, NewRating, NotificationPreferences, Schedule},
    schema::{
        challenges,
        games::{self, current_player_id, finished, game_status, tournament_id},
        ratings::{self, rating},
        tournaments,
        tournaments_invitations,
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
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::{exists, sql},
    query_dsl::BelongingToDsl,
    select,
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
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::GameControl;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, GameSpeed, Takeback, TournamentId, TournamentStatus};
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
    if shared_types::RESERVED_USERNAMES.contains(&username.to_lowercase().as_str()) {
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

#[derive(Debug, Default)]
pub struct SoftDeleteReport {
    pub deleted_games: Vec<Game>,
    pub resigned_games: Vec<Game>,
    pub deleted_challenges: Vec<Challenge>,
    pub deleted_tournament_ids: Vec<TournamentId>,
    pub removed_membership_tournament_ids: Vec<TournamentId>,
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

    pub async fn is_admin(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        Ok(select(exists(
            users_table.filter(
                users::id
                    .eq(uuid)
                    .and(users::admin.eq(true))
                    .and(deleted_field.eq(false)),
            ),
        ))
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
        conn.transaction::<_, DbError, _>(async move |tc| {
            let user: User = users_table.find(user_id).for_update().first(tc).await?;
            if user.deleted {
                return Err(DbError::InvalidAction {
                    info: String::from("Account is already deleted"),
                });
            }
            let mut report = SoftDeleteReport::default();

            let unfinished_games: Vec<Game> = games::table
                .filter(
                    games::finished
                        .eq(false)
                        .and(games::white_id.eq(user_id).or(games::black_id.eq(user_id))),
                )
                .for_update()
                .load(tc)
                .await?;
            let unfinished_game_ids: Vec<Uuid> =
                unfinished_games.iter().map(|game| game.id).collect();
            Schedule::delete_for_games(&unfinished_game_ids, tc).await?;

            let deleted_challenges: Vec<Challenge> = challenges::table
                .filter(
                    challenges::challenger_id
                        .eq(user_id)
                        .or(challenges::opponent_id.eq(user_id)),
                )
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
                    .filter(tournaments::status.eq(TournamentStatus::NotStarted.to_string()))
                    .select((tournaments_organizers::tournament_id, tournaments::nanoid))
                    .load(tc)
                    .await?;
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
                        .filter(tournaments::id.eq_any(not_started_organized_tournament_ids))
                        .filter(tournaments::status.eq(TournamentStatus::NotStarted.to_string())),
                )
                .execute(tc)
                .await?;
            }

            let not_started_tournaments: Vec<(Uuid, String)> = tournaments_users::table
                .inner_join(tournaments::table)
                .filter(tournaments_users::user_id.eq(user_id))
                .filter(tournaments::status.eq(TournamentStatus::NotStarted.to_string()))
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

            for game in unfinished_games {
                let color = game
                    .user_color(user_id)
                    .ok_or_else(|| DbError::InvalidAction {
                        info: String::from("Deleted account is not a player"),
                    })?;
                if game.tournament_id.is_none() && game.turn < 2 {
                    let mut deleted_game = game.clone();
                    deleted_game.finished = true;
                    game.delete(tc).await?;
                    report.deleted_games.push(deleted_game);
                } else {
                    let game_control = GameControl::Resign(color);
                    let resigned_game = game.resign(&game_control, tc).await?;
                    report.resigned_games.push(resigned_game);
                }
            }

            let (deleted_username, deleted_email) = Self::deleted_identity(user_id);
            diesel::update(users_table.find(user_id))
                .set((
                    username_field.eq(&deleted_username),
                    normalized_username.eq(&deleted_username),
                    email_field.eq(&deleted_email),
                    password_field.eq(replacement_password_hash),
                    users::admin.eq(false),
                    deleted_field.eq(true),
                    updated_at.eq(Utc::now()),
                ))
                .execute(tc)
                .await?;

            Ok(report)
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
                tournament_id
                    .is_not_null()
                    .and(game_status.ne("NotStarted"))
                    .or(tournament_id.is_null()),
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
        game_speed: &GameSpeed,
        maybe_user: Option<Uuid>,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(User, Rating, i64)>, DbError> {
        let speed = game_speed.to_string();
        let mut top = Self::assign_ranks(
            users::table
                .inner_join(ratings::table)
                .filter(users::deleted.eq(false))
                .filter(ratings::deviation.le(shared_types::RANKABLE_DEVIATION))
                .filter(ratings::speed.eq(speed.clone()))
                .select((User::as_select(), Rating::as_select()))
                .order_by(rating.desc())
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
            .filter(ratings::deviation.le(shared_types::RANKABLE_DEVIATION))
            .filter(ratings::speed.eq(speed))
            .select((
                User::as_select(),
                Rating::as_select(),
                sql::<diesel::sql_types::BigInt>("RANK() OVER (ORDER BY ratings.rating DESC)"),
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

        if viewer_row.2 > limit {
            top.push(viewer_row);
        }

        Ok(top)
    }

    pub async fn get_username_by_id(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<String, DbError> {
        Ok(users_table
            .select(users::username)
            .filter(users::id.eq(uuid))
            .first(conn)
            .await?)
    }
}
