use actix_web::{
    dev::Payload,
    error::InternalError,
    http::header,
    web::Data,
    FromRequest,
    HttpRequest,
    HttpResponse,
};
use std::future::{ready, Ready};

/// The shared secret eval workers present. Workers may run on rented machines, so this token
/// grants nothing beyond the `/api/v1/evals/` endpoints.
pub struct EvalWorkerToken(Option<String>);

impl EvalWorkerToken {
    pub fn from_env() -> Self {
        Self(
            std::env::var("EVAL_WORKER_TOKEN")
                .ok()
                .filter(|token| !token.is_empty()),
        )
    }
}

/// Proof that the request came from an eval worker.
pub struct EvalWorker;

impl FromRequest for EvalWorker {
    type Error = InternalError<String>;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let reject = |response: HttpResponse, message: &str| {
            InternalError::from_response(message.to_string(), response)
        };
        let Some(Some(expected)) = req
            .app_data::<Data<EvalWorkerToken>>()
            .map(|token| token.0.clone())
        else {
            return ready(Err(reject(
                HttpResponse::ServiceUnavailable().finish(),
                "eval workers are not configured",
            )));
        };
        let presented = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        match presented {
            Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
                ready(Ok(EvalWorker))
            }
            _ => ready(Err(reject(
                HttpResponse::Unauthorized().finish(),
                "bad worker token",
            ))),
        }
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    fn request(token: Option<&str>, header_value: Option<&str>) -> HttpRequest {
        let mut req =
            TestRequest::default().app_data(Data::new(EvalWorkerToken(token.map(String::from))));
        if let Some(value) = header_value {
            req = req.insert_header((header::AUTHORIZATION, value));
        }
        req.to_http_request()
    }

    async fn status(req: HttpRequest) -> Option<u16> {
        EvalWorker::extract(&req).await.err().map(|e| {
            actix_web::ResponseError::error_response(&e)
                .status()
                .as_u16()
        })
    }

    #[actix_web::test]
    async fn the_configured_token_is_accepted() {
        assert_eq!(
            status(request(Some("s3cret"), Some("Bearer s3cret"))).await,
            None
        );
    }

    #[actix_web::test]
    async fn a_wrong_or_missing_token_is_unauthorized() {
        assert_eq!(
            status(request(Some("s3cret"), Some("Bearer nope"))).await,
            Some(401)
        );
        assert_eq!(status(request(Some("s3cret"), None)).await, Some(401));
    }

    #[actix_web::test]
    async fn without_a_configured_token_every_worker_is_refused() {
        assert_eq!(status(request(None, Some("Bearer "))).await, Some(503));
    }
}
