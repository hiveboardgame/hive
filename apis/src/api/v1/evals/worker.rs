use super::{presence::worker_seen, worker_auth::EvalWorker};
use crate::jobs::eval_sweeper::MAX_ATTEMPTS;
use actix_web::{
    post,
    web::{Data, Json, Path, Payload},
    HttpResponse,
};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{AutoEvals, ClaimedEval, Game, GameEval},
    DbPool,
};
use shared_types::{
    history_moves,
    EvalClaim,
    EvalFailure,
    EvalJob,
    EvalProgress,
    EvalStatus,
    EvalSubmission,
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

const MAX_WORKER_NAME: usize = 64;
/// A result takes about 200 bytes a move, and the longest stored games run to a few thousand
/// moves; nginx's `client_max_body_size` on the eval locations must allow as much.
const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;
/// Picking a new game scans the finished games; once none is left, idle workers asking every
/// five seconds must not repeat that scan each time.
const PICK_AGAIN_AFTER: Duration = Duration::from_secs(300);

static NOTHING_TO_PICK_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

fn bad_worker_name(name: &str) -> bool {
    name.is_empty() || name.len() > MAX_WORKER_NAME
}

fn auto_evals(wanted: bool) -> AutoEvals {
    let picked_nothing_lately = NOTHING_TO_PICK_UNTIL
        .lock()
        .is_ok_and(|until| until.is_some_and(|until| Instant::now() < until));
    match (wanted, picked_nothing_lately) {
        (false, _) => AutoEvals::Off,
        (true, true) => AutoEvals::Queued,
        (true, false) => AutoEvals::QueuedOrNew,
    }
}

fn nothing_to_pick() {
    if let Ok(mut until) = NOTHING_TO_PICK_UNTIL.lock() {
        *until = Some(Instant::now() + PICK_AGAIN_AFTER);
    }
}

#[post("/api/v1/evals/claim")]
pub async fn eval_claim(
    _worker: EvalWorker,
    claim: Json<EvalClaim>,
    pool: Data<DbPool>,
) -> HttpResponse {
    if bad_worker_name(&claim.worker) {
        return HttpResponse::BadRequest().body("worker name must be 1-64 bytes");
    }
    worker_seen(&claim.worker);
    let Ok(mut conn) = get_conn(&pool).await else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let auto = auto_evals(claim.auto);
    match GameEval::claim(&claim.worker, auto, &mut conn).await {
        Ok(Some(ClaimedEval { eval, game })) => HttpResponse::Ok().json(EvalJob {
            eval_id: eval.id,
            game_type: game.game_type,
            moves: history_moves(&game.history),
        }),
        Ok(None) => {
            if auto == AutoEvals::QueuedOrNew {
                nothing_to_pick();
            }
            HttpResponse::NoContent().finish()
        }
        Err(e) => db_failure("claim", &e),
    }
}

fn db_failure(action: &str, error: &DbError) -> HttpResponse {
    log::error!("eval worker {action}: {error}");
    HttpResponse::InternalServerError().finish()
}

#[post("/api/v1/evals/{id}/progress")]
pub async fn eval_progress(
    _worker: EvalWorker,
    id: Path<Uuid>,
    progress: Json<EvalProgress>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let Ok(mut conn) = get_conn(&pool).await else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    worker_seen(&progress.worker);
    let pct = i16::from(progress.progress_pct);
    match GameEval::report_progress(*id, &progress.worker, pct, &mut conn).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::Conflict().body("this eval is not yours"),
        Err(e) => db_failure("progress", &e),
    }
}

/// Reads the body itself: a long game's result outgrows both actix's JSON and raw-body
/// defaults.
#[post("/api/v1/evals/{id}/result")]
pub async fn eval_result(
    _worker: EvalWorker,
    id: Path<Uuid>,
    body: Payload,
    pool: Data<DbPool>,
) -> HttpResponse {
    let body = match body.to_bytes_limited(MAX_RESULT_BYTES).await {
        Ok(Ok(body)) => body,
        Ok(Err(e)) => return HttpResponse::BadRequest().body(e.to_string()),
        Err(_) => {
            return HttpResponse::PayloadTooLarge()
                .body(format!("a result may be at most {MAX_RESULT_BYTES} bytes"))
        }
    };
    let submission: EvalSubmission = match serde_json::from_slice(&body) {
        Ok(submission) => submission,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };
    let Ok(mut conn) = get_conn(&pool).await else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let eval = match GameEval::find(*id, &mut conn).await {
        Ok(eval) => eval,
        Err(DbError::NotFound { .. }) => return HttpResponse::NotFound().finish(),
        Err(e) => return db_failure("result", &e),
    };
    // A retry after the first answer got lost on the way back.
    if eval.status() == EvalStatus::Done && eval.worker.as_deref() == Some(&submission.worker) {
        return HttpResponse::Ok().finish();
    }
    let game = match Game::find_by_uuid(&eval.game_id, &mut conn).await {
        Ok(game) => game,
        Err(e) => return db_failure("result", &e),
    };
    if let Err(e) = submission.result.validate(&history_moves(&game.history)) {
        let reason = format!("rejected result: {e}");
        if let Err(e) = GameEval::fail(*id, &submission.worker, &reason, &mut conn).await {
            log::error!("eval worker result: recording {reason}: {e}");
        }
        return HttpResponse::UnprocessableEntity().body(reason);
    }
    match GameEval::finish(
        *id,
        &submission.worker,
        &submission.engine,
        &submission.result,
        &mut conn,
    )
    .await
    {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::Conflict().body("this eval is no longer open"),
        Err(e) => db_failure("result", &e),
    }
}

/// The worker could not finish: the eval waits for another try, up to the sweeper's limit.
#[post("/api/v1/evals/{id}/fail")]
pub async fn eval_fail(
    _worker: EvalWorker,
    id: Path<Uuid>,
    failure: Json<EvalFailure>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let Ok(mut conn) = get_conn(&pool).await else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let error: String = failure.error.chars().take(500).collect();
    match GameEval::give_back(*id, &failure.worker, &error, MAX_ATTEMPTS, &mut conn).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::Conflict().body("this eval is not yours"),
        Err(e) => db_failure("fail", &e),
    }
}
