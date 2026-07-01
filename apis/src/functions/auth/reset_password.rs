use leptos::prelude::*;

#[server]
pub async fn reset_password(
    token: String,
    new_password: String,
    new_password_confirmation: String,
) -> Result<(), ServerFnError> {
    use crate::{
        email::hash_token,
        functions::{
            auth::password::{hash_password, validate_password},
            db::pool,
        },
    };
    use db_lib::{
        db_error::DbError,
        get_conn,
        models::{EmailToken, User},
    };
    use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};

    const PURPOSE: &str = "reset_password";

    validate_password(&new_password, &new_password_confirmation).map_err(ServerFnError::new)?;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    let password_hash = hash_password(&new_password)?;
    let token_hash = hash_token(&token);

    conn.transaction::<_, DbError, _>(|tc| {
        async move {
            let email_token = EmailToken::find_valid(&token_hash, PURPOSE, tc).await?;
            if !EmailToken::consume(email_token.id, tc).await? {
                return Err(DbError::NotFound {
                    reason: "reset token already used".to_owned(),
                });
            }
            let user = User::find_active_by_uuid(&email_token.user_id, tc).await?;
            user.edit(&password_hash, "", tc).await?;
            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(|e| match e {
        DbError::NotFound { .. } => {
            ServerFnError::new("This reset link is invalid or has expired.")
        }
        other => ServerFnError::new(other),
    })?;

    leptos_actix::redirect("/login");
    Ok(())
}

/// Checks whether a reset token is still usable, without consuming it, so the
/// page can show an "expired link" state up front instead of after submit.
#[server]
pub async fn verify_reset_token(token: String) -> Result<bool, ServerFnError> {
    use crate::{email::hash_token, functions::db::pool};
    use db_lib::{db_error::DbError, get_conn, models::EmailToken};

    if token.is_empty() {
        return Ok(false);
    }
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    match EmailToken::find_valid(&hash_token(&token), "reset_password", &mut conn).await {
        Ok(_) => Ok(true),
        Err(DbError::NotFound { .. }) => Ok(false),
        Err(err) => Err(ServerFnError::new(err)),
    }
}
