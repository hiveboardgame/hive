use actix_web::{http::header, post, web::Data, HttpRequest, HttpResponse};
use db_lib::{get_conn, models::Puzzle, DbPool};
use serde::Serialize;
use shared_types::{start_hash, PuzzleImport};
use subtle::ConstantTimeEq;

/// Bodies are JSON lines; a generator batch of a few thousand puzzles stays well under this.
const MAX_REJECTIONS_REPORTED: usize = 200;

/// The shared secret the puzzle generator presents. It grants nothing but puzzle import.
pub struct PuzzleImportToken(Option<String>);

impl PuzzleImportToken {
    pub fn from_env() -> Self {
        Self(
            std::env::var("PUZZLE_IMPORT_TOKEN")
                .ok()
                .filter(|token| !token.is_empty()),
        )
    }
}

#[derive(Serialize, Default)]
struct ImportReport {
    inserted: usize,
    duplicate: usize,
    rejected: usize,
    rejections: Vec<Rejection>,
}

#[derive(Serialize)]
struct Rejection {
    line: usize,
    reason: String,
}

fn authorized(request: &HttpRequest, token: &PuzzleImportToken) -> Result<(), HttpResponse> {
    let Some(expected) = token.0.as_deref() else {
        return Err(HttpResponse::ServiceUnavailable().body("puzzle import is not configured"));
    };
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if presented.as_bytes().ct_eq(expected.as_bytes()).into() {
        Ok(())
    } else {
        Err(HttpResponse::Unauthorized().finish())
    }
}

/// Imports puzzles, one `PuzzleImport` JSON per line. Every line is replayed with the site's
/// engine before it is stored, so the generator never has to be trusted; re-sending a batch is
/// harmless because positions already stored count as duplicates.
#[post("/api/v1/puzzles/import")]
pub async fn api_import_puzzles(
    request: HttpRequest,
    body: String,
    pool: Data<DbPool>,
    token: Data<PuzzleImportToken>,
) -> HttpResponse {
    if let Err(response) = authorized(&request, &token) {
        return response;
    }
    let mut conn = match get_conn(&pool).await {
        Ok(conn) => conn,
        Err(error) => return HttpResponse::ServiceUnavailable().body(error.to_string()),
    };
    let mut report = ImportReport::default();
    let reject = |report: &mut ImportReport, line: usize, reason: String| {
        report.rejected += 1;
        if report.rejections.len() < MAX_REJECTIONS_REPORTED {
            report.rejections.push(Rejection { line, reason });
        }
    };
    for (index, text) in body.lines().enumerate() {
        let line = index + 1;
        if text.trim().is_empty() {
            continue;
        }
        let puzzle: PuzzleImport = match serde_json::from_str(text) {
            Ok(puzzle) => puzzle,
            Err(error) => {
                reject(&mut report, line, format!("not a puzzle: {error}"));
                continue;
            }
        };
        if let Err(error) = puzzle.verify() {
            reject(&mut report, line, error.to_string());
            continue;
        }
        let hash = match puzzle.start_state().and_then(|state| start_hash(&state)) {
            Ok(hash) => hash,
            Err(error) => {
                reject(&mut report, line, error.to_string());
                continue;
            }
        };
        match Puzzle::import(&puzzle, hash, &mut conn).await {
            Ok(Some(_)) => report.inserted += 1,
            Ok(None) => report.duplicate += 1,
            Err(error) => reject(&mut report, line, error.to_string()),
        }
    }
    log::info!(
        "puzzle import: {} inserted, {} duplicate, {} rejected",
        report.inserted,
        report.duplicate,
        report.rejected
    );
    HttpResponse::Ok().json(report)
}
