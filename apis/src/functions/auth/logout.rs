use leptos::prelude::*;

#[server(client = crate::client::ApiClient)]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::identity;
    identity().await?.logout();
    Ok(())
}
