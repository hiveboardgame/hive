use crate::functions::accounts::get::get_account;
use crate::functions::auth::logout::Logout;
use crate::providers::websocket::WebsocketContext;
use crate::responses::AccountResponse;
use leptos::prelude::*;
#[derive(Clone)]
pub struct AuthContext {
    pub logout: ServerAction<Logout>,
    pub user: Signal<Option<AccountResponse>>,
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

    let user = Signal::derive(move || action.value().get().and_then(|v| v.ok()));
    let ws_refresh = StoredValue::new(false);

    provide_context(AuthContext {
        user,
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
            ctx.refresh(true);
        },
        false,
    );
}
