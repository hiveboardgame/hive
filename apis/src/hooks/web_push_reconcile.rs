use crate::{providers::AuthContext, pwa};
use leptos::{prelude::*, task::spawn_local};

pub fn use_web_push_reconcile() {
    let auth = expect_context::<AuthContext>();
    Effect::new(move |_| {
        if auth.logged_in.get() == Some(true) {
            spawn_local(async move {
                if pwa::is_subscribed().await {
                    pwa::reconcile_subscription().await;
                }
            });
        }
    });
}
