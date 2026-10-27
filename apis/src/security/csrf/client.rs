use super::csrf_token;
use crate::common::UiError;
use futures_util::{Sink, Stream};
use leptos::prelude::window;
use server_fn::{
    client::{browser::BrowserClient, Client},
    error::{FromServerFnError, ServerFnErrorErr},
    request::browser::BrowserRequest,
    response::browser::BrowserResponse,
    Bytes,
};
use std::future::Future;
use web_sys::Url;

/// Adds the current session's token without changing Leptos's cancellation or response handling.
pub struct CsrfClient;

impl<E, I, O> Client<E, I, O> for CsrfClient
where
    E: FromServerFnError + Send,
    I: FromServerFnError,
    O: FromServerFnError,
{
    type Request = BrowserRequest;
    type Response = BrowserResponse;

    fn send(request: BrowserRequest) -> impl Future<Output = Result<BrowserResponse, E>> + Send {
        let validation = attach_token(&request).map_err(|message| {
            E::from_server_fn_error(ServerFnErrorErr::Request(message.encode()))
        });
        async move {
            validation?;
            <BrowserClient as Client<E, I, O>>::send(request).await
        }
    }

    fn open_websocket(
        path: &str,
    ) -> impl Future<
        Output = Result<
            (
                impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
                impl Sink<Bytes> + Send + 'static,
            ),
            E,
        >,
    > + Send {
        <BrowserClient as Client<E, I, O>>::open_websocket(path)
    }

    fn spawn(future: impl Future<Output = ()> + Send + 'static) {
        <BrowserClient as Client<E, I, O>>::spawn(future);
    }
}

fn attach_token(request: &BrowserRequest) -> Result<(), UiError> {
    if request.method().is_safe() {
        return Ok(());
    }
    let origin = window()
        .location()
        .origin()
        .map_err(|_| UiError::RequestOriginMissing)?;
    let url =
        Url::new_with_base(&request.url(), &origin).map_err(|_| UiError::RequestUrlInvalid)?;
    if url.origin() != origin {
        return Err(UiError::RequestOriginMismatch);
    }
    let token = csrf_token();
    if token.is_empty() {
        return Err(UiError::RequestTokenMissing);
    }
    request.headers().set("X-CSRF-Token", &token);
    Ok(())
}
