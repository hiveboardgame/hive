use crate::functions::accounts::account_response::AccountResponse;
use crate::functions::accounts::get::get_account;
use crate::functions::auth::{login::Login, logout::Logout, register::Register};
use leptos::*;

use super::web_socket::WebsocketContext;

#[derive(Clone)]
pub struct AuthContext {
    pub login: Action<Login, Result<AccountResponse, ServerFnError>>,
    pub logout: Action<Logout, Result<(), ServerFnError>>,
    pub register: Action<Register, Result<(), ServerFnError>>,
    pub user: Resource<(usize, usize, usize), Result<Option<AccountResponse>, ServerFnError>>,
}

/// Get the current user and place it in Context
pub fn provide_auth() {
    let websocket_context = expect_context::<WebsocketContext>();
    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let register = create_server_action::<Register>();

    let user = create_local_resource(
        move || {
            (
                login.version().get(),
                logout.version().get(),
                register.version().get(),
            )
        },
        move |_| {
            // TODO: @leex try this
            // first we get the account: get_account
            // then once the account is resolved
            // we provide the WebsocketContext which then should
            // already have access to the auth id
            websocket_context.close();
            websocket_context.open();
            get_account()
        },
    );

    provide_context(AuthContext {
        user,
        login,
        logout,
        register,
    })
}
