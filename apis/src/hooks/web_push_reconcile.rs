use crate::{
    providers::{AuthContext, AuthIdentity},
    pwa,
};
use leptos::{prelude::*, task::spawn_local};

pub fn use_web_push_reconcile() {
    let auth = expect_context::<AuthContext>();
    Effect::new(move |_| {
        if matches!(auth.identity.get(), Some(AuthIdentity::User(_))) {
            spawn_local(async move {
                if pwa::is_subscribed().await {
                    pwa::reconcile_subscription().await;
                }
            });
        }
    });
}
