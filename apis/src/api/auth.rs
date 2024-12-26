use actix_web::{dev::ServiceRequest, Error, error::ErrorUnauthorized};
use actix_web_httpauth::extractors::bearer::BearerAuth;

const SECRET_KEY: &[u8] = b"leexTODO";

pub async fn psk_auth(
    req: ServiceRequest,
    credentials: Option<&BearerAuth>,
) -> Result<ServiceRequest, Error> {
    match credentials {
        Some(bearer) => {
            if bearer.token().as_bytes() != SECRET_KEY {
                return Err(ErrorUnauthorized("Token not valid"));
            }
        },
        None => return Err(ErrorUnauthorized("No token provided"))
    }
    Ok(req)
}
