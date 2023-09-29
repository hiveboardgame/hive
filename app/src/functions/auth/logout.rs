use leptos::*;

#[server(Logout, "/api")]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::identity;
    identity()?.logout();
    leptos_actix::redirect("/");
    Ok(())
}
