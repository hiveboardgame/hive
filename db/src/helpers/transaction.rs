use crate::{db_error::DbError, DbConn};
use diesel::result::Error as DieselError;
use diesel_async::{AsyncConnection, SimpleAsyncConnection};
use std::{future::Future, pin::Pin};

const SERIALIZABLE_ATTEMPTS: usize = 3;

/// Runs one coherent read-only snapshot without taking row locks.
pub async fn run_read_only_repeatable_read<R, E, F>(
    conn: &mut DbConn<'_>,
    operation: F,
) -> Result<R, E>
where
    R: Send,
    E: From<DieselError> + Send,
    F: for<'r> Fn(&'r mut DbConn<'_>) -> Pin<Box<dyn Future<Output = Result<R, E>> + Send + 'r>>
        + Send
        + Sync,
{
    conn.transaction::<_, E, _>(async |tc| {
        tc.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .await?;
        operation(tc).await
    })
    .await
}

/// Runs a top-level transaction at PostgreSQL's serializable isolation level.
///
/// This is reserved for invariants that cross an account row and a relationship
/// predicate, where there is no single aggregate row that both writers can lock.
pub async fn run_serializable<R, F>(conn: &mut DbConn<'_>, operation: F) -> Result<R, DbError>
where
    R: Send,
    F: for<'r> Fn(
            &'r mut DbConn<'_>,
        ) -> Pin<Box<dyn Future<Output = Result<R, DbError>> + Send + 'r>>
        + Send
        + Sync,
{
    for attempt in 0..SERIALIZABLE_ATTEMPTS {
        let result = conn
            .transaction::<_, DbError, _>(async |tc| {
                tc.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .await?;
                operation(tc).await
            })
            .await;
        match result {
            Err(DbError::SerializationConflict) if attempt + 1 < SERIALIZABLE_ATTEMPTS => {}
            result => return result,
        }
    }
    unreachable!("the final serializable attempt always returns")
}
