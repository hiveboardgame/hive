use crate::{
    hiveground::{board_to_png, PreviewOpts},
    responses::GameResponse,
};
use actix_web::{
    get,
    http::header,
    web::{self, Bytes, Data, Path},
    HttpResponse,
};
use db_lib::{get_conn, DbPool};
use lru::LruCache;
use shared_types::GameId;
use std::{
    num::NonZeroUsize,
    sync::{LazyLock, Mutex},
};

/// Repeated unfurls should not pay the PNG render cost.
static CACHE: LazyLock<Mutex<LruCache<String, Bytes>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())));

/// Broken previews are worse than generic ones, so load failures redirect to
/// the static site image. `?turn=N` lets links highlight a specific position.
#[derive(serde::Deserialize)]
struct ImageQuery {
    turn: Option<usize>,
}

#[get("/og/game/{nanoid}.png")]
pub async fn og_game_image(
    nanoid: Path<String>,
    query: web::Query<ImageQuery>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let nanoid = nanoid.into_inner();
    match render_image(&nanoid, query.turn, pool).await {
        Ok(response) => response,
        Err(err) => {
            log::warn!("OG image render failed for {nanoid}: {err:?}");
            HttpResponse::Found()
                .insert_header((header::LOCATION, "/assets/stacked_3D.png"))
                .finish()
        }
    }
}

async fn render_image(
    nanoid: &str,
    turn: Option<usize>,
    pool: Data<DbPool>,
) -> anyhow::Result<HttpResponse> {
    let game = {
        let mut conn = get_conn(&pool).await?;
        GameResponse::new_from_game_id(&GameId(nanoid.to_string()), &mut conn).await?
    };
    let rendered_turn = canonical_render_turn(game.history.len(), turn);
    let key = format!("{}-{rendered_turn}", game.game_id.0);

    // Cache misses are slow enough that hits must not wait behind them.
    let cached = CACHE.lock().unwrap().get(&key).cloned();
    let png = match cached {
        Some(png) => png,
        None => {
            // PNG encoding can monopolize an async worker.
            let board = match turn {
                Some(_) => game.create_state_at_turn(rendered_turn).board,
                None => game.create_state().board,
            };
            let png =
                web::block(move || board_to_png(&board, &PreviewOpts::default()).map(Bytes::from))
                    .await??;
            CACHE.lock().unwrap().put(key.clone(), png.clone());
            png
        }
    };

    // Long-lived headers keep repeated crawler unfurls cheap.
    let immutable = game.finished || turn.is_some_and(|turn| turn < game.turn);
    let cache_control = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=30"
    };

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .insert_header((header::CACHE_CONTROL, cache_control))
        .insert_header((header::ETAG, format!("\"{key}\"")))
        .body(png))
}

fn canonical_render_turn(history_len: usize, turn: Option<usize>) -> usize {
    turn.unwrap_or(history_len).min(history_len)
}

#[cfg(test)]
mod tests {
    use super::canonical_render_turn;

    #[test]
    fn canonical_render_turn_uses_latest_by_default() {
        assert_eq!(canonical_render_turn(12, None), 12);
    }

    #[test]
    fn canonical_render_turn_clamps_out_of_range_turns() {
        assert_eq!(canonical_render_turn(12, Some(999)), 12);
    }

    #[test]
    fn canonical_render_turn_keeps_in_range_turns() {
        assert_eq!(canonical_render_turn(12, Some(5)), 5);
    }
}
