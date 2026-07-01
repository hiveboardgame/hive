use crate::{
    db_error::DbError,
    schema::email_request_log::{self, dsl},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, Insertable, QueryDsl};
use diesel_async::RunQueryDsl;

#[derive(Insertable, Debug)]
#[diesel(table_name = email_request_log)]
pub struct NewEmailRequestLog {
    pub email: String,
    pub ip: String,
    pub purpose: String,
}

pub struct EmailRequestLog;

impl EmailRequestLog {
    pub async fn record(
        email: &str,
        ip: &str,
        purpose: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::insert_into(dsl::email_request_log)
            .values(NewEmailRequestLog {
                email: email.to_lowercase(),
                ip: ip.to_owned(),
                purpose: purpose.to_owned(),
            })
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn count_recent_email(
        email: &str,
        purpose: &str,
        since: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<i64, DbError> {
        Ok(dsl::email_request_log
            .filter(dsl::email.eq(email.to_lowercase()))
            .filter(dsl::purpose.eq(purpose))
            .filter(dsl::created_at.ge(since))
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn count_recent_ip(
        ip: &str,
        purpose: &str,
        since: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<i64, DbError> {
        Ok(dsl::email_request_log
            .filter(dsl::ip.eq(ip))
            .filter(dsl::purpose.eq(purpose))
            .filter(dsl::created_at.ge(since))
            .count()
            .get_result(conn)
            .await?)
    }

    pub async fn delete_before(
        threshold: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(
            diesel::delete(dsl::email_request_log.filter(dsl::created_at.lt(threshold)))
                .execute(conn)
                .await?,
        )
    }
}
