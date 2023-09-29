use actix_identity::Identity;
use leptos::*;

pub fn identity() -> Result<Identity, ServerFnError> {
    use actix_web::HttpRequest;
    use actix_identity::IdentityExt;
    let req = use_context::<HttpRequest>().expect("Failed to get request");
    IdentityExt::get_identity(&req).map_err(|e| ServerFnError::ServerError(e.to_string()))
}
