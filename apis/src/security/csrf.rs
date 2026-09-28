use crate::providers::AuthContext;
use leptos::prelude::*;

mod client;
mod form;
pub use client::CsrfClient;
pub use form::ActionForm;

// Validate against the protected session, never against this readable cookie.
const TOKEN_COOKIE: &str = if cfg!(debug_assertions) {
    "hive_csrf"
} else {
    "__Host-hive_csrf"
};
#[cfg(feature = "ssr")]
const TOKEN_KEY: &str = "csrf_token";

fn csrf_token() -> String {
    #[cfg(feature = "ssr")]
    {
        use actix_session::SessionExt;
        use leptos_actix::Request;

        use_context::<Request>()
            .and_then(|request| {
                request
                    .get_session()
                    .get::<String>(TOKEN_KEY)
                    .ok()
                    .flatten()
            })
            .unwrap_or_default()
    }
    #[cfg(not(feature = "ssr"))]
    {
        use wasm_bindgen::JsCast;
        use web_sys::HtmlDocument;

        document()
            .unchecked_into::<HtmlDocument>()
            .cookie()
            .ok()
            .and_then(|cookies| {
                cookies.split(';').find_map(|cookie| {
                    let (name, value) = cookie.trim().split_once('=')?;
                    (name == TOKEN_COOKIE).then(|| value.to_string())
                })
            })
            .unwrap_or_default()
    }
}

#[component]
fn CsrfInput() -> impl IntoView {
    let auth = use_context::<AuthContext>();
    view! {
        <input
            type="hidden"
            name="csrf_token"
            value=move || {
                if let Some(auth) = &auth {
                    let _ = auth.identity.get();
                }
                csrf_token()
            }
        />
    }
}

#[cfg(feature = "ssr")]
pub use server::{
    ensure_document_token,
    protection,
    reset_session,
    session_fingerprint,
    token_cookie,
    token_endpoint,
};

#[cfg(feature = "ssr")]
mod server {
    use super::{TOKEN_COOKIE, TOKEN_KEY};
    use crate::security::origin::validate_request_origin;
    use actix_session::{Session, SessionExt};
    use actix_web::{
        body::{EitherBody, MessageBody},
        cookie::{Cookie, SameSite},
        dev::{ServiceRequest, ServiceResponse},
        error::{ErrorForbidden, ErrorInternalServerError},
        get,
        http::{header, Method},
        middleware::Next,
        web,
        Error,
        HttpMessage,
        HttpRequest,
        HttpResponse,
    };
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use leptos::prelude::*;
    use leptos_actix::Request;
    use server_fn::{actix::get_server_fn_service, error::ServerFnErrorErr};
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;
    use url::form_urlencoded;

    fn ensure_token(session: &Session) -> Result<String, Error> {
        if let Some(token) = session.get::<String>(TOKEN_KEY)? {
            return Ok(token);
        }
        let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
        session.insert(TOKEN_KEY, &token)?;
        Ok(token)
    }

    pub fn ensure_document_token() {
        // Route discovery also renders the shell, without an HTTP request.
        if let Some(request) = use_context::<Request>() {
            ensure_token(&request.get_session()).expect("store CSRF token in session");
        }
    }

    pub fn reset_session(session: &Session) -> Result<(), ServerFnError> {
        // Rotating the token also invalidates OAuth cookies bound to this login.
        session.clear();
        session.renew();
        ensure_token(session).map_err(ServerFnError::new)?;
        Ok(())
    }

    pub fn session_fingerprint(session: &Session) -> Result<Option<[u8; 32]>, Error> {
        Ok(session
            .get::<String>(TOKEN_KEY)?
            .filter(|token| !token.is_empty())
            .map(|token| Sha256::digest(token.as_bytes()).into()))
    }

    fn validate_csrf_request(request: &HttpRequest, submitted: &str) -> Result<(), Error> {
        validate_request_origin(request, false)?;
        let expected = request.get_session().get::<String>(TOKEN_KEY)?;
        let valid = expected.as_deref().is_some_and(|expected| {
            !expected.is_empty() && bool::from(expected.as_bytes().ct_eq(submitted.as_bytes()))
        });
        if !valid {
            return Err(ErrorForbidden("CSRF verification failed"));
        }
        Ok(())
    }

    async fn submitted_token(request: &mut ServiceRequest) -> Result<String, Error> {
        let mut headers = request.headers().get_all("x-csrf-token");
        if let Some(value) = headers.next() {
            if headers.next().is_some() {
                return Err(ErrorForbidden("Multiple CSRF tokens"));
            }
            return value.to_str().map(str::to_owned).map_err(ErrorForbidden);
        }
        if !request
            .mime_type()?
            .is_some_and(|mime| mime.essence_str() == "application/x-www-form-urlencoded")
        {
            return Err(ErrorForbidden("Missing CSRF token"));
        }

        // Bytes uses Actix's payload limit; restore it for the actual form handler.
        let body = request.extract::<web::Bytes>().await?;
        let mut fields = form_urlencoded::parse(&body).filter(|(name, _)| name == TOKEN_KEY);
        let token = fields.next().map(|(_, value)| value.into_owned());
        let duplicated = fields.next().is_some();
        request.set_payload(body.into());
        if duplicated {
            return Err(ErrorForbidden("Multiple CSRF tokens"));
        }
        token.ok_or_else(|| ErrorForbidden("Missing CSRF token"))
    }

    pub async fn protection<B: MessageBody>(
        mut request: ServiceRequest,
        next: Next<B>,
    ) -> Result<ServiceResponse<EitherBody<B>>, Error> {
        // /api/v1 is the credential-only bot API: it never authenticates with cookies.
        if matches!(
            *request.method(),
            Method::GET | Method::HEAD | Method::OPTIONS
        ) || request.path().starts_with("/api/v1/")
        {
            return next
                .call(request)
                .await
                .map(ServiceResponse::map_into_left_body);
        }

        let validation = match submitted_token(&mut request).await {
            Ok(token) => validate_csrf_request(request.request(), &token),
            Err(error) => Err(error),
        };
        if let Err(error) = validation {
            let status = error.as_response_error().status_code();
            let message = "This request could not be verified. Refresh the page and try again.";
            let mut response = HttpResponse::build(status);
            response.insert_header((header::CACHE_CONTROL, "private, no-store"));
            let response =
                if let Some(service) = get_server_fn_service(request.path(), request.method()) {
                    response.body((service.ser)(ServerFnErrorErr::ServerError(message.into())))
                } else {
                    response.body(message)
                };
            return Ok(request.into_response(response).map_into_right_body());
        }
        next.call(request)
            .await
            .map(ServiceResponse::map_into_left_body)
    }

    #[get("/api/csrf")]
    pub async fn token_endpoint(request: HttpRequest) -> Result<HttpResponse, Error> {
        validate_request_origin(&request, false)?;
        let token = ensure_token(&request.get_session())?;
        Ok(HttpResponse::Ok()
            .insert_header((header::CACHE_CONTROL, "private, no-store"))
            .content_type("text/plain; charset=utf-8")
            .body(token))
    }

    pub async fn token_cookie(
        request: ServiceRequest,
        next: Next<impl MessageBody>,
    ) -> Result<ServiceResponse<impl MessageBody>, Error> {
        let session = request.get_session();
        let incoming = request
            .cookie(TOKEN_COOKIE)
            .map(|cookie| cookie.value().to_string());
        let mut response = next.call(request).await?;
        let changed_token = session
            .get::<String>(TOKEN_KEY)?
            .filter(|token| incoming.as_deref() != Some(token.as_str()));
        let cookie_changed = changed_token.is_some();
        if let Some(token) = changed_token {
            response
                .response_mut()
                .add_cookie(
                    &Cookie::build(TOKEN_COOKIE, token)
                        .path("/")
                        .secure(!cfg!(debug_assertions))
                        .http_only(false)
                        .same_site(SameSite::Lax)
                        .finish(),
                )
                .map_err(ErrorInternalServerError)?;
        }
        // HTML forms contain session-specific tokens, including anonymous forms.
        let is_html = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"));
        if cookie_changed || is_html {
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("private, no-store"),
            );
        }
        Ok(response)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::security::origin::ApplicationOrigin;
        use actix_session::{storage::CookieSessionStore, SessionMiddleware};
        use actix_web::{cookie::Key, http::StatusCode, middleware::from_fn, test, web, App};
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Submission {
            value: String,
        }

        async fn mutation(form: web::Form<Submission>) -> Result<HttpResponse, Error> {
            assert_eq!(form.value, "preserved");
            Ok(HttpResponse::NoContent().finish())
        }

        async fn transition(request: HttpRequest) -> Result<HttpResponse, Error> {
            reset_session(&request.get_session()).map_err(ErrorInternalServerError)?;
            Ok(HttpResponse::NoContent().finish())
        }

        fn submission(
            path: &str,
            cookie: Option<&Cookie<'static>>,
            token: &str,
        ) -> test::TestRequest {
            let request = test::TestRequest::post()
                .uri(path)
                .set_form([("csrf_token", token), ("value", "preserved")]);
            match cookie {
                Some(cookie) => request.cookie(cookie.clone()),
                None => request,
            }
        }

        fn response_cookie<B>(response: &ServiceResponse<B>, name: &str) -> Cookie<'static> {
            response
                .response()
                .cookies()
                .find(|cookie| cookie.name() == name)
                .unwrap()
                .into_owned()
        }

        #[actix_web::test]
        async fn csrf_tokens_are_session_bound_and_survive_repeated_loads() {
            let app = test::init_service(
                App::new()
                    .app_data(web::Data::new(
                        ApplicationOrigin::parse("https://hive.test").unwrap(),
                    ))
                    .service(token_endpoint)
                    .route("/mutation", web::post().to(mutation))
                    .route("/transition", web::post().to(transition))
                    .wrap(from_fn(protection))
                    .wrap(from_fn(token_cookie))
                    .wrap(
                        SessionMiddleware::builder(CookieSessionStore::default(), Key::generate())
                            .build(),
                    ),
            )
            .await;

            let first =
                test::call_service(&app, test::TestRequest::get().uri("/api/csrf").to_request())
                    .await;
            assert_eq!(first.status(), StatusCode::OK);
            let first_cookie = response_cookie(&first, "id");
            let first_token = String::from_utf8(test::read_body(first).await.to_vec()).unwrap();
            let repeated = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/api/csrf")
                    .cookie(first_cookie.clone())
                    .to_request(),
            )
            .await;
            assert_eq!(
                test::read_body(repeated).await.as_ref(),
                first_token.as_bytes()
            );

            let other =
                test::call_service(&app, test::TestRequest::get().uri("/api/csrf").to_request())
                    .await;
            let other_token = String::from_utf8(test::read_body(other).await.to_vec()).unwrap();
            for submitted in ["", "incorrect", other_token.as_str()] {
                let request = submission("/mutation", Some(&first_cookie), submitted)
                    .insert_header((header::ORIGIN, "https://hive.test"))
                    .to_request();
                assert_eq!(
                    test::call_service(&app, request).await.status(),
                    StatusCode::FORBIDDEN
                );
            }
            // Token validation must not create a missing session from the submitted value.
            let request = submission("/mutation", None, &first_token).to_request();
            assert_eq!(
                test::call_service(&app, request).await.status(),
                StatusCode::FORBIDDEN
            );

            let request = submission("/mutation", Some(&first_cookie), &first_token).to_request();
            assert_eq!(
                test::call_service(&app, request).await.status(),
                StatusCode::NO_CONTENT
            );

            let request = submission("/mutation", Some(&first_cookie), &first_token)
                .insert_header((header::ORIGIN, "https://attacker.test"))
                .to_request();
            assert_eq!(
                test::call_service(&app, request).await.status(),
                StatusCode::FORBIDDEN
            );

            let request = submission("/transition", Some(&first_cookie), &first_token).to_request();
            let changed = test::call_service(&app, request).await;
            assert_eq!(changed.status(), StatusCode::NO_CONTENT);
            let changed_cookie = response_cookie(&changed, "id");
            let changed_token = response_cookie(&changed, TOKEN_COOKIE).value().to_string();
            let request = submission("/mutation", Some(&changed_cookie), &first_token).to_request();
            assert_eq!(
                test::call_service(&app, request).await.status(),
                StatusCode::FORBIDDEN
            );
            let request =
                submission("/mutation", Some(&changed_cookie), &changed_token).to_request();
            assert_eq!(
                test::call_service(&app, request).await.status(),
                StatusCode::NO_CONTENT
            );
        }

        #[actix_web::test]
        async fn middleware_rejects_unprotected_handlers_and_preserves_header_request_bodies() {
            use std::sync::{
                atomic::{AtomicUsize, Ordering},
                Arc,
            };
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let app = test::init_service(
                App::new()
                    .app_data(web::Data::new(
                        ApplicationOrigin::parse("https://hive.test").unwrap(),
                    ))
                    .service(token_endpoint)
                    .default_service(web::to(move |body: web::Bytes| {
                        count.fetch_add(1, Ordering::SeqCst);
                        async move { HttpResponse::Ok().body(body) }
                    }))
                    .wrap(from_fn(protection))
                    .wrap(from_fn(token_cookie))
                    .wrap(
                        SessionMiddleware::builder(CookieSessionStore::default(), Key::generate())
                            .build(),
                    ),
            )
            .await;
            let first =
                test::call_service(&app, test::TestRequest::get().uri("/api/csrf").to_request())
                    .await;
            let cookie = response_cookie(&first, "id");
            let token = String::from_utf8(test::read_body(first).await.to_vec()).unwrap();

            for method in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
                let denied = test::TestRequest::default()
                    .method(method.clone())
                    .uri("/new-handler")
                    .cookie(cookie.clone())
                    .insert_header((header::AUTHORIZATION, "Bearer ignored"))
                    .set_json(serde_json::json!({"value": "unchanged"}))
                    .to_request();
                assert_eq!(
                    test::call_service(&app, denied).await.status(),
                    StatusCode::FORBIDDEN
                );
                assert_eq!(calls.load(Ordering::SeqCst), 0);

                let valid = test::TestRequest::default()
                    .method(method)
                    .uri("/new-handler")
                    .cookie(cookie.clone())
                    .insert_header(("X-CSRF-Token", token.as_str()))
                    .insert_header((header::CONTENT_TYPE, "application/cbor"))
                    .set_payload(vec![0, 255, 1, 2])
                    .to_request();
                let response = test::call_service(&app, valid).await;
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(&test::read_body(response).await[..], &[0, 255, 1, 2]);
                assert_eq!(calls.swap(0, Ordering::SeqCst), 1);
            }

            let duplicated = test::TestRequest::post()
                .uri("/new-handler")
                .cookie(cookie.clone())
                .set_form([
                    ("csrf_token", token.as_str()),
                    ("csrf_token", token.as_str()),
                ])
                .to_request();
            assert_eq!(
                test::call_service(&app, duplicated).await.status(),
                StatusCode::FORBIDDEN
            );
            let oversized = test::TestRequest::post()
                .uri("/new-handler")
                .cookie(cookie.clone())
                .set_form([
                    ("csrf_token", token.as_str()),
                    ("value", &"x".repeat(300_000)),
                ])
                .to_request();
            assert_eq!(
                test::call_service(&app, oversized).await.status(),
                StatusCode::PAYLOAD_TOO_LARGE
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }

        #[actix_web::test]
        async fn middleware_errors_use_the_server_function_error_encoding() {
            use crate::functions::accounts::edit::EditLang;
            use server_fn::{error::FromServerFnError, ServerFn};
            let app = test::init_service(App::new().wrap(from_fn(protection))).await;
            let request = test::TestRequest::post().uri(EditLang::PATH).to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            let error: ServerFnError = FromServerFnError::de(test::read_body(response).await);
            assert!(matches!(error, ServerFnError::ServerError(_)));
        }

        #[actix_web::test]
        async fn issuing_csrf_preserves_an_existing_login_and_reset_discards_old_session_state() {
            let request = test::TestRequest::default().to_http_request();
            let session = request.get_session();
            session.insert("identity", "existing-user").unwrap();
            let old_token = ensure_token(&session).unwrap();
            assert_eq!(
                session.get::<String>("identity").unwrap().as_deref(),
                Some("existing-user")
            );
            reset_session(&session).unwrap();
            assert_eq!(session.get::<String>("identity").unwrap(), None);
            assert_ne!(ensure_token(&session).unwrap(), old_token);
        }
    }
}
