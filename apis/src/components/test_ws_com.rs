use leptos::prelude::*;

use crate::{common::ClientRequest, websocket::new_style::ClientApi};
#[component]
pub fn TestWsCom() -> impl IntoView {
    let ws_sender = expect_context::<ClientApi>();
    let latest = ws_sender.latest;
    view! {
        <div class="pt-20">
            <h1>Simple Echo WebSocket Communication</h1>
            <input
                type="text"
                on:input:target=move |ev| {
                    let msg = ev.target().value();
                    let msg = ClientRequest::DbgMsg(msg);
                    ws_sender.send(msg);
                }
            />
            <div class="flex flex-col">
                <ErrorBoundary fallback=|errors| {
                    view! {
                        <p>
                            {move || {
                                errors
                                    .get()
                                    .into_iter()
                                    .map(|(_, e)| format!("{e:?}"))
                                    .collect::<Vec<String>>()
                                    .join(" ")
                            }}
                        </p>
                    }
                }>
                    <p>
                        {if let Ok(msg) = latest() {
                            format!("{msg:?}")
                        } else {
                            "BAD".to_string()
                        }}
                    </p>
                </ErrorBoundary>
            </div>
        </div>
    }
}
