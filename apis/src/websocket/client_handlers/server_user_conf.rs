use crate::providers::AuthContext;
use leptos::prelude::expect_context;
pub fn handle_server_user_conf(success: bool) {
    let auth_context = expect_context::<AuthContext>();
    if success {
        auth_context.action.dispatch(());
    }
}
