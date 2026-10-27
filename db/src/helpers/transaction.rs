use crate::{db_error::DbError, DbConn};
use diesel::result::Error as DieselError;
use diesel_async::{
    AnsiTransactionManager,
    AsyncConnection,
    SimpleAsyncConnection,
    TransactionManager,
};
use std::{future::Future, pin::Pin};

const SERIALIZABLE_ATTEMPTS: usize = 5;

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
    if AnsiTransactionManager::transaction_manager_status_mut(&mut **conn)
        .transaction_depth()?
        .is_some()
    {
        return Err(DieselError::AlreadyInTransaction.into());
    }
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
    if AnsiTransactionManager::transaction_manager_status_mut(&mut **conn)
        .transaction_depth()?
        .is_some()
    {
        return Err(DieselError::AlreadyInTransaction.into());
    }
    for attempt in 0..SERIALIZABLE_ATTEMPTS {
        let result = conn
            .transaction::<_, DbError, _>(async |tc| {
                tc.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .await?;
                operation(tc).await
            })
            .await;
        match result {
            Err(DbError::SerializationConflict) if attempt + 1 < SERIALIZABLE_ATTEMPTS => {
                // Back off after rollback so contenders do not immediately collide again.
                tokio::time::sleep(std::time::Duration::from_millis(10 << attempt)).await;
            }
            result => return result,
        }
    }
    unreachable!("the final serializable attempt always returns")
}

#[cfg(test)]
mod tests {
    use super::{run_read_only_repeatable_read, run_serializable};
    use crate::{
        db_error::DbError,
        get_conn,
        schema::users,
        test_support::{db::test_db, tournament::create_user},
    };
    use diesel::{prelude::*, result::Error as DieselError};
    use diesel_async::{AsyncConnection, RunQueryDsl};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn serializable_retry_rolls_back_failed_attempts_before_one_commit() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&attempts);
        run_serializable(&mut conn, move |tc| {
            let counter = Arc::clone(&counter);
            Box::pin(async move {
                create_user("retry_tx_user", tc).await;
                if counter.fetch_add(1, Ordering::SeqCst) < 3 {
                    Err(DbError::SerializationConflict)
                } else {
                    Ok(())
                }
            })
        })
        .await
        .unwrap();
        assert_eq!(attempts.as_ref().load(Ordering::SeqCst), 4);
        assert_eq!(
            users::table
                .filter(users::username.eq("retry_tx_user"))
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn isolation_helpers_reject_an_existing_transaction_before_running_the_operation() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        conn.transaction::<_, DbError, _>(async |tc| {
            let result =
                run_read_only_repeatable_read(tc, |_| Box::pin(async { Ok::<_, DieselError>(()) }))
                    .await;
            assert!(matches!(result, Err(DieselError::AlreadyInTransaction)));
            let result = run_serializable(tc, |_| Box::pin(async { Ok(()) })).await;
            assert!(matches!(result, Err(DbError::InternalError { .. })));
            Ok(())
        })
        .await
        .unwrap();
    }
}
