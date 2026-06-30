use db_lib::{
    models::{NewEmailQueueItem, User},
    DbConn,
};

/// Queues a password-reset email. The plaintext token rides in the payload so the
/// drain worker can render the link at send time (only the hash is in `email_tokens`).
pub async fn enqueue_password_reset(
    conn: &mut DbConn<'_>,
    user: &User,
    plaintext_token: &str,
) -> Result<(), db_lib::db_error::DbError> {
    let payload = serde_json::json!({
        "token": plaintext_token,
        "username": user.username,
    });
    db_lib::models::EmailQueueItem::enqueue(
        NewEmailQueueItem {
            user_id: Some(user.id),
            kind: "password_reset".to_string(),
            payload,
            to_address: user.email.clone(),
        },
        conn,
    )
    .await?;
    Ok(())
}
