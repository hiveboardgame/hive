use crate::{
    error::DbError, models::{game::Game, game_user::GameUser, rating::NewRating},
    schema::{games, ratings, users, users::dsl::users as users_table},
    get_conn, DbPool
};
use diesel::{
    query_dsl::BelongingToDsl, result::Error, Identifiable, Insertable, QueryDsl, Queryable,
    SelectableHelper,
};
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use serde::{Deserialize, Serialize};

const MAX_USERNAME_LENGTH: usize = 40;
const VALID_USERNAME_CHARS: &str = "-_";

fn valid_uid_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn validate_uid(uid: &str) -> Result<(), DbError> {
    if !uid.chars().all(valid_uid_char) {
        return Err(DbError::UserInputError {
            field: "uid".into(),
            reason: "invalid characters".into(),
        });
    }
    Ok(())
}

fn valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || VALID_USERNAME_CHARS.contains(c)
}

fn validate_username(username: &str) -> Result<(), DbError> {
    if !username.chars().all(valid_username_char) {
        let reason = format!("invalid username characters: {:?}", username);
        return Err(DbError::UserInputError {
            field: "username".into(),
            reason,
        });
    } else if username.len() > MAX_USERNAME_LENGTH {
        let reason = format!("username must be <= {} chars", MAX_USERNAME_LENGTH);
        return Err(DbError::UserInputError {
            field: "username".into(),
            reason,
        });
    }
    Ok(())
}

#[derive(Queryable, Identifiable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(uid))]
pub struct User {
    pub uid: String,
    pub username: String,
    pub password: String,
    pub token: String,
}

impl User {
    pub fn new(uid: &str, username: &str, password: &str, token: &str) -> Result<User, DbError> {
        validate_uid(uid)?;
        validate_username(username)?;
        Ok(User {
            uid: uid.into(),
            username: username.into(),
            password: password.into(),
            token: token.into(),
        })
    }

    pub async fn find_by_uid(uid: &str, pool: &DbPool) -> Result<User, Error> {
        let conn = &mut get_conn(pool).await?;
        users_table.find(uid).first(conn).await
    }

    pub async fn insert(&self, pool: &DbPool) -> Result<(), Error> {
        let connection = &mut get_conn(pool).await?;
        connection
            .transaction::<_, diesel::result::Error, _>(|conn| {
                async move {
                    self.insert_into(users::table).execute(conn).await?;
                    let new_rating = NewRating::for_uid(&self.uid);
                    diesel::insert_into(ratings::table)
                        .values(&new_rating)
                        .execute(conn)
                        .await?;
                    Ok(())
                }
                .scope_boxed()
            })
            .await?;
        Ok(())
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<usize, Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(users_table.find(&self.uid))
            .execute(conn)
            .await
    }

    pub async fn get_games(&self, pool: &DbPool) -> Result<Vec<Game>, Error> {
        let conn = &mut get_conn(pool).await?;
        GameUser::belonging_to(self)
            .inner_join(games::table)
            .select(Game::as_select())
            .get_results(conn)
            .await
    }
}
