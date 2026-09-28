use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    http::header::{HeaderMap, HeaderName, HOST, ORIGIN, REFERER},
    web::Data,
    Error,
    HttpRequest,
};
use url::{Host, Origin, Url};

#[derive(Clone, Debug)]
pub enum ApplicationOrigin {
    Fixed(Origin),
    Development { port: u16 },
}

impl ApplicationOrigin {
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        let value = value.strip_suffix('/').unwrap_or(value);
        parse_origin(value)
            .map(Self::Fixed)
            .ok_or("Application origin must be an HTTP(S) origin without a path")
    }

    fn for_request(&self, request: &HttpRequest) -> Result<Origin, Error> {
        match self {
            Self::Fixed(origin) => Ok(origin.clone()),
            Self::Development { port } => {
                let host = single_header(request.headers(), HOST)?.ok_or_else(invalid_origin)?;
                let origin = parse_origin(&format!("http://{host}")).ok_or_else(invalid_origin)?;
                let Origin::Tuple(_, host, request_port) = &origin else {
                    return Err(invalid_origin());
                };
                // Arbitrary DNS names could rebind to the development server.
                let local_host = matches!(host, Host::Domain(name) if name == "localhost")
                    || matches!(host, Host::Ipv4(_) | Host::Ipv6(_));
                if !local_host || request_port != port {
                    return Err(invalid_origin());
                }
                Ok(origin)
            }
        }
    }
}

/// HTTP mutations also require a session-bound CSRF token, so absent origin
/// headers are permitted there. Browser WebSockets must supply Origin.
pub fn validate_request_origin(req: &HttpRequest, require_origin: bool) -> Result<(), Error> {
    let application_origin = req
        .app_data::<Data<ApplicationOrigin>>()
        .ok_or_else(|| ErrorInternalServerError("Application origin is not configured"))?;
    let expected_origin = application_origin.for_request(req)?;
    let request_origin = match single_header(req.headers(), ORIGIN)? {
        Some(value) => parse_origin(value),
        None if require_origin => return Err(invalid_origin()),
        None => match single_header(req.headers(), REFERER)? {
            Some(value) => parse_referer(value),
            None => return Ok(()),
        },
    };

    if request_origin != Some(expected_origin) {
        return Err(invalid_origin());
    }
    Ok(())
}

fn single_header(headers: &HeaderMap, name: HeaderName) -> Result<Option<&str>, Error> {
    let mut values = headers.get_all(name);
    let value = values.next();
    if values.next().is_some() {
        return Err(invalid_origin());
    }
    value
        .map(|value| value.to_str().map_err(|_| invalid_origin()))
        .transpose()
}

fn parse_origin(value: &str) -> Option<Origin> {
    let (_, authority) = value.split_once("://")?;
    if authority.contains(['/', '?', '#']) {
        return None;
    }
    parse_http_url(value).map(|url| url.origin())
}

fn parse_referer(value: &str) -> Option<Origin> {
    let url = parse_http_url(value)?;
    if url.fragment().is_some() {
        return None;
    }
    Some(url.origin())
}

fn parse_http_url(value: &str) -> Option<Url> {
    // URL parsers repair inputs such as backslashes, whitespace and escaped
    // hosts. Security headers must already contain a well-formed browser URL.
    if !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        || value.contains('\\')
    {
        return None;
    }
    let (_, rest) = value.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.ends_with(':') || authority.contains(['@', '%']) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return None;
    }
    Some(url)
}

fn invalid_origin() -> Error {
    ErrorForbidden("Request origin is not allowed")
}

#[cfg(test)]
mod tests {
    use super::{validate_request_origin, ApplicationOrigin};
    use actix_web::{http::StatusCode, test::TestRequest, web::Data};

    fn request() -> TestRequest {
        TestRequest::default().app_data(Data::new(
            ApplicationOrigin::parse("https://hivegame.com").unwrap(),
        ))
    }

    #[test]
    fn origin_matches_scheme_host_and_effective_port() {
        for origin in [
            "https://hivegame.com",
            "https://hivegame.com:443",
            "HTTPS://HIVEGAME.COM",
        ] {
            let req = request()
                .insert_header(("Origin", origin))
                .to_http_request();
            assert!(validate_request_origin(&req, false).is_ok(), "{origin}");
        }
        for origin in [
            "http://hivegame.com",
            "https://hivegame.com:444",
            "https://chat.hivegame.com",
            "https://hivegame.com.attacker.example",
            "https://attacker.example",
        ] {
            let req = request()
                .insert_header(("Origin", origin))
                .to_http_request();
            let error = validate_request_origin(&req, false).unwrap_err();
            assert_eq!(
                error.as_response_error().status_code(),
                StatusCode::FORBIDDEN,
                "{origin}"
            );
        }
    }

    #[test]
    fn malformed_or_foreign_origin_cannot_fall_back_to_a_matching_referer() {
        for origin in [
            "null",
            "",
            "https://attacker.example",
            "https://hivegame.com https://attacker.example",
            "https://hivegame.com,https://attacker.example",
            "https://hivegame.com/",
            "https://hivegame.com/path",
            "https://hivegame.com?query",
            "https://hivegame.com#fragment",
            "https://hivegame.com:",
            "https://user@hivegame.com",
            "https://%68ivegame.com",
            "https://hivegame.com\\",
            " https://hivegame.com",
            "https:/hivegame.com",
            "file://hivegame.com",
        ] {
            let req = request()
                .insert_header(("Origin", origin))
                .insert_header(("Referer", "https://hivegame.com/account"))
                .to_http_request();
            assert!(validate_request_origin(&req, false).is_err(), "{origin}");
        }

        let req = request()
            .append_header(("Origin", "https://hivegame.com"))
            .append_header(("Origin", "https://attacker.example"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_err());
    }

    #[test]
    fn http_checks_referer_only_when_origin_is_absent() {
        let req = request().to_http_request();
        assert!(validate_request_origin(&req, false).is_ok());

        let req = request()
            .insert_header(("Referer", "https://hivegame.com:443/account?tab=settings"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_ok());

        for referer in ["https://attacker.example/account", "null", "/account"] {
            let req = request()
                .insert_header(("Referer", referer))
                .to_http_request();
            assert!(validate_request_origin(&req, false).is_err(), "{referer}");
        }

        let req = request()
            .append_header(("Referer", "https://hivegame.com/account"))
            .append_header(("Referer", "https://hivegame.com/login"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_err());

        let req = request()
            .insert_header(("Origin", "https://hivegame.com"))
            .insert_header(("Referer", "https://attacker.example"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_ok());
    }

    #[test]
    fn websocket_requires_origin_even_with_a_matching_referer() {
        let req = request().to_http_request();
        assert!(validate_request_origin(&req, true).is_err());
        let req = request()
            .insert_header(("Referer", "https://hivegame.com/account"))
            .to_http_request();
        assert!(validate_request_origin(&req, true).is_err());
        let req = request()
            .insert_header(("Origin", "https://hivegame.com"))
            .to_http_request();
        assert!(validate_request_origin(&req, true).is_ok());
    }

    #[test]
    fn request_host_and_forwarding_headers_do_not_define_trusted_origin() {
        let req = request()
            .insert_header(("Host", "attacker.example"))
            .insert_header(("Forwarded", "host=attacker.example;proto=https"))
            .insert_header(("X-Forwarded-Host", "attacker.example"))
            .insert_header(("X-Forwarded-Proto", "https"))
            .insert_header(("Origin", "https://attacker.example"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_err());
    }

    #[test]
    fn development_accepts_only_the_origin_of_the_requested_local_address() {
        for host in [
            "localhost:3100",
            "127.0.0.1:3100",
            "192.168.1.20:3100",
            "[::1]:3100",
        ] {
            let req = TestRequest::default()
                .app_data(Data::new(ApplicationOrigin::Development { port: 3100 }))
                .insert_header(("Host", host))
                .insert_header(("Origin", format!("http://{host}")))
                .to_http_request();
            assert!(validate_request_origin(&req, true).is_ok(), "{host}");
        }

        let req = TestRequest::default()
            .app_data(Data::new(ApplicationOrigin::Development { port: 80 }))
            .insert_header(("Host", "LOCALHOST"))
            .insert_header(("Origin", "http://localhost:80"))
            .to_http_request();
        assert!(validate_request_origin(&req, true).is_ok());
    }

    #[test]
    fn development_rejects_foreign_origins_and_untrusted_target_headers() {
        for (host, origin) in [
            ("localhost:3100", "http://127.0.0.1:3100"),
            ("attacker.example:3100", "http://attacker.example:3100"),
            (
                "localhost.attacker.example:3100",
                "http://localhost.attacker.example:3100",
            ),
            ("localhost:3199", "http://localhost:3199"),
            ("localhost", "http://localhost"),
            ("localhost:3100/path", "http://localhost:3100"),
        ] {
            let req = TestRequest::default()
                .app_data(Data::new(ApplicationOrigin::Development { port: 3100 }))
                .insert_header(("Host", host))
                .insert_header(("Origin", origin))
                .to_http_request();
            assert!(
                validate_request_origin(&req, true).is_err(),
                "{host} / {origin}"
            );
        }

        let req = TestRequest::default()
            .app_data(Data::new(ApplicationOrigin::Development { port: 3100 }))
            .insert_header(("Forwarded", "host=localhost:3100;proto=http"))
            .insert_header(("X-Forwarded-Host", "localhost:3100"))
            .insert_header(("Origin", "http://localhost:3100"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_err());

        let req = TestRequest::default()
            .app_data(Data::new(ApplicationOrigin::Development { port: 3100 }))
            .append_header(("Host", "localhost:3100"))
            .append_header(("Host", "attacker.example:3100"))
            .insert_header(("Origin", "http://localhost:3100"))
            .to_http_request();
        assert!(validate_request_origin(&req, false).is_err());
    }
}
