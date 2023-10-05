use crate::{
    error::DbError,
    get_conn,
    models::{game::Game, game_user::GameUser, rating::NewRating},
    schema::{
        games, ratings, users, users::dsl::email as email_field,
        users::dsl::password as password_field, users::dsl::username as username_field,
        users::dsl::users as users_table,
    },
    DbPool,
};
use diesel::{
    query_dsl::BelongingToDsl, result::Error, ExpressionMethods, Identifiable, Insertable,
    QueryDsl, Queryable, SelectableHelper,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_USERNAME_LENGTH: usize = 40;
const VALID_USERNAME_CHARS: &str = "-_";

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

#[derive(Insertable, Debug)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub email: String,
}

impl NewUser {
    pub fn new(username: &str, password: &str, email: &str) -> Result<Self, DbError> {
        validate_username(username)?;
        Ok(Self {
            username: username.to_owned(),
            password: password.to_owned(),
            email: email.to_owned(),
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
}

impl User {
    pub async fn create(new_user: &NewUser, pool: &DbPool) -> Result<User, Error> {
        let connection = &mut get_conn(pool).await?;
        connection
            .transaction::<_, diesel::result::Error, _>(|conn| {
                async move {
                    let user: User = diesel::insert_into(users::table)
                        .values(new_user)
                        .get_result(conn)
                        .await?;
                    let new_rating = NewRating::for_uuid(&user.id);
                    diesel::insert_into(ratings::table)
                        .values(&new_rating)
                        .execute(conn)
                        .await?;
                    Ok(user)
                }
                .scope_boxed()
            })
            .await
    }

    pub async fn edit(
        &self,
        new_password: &str,
        new_email: &str,
        pool: &DbPool,
    ) -> Result<User, Error> {
        let conn = &mut get_conn(pool).await?;
        match (new_password.is_empty(), new_email.is_empty()) {
            (true, true) => users_table.find(&self.id).first(conn).await,
            (true, false) => {
                diesel::update(self)
                    .set(email_field.eq(new_email))
                    .get_result(conn)
                    .await
            }
            (false, true) => {
                diesel::update(self)
                    .set(password_field.eq(new_password))
                    .get_result(conn)
                    .await
            }
            (false, false) => {
                diesel::update(self)
                    .set((password_field.eq(new_password), email_field.eq(new_email)))
                    .get_result(conn)
                    .await
            }
        }
    }

    pub async fn find_by_uuid(uuid: &Uuid, pool: &DbPool) -> Result<User, Error> {
        let conn = &mut get_conn(pool).await?;
        users_table.find(uuid).first(conn).await
    }

    pub async fn find_by_username(username: &str, pool: &DbPool) -> Result<User, Error> {
        let conn = &mut get_conn(pool).await?;
        users_table
            .filter(username_field.eq(username))
            .first(conn)
            .await
    }

    pub async fn delete(&self, pool: &DbPool) -> Result<usize, Error> {
        let conn = &mut get_conn(pool).await?;
        diesel::delete(users_table.find(&self.id))
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
