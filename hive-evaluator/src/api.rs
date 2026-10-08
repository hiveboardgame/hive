use anyhow::{bail, Result};
use reqwest::{Client, StatusCode};
use shared_types::{EvalClaim, EvalFailure, EvalJob, EvalProgress, EvalResult, EvalSubmission};
use uuid::Uuid;

#[derive(Clone)]
pub struct Api {
    client: Client,
    base_url: String,
    token: String,
    pub worker: String,
    /// Whether to accept games the site picks itself when no user is waiting.
    pub auto: bool,
}

/// Whether the server still considers an eval ours.
#[derive(Debug, PartialEq)]
pub enum Ownership {
    Ours,
    Lost,
}

impl Api {
    pub fn new(base_url: &str, token: &str, worker: &str) -> Self {
        Api {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            worker: worker.to_string(),
            auto: true,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1/evals/{path}", self.base_url)
    }

    pub async fn claim(&self) -> Result<Option<EvalJob>> {
        let response = self
            .client
            .post(self.url("claim"))
            .bearer_auth(&self.token)
            .json(&EvalClaim {
                worker: self.worker.clone(),
                auto: self.auto,
            })
            .send()
            .await?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(None),
            StatusCode::OK => Ok(Some(response.json().await?)),
            status => bail!(
                "claim: {status}: {}",
                response.text().await.unwrap_or_default()
            ),
        }
    }

    async fn post<T: serde::Serialize>(
        &self,
        eval: Uuid,
        action: &str,
        body: &T,
    ) -> Result<Ownership> {
        let response = self
            .client
            .post(self.url(&format!("{eval}/{action}")))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await?;
        match response.status() {
            StatusCode::OK => Ok(Ownership::Ours),
            StatusCode::CONFLICT => Ok(Ownership::Lost),
            status => bail!(
                "{action}: {status}: {}",
                response.text().await.unwrap_or_default()
            ),
        }
    }

    pub async fn progress(&self, eval: Uuid, pct: u8) -> Result<Ownership> {
        let body = EvalProgress {
            worker: self.worker.clone(),
            progress_pct: pct,
        };
        self.post(eval, "progress", &body).await
    }

    pub async fn submit(&self, eval: Uuid, engine: &str, result: EvalResult) -> Result<Ownership> {
        let body = EvalSubmission {
            worker: self.worker.clone(),
            engine: engine.to_string(),
            result,
        };
        self.post(eval, "result", &body).await
    }

    pub async fn fail(&self, eval: Uuid, error: &str) -> Result<Ownership> {
        let body = EvalFailure {
            worker: self.worker.clone(),
            error: error.to_string(),
        };
        self.post(eval, "fail", &body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{header, method, path},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    #[tokio::test]
    async fn an_empty_queue_is_no_job() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/evals/claim"))
            .and(header("authorization", "Bearer tok"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let api = Api::new(&server.uri(), "tok", "w1");
        assert_eq!(api.claim().await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_conflict_means_the_eval_was_taken_away() {
        let server = MockServer::start().await;
        let eval = Uuid::new_v4();
        Mock::given(method("POST"))
            .and(path(format!("/api/v1/evals/{eval}/progress")))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;
        let api = Api::new(&server.uri(), "tok", "w1");
        assert_eq!(api.progress(eval, 10).await.unwrap(), Ownership::Lost);
    }

    #[tokio::test]
    async fn a_claimed_job_is_decoded() {
        let server = MockServer::start().await;
        let job = EvalJob {
            eval_id: Uuid::new_v4(),
            game_type: "Base+MLP".to_string(),
            moves: vec!["wL".to_string()],
        };
        Mock::given(method("POST"))
            .and(path("/api/v1/evals/claim"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&job))
            .mount(&server)
            .await;
        let api = Api::new(&server.uri(), "tok", "w1");
        assert_eq!(api.claim().await.unwrap(), Some(job));
    }
}
