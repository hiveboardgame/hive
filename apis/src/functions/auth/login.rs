use crate::responses::AccountResponse;
use leptos::prelude::*;

#[server]
pub async fn login(
    email: String,
    password: String,
    pathname: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::{auth::password::verify_password, db::pool};
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_for_login(&email, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    verify_password(&password, &user.password)?;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    Identity::login(&req.extensions(), user.id.to_string())?;
    leptos_actix::redirect(&pathname);
    AccountResponse::from_uuid(&user.id, &mut conn).await
}
