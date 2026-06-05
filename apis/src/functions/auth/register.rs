use leptos::prelude::*;

#[server]
pub async fn register(
    username: String,
    email: String,
    password: String,
    password_confirmation: String,
    pathname: String,
) -> Result<(), ServerFnError> {
    use crate::functions::{
        auth::password::{hash_password, validate_password},
        db::pool,
    };
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use db_lib::{
        db_error::DbError,
        get_conn,
        models::{NewUser, User},
    };
    use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};

    validate_password(&password, &password_confirmation).map_err(ServerFnError::new)?;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let password = hash_password(&password)?;
    let email = email.to_lowercase();
    let new_user = NewUser::new(&username, &password, &email)?;

    let user = conn
        .transaction::<_, DbError, _>(move |tc| {
            async move { User::create(new_user, tc).await }.scope_boxed()
        })
        .await?;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    Identity::login(&req.extensions(), user.id.to_string()).expect("To have logged in");
    leptos_actix::redirect(&pathname);

    Ok(())
}
