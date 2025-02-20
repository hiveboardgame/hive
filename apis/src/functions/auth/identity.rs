use actix_identity::Identity;
use leptos::prelude::*;
use uuid::Uuid;

pub async fn identity() -> Result<Identity, ServerFnError> {
    leptos_actix::extract().await?
}

pub async fn uuid() -> Result<Uuid, ServerFnError> {
    let id_str = identity().await?.id()?;
    Uuid::parse_str(&id_str)
        .map_err(|e| ServerFnError::new(format!("Could not retrieve Uuid from identity: {e}")))
}
