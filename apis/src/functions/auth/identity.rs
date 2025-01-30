use actix_identity::Identity;
use leptos::prelude::*;
use uuid::Uuid;

pub fn identity() -> Result<Identity, ServerFnError> {
    use actix_identity::IdentityExt;
    use actix_web::HttpRequest;
    let req = use_context::<HttpRequest>().ok_or(ServerFnError::new("Could not get request"))?;
    IdentityExt::get_identity(&req).map_err(ServerFnError::new)
}

pub fn uuid() -> Result<Uuid, ServerFnError> {
    let id_str = identity()?.id()?;
    Uuid::parse_str(&id_str)
        .map_err(|e| ServerFnError::new(format!("Could not retrieve Uuid from identity: {e}")))
}
