use leptos::prelude::*;

#[server]
pub async fn forgot_password(email: String) -> Result<(), ServerFnError> {
    use crate::{
        email::{enqueue_password_reset, generate_token},
        functions::db::pool,
    };
    use chrono::{Duration, Utc};
    use db_lib::{
        get_conn,
        models::{EmailRequestLog, EmailToken, User},
    };

    const PURPOSE: &str = "reset_password";
    const MAX_PER_EMAIL: i64 = 3;
    const MAX_PER_IP: i64 = 10;

    let email = email.trim().to_lowercase();
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    let since = Utc::now() - Duration::minutes(60);
    let by_email = EmailRequestLog::count_recent_email(&email, PURPOSE, since, &mut conn).await?;
    let by_ip = EmailRequestLog::count_recent_ip(&ip, PURPOSE, since, &mut conn).await?;
    EmailRequestLog::record(&email, &ip, PURPOSE, &mut conn).await?;
    if by_email >= MAX_PER_EMAIL || by_ip >= MAX_PER_IP {
        return Ok(());
    }

    if let Ok(user) = User::find_by_email(&email, &mut conn).await {
        let (plaintext, token_hash) = generate_token();
        let expires_at = Utc::now() + Duration::hours(1);
        EmailToken::issue(user.id, PURPOSE, &token_hash, expires_at, &mut conn).await?;
        enqueue_password_reset(&mut conn, &user, &plaintext).await?;
    }

    Ok(())
}
