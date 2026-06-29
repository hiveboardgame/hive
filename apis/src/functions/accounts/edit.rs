use crate::responses::AccountResponse;
use leptos::prelude::*;
use shared_types::Takeback;

#[server]
pub async fn edit_account(
    new_password: String,
    new_password_confirmation: String,
    password: String,
    pathname: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::{
        auth::{
            identity::uuid,
            password::{hash_password, validate_password, verify_password},
        },
        db::pool,
    };
    use db_lib::{get_conn, models::User};

    validate_password(&new_password, &new_password_confirmation).map_err(ServerFnError::new)?;
    let uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_active_by_uuid(&uuid, &mut conn).await?;
    verify_password(&password, &user.password)?;
    let hashed_password = hash_password(&new_password)?;

    user.edit(&hashed_password, "", &mut conn).await?;
    leptos_actix::redirect(&pathname);
    AccountResponse::from_uuid(&user.id, &mut conn).await
}

#[server]
pub async fn edit_takeback(takeback: Takeback) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_active_by_uuid(&uuid().await?, &mut conn).await?;
    user.set_takeback(takeback.clone(), &mut conn).await?;
    Ok(())
}

#[server]
pub async fn edit_lang(lang: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_active_by_uuid(&uuid().await?, &mut conn).await?;
    user.set_lang(&lang, &mut conn).await?;
    Ok(())
}
