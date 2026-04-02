use crate::{
    functions::{accounts::get::get_account, auth::logout::Logout},
    providers::websocket::WebsocketContext,
    responses::AccountResponse,
};
use leptos::prelude::*;
#[derive(Clone)]
pub struct AuthContext {
    pub logout: ServerAction<Logout>,
    pub user: Signal<Option<AccountResponse>>,
    /// Used to avoid redirecting to login while auth is still loading (e.g. on refresh).
    pub action: Action<(), Result<AccountResponse, ServerFnError>>,
    ws_refresh: StoredValue<bool>,
}

impl AuthContext {
    pub fn refresh(&self, ws_reconnect: bool) {
        self.ws_refresh.set_value(ws_reconnect);
        self.action.dispatch(());
    }
}
pub fn provide_auth() {
    let websocket_context = expect_context::<WebsocketContext>();
    let logout = ServerAction::<Logout>::new();
    let action = Action::new(move |_: &()| async { get_account().await });

    // Get the current user and place it in Context
    action.dispatch(());

    let user = Signal::derive(move || action.value().get().and_then(|v| v.ok()));
    let ws_refresh = StoredValue::new(false);

    provide_context(AuthContext {
        user,
        logout,
        ws_refresh,
        action,
    });

    let ctx = use_context::<AuthContext>().unwrap();

    Effect::watch(
        ctx.action.version(),
        move |_, _, _| {
            if ctx.ws_refresh.get_value() {
                websocket_context.close();
                websocket_context.open();
            }
        },
        false,
    );
    Effect::watch(
        ctx.logout.version(),
        move |_, _, _| {
            ctx.refresh(true);
        },
        false,
    );
}
