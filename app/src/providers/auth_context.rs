use crate::functions::accounts::account_response::AccountResponse;
use crate::functions::accounts::get::get_account;
use crate::functions::auth::{login::Login, logout::Logout, signup::Signup};
use leptos::*;

#[derive(Clone)]
pub struct AuthContext {
    pub login: Action<Login, Result<AccountResponse, ServerFnError>>,
    pub logout: Action<Logout, Result<(), ServerFnError>>,
    pub signup: Action<Signup, Result<(), ServerFnError>>,
    pub user: Resource<(usize, usize, usize), Result<AccountResponse, ServerFnError>>,
}
/// Get the current user and place it in Context
pub fn provide_auth() {
    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let signup = create_server_action::<Signup>();

    let user = create_resource(
        move || {
            (
                login.version().get(),
                signup.version().get(),
                logout.version().get(),
            )
        },
        move |_| get_account(),
    );

    provide_context(AuthContext {
        user,
        login,
        logout,
        signup,
    })
}
