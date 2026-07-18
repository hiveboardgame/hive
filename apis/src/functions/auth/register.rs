use crate::responses::AccountResponse;
use leptos::prelude::*;

#[server]
pub async fn register(
    username: String,
    email: String,
    password: String,
    password_confirmation: String,
    pathname: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::{
        auth::password::{hash_password, validate_password},
        db::pool,
    };
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use db_lib::{
        get_conn,
        models::{NewUser, User},
    };
    use diesel_async::AsyncConnection;

    validate_password(&password, &password_confirmation).map_err(ServerFnError::new)?;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let password = hash_password(&password)?;
    let email = email.to_lowercase();
    let new_user = NewUser::new(&username, &password, &email)?;

    let (user, account) = conn
        .transaction::<_, ServerFnError, _>(async move |tc| {
            let user = User::create(new_user, tc).await?;
            let account = AccountResponse::from_uuid(&user.id, tc).await?;
            Ok((user, account))
        })
        .await?;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    Identity::login(&req.extensions(), user.id.to_string()).expect("To have logged in");
    leptos_actix::redirect(&pathname);

    Ok(account)
}
