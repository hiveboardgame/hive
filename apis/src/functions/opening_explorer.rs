use crate::responses::ExplorerResponse;
use leptos::prelude::*;
use server_fn::codec;
use shared_types::ExplorerFilters;

/// Opening explorer for a single position, identified by its canonical board hash. `hash == 0`
/// is the empty board: the response lists the opening roots (first moves) instead of running a
/// self-join. Per-request work is DB-only (the opening roots are computed from the engine, which
/// is deterministic and cheap).
#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn opening_explorer(
    hash: i64,
    filters: ExplorerFilters,
) -> Result<ExplorerResponse, ServerFnError> {
    use crate::{functions::db::pool, responses::GameResponse};
    use db_lib::{get_conn, models::GameHash};
    use hive_lib::State;
    use shared_types::ExplorerMove;

    const TOP_GAMES: i64 = 4;
    const RECENT_GAMES: i64 = 4;
    const SUGGESTIONS: i64 = 8;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    if hash == 0 {
        // Empty board: aggregate each deterministic opening root via the single-hash aggregate.
        let mut moves = Vec::new();
        for (piece, position, root_hash) in State::opening_hashes(filters.game_type) {
            let mut stats = GameHash::aggregate_one(root_hash as i64, &filters, &mut conn)
                .await
                .map_err(ServerFnError::new)?;
            stats.piece = piece;
            stats.position = position;
            moves.push(stats);
        }
        moves.sort_by(|a, b| b.total.cmp(&a.total));

        // Header for the empty board: summed stats over all roots.
        let position_total = ExplorerMove {
            next_hash: 0,
            piece: String::new(),
            position: String::new(),
            total: moves.iter().map(|m| m.total).sum(),
            white_wins: moves.iter().map(|m| m.white_wins).sum(),
            black_wins: moves.iter().map(|m| m.black_wins).sum(),
            draws: moves.iter().map(|m| m.draws).sum(),
            avg_rating: None,
        };

        return Ok(ExplorerResponse {
            position_total,
            moves,
            top_games: Vec::new(),
            recent_games: Vec::new(),
        });
    }

    let moves = GameHash::next_moves(hash, &filters, Some(SUGGESTIONS), &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    let position_total = GameHash::aggregate_one(hash, &filters, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    let top_ids = GameHash::top_game_ids(hash, &filters, Some(TOP_GAMES), &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    let mut top_games = GameResponse::from_game_ids(&top_ids, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    top_games.sort_by_key(|g| {
        top_ids
            .iter()
            .position(|id| *id == g.uuid)
            .unwrap_or(usize::MAX)
    });

    let recent_ids = GameHash::recent_game_ids(hash, &filters, Some(RECENT_GAMES), &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    let mut recent_games = GameResponse::from_game_ids(&recent_ids, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    recent_games.sort_by_key(|g| {
        recent_ids
            .iter()
            .position(|id| *id == g.uuid)
            .unwrap_or(usize::MAX)
    });

    Ok(ExplorerResponse {
        position_total,
        moves,
        top_games,
        recent_games,
    })
}
