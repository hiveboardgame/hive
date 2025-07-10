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
    use crate::functions::auth::identity::uuid;
    use crate::functions::auth::register::validate_password;
    use crate::functions::db::pool;
    use argon2::{
        password_hash::{
            rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        },
        Argon2,
    };
    use db_lib::get_conn;
    use db_lib::models::User;

    validate_password(&new_password, &new_password_confirmation).map_err(ServerFnError::new)?;
    let uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_uuid(&uuid, &mut conn).await?;
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&user.password).map_err(ServerFnError::new)?;

    if argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(ServerFnError::new("Password does not match."));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = argon2
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(ServerFnError::new)?
        .to_string();

    user.edit(&hashed_password, "", &mut conn).await?;
    leptos_actix::redirect(&pathname);
    AccountResponse::from_uuid(&user.id, &mut conn).await
}

#[server]
pub async fn edit_takeback(takeback: Takeback) -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::User;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_uuid(&uuid().await?, &mut conn).await?;
    user.set_takeback(takeback.clone(), &mut conn).await?;
    Ok(())
}
