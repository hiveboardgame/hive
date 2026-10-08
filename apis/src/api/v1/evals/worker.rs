use super::{presence::worker_polled, worker_auth::EvalWorker};
use actix_web::{
    post,
    web::{Bytes, Data, Json, Path},
    HttpResponse,
};
use db_lib::{
    get_conn,
    models::{Game, GameEval},
    DbPool,
};
use shared_types::{history_moves, EvalClaim, EvalFailure, EvalJob, EvalProgress, EvalSubmission};
use uuid::Uuid;

const MAX_WORKER_NAME: usize = 64;

fn bad_worker_name(name: &str) -> bool {
    name.is_empty() || name.len() > MAX_WORKER_NAME
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
    worker_polled();
    let Ok(mut conn) = get_conn(&pool).await else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let eval = match GameEval::claim(&claim.worker, claim.auto, &mut conn).await {
        Ok(Some(eval)) => eval,
        Ok(None) => return HttpResponse::NoContent().finish(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match Game::find_by_uuid(&eval.game_id, &mut conn).await {
        Ok(game) => HttpResponse::Ok().json(EvalJob {
            eval_id: eval.id,
            game_type: game.game_type,
            moves: history_moves(&game.history),
        }),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
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
    let pct = i16::from(progress.progress_pct);
    match GameEval::report_progress(*id, &progress.worker, pct, &mut conn).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::Conflict().body("this eval is not yours"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// Takes raw bytes: a long game's result outgrows actix's 32 KB JSON default.
#[post("/api/v1/evals/{id}/result")]
pub async fn eval_result(
    _worker: EvalWorker,
    id: Path<Uuid>,
    body: Bytes,
    pool: Data<DbPool>,
) -> HttpResponse {
    let submission: EvalSubmission = match serde_json::from_slice(&body) {
        Ok(submission) => submission,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };
    let Ok(mut conn) = get_conn(&pool).await else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let Ok(eval) = GameEval::find(*id, &mut conn).await else {
        return HttpResponse::NotFound().finish();
    };
    let Ok(game) = Game::find_by_uuid(&eval.game_id, &mut conn).await else {
        return HttpResponse::InternalServerError().finish();
    };
    if let Err(e) = submission.result.validate(&history_moves(&game.history)) {
        let reason = format!("rejected result: {e}");
        let _ = GameEval::fail(*id, &submission.worker, &reason, &mut conn).await;
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
        Ok(false) => HttpResponse::Conflict().body("this eval is not yours"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

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
    match GameEval::fail(*id, &failure.worker, &error, &mut conn).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::Conflict().body("this eval is not yours"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
