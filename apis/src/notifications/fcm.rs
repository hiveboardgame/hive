use std::sync::Arc;

use gcp_auth::{CustomServiceAccount, TokenProvider};
use reqwest::StatusCode;
use serde_json::json;

use super::{NotifyOutcome, Push};

const FCM_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

pub struct FcmNotifier {
    http: reqwest::Client,
    provider: Arc<CustomServiceAccount>,
    project_id: String,
}

impl FcmNotifier {
    /// Build a notifier from the path to a Firebase Admin SDK service account
    /// JSON (the one downloaded from Firebase Console → Project settings →
    /// Service accounts → "Generate new private key").
    ///
    /// `gcp_auth` caches the OAuth2 access token internally and refreshes it
    /// transparently before expiry, so callers can `send()` freely without
    /// thinking about token lifecycle.
    pub fn from_credentials_path<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let sa = CustomServiceAccount::from_file(path)?;
        let project_id = sa
            .project_id()
            .ok_or_else(|| anyhow::anyhow!("service account JSON missing project_id"))?
            .to_string();
        Ok(Self {
            http: reqwest::Client::new(),
            provider: Arc::new(sa),
            project_id,
        })
    }

    pub async fn send(&self, token: &str, push: &Push) -> NotifyOutcome {
        let access = match self.provider.token(&[FCM_SCOPE]).await {
            Ok(t) => t,
            Err(err) => return NotifyOutcome::Failed(format!("gcp_auth: {err}")),
        };

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.project_id
        );

        // FCM v1 message shape. We send a `notification` block so the OS
        // renders the title/body when the app is backgrounded, plus a `data`
        // block carrying the deep link + event type so the foreground tap
        // handler can route without re-fetching anything.
        let mut data = serde_json::Map::new();
        data.insert("event_type".into(), json!(push.event_type));
        if let Some(link) = &push.link {
            data.insert("link".into(), json!(link));
        }

        let body = json!({
            "message": {
                "token": token,
                "notification": {
                    "title": push.title,
                    "body": push.body,
                },
                "data": data,
                "android": { "priority": "high" },
            }
        });

        let resp = match self
            .http
            .post(&url)
            .bearer_auth(access.as_str())
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(err) => return NotifyOutcome::Failed(format!("send error: {err}")),
        };

        let status = resp.status();
        if status.is_success() {
            return NotifyOutcome::Delivered;
        }

        // FCM v1 error envelope (https://firebase.google.com/docs/cloud-messaging/send-message#rest)
        // The shape that matters for token lifecycle:
        //
        //   { "error": { "status": "NOT_FOUND",
        //                "details": [{ "@type": ".../FcmError",
        //                              "errorCode": "UNREGISTERED" }] } }
        //
        // We treat both UNREGISTERED (token revoked / app uninstalled) and
        // INVALID_ARGUMENT *on the token field* as dead-token signals — both
        // cases mean re-sending to this token will never succeed and the row
        // should be cleaned up. Other 4xx are treated as Failed so they
        // surface in logs without nuking the row.
        let text = resp.text().await.unwrap_or_default();
        let looks_dead = (status == StatusCode::NOT_FOUND && text.contains("UNREGISTERED"))
            || (status == StatusCode::BAD_REQUEST
                && text.contains("INVALID_ARGUMENT")
                && text.contains("registration token"));
        if looks_dead {
            return NotifyOutcome::TokenDead;
        }
        if status.is_server_error() {
            return NotifyOutcome::Retryable;
        }
        NotifyOutcome::Failed(format!("FCM {status}: {text}"))
    }
}
