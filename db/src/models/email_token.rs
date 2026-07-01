use crate::{
    db_error::DbError,
    schema::email_tokens::{
        self,
        dsl::{
            email_tokens as email_tokens_table,
            expires_at as expires_at_field,
            id as id_field,
            purpose as purpose_field,
            token_hash as token_hash_field,
            used_at,
            user_id as user_id_field,
        },
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = email_tokens)]
pub struct NewEmailToken {
    pub user_id: Uuid,
    pub purpose: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Queryable, Debug, Clone)]
pub struct EmailToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub purpose: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl EmailToken {
    /// Issue a fresh token for `(user_id, purpose)`, invalidating any prior unused
    /// tokens for that pair so only the most recent link works.
    pub async fn issue(
        user_id: Uuid,
        purpose: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<EmailToken, DbError> {
        diesel::update(
            email_tokens_table
                .filter(user_id_field.eq(user_id))
                .filter(purpose_field.eq(purpose))
                .filter(used_at.is_null()),
        )
        .set(used_at.eq(Utc::now()))
        .execute(conn)
        .await?;
        let new = NewEmailToken {
            user_id,
            purpose: purpose.to_owned(),
            token_hash: token_hash.to_owned(),
            expires_at,
        };
        Ok(diesel::insert_into(email_tokens_table)
            .values(new)
            .get_result(conn)
            .await?)
    }

    pub async fn find_valid(
        token_hash: &str,
        purpose: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<EmailToken, DbError> {
        Ok(email_tokens_table
            .filter(token_hash_field.eq(token_hash))
            .filter(purpose_field.eq(purpose))
            .filter(used_at.is_null())
            .filter(expires_at_field.gt(Utc::now()))
            .first(conn)
            .await?)
    }

    pub async fn consume(id: Uuid, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        let updated = diesel::update(
            email_tokens_table
                .filter(id_field.eq(id))
                .filter(used_at.is_null()),
        )
        .set(used_at.eq(Utc::now()))
        .execute(conn)
        .await?;
        Ok(updated == 1)
    }

    pub async fn delete_used_before(
        threshold: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(diesel::delete(
            email_tokens_table
                .filter(used_at.is_not_null())
                .filter(used_at.lt(threshold)),
        )
        .execute(conn)
        .await?)
    }

    pub async fn delete_expired_before(
        threshold: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(diesel::delete(
            email_tokens_table
                .filter(used_at.is_null())
                .filter(expires_at_field.lt(threshold)),
        )
        .execute(conn)
        .await?)
    }
}
