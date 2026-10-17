use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("token request refused: {status} - {body}")]
    Refused { status: StatusCode, body: String },
}

#[derive(Serialize)]
struct Credentials<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct TokenResponse {
    data: TokenData,
}

#[derive(Deserialize)]
struct TokenData {
    token: String,
}

pub async fn fetch(
    client: &Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<String, TokenError> {
    let response = client
        .post(format!("{base_url}/api/v1/auth/token"))
        .json(&Credentials { email, password })
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(TokenError::Refused {
            status,
            body: response.text().await.unwrap_or_default(),
        });
    }
    Ok(response.json::<TokenResponse>().await?.data.token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::{
        matchers::{body_json, method, path},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    #[tokio::test]
    async fn fetches_a_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/token"))
            .and(body_json(
                json!({"email": "bot@example.com", "password": "hivegame"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {"token": "test_token_123"}
            })))
            .mount(&server)
            .await;

        let token = fetch(&Client::new(), &server.uri(), "bot@example.com", "hivegame")
            .await
            .unwrap();

        assert_eq!(token, "test_token_123");
    }

    #[tokio::test]
    async fn reports_a_refused_login() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "success": false,
                "data": {"message": "Not a bot"}
            })))
            .mount(&server)
            .await;

        let result = fetch(&Client::new(), &server.uri(), "bot@example.com", "wrong").await;

        assert!(matches!(
            result,
            Err(TokenError::Refused { status, body })
                if status == StatusCode::BAD_REQUEST && body.contains("Not a bot")
        ));
    }
}
