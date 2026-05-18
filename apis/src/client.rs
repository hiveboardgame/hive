//! Custom `server_fn::client::Client` implementation that attaches a bearer
//! token to every request (both HTTP and WebSocket).
//!
//! Used by the HiveGame mobile (culex) CSR build, where cookies don't flow
//! reliably between the Tauri webview origin and the backend. SSR + hydrate
//! same-origin builds also link this — the token store stays empty in those
//! paths, so it transparently becomes a no-op pass-through to the underlying
//! `BrowserClient`.
//!
//! Backend resolution happens in `apis/src/functions/auth/identity.rs::uuid()`
//! (HTTP) and `apis/src/websocket/start_conn.rs` (WebSocket).
//!
//! `server_fn`'s `browser` feature is forced on in `apis/Cargo.toml` so
//! `BrowserClient`/`BrowserRequest`/`BrowserResponse` are available for all
//! builds (including the SSR bin). Without this, the ServerFn macro's
//! `type Client = ApiClient` would fail the `Client<E>` trait bound in SSR.

use bytes::Bytes;
use futures_util::{Sink, Stream};
use server_fn::{
    client::{browser::BrowserClient, Client},
    error::FromServerFnError,
    request::browser::BrowserRequest,
    response::browser::BrowserResponse,
};
use std::future::Future;
use std::sync::{OnceLock, RwLock};

const STORAGE_KEY: &str = "hivegame_token";

fn store() -> &'static RwLock<Option<String>> {
    static TOKEN: OnceLock<RwLock<Option<String>>> = OnceLock::new();
    TOKEN.get_or_init(|| RwLock::new(None))
}

/// Read the current bearer token. Returns `None` server-side (no localStorage)
/// and on a freshly-loaded client before login.
pub fn get_token() -> Option<String> {
    store().read().ok().and_then(|g| g.clone())
}

/// Set the bearer token in memory and mirror it to `localStorage` so it
/// survives page reload. Pass `None` to clear (logout). No-op server-side.
pub fn set_token(token: Option<String>) {
    if let Ok(mut g) = store().write() {
        g.clone_from(&token);
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    match token {
        Some(t) => {
            let _ = storage.set_item(STORAGE_KEY, &t);
        }
        None => {
            let _ = storage.remove_item(STORAGE_KEY);
        }
    }
}

/// Hydrate the in-memory token from `localStorage`. Call once on client
/// startup before any server function fires. No-op server-side.
pub fn load_token_from_storage() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    if let Ok(Some(t)) = storage.get_item(STORAGE_KEY) {
        if let Ok(mut g) = store().write() {
            *g = Some(t);
        }
    }
}

/// Server-fn client that attaches `Authorization: Bearer <token>` to every
/// HTTP request and appends `?token=<token>` to every WebSocket connection.
/// Wraps the default `BrowserClient`.
pub struct ApiClient;

impl<E, IE, OE> Client<E, IE, OE> for ApiClient
where
    E: FromServerFnError,
    IE: FromServerFnError,
    OE: FromServerFnError,
{
    type Request = BrowserRequest;
    type Response = BrowserResponse;

    fn send(
        req: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, E>> + Send {
        if let Some(token) = get_token() {
            // BrowserRequest derefs to gloo_net::http::Request; its
            // .headers() returns a web_sys::Headers wrapper backed by the
            // same JS object the underlying fetch RequestInit holds, so
            // mutating it here propagates to the actual fetch call.
            let _ = req
                .headers()
                .append("Authorization", &format!("Bearer {token}"));
        }
        <BrowserClient as Client<E, IE, OE>>::send(req)
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
        // No header/URL mangling: the connection opens anonymously and the
        // caller is expected to send `ClientRequest::Auth(token)` as the
        // first frame if a token is present. Keeps tokens out of URLs.
        <BrowserClient as Client<E, IE, OE>>::open_websocket(path)
    }

    fn spawn(future: impl Future<Output = ()> + Send + 'static) {
        <BrowserClient as Client<E, IE, OE>>::spawn(future)
    }
}
