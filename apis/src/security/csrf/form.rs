use super::{CsrfClient, CsrfInput};
use leptos::{form::ActionForm as LeptosActionForm, html::Form, prelude::*};
use serde::de::DeserializeOwned;
use server_fn::{codec::PostUrl, Http, ServerFn};

/// Includes a token for native submissions; hydrated submissions use `CsrfClient`.
#[component]
pub fn ActionForm<ServFn, OutputProtocol>(
    action: ServerAction<ServFn>,
    #[prop(optional)] node_ref: NodeRef<Form>,
    children: Children,
) -> impl IntoView
where
    ServFn: DeserializeOwned
        + ServerFn<Protocol = Http<PostUrl, OutputProtocol>, Client = CsrfClient>
        + Clone
        + Send
        + Sync
        + 'static,
    ServFn::Output: Send + Sync + 'static,
    ServFn::Error: Send + Sync + 'static,
{
    view! {
        <LeptosActionForm action=action node_ref=node_ref>
            <CsrfInput />
            {children()}
        </LeptosActionForm>
    }
}
