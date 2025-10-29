use crate::common::{log_operation_complete, log_operation_start, setup_database};
use anyhow::{Context, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use log::info;
use uuid::Uuid;

pub async fn list_users(database_url: Option<String>) -> Result<()> {
    log_operation_start("user listing");

    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;

    let users = db_lib::schema::users::table
        .select((
            db_lib::schema::users::id,
            db_lib::schema::users::username,
            db_lib::schema::users::created_at,
        ))
        .load::<(Uuid, String, chrono::DateTime<Utc>)>(&mut conn)
        .await
        .context("Failed to load users from database")?;

    info!("Found {} users:", users.len());
    let user_count = users.len();
    for (id, username, created_at) in users {
        info!(
            "  {} ({}) - created {}",
            username,
            id,
            created_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    log_operation_complete("User listing", user_count, 0);
    Ok(())
}

pub async fn cleanup_test_data(database_url: Option<String>) -> Result<()> {
    log_operation_start("test data cleanup");

    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;

    let deleted_users = diesel::delete(
        db_lib::schema::users::table.filter(
            db_lib::schema::users::username
                .like("testuser%")
                .and(db_lib::schema::users::email.like("test%@example.com")),
        ),
    )
    .execute(&mut conn)
    .await
    .context("Failed to delete test users from database")?;

    info!("Deleted {} test users", deleted_users);

    log_operation_complete("Test data cleanup", deleted_users, 0);
    Ok(())
}
