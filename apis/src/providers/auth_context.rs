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
    pub logged_in: Signal<Option<bool>>,
    pub admin: Signal<Option<bool>>,
    ws_refresh: StoredValue<bool>,
    action: Action<(), Result<AccountResponse, ServerFnError>>,
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

    let account = action.value();
    let user = Signal::derive(move || account.get().and_then(|v| v.ok()));
    let account = action.value();
    let logged_in = Signal::derive(move || match account.get() {
        Some(Ok(_)) => Some(true),
        Some(Err(_)) => Some(false),
        None => None,
    });
    let account = action.value();
    let admin = Signal::derive(move || match account.get() {
        Some(Ok(account)) => Some(account.user.admin),
        Some(Err(_)) => Some(false),
        None => None,
    });
    let ws_refresh = StoredValue::new(false);

    provide_context(AuthContext {
        user,
        logged_in,
        admin,
        logout,
        action,
        ws_refresh,
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
            // Clear any client-side bearer token (HiveGame mobile / cross-origin
            // clients). The backend's Identity cookie is wiped by the logout
            // server fn separately. For same-origin SSR + hydrate this is a
            // no-op since no token was ever stored.
            crate::client::set_token(None);
            ctx.refresh(true);
        },
        false,
    );
}
