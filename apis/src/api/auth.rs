use actix_web::{dev::ServiceRequest, error::ErrorUnauthorized, web::Data, Error};
use actix_web_httpauth::extractors::basic::BasicAuth;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use db_lib::{get_conn, models::User, DbPool};
use uuid::Uuid;

pub async fn validate_basic_auth(
    req: ServiceRequest,
    credentials: Option<&BasicAuth>,
) -> Result<ServiceRequest, Error> {
    match credentials {
        Some(basic) => {
            let pool = req
                .app_data::<Data<DbPool>>()
                .ok_or("Failed to get pool")
                .map_err(ErrorUnauthorized)?
                .get_ref()
                .clone();
            let user_id = basic.user_id();
            let uuid = Uuid::parse_str(user_id)
                .map_err(|e| ErrorUnauthorized(format!("Not a valid uuid: {e}")))?;
            let mut conn = get_conn(&pool)
                .await
                .map_err(|e| ErrorUnauthorized(format!("No DB connection: {e}")))?;
            let user = User::find_by_uuid(&uuid, &mut conn)
                .await
                .map_err(|e| ErrorUnauthorized(format!("No such uuid in the DB: {e}")))?;
            let argon2 = Argon2::default();
            let parsed_hash = PasswordHash::new(&user.password)
                .map_err(|e| ErrorUnauthorized(format!("Password could not be hashed: {e}")))?;
            let password = basic.password();
            match password {
                None => return Err(ErrorUnauthorized("No password provided")),
                Some(password) => {
                    if argon2
                        .verify_password(password.as_bytes(), &parsed_hash)
                        .is_err()
                    {
                        return Err(ErrorUnauthorized("Password does not match."));
                    }
                }
            }
        }
        None => return Err(ErrorUnauthorized("No basic auth provided")),
    }
    Ok(req)
}
