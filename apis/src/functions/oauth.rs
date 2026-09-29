use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscordHandleStatus {
    Linked(String),
    NotLinked,
    NotLoggedIn,
    Unavailable,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

use crate::security::csrf::session_fingerprint;
use actix_identity::Identity;
use actix_session::SessionExt;
use actix_web::{
    cookie::{time::Duration, Cookie, CookieJar, Key, SameSite},
    error::{ErrorForbidden, ErrorInternalServerError},
    get,
    http::header::{HeaderValue, CACHE_CONTROL, SET_COOKIE},
    web,
    Error,
    HttpRequest,
    HttpResponse,
};
use chrono::Utc;
use leptos_actix::ResponseOptions;
use reqwest::{Client, Response};
use subtle::ConstantTimeEq;
use url::Url;
use uuid::Uuid;

const DISCORD_LINK_COOKIE: &str = if cfg!(debug_assertions) {
    "hive_discord_link"
} else {
    "__Host-hive_discord_link"
};
const DISCORD_LINK_LIFETIME: Duration = Duration::minutes(20);

pub struct DiscordLinkKey(pub Key);

#[derive(Clone, Deserialize, Serialize)]
struct PendingDiscordLink {
    state: String,
    user_id: Uuid,
    session_fingerprint: [u8; 32],
    expires_at: i64,
}

fn discord_link_key(request: &HttpRequest) -> Result<&Key, Error> {
    request
        .app_data::<web::Data<DiscordLinkKey>>()
        .map(|key| &key.get_ref().0)
        .ok_or_else(|| ErrorInternalServerError("Discord link cookie key is not configured"))
}

fn discord_link_cookie(value: String) -> Cookie<'static> {
    Cookie::build(DISCORD_LINK_COOKIE, value)
        .path("/")
        .secure(!cfg!(debug_assertions))
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(DISCORD_LINK_LIFETIME)
        .finish()
}

fn pending_discord_link_cookie(
    request: &HttpRequest,
    user_id: Uuid,
    state: String,
) -> Result<Cookie<'static>, Error> {
    let fingerprint = session_fingerprint(&request.get_session())?
        .ok_or_else(|| ErrorForbidden("Discord linking requires an existing login session"))?;
    let pending = PendingDiscordLink {
        state,
        user_id,
        session_fingerprint: fingerprint,
        expires_at: Utc::now().timestamp() + DISCORD_LINK_LIFETIME.whole_seconds(),
    };
    let value = serde_json::to_string(&pending).map_err(ErrorInternalServerError)?;
    let mut jar = CookieJar::new();
    jar.private_mut(discord_link_key(request)?)
        .add(discord_link_cookie(value));
    Ok(jar.get(DISCORD_LINK_COOKIE).expect("inserted Discord link cookie").clone())
}

fn validate_pending_discord_link(
    request: &HttpRequest,
    user_id: Option<Uuid>,
    state: &str,
) -> Result<bool, Error> {
    let Some(cookie) = request.cookie(DISCORD_LINK_COOKIE) else {
        return Ok(false);
    };
    let mut jar = CookieJar::new();
    jar.add_original(cookie);
    let Some(cookie) = jar.private(discord_link_key(request)?).get(DISCORD_LINK_COOKIE) else {
        return Ok(false);
    };
    let Ok(pending) = serde_json::from_str::<PendingDiscordLink>(cookie.value()) else {
        return Ok(false);
    };
    let fingerprint = session_fingerprint(&request.get_session())?;
    Ok(user_id == Some(pending.user_id)
        && state == pending.state
        && Utc::now().timestamp() < pending.expires_at
        && fingerprint.is_some_and(|fingerprint| {
            bool::from(fingerprint.ct_eq(&pending.session_fingerprint))
        }))
}

fn clear_discord_link_cookie(response: &mut HttpResponse) -> Result<(), Error> {
    let mut cookie = discord_link_cookie(String::new());
    cookie.make_removal();
    response.add_cookie(&cookie).map_err(ErrorInternalServerError)?;
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct OAuthParams {
    code: String,
    state: String,
}

#[get("/oauth/callback")]
pub async fn callback(
    request: HttpRequest,
    params: web::Query<OAuthParams>,
    identity: Option<Identity>,
) -> actix_web::Result<HttpResponse> {
    let user_id = identity
        .and_then(|identity| identity.id().ok())
        .and_then(|id| Uuid::parse_str(&id).ok());
    if !validate_pending_discord_link(&request, user_id, &params.state)? {
        return Ok(HttpResponse::Forbidden()
            .body("Discord link could not be verified. Start again from notification settings."));
    }

    let client = Client::builder().build().map_err(ErrorInternalServerError)?;
    let mut response = if client
        .post("http://localhost:8080/oauth/callback")
        .query(&params.into_inner())
        .send()
        .await
        .and_then(Response::error_for_status)
        .is_err()
    {
        // Do not log reqwest errors here: their URLs contain the authorization code/state.
        log::warn!("Discord OAuth callback exchange failed");
        HttpResponse::BadGateway()
            .body("Discord linking failed. Start again from notification settings.")
    } else {
        HttpResponse::Found()
            .insert_header(("Location", "/notifications"))
            .finish()
    };
    clear_discord_link_cookie(&mut response)?;
    Ok(response)
}
}}

#[server(client = CsrfClient)]
pub async fn start_discord_link() -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::uuid;

    let user_id = uuid().await?;
    let request: HttpRequest = leptos_actix::extract().await?;
    let response = Client::new()
        .post(format!("http://localhost:8080/oauth/new/{user_id}"))
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|_| ServerFnError::new("Could not start Discord linking"))?;

    #[derive(Deserialize)]
    struct AuthorizationUrl {
        url: String,
    }

    let authorization: AuthorizationUrl = response
        .json()
        .await
        .map_err(|_| ServerFnError::new("Invalid Discord authorization response"))?;
    let url = Url::parse(&authorization.url)
        .map_err(|_| ServerFnError::new("Invalid Discord authorization URL"))?;
    let mut states = url.query_pairs().filter(|(key, _)| key == "state");
    let state = states.next().map(|(_, state)| state.into_owned());
    let state = state
        .filter(|state| !state.is_empty() && states.next().is_none())
        .ok_or_else(|| ServerFnError::new("Invalid Discord authorization state"))?;
    // OAuth responses must not rewrite the login snapshot carried by this request.
    let cookie =
        pending_discord_link_cookie(&request, user_id, state).map_err(ServerFnError::new)?;
    let response = expect_context::<ResponseOptions>();
    response.append_header(SET_COOKIE, HeaderValue::from_str(&cookie.to_string())?);
    response.insert_header(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    leptos_actix::redirect(&authorization.url);
    Ok(())
}

#[server(client = CsrfClient)]
pub async fn get_discord_handle() -> Result<DiscordHandleStatus, ServerFnError> {
    use crate::functions::auth::identity::uuid;

    use serde_json::Value;

    if let Ok(uuid) = uuid().await {
        let url = format!("http://localhost:8080/discord/{uuid}");
        let client = match reqwest::Client::builder().build() {
            Ok(client) => client,
            Err(e) => {
                println!("Error creating discord handle client: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(e) => {
                println!("Error loading discord handle: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => {
                println!("Error reading discord handle response: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        let data: Value = match serde_json::from_str(&body) {
            Ok(data) => data,
            Err(e) => {
                println!("Error parsing discord handle response: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        if let Some(username) = data.get("username").and_then(Value::as_str) {
            return Ok(DiscordHandleStatus::Linked(username.to_string()));
        }
        return Ok(DiscordHandleStatus::NotLinked);
    }
    Ok(DiscordHandleStatus::NotLoggedIn)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{
        clear_discord_link_cookie,
        pending_discord_link_cookie,
        validate_pending_discord_link,
        DiscordLinkKey,
        PendingDiscordLink,
        DISCORD_LINK_COOKIE,
    };
    use crate::security::csrf::{reset_session, token_cookie};
    use actix_identity::{Identity, IdentityMiddleware};
    use actix_session::{
        config::PersistentSession,
        storage::CookieSessionStore,
        SessionExt,
        SessionMiddleware,
    };
    use actix_web::{
        cookie::{time::Duration, Cookie, CookieJar, Key},
        middleware::from_fn,
        test::{call_service, init_service, TestRequest},
        web::{self, Data},
        App,
        HttpMessage,
        HttpRequest,
        HttpResponse,
    };
    use uuid::Uuid;

    fn request(key: &Data<DiscordLinkKey>, cookie: Option<Cookie<'static>>) -> HttpRequest {
        let mut request = TestRequest::default().app_data(key.clone());
        if let Some(cookie) = cookie {
            request = request.cookie(cookie);
        }
        let request = request.to_http_request();
        request
            .get_session()
            .insert("csrf_token", "initiating-login-token")
            .unwrap();
        request
    }

    #[test]
    fn linking_requires_the_initiating_session_and_current_identity() {
        let key = Data::new(DiscordLinkKey(Key::generate()));
        let user_id = Uuid::new_v4();
        let cookie =
            pending_discord_link_cookie(&request(&key, None), user_id, "provider-state".into())
                .unwrap();
        let initiator = request(&key, Some(cookie.clone()));
        assert!(
            validate_pending_discord_link(&initiator, Some(user_id), "provider-state").unwrap()
        );
        assert!(!validate_pending_discord_link(&initiator, None, "provider-state").unwrap());
        assert!(
            !validate_pending_discord_link(&initiator, Some(Uuid::new_v4()), "provider-state")
                .unwrap()
        );
        assert!(!validate_pending_discord_link(&initiator, Some(user_id), "other-state").unwrap());
        assert!(!validate_pending_discord_link(
            &request(&key, None),
            Some(user_id),
            "provider-state"
        )
        .unwrap());

        let other_login = request(&key, Some(cookie));
        reset_session(&other_login.get_session()).unwrap();
        assert!(
            !validate_pending_discord_link(&other_login, Some(user_id), "provider-state").unwrap()
        );
        other_login.get_session().clear();
        assert!(
            !validate_pending_discord_link(&other_login, Some(user_id), "provider-state").unwrap()
        );
        assert!(
            pending_discord_link_cookie(&other_login, user_id, "provider-state".into()).is_err()
        );
    }

    #[test]
    fn linking_rejects_tampered_malformed_and_expired_cookies() {
        let key = Data::new(DiscordLinkKey(Key::generate()));
        let user_id = Uuid::new_v4();
        let cookie =
            pending_discord_link_cookie(&request(&key, None), user_id, "provider-state".into())
                .unwrap();
        let mut tampered = cookie.clone();
        tampered.set_value("tampered");
        assert!(!validate_pending_discord_link(
            &request(&key, Some(tampered)),
            Some(user_id),
            "provider-state"
        )
        .unwrap());

        let mut jar = CookieJar::new();
        jar.add_original(cookie);
        let mut plaintext = jar
            .private(&key.get_ref().0)
            .get(DISCORD_LINK_COOKIE)
            .unwrap();
        let mut pending: PendingDiscordLink = serde_json::from_str(plaintext.value()).unwrap();
        pending.expires_at = 0;
        for value in ["{}".to_string(), serde_json::to_string(&pending).unwrap()] {
            plaintext.set_value(value);
            jar.private_mut(&key.get_ref().0).add(plaintext.clone());
            let invalid = request(&key, jar.get(DISCORD_LINK_COOKIE).cloned());
            assert!(
                !validate_pending_discord_link(&invalid, Some(user_id), "provider-state").unwrap()
            );
        }
    }

    #[actix_web::test]
    async fn discord_link_cookies_do_not_reissue_the_auth_session() {
        let key = Key::generate();
        let app = init_service(
            App::new()
                .app_data(Data::new(DiscordLinkKey(key.clone())))
                .route(
                    "/login",
                    web::get().to(|request: HttpRequest| async move {
                        reset_session(&request.get_session()).unwrap();
                        Identity::login(&request.extensions(), Uuid::nil().to_string()).unwrap();
                        HttpResponse::NoContent().finish()
                    }),
                )
                .route(
                    "/link",
                    web::get().to(|request: HttpRequest, identity: Identity| async move {
                        let user_id = Uuid::parse_str(&identity.id().unwrap()).unwrap();
                        let cookie =
                            pending_discord_link_cookie(&request, user_id, "state".into()).unwrap();
                        HttpResponse::NoContent().cookie(cookie).finish()
                    }),
                )
                .route(
                    "/verify",
                    web::get().to(|request: HttpRequest, identity: Identity| async move {
                        let user_id = Uuid::parse_str(&identity.id().unwrap()).unwrap();
                        assert!(
                            validate_pending_discord_link(&request, Some(user_id), "state")
                                .unwrap()
                        );
                        let mut response = HttpResponse::NoContent().finish();
                        clear_discord_link_cookie(&mut response).unwrap();
                        response
                    }),
                )
                .wrap(from_fn(token_cookie))
                .wrap(IdentityMiddleware::default())
                .wrap(
                    SessionMiddleware::builder(CookieSessionStore::default(), key)
                        .session_lifecycle(
                            PersistentSession::default().session_ttl(Duration::weeks(12)),
                        )
                        .build(),
                ),
        )
        .await;
        let login = call_service(&app, TestRequest::get().uri("/login").to_request()).await;
        assert!(login.status().is_success());
        let mut cookies: Vec<_> = login.response().cookies().map(Cookie::into_owned).collect();
        assert!(cookies.iter().any(|cookie| cookie.name() == "id"));

        // Without an auth Set-Cookie, even a delayed response cannot restore an old login.
        for path in ["/link", "/verify"] {
            let request = cookies
                .iter()
                .fold(TestRequest::get().uri(path), |request, cookie| {
                    request.cookie(cookie.clone())
                });
            let response = call_service(&app, request.to_request()).await;
            assert!(response.status().is_success());
            assert!(response
                .response()
                .cookies()
                .all(|cookie| cookie.name() != "id"));
            cookies.extend(response.response().cookies().map(Cookie::into_owned));
        }
    }
}
