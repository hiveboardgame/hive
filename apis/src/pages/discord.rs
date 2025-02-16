use crate::functions;
use crate::functions::oauth::GetDiscordHandle;
use crate::providers::AuthContext;
use crate::{components::molecules::banner::Banner, providers::ApiRequests};
use leptos::*;

#[component]
pub fn Discord() -> impl IntoView {
    let onclick = move |_| {
        let api = ApiRequests::new();
        api.link_discord();
    };
    let auth_context = expect_context::<AuthContext>();
    let discord_name = create_server_action::<GetDiscordHandle>();
    discord_name.dispatch(functions::oauth::GetDiscordHandle {});

    view! {
        <div class="pt-20">
            <div class="px-4 mx-auto max-w-4xl sm:px-6 lg:px-8">
                <Banner title="Link your Discord account".into_view() />
                <div>
                    <h2> Already linked account </h2>
                    <Show when=move || {
                        matches!((auth_context.user)(), Some(Ok(Some(_account))))
                    }>
                        { move || { discord_name.value().get() } }
                    </Show>
                </div>
                <div>
                    <button on:click=onclick>Link Discord</button>
                </div>
            </div>
        </div>
    }
}
