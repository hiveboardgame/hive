use crate::functions::accounts::get::get_account;
use crate::functions::auth::{login::Login, logout::Logout, register::Register};
use crate::providers::websocket::WebsocketContext;
use crate::responses::AccountResponse;
use leptos::{prelude::*, task::spawn};

#[derive(Clone)]
pub struct AuthContext {
    pub login: ServerAction<Login>,
    pub logout: ServerAction<Logout>,
    pub register: ServerAction<Register>,
    pub user: RwSignal<Option<Result<AccountResponse, ServerFnError>>>,
}

/// Get the current user and place it in Context
pub fn provide_auth(websocket_context: WebsocketContext) {
    let login = ServerAction::<Login>::new();
    let logout = ServerAction::<Logout>::new();
    let register = ServerAction::<Register>::new();
    let user = RwSignal::new(None);
    let ws_clone = websocket_context.clone();
    Effect::watch(
        move || {
            (
                login.version().get(),
                logout.version().get(),
                register.version().get(),
            )
        },
        move |curr, prev, _| {
            let ws = websocket_context.clone();
            spawn(async move {
                let account = get_account().await;
                if let Ok(account) = account {
                    user.set(Some(Ok(account)));
                    ws.close();
                    ws.open();
                } else {
                    user.set(None);
                }
            });
        },
        true,
    );
    Effect::watch(
        move || logout.version().get(),
        move |_, _, _| {
            //let ws = websocket_context.clone();
            logout.version().get();
            ws_clone.close();
        },
        true,
    );
    provide_context(AuthContext {
        user,
        login,
        logout,
        register,
    })
}
