use crate::functions::accounts::get::get_account;
use crate::functions::auth::logout::Logout;
use crate::providers::websocket::WebsocketContext;
use crate::responses::AccountResponse;
use leptos::prelude::*;
#[derive(Clone)]
pub struct AuthContext {
    pub logout: ServerAction<Logout>,
    pub user: Signal<Option<AccountResponse>>,
    action: Action<(), Result<AccountResponse, ServerFnError>>,
}

impl AuthContext {
    pub fn refresh(&self) {
        self.action.dispatch(());
    }
}
pub fn provide_auth(websocket_context: WebsocketContext) {
    let logout = ServerAction::<Logout>::new();
    let action = Action::new(move |_: &()| async { get_account().await });

    // Get the current user and place it in Context
    action.dispatch(());

    Effect::watch(
        logout.version(),
        move |_, _, _| {
            action.dispatch(());
        },
        false,
    );
    Effect::watch(
        action.version(),
        move |_, _, _| {
            websocket_context.close();
            websocket_context.open();
        },
        false,
    );
    let user = Signal::derive(move || action.value().get().and_then(|v| v.ok()));
    provide_context(AuthContext {
        user,
        logout,
        action,
    })
}
