use crate::{functions::accounts::edit::edit_lang, i18n::*, providers::AuthContext};
use leptos::{prelude::*, task::spawn_local};

pub fn use_sync_user_locale() {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    Effect::new(move |_| {
        let Some(account) = auth.user.get() else {
            return;
        };
        match account.user.lang.as_deref() {
            Some(code) if !code.is_empty() => {
                if let Some(locale) = Locale::get_all()
                    .iter()
                    .find(|l| l.to_string() == code)
                    .copied()
                {
                    if untrack(|| i18n.get_locale()) != locale {
                        i18n.set_locale(locale);
                    }
                }
            }
            _ => {
                let current = untrack(|| i18n.get_locale()).to_string();
                spawn_local(async move {
                    let _ = edit_lang(current).await;
                });
            }
        }
    });
}
