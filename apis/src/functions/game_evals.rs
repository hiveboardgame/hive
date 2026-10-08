use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use server_fn::codec;
use shared_types::{GameEvalView, GameId};

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_eval(game_id: GameId) -> Result<GameEvalView, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{
        get_conn,
        models::{Game, GameEval},
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_eval_access(&mut conn).await?;
    let game = Game::find_by_game_id(&game_id, &mut conn).await?;
    let Some(eval) = GameEval::find_by_game(game.id, &mut conn).await? else {
        return Ok(GameEvalView::NotRequested);
    };
    let queue = queue_snapshot(&mut conn).await?;
    Ok(eval.view(&queue))
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn request_game_eval(game_id: GameId) -> Result<GameEvalView, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        db_error::DbError,
        get_conn,
        models::{Game, GameEval},
    };
    use shared_types::{eval_unavailable_reason, history_moves};

    let user = uuid()
        .await
        .map_err(|_| ServerFnError::new("Log in to request an eval."))?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_eval_access(&mut conn).await?;
    let game = Game::find_by_game_id(&game_id, &mut conn).await?;
    let moves = history_moves(&game.history).len();
    if let Some(reason) = eval_unavailable_reason(
        game.finished,
        &game.game_type,
        game.tournament_queen_rule,
        moves,
    ) {
        return Err(ServerFnError::new(reason));
    }
    let eval = GameEval::request(game.id, user, moves as i32, &mut conn)
        .await
        .map_err(|e| match e {
            DbError::InvalidAction { info } => ServerFnError::new(info),
            e => ServerFnError::new(e),
        })?;
    let queue = queue_snapshot(&mut conn).await?;
    Ok(eval.view(&queue))
}

/// Evals stay admin-only while the 2026 world championship runs. To open them to everyone,
/// delete this and its calls, and `evals_visible` on the client.
#[cfg(feature = "ssr")]
async fn ensure_eval_access(conn: &mut db_lib::DbConn<'_>) -> Result<(), ServerFnError> {
    crate::functions::auth::identity::ensure_admin(conn).await
}

#[cfg(feature = "ssr")]
async fn queue_snapshot(
    conn: &mut db_lib::DbConn<'_>,
) -> Result<db_lib::models::QueueSnapshot, ServerFnError> {
    use crate::api::v1::evals::presence::worker_seen_recently;
    let mut queue = db_lib::models::GameEval::queue_snapshot(conn).await?;
    queue.worker_online = worker_seen_recently() || !queue.running.is_empty();
    Ok(queue)
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_recent_evals() -> Result<shared_types::RecentEvals, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::GameEval};
    const SHOWN: i64 = 6;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_eval_access(&mut conn).await?;
    Ok(shared_types::RecentEvals {
        recent: GameEval::recent(SHOWN, &mut conn).await?,
        in_line: GameEval::in_line(&mut conn).await?,
    })
}
