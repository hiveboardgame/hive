use actix_identity::Identity;
use leptos::*;

pub fn identity() -> Result<Identity, ServerFnError> {
    use actix_identity::IdentityExt;
    use actix_web::HttpRequest;
    let req = use_context::<HttpRequest>().ok_or(ServerFnError::ServerError(String::from(
        "Could not get request",
    )))?;
    IdentityExt::get_identity(&req).map_err(|e| ServerFnError::ServerError(e.to_string()))
}
