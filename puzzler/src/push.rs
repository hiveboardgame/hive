use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{path::Path, time::Duration};

/// Lines per request; keeps every body far below actix's default payload limit.
const CHUNK: usize = 100;

#[derive(Deserialize, Default, Debug)]
pub struct ImportReport {
    pub inserted: usize,
    pub duplicate: usize,
    pub rejected: usize,
    #[serde(default)]
    pub rejections: Vec<Rejection>,
}

#[derive(Deserialize, Debug)]
pub struct Rejection {
    pub line: usize,
    pub reason: String,
}

/// Posts a JSONL file of puzzles to the site in chunks. The site verifies each puzzle itself and
/// skips positions it already has, so a push can be repeated or resumed safely.
pub fn push(path: &Path, base_url: &str, token: &str) -> Result<ImportReport> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;
    let url = format!("{}/api/v1/puzzles/import", base_url.trim_end_matches('/'));
    let mut total = ImportReport::default();
    for (chunk_index, chunk) in lines.chunks(CHUNK).enumerate() {
        let response = client
            .post(&url)
            .bearer_auth(token)
            .header("content-type", "application/x-ndjson")
            .body(chunk.join("\n"))
            .send()
            .with_context(|| format!("posting to {url}"))?;
        let status = response.status();
        if !status.is_success() {
            bail!(
                "{url} answered {status}: {}",
                response.text().unwrap_or_default()
            );
        }
        let report: ImportReport = response.json()?;
        total.inserted += report.inserted;
        total.duplicate += report.duplicate;
        total.rejected += report.rejected;
        for rejection in report.rejections {
            total.rejections.push(Rejection {
                line: chunk_index * CHUNK + rejection.line,
                reason: rejection.reason,
            });
        }
        tracing::info!(
            sent = (chunk_index + 1) * CHUNK.min(chunk.len()),
            of = lines.len(),
            inserted = total.inserted,
            duplicate = total.duplicate,
            rejected = total.rejected,
            "pushing"
        );
    }
    Ok(total)
}
