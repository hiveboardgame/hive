use super::rating::Rating;
use crate::{
    db_error::DbError,
    models::{Game, GameUser, NewRating},
    schema::{
        games::{self, current_player_id, finished},
        ratings::{self, rating},
        users::{
            self,
            dsl::{
                email as email_field, normalized_username, password as password_field, updated_at,
                users as users_table,
            },
        },
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::exists, query_dsl::BelongingToDsl, select, ExpressionMethods, Identifiable, Insertable,
    PgTextExpressionMethods, QueryDsl, Queryable, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, GameSpeed};
use uuid::Uuid;

const MAX_USERNAME_LENGTH: usize = 20;
const MIN_USERNAME_LENGTH: usize = 2;
const VALID_USERNAME_CHARS: &str = "-_";
const BANNED_USERNAMES: [&str; 3] = ["black", "white", "admin"];

lazy_static! {
    static ref EMAIL_RE: Regex = Regex::new(r"^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$").unwrap();
}

fn valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || VALID_USERNAME_CHARS.contains(c)
}

fn validate_email(email: &str) -> Result<(), DbError> {
    if !EMAIL_RE.is_match(email) {
        let reason = format!("invalid e-mail address: {:?}", email);
        return Err(DbError::InvalidInput {
            info: String::from("E-mail address is invalid"),
            error: reason,
        });
    }
    Ok(())
}

fn validate_username(username: &str) -> Result<(), DbError> {
    if !username.chars().all(valid_username_char) {
        let reason = format!("invalid username characters: {:?}", username);
        return Err(DbError::InvalidInput {
            info: String::from("Username has invalid characters"),
            error: reason,
        });
    }
    if username.len() > MAX_USERNAME_LENGTH {
        let reason = format!("username must be <= {} chars", MAX_USERNAME_LENGTH);
        return Err(DbError::InvalidInput {
            info: String::from("Username is too long."),
            error: reason,
        });
    }
    if username.len() < MIN_USERNAME_LENGTH {
        let reason = format!("username must be >= {} chars", MAX_USERNAME_LENGTH);
        return Err(DbError::InvalidInput {
            info: String::from("Username is too short."),
            error: reason,
        });
    }
    if BANNED_USERNAMES.contains(&username.to_lowercase().as_str()) {
        return Err(DbError::InvalidInput {
            info: String::from("Pick another username."),
            error: "Username is not allowed.".to_string(),
        });
    }
    Ok(())
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
        })
    }
}

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, Clone)]
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
}

impl User {
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

    pub async fn find_by_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table.find(uuid).first(conn).await?)
    }

    pub async fn find_by_username(username: &str, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table
            .filter(normalized_username.eq(username.to_lowercase()))
            .first(conn)
            .await?)
    }

    pub async fn search_usernames(
        pattern: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<User>, DbError> {
        if pattern.is_empty() {
            return Ok(vec![]);
        }
        Ok(users_table
            .filter(normalized_username.ilike(format!("%{}%", pattern)))
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

    pub async fn find_by_email(email: &str, conn: &mut DbConn<'_>) -> Result<User, DbError> {
        Ok(users_table
            .filter(email_field.eq(email.to_lowercase()))
            .first(conn)
            .await?)
    }

    pub async fn delete(&self, conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        Ok(diesel::delete(users_table.find(&self.id))
            .execute(conn)
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
            .get_results(conn)
            .await?)
    }

    pub async fn get_urgent_nanoids(&self, conn: &mut DbConn<'_>) -> Result<Vec<GameId>, DbError> {
        Ok(GameUser::belonging_to(self)
            .inner_join(games::table)
            .select(Game::as_select())
            .filter(current_player_id.eq(self.id))
            .filter(finished.eq(false))
            .get_results(conn)
            .await?
            .into_iter()
            .map(|game| GameId(game.nanoid))
            .collect())
    }

    pub async fn get_top_users(
        game_speed: &GameSpeed,
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(User, Rating)>, DbError> {
        Ok(users::table
            .inner_join(ratings::table)
            .filter(ratings::deviation.le(shared_types::RANKABLE_DEVIATION))
            .filter(ratings::speed.eq(game_speed.to_string()))
            .order_by(rating.desc())
            .limit(limit)
            .load::<(User, Rating)>(conn)
            .await?)
    }
}
