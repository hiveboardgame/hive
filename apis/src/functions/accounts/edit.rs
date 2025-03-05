use crate::responses::AccountResponse;
use leptos::prelude::*;
use shared_types::Takeback;
use crate::functions::accounts::edit::server_fn::codec;

#[server]
pub async fn edit_account(
    new_email: String,
    new_password: String,
    new_password_confirmation: String,
    password: String,
    pathname: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use argon2::{
        password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
        Argon2,
    };
    use db_lib::get_conn;
    use db_lib::models::User;
    use rand_core::OsRng;

    if new_password != new_password_confirmation {
        return Err(ServerFnError::new("Passwords don't match."));
    }
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

    user.edit(&hashed_password, &new_email, &mut conn).await?;
    leptos_actix::redirect(&pathname);
    AccountResponse::from_uuid(&user.id, &mut conn).await
}

#[server(input = codec::Json)]
pub async fn edit_config(takeback: Takeback)  -> Result<(),ServerFnError> {
    use db_lib::models::User;
    use db_lib::get_conn;
    use crate::functions::db::pool;
    use crate::functions::auth::identity::uuid;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_uuid(&uuid().await?, &mut conn).await?;
    user.set_takeback(takeback.clone(), &mut conn).await?;
    Ok(())
}
