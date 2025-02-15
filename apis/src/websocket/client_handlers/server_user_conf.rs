use crate::{functions::accounts::get::get_account, providers::{websocket::WebsocketContext, AuthContext}};
use leptos::{prelude::{expect_context, Set}, task::spawn};
pub fn handle_server_user_conf(success: bool) {
    let auth_context = expect_context::<AuthContext>();
    if success {
        spawn(async move {
            let account = get_account().await;
            if let Ok(account) = account {
                auth_context.user.set(Some(Ok(account)));
                let websocket_context = expect_context::<WebsocketContext>();
                websocket_context.close();
                websocket_context.open();
            } else {
                auth_context.user.set(None);
            }
        });
    }
}
