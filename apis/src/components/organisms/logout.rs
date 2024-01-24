use crate::providers::{auth_context::AuthContext, web_socket::WebsocketContext};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let websocket_context = expect_context::<WebsocketContext>();
    view! {
        <ActionForm
            action=auth_context.logout
            class=format!(
                "bg-ant-blue hover:bg-ladybug-red transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded {extend_tw_classes}",
            )
        >

            <input
                on:click=move |_| {
                    websocket_context.close();
                }

                type="submit"
                value="Logout"
            />
        </ActionForm>
    }
}
