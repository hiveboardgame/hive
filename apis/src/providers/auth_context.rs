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
    pub user: RwSignal<Option<Result<AccountResponse, ServerFnError>>>,
    pub action: Action<(), Result<AccountResponse, ServerFnError>>,
}

/// Get the current user and place it in Context
pub fn provide_auth(websocket_context: WebsocketContext) {
    let login = ServerAction::<Login>::new();
    let logout = ServerAction::<Logout>::new();
    let register = ServerAction::<Register>::new();
    let action = Action::new(
        move |_: &()| {
            async {
                get_account().await
            }
        });
    let user = action.value();
    Effect::watch(
        move || {
            (
                login.version().get(),
                logout.version().get(),
                register.version().get(),
            )
        },
        move |_, _, _| {
            action.dispatch(());
        },
        true,
    );
    Effect::watch(
        move || logout.version().get(),
        move |_, _, _| {
            websocket_context.close();
        },
        true,
    );
    provide_context(AuthContext {
        user,
        login,
        logout,
        register,
        action,
    })
}
