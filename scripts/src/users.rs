use crate::common::{log_operation_complete, log_operation_start, setup_database};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use log::info;
use uuid::Uuid;

pub async fn list_users(database_url: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    log_operation_start("user listing");

    let mut conn = setup_database(database_url).await?;

    let users = db_lib::schema::users::table
        .select((
            db_lib::schema::users::id,
            db_lib::schema::users::username,
            db_lib::schema::users::created_at,
        ))
        .load::<(Uuid, String, chrono::DateTime<Utc>)>(&mut conn)
        .await?;

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

pub async fn cleanup_test_data(
    database_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    log_operation_start("test data cleanup");

    let mut conn = setup_database(database_url).await?;

    // Delete test users (those starting with "testuser" and having test email)
    let deleted_users = diesel::delete(
        db_lib::schema::users::table.filter(
            db_lib::schema::users::username
                .like("testuser%")
                .and(db_lib::schema::users::email.like("test%@example.com")),
        ),
    )
    .execute(&mut conn)
    .await?;

    info!("Deleted {} test users", deleted_users);

    // Note: Games and ratings will be automatically deleted due to foreign key constraints

    log_operation_complete("Test data cleanup", deleted_users, 0);
    Ok(())
}
