use crate::functions::accounts::get::get_account;
use crate::functions::auth::{login::Login, logout::Logout, register::Register};
use crate::providers::websocket::WebsocketContext;
use crate::responses::AccountResponse;
use leptos::prelude::*;

#[derive(Clone)]
pub struct AuthContext {
    pub login: ServerAction<Login>,
    pub logout: ServerAction<Logout>,
    pub register: ServerAction<Register>,
    pub user: Resource<Result<AccountResponse, ServerFnError>>
}

/// Get the current user and place it in Context
pub fn provide_auth() {
    let login = ServerAction::<Login>::new();
    let logout = ServerAction::<Logout>::new();
    let register = ServerAction::<Register>::new();

    let user = Resource::new(
        move || {
            (
                login.version().get(),
                logout.version().get(),
                register.version().get(),
            )
        },
        move |_| get_account(),
    );

    Effect::new(move |_| {
        user.and_then(|user| {
            let websocket_context = expect_context::<WebsocketContext>();
            websocket_context.close();
            websocket_context.open();
        });
    });

    Effect::new(move |_| {
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
