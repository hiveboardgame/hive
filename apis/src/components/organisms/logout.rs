use crate::{
    i18n::*,
    providers::{online_users::OnlineUsersSignal, AuthContext},
};
use leptos::{form::ActionForm, prelude::*};

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let mut online_users = expect_context::<OnlineUsersSignal>();
    let i18n = use_i18n();

    // Push-device id is CSR-only — it lives in localStorage on mobile and
    // is None on SSR/hydrate. We feed it through a hidden form field so the
    // Logout server fn can delete the matching push_devices row while the
    // bearer is still attached to the request.
    let push_device_id = RwSignal::new(None::<String>);
    #[cfg(not(feature = "ssr"))]
    {
        Effect::new(move |_| {
            if let Some(storage) = web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
            {
                if let Ok(Some(id)) = storage.get_item("hive-push-device-id") {
                    push_device_id.set(Some(id));
                }
            }
        });
    }

    view! {
        <div class=format!("m-1 {extend_tw_classes}")>
            <ActionForm action=auth_context.logout>
                <Show when=move || push_device_id.with(Option::is_some)>
                    <input
                        type="hidden"
                        name="device_id"
                        value=move || push_device_id.get().unwrap_or_default()
                    />
                </Show>

                <button
                    on:click=move |_| {
                        auth_context
                            .user
                            .with(|a| {
                                if let Some(account) = a {
                                    online_users.remove(account.user.username.clone());
                                }
                            });
                    }

                    class="flex place-content-start py-2 px-4 font-bold text-white rounded transition-transform duration-300 active:scale-95 size-full bg-button-dawn dark:bg-button-twilight hover:bg-ladybug-red"
                    type="submit"
                >
                    {t!(i18n, header.user_menu.logout)}
                </button>
            </ActionForm>
        </div>
    }
}
