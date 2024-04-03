use crate::functions::accounts::account_response::AccountResponse;
use crate::functions::accounts::get::get_account;
use crate::functions::auth::{login::Login, logout::Logout, register::Register};
use crate::providers::websocket::context::WebsocketContext;
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

    let user = create_local_resource(
        move || {
            (
                login.version().get(),
                logout.version().get(),
                register.version().get(),
            )
        },
        move |_| get_account(),
    );

    create_effect(move |_| {
        user.and_then(|user| {
            let websocket_context = expect_context::<WebsocketContext>();
            websocket_context.close();
            if user.is_some() {
                websocket_context.open();
            }
        });
    });

    create_effect(move |_| {
        let websocket_context = expect_context::<WebsocketContext>();
        logout.version().get();
        websocket_context.close();
    });

    provide_context(AuthContext {
        user,
        login,
        logout,
        register,
    })
}
