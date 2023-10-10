use actix_identity::Identity;
use leptos::*;
use uuid::Uuid;

pub fn identity() -> Result<Identity, ServerFnError> {
    use actix_identity::IdentityExt;
    use actix_web::HttpRequest;
    let req = use_context::<HttpRequest>().ok_or(ServerFnError::ServerError(String::from(
        "Could not get request",
    )))?;
    IdentityExt::get_identity(&req).map_err(|e| ServerFnError::ServerError(e.to_string()))
}

pub fn uuid() -> Result<Uuid, ServerFnError> {
    let id_str = identity()?.id()?;
    Uuid::parse_str(&id_str).map_err(|e| {
        ServerFnError::ServerError(format!("Could not retrieve Uuid from identity: {e}"))
    })
}
