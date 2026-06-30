use super::EmailConfig;
use std::io::Write;

const FROM: &str = "Hivegame <noreply@hivegame.com>";
const DEBUG_LOG: &str = "email_debug.log";

pub async fn deliver(
    config: &EmailConfig,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    match &config.client {
        None => {
            write_debug_log(to, subject, body);
            Ok(())
        }
        Some(client) => {
            use lettermint_rs::{api::email::SendEmailRequest, Query};
            let request = SendEmailRequest::builder()
                .from(FROM)
                .to(vec![to.to_string()])
                .subject(subject)
                .text(body)
                .build();
            request
                .execute(client.as_ref())
                .await
                .map(|_| ())
                .map_err(|err| format!("{err:?}"))
        }
    }
}

fn write_debug_log(to: &str, subject: &str, body: &str) {
    let entry = format!(
        "===== {} =====\nTo: {to}\nSubject: {subject}\n\n{body}\n\n",
        chrono::Utc::now().to_rfc3339()
    );
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEBUG_LOG)
    {
        Ok(mut file) => {
            if let Err(err) = file.write_all(entry.as_bytes()) {
                log::warn!("email: failed writing {DEBUG_LOG}: {err}");
            }
        }
        Err(err) => log::warn!("email: failed opening {DEBUG_LOG}: {err}"),
    }
}
