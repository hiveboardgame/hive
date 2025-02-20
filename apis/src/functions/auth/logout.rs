use leptos::prelude::*;

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::identity;
    identity()?.logout();
    Ok(())
}
