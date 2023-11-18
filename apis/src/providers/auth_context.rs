use crate::functions::accounts::account_response::AccountResponse;
use crate::functions::accounts::get::get_account;
use crate::functions::auth::{login::Login, logout::Logout, register::Register};
use leptos::*;

#[derive(Clone)]
pub struct AuthContext {
    pub login: Action<Login, Result<AccountResponse, ServerFnError>>,
    pub logout: Action<Logout, Result<(), ServerFnError>>,
    pub register: Action<Register, Result<(), ServerFnError>>,
    pub user: Resource<(usize, usize, usize), Result<Option<AccountResponse>, ServerFnError>>,
}

/// Get the current user and place it in Context
pub fn provide_auth() {
    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let register = create_server_action::<Register>();

    let user = create_blocking_resource(
        move || {
            (
                login.version().get(),
                logout.version().get(),
                register.version().get(),
            )
        },
        move |_| get_account(),
    );

    provide_context(AuthContext {
        user,
        login,
        logout,
        register,
    })
}

