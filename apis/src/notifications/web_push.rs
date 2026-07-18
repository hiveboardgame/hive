use ::web_push::{
    ContentEncoding,
    HyperWebPushClient,
    PartialVapidSignatureBuilder,
    SubscriptionInfo,
    Urgency,
    VapidSignatureBuilder,
    WebPushClient,
    WebPushError,
    WebPushMessageBuilder,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use serde_json::json;
use url::{Host, Url};

fn urgency_for(event_type: &str) -> Urgency {
    match event_type {
        "your_turn" | "challenge" | "game_started" | "schedule_propose" | "schedule_accept"
        | "dm" | "test" => Urgency::High,
        _ => Urgency::Normal,
    }
}

use super::{NotifyOutcome, Push};

const ALLOWED_ENDPOINT_HOST_SUFFIXES: &[&str] = &[
    "fcm.googleapis.com",
    "android.googleapis.com",
    "jmt17.google.com",
    "push.services.mozilla.com",
    "web.push.apple.com",
    "notify.windows.com",
];

fn host_matches_allowed_suffix(host: &str, suffix: &str) -> bool {
    host == suffix
        || host
            .strip_suffix(suffix)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

pub fn validate_push_endpoint(endpoint: &str) -> Result<(), String> {
    let url = Url::parse(endpoint).map_err(|e| format!("invalid push endpoint: {e}"))?;
    if url.scheme() != "https" {
        return Err("push endpoint must be https".into());
    }
    match url.host() {
        Some(Host::Domain(domain)) => {
            let domain = domain.to_ascii_lowercase();
            let ok = ALLOWED_ENDPOINT_HOST_SUFFIXES
                .iter()
                .any(|suffix| host_matches_allowed_suffix(&domain, suffix));
            if ok {
                Ok(())
            } else {
                Err(format!("push endpoint host not allowed: {domain}"))
            }
        }
        Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => {
            Err("push endpoint must be a hostname, not an IP address".into())
        }
        None => Err("push endpoint has no host".into()),
    }
}

pub struct WebPushNotifier {
    client: HyperWebPushClient,
    vapid: PartialVapidSignatureBuilder,
    subject: String,
    public_key_b64: String,
}

impl WebPushNotifier {
    pub fn from_base64(b64_pem: &str, subject: String) -> anyhow::Result<Self> {
        let (vapid, public_key_b64) = parse_vapid(b64_pem)?;
        Ok(Self {
            client: HyperWebPushClient::new(),
            vapid,
            subject,
            public_key_b64,
        })
    }

    pub fn public_key(&self) -> &str {
        &self.public_key_b64
    }

    pub async fn send(
        &self,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
        push: &Push,
    ) -> NotifyOutcome {
        let sub = SubscriptionInfo::new(endpoint, p256dh, auth);

        let mut sig_builder = self.vapid.clone().add_sub_info(&sub);
        sig_builder.add_claim("sub", self.subject.as_str());
        let signature = match sig_builder.build() {
            Ok(s) => s,
            Err(e) => return NotifyOutcome::Failed(format!("vapid signature: {e}")),
        };

        let payload = json!({
            "title": push.title,
            "body": push.body,
            "link": push.link,
            "event_type": push.event_type,
        })
        .to_string();

        let mut builder = WebPushMessageBuilder::new(&sub);
        builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
        builder.set_vapid_signature(signature);
        builder.set_urgency(urgency_for(&push.event_type));
        builder.set_ttl(push.ttl_secs);
        let message = match builder.build() {
            Ok(m) => m,
            Err(e) => return NotifyOutcome::Failed(format!("message build: {e}")),
        };

        const SEND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
        match tokio::time::timeout(SEND_TIMEOUT, self.client.send(message)).await {
            Err(_) => NotifyOutcome::Retryable,
            Ok(Ok(())) => NotifyOutcome::Delivered,
            Ok(Err(WebPushError::EndpointNotFound(_) | WebPushError::EndpointNotValid(_))) => {
                NotifyOutcome::TokenDead
            }
            Ok(Err(WebPushError::ServerError { .. })) => NotifyOutcome::Retryable,
            Ok(Err(e)) => NotifyOutcome::Failed(format!("web push: {e}")),
        }
    }
}

fn parse_vapid(b64_pem: &str) -> anyhow::Result<(PartialVapidSignatureBuilder, String)> {
    let pem = STANDARD
        .decode(b64_pem.trim())
        .map_err(|e| anyhow::anyhow!("VAPID_PRIVATE_KEY is not valid base64: {e}"))?;
    let vapid = VapidSignatureBuilder::from_pem_no_sub(std::io::Cursor::new(pem))
        .map_err(|e| anyhow::anyhow!("VAPID key parse failed: {e}"))?;
    let public_key_b64 = URL_SAFE_NO_PAD.encode(vapid.get_public_key());
    Ok((vapid, public_key_b64))
}

pub fn public_key_from_base64(b64_pem: &str) -> anyhow::Result<String> {
    Ok(parse_vapid(b64_pem)?.1)
}

pub fn cached_public_key() -> Option<&'static str> {
    static PUBLIC_KEY: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    PUBLIC_KEY
        .get_or_init(|| {
            let raw = std::env::var("VAPID_PRIVATE_KEY")
                .ok()
                .filter(|s| !s.is_empty())?;
            match public_key_from_base64(&raw) {
                Ok(k) => Some(k),
                Err(err) => {
                    log::warn!("vapid public key derive failed: {err}");
                    None
                }
            }
        })
        .as_deref()
}
