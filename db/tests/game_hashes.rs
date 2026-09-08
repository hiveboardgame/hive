mod common;

use chrono::Utc;
use db_lib::{
    get_conn,
    models::{Game, GameFinishContext, GameHash, NewGame, NewUser, User},
    schema::{game_hashes as gh_schema, games},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{GameStatus, GameType, State};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};

fn test_ctx(white_rating: Option<f64>, black_rating: Option<f64>) -> GameFinishContext {
    GameFinishContext {
        white_rating,
        black_rating,
        result: "Finished(Winner(White))".to_string(),
        speed: GameSpeed::Bullet.to_string(),
        game_type: GameType::MLP.to_string(),
        rated: true,
        played_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Unit tests — GameHash::from_engine_hashes
// ---------------------------------------------------------------------------

#[test]
fn alternates_white_and_black_ratings_by_turn() {
    let id = uuid::Uuid::new_v4();
    let ctx = test_ctx(Some(1500.0), Some(1700.0));
    let entries = GameHash::from_engine_hashes(id, &[10, 20, 30, 40], &[], &ctx);

    assert_eq!(entries.len(), 4);
    for (i, e) in entries.iter().enumerate() {
        assert_eq!(e.game_id, id);
        assert_eq!(e.turn, i as i32);
        assert_eq!(e.result, ctx.result);
        assert_eq!(e.speed, ctx.speed);
        assert_eq!(e.game_type, ctx.game_type);
        assert!(e.rated);
    }
    // Even turns → white, odd → black
    assert_eq!(entries[0].rating, Some(1500.0));
    assert_eq!(entries[1].rating, Some(1700.0));
    assert_eq!(entries[2].rating, Some(1500.0));
    assert_eq!(entries[3].rating, Some(1700.0));
}

#[test]
fn none_rating_propagates_to_matching_turns() {
    let id = uuid::Uuid::new_v4();
    let ctx = test_ctx(None, Some(1600.0));
    let entries = GameHash::from_engine_hashes(id, &[1, 2], &[], &ctx);

    assert_eq!(entries[0].rating, None); // white is None
    assert_eq!(entries[1].rating, Some(1600.0));
}

#[test]
fn empty_hashes_produce_no_entries() {
    let ctx = test_ctx(Some(1.0), Some(2.0));
    let entries = GameHash::from_engine_hashes(uuid::Uuid::new_v4(), &[], &[], &ctx);
    assert!(entries.is_empty());
}

#[test]
fn u64_i64_roundtrip_preserves_high_bit_values() {
    let big: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    let ctx = test_ctx(None, None);
    let entries = GameHash::from_engine_hashes(uuid::Uuid::new_v4(), &[big], &[], &ctx);
    // The stored i64 is the bitwise reinterpretation: -1
    assert_eq!(entries[0].hash, -1_i64);
    // Converting back gives the original u64
    assert_eq!(entries[0].hash as u64, big);
}

// ---------------------------------------------------------------------------
// Integration tests — DB round-trips
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn insert_and_find_returns_correct_entries() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;

    let ctx = test_ctx(Some(1500.0), Some(1600.0));
    let entries = GameHash::from_engine_hashes(game.id, &[100, 200], &[], &ctx);
    GameHash::insert_batch(&entries, &mut conn).await.unwrap();

    let found = GameHash::find_by_hash(100, &mut conn).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].game_id, game.id);
    assert_eq!(found[0].turn, 0);
    assert_eq!(found[0].rating, Some(1500.0));
    assert_eq!(found[0].speed, GameSpeed::Bullet.to_string());
    assert!(found[0].rated);

    let found2 = GameHash::find_by_hash(200, &mut conn).await.unwrap();
    assert_eq!(found2.len(), 1);
    assert_eq!(found2[0].turn, 1);
    assert_eq!(found2[0].rating, Some(1600.0));
}

#[tokio::test(flavor = "multi_thread")]
async fn same_hash_across_multiple_games() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let shared_hash = 0xCAFE_u64;

    let (game_a, _, _) = setup_game_named("a1", "a2", &mut conn).await;
    let (game_b, _, _) = setup_game_named("b1", "b2", &mut conn).await;

    let ctx_a = test_ctx(Some(1500.0), None);
    let ctx_b = test_ctx(None, Some(1800.0));

    let entries_a = GameHash::from_engine_hashes(game_a.id, &[shared_hash], &[], &ctx_a);
    let entries_b = GameHash::from_engine_hashes(game_b.id, &[shared_hash, 999], &[], &ctx_b);

    GameHash::insert_batch(&entries_a, &mut conn).await.unwrap();
    GameHash::insert_batch(&entries_b, &mut conn).await.unwrap();

    let found = GameHash::find_by_hash(shared_hash, &mut conn)
        .await
        .unwrap();
    assert_eq!(found.len(), 2);

    let game_ids: Vec<uuid::Uuid> = found.iter().map(|e| e.game_id).collect();
    assert!(game_ids.contains(&game_a.id));
    assert!(game_ids.contains(&game_b.id));
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_insert_is_idempotent() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;

    let ctx = test_ctx(None, None);
    let entries = GameHash::from_engine_hashes(game.id, &[42], &[], &ctx);
    GameHash::insert_batch(&entries, &mut conn).await.unwrap();
    GameHash::insert_batch(&entries, &mut conn).await.unwrap();

    let found = GameHash::find_by_hash(42, &mut conn).await.unwrap();
    assert_eq!(found.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_batch_insert_is_noop() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    GameHash::insert_batch(&[], &mut conn).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cascade_delete_removes_hash_entries_when_game_deleted() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;

    let ctx = test_ctx(Some(1500.0), Some(1600.0));
    let entries = GameHash::from_engine_hashes(game.id, &[777, 888], &[], &ctx);
    GameHash::insert_batch(&entries, &mut conn).await.unwrap();

    assert_eq!(
        GameHash::find_by_hash(777, &mut conn).await.unwrap().len(),
        1
    );

    game.delete(&mut conn).await.unwrap();

    assert!(GameHash::find_by_hash(777, &mut conn)
        .await
        .unwrap()
        .is_empty());
    assert!(GameHash::find_by_hash(888, &mut conn)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn find_nonexistent_hash_returns_empty() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let found = GameHash::find_by_hash(0xDEAD_BEEF, &mut conn)
        .await
        .unwrap();
    assert!(found.is_empty());
}

// ---------------------------------------------------------------------------
// Integration tests — GameHash::best
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn best_returns_highest_rated_games_first() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let hash = 0xBE57_u64;
    let (g1, _, _) = setup_game_named("lo1", "lo2", &mut conn).await;
    let (g2, _, _) = setup_game_named("mid1", "mid2", &mut conn).await;
    let (g3, _, _) = setup_game_named("hi1", "hi2", &mut conn).await;

    for (gid, rating) in [(g1.id, 1200.0), (g2.id, 1800.0), (g3.id, 2400.0)] {
        let ctx = test_ctx(Some(rating), None);
        let entries = GameHash::from_engine_hashes(gid, &[hash], &[], &ctx);
        GameHash::insert_batch(&entries, &mut conn).await.unwrap();
    }

    let best = GameHash::best(hash, None, &mut conn).await.unwrap();
    assert_eq!(best.len(), 3);
    assert_eq!(best[0].game_id, g3.id); // 2400
    assert_eq!(best[1].game_id, g2.id); // 1800
    assert_eq!(best[2].game_id, g1.id); // 1200
}

#[tokio::test(flavor = "multi_thread")]
async fn best_respects_limit() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let hash = 0x1111_u64;
    let (g1, _, _) = setup_game_named("l1", "l2", &mut conn).await;
    let (g2, _, _) = setup_game_named("l3", "l4", &mut conn).await;
    let (g3, _, _) = setup_game_named("l5", "l6", &mut conn).await;

    for (gid, rating) in [(g1.id, 1000.0), (g2.id, 2000.0), (g3.id, 3000.0)] {
        let ctx = test_ctx(Some(rating), None);
        let entries = GameHash::from_engine_hashes(gid, &[hash], &[], &ctx);
        GameHash::insert_batch(&entries, &mut conn).await.unwrap();
    }

    let best = GameHash::best(hash, Some(2), &mut conn).await.unwrap();
    assert_eq!(best.len(), 2);
    assert_eq!(best[0].rating, Some(3000.0));
    assert_eq!(best[1].rating, Some(2000.0));
}

#[tokio::test(flavor = "multi_thread")]
async fn best_excludes_null_ratings() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let hash = 0x2222_u64;
    let (g1, _, _) = setup_game_named("n1", "n2", &mut conn).await;
    let (g2, _, _) = setup_game_named("n3", "n4", &mut conn).await;

    let ctx_unrated = test_ctx(None, None);
    let ctx_rated = test_ctx(Some(1500.0), None);

    let entries1 = GameHash::from_engine_hashes(g1.id, &[hash], &[], &ctx_unrated);
    let entries2 = GameHash::from_engine_hashes(g2.id, &[hash], &[], &ctx_rated);
    GameHash::insert_batch(&entries1, &mut conn).await.unwrap();
    GameHash::insert_batch(&entries2, &mut conn).await.unwrap();

    let best = GameHash::best(hash, None, &mut conn).await.unwrap();
    assert_eq!(best.len(), 1);
    assert_eq!(best[0].game_id, g2.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn best_defaults_to_10() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let hash = 0xDEF_u64;
    for i in 0..15 {
        let (g, _, _) = setup_game_named(
            &format!("d{}", i * 2),
            &format!("d{}", i * 2 + 1),
            &mut conn,
        )
        .await;
        let ctx = test_ctx(Some(1000.0 + i as f64), None);
        let entries = GameHash::from_engine_hashes(g.id, &[hash], &[], &ctx);
        GameHash::insert_batch(&entries, &mut conn).await.unwrap();
    }

    let best = GameHash::best(hash, None, &mut conn).await.unwrap();
    assert_eq!(best.len(), 10);
    assert_eq!(best[0].rating, Some(1014.0));
    assert_eq!(best[9].rating, Some(1005.0));
}

#[tokio::test(flavor = "multi_thread")]
async fn best_returns_empty_for_unknown_hash() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let best = GameHash::best(0x3333, None, &mut conn).await.unwrap();
    assert!(best.is_empty());
}

// ---------------------------------------------------------------------------
// Integration tests — denormalized fields round-trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn denormalized_fields_are_stored_and_retrieved() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;

    let ctx = GameFinishContext {
        white_rating: Some(2000.0),
        black_rating: Some(1800.0),
        result: "Finished(Draw)".to_string(),
        speed: GameSpeed::Correspondence.to_string(),
        game_type: "Base".to_string(),
        rated: false,
        played_at: Utc::now(),
    };
    let entries = GameHash::from_engine_hashes(game.id, &[555], &[], &ctx);
    GameHash::insert_batch(&entries, &mut conn).await.unwrap();

    let found = GameHash::find_by_hash(555, &mut conn).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].result, "Finished(Draw)");
    assert_eq!(found[0].speed, GameSpeed::Correspondence.to_string());
    assert_eq!(found[0].game_type, "Base");
    assert!(!found[0].rated);
    assert_eq!(found[0].rating, Some(2000.0));
}

// ---------------------------------------------------------------------------
// Integration tests — Game.hashes() accessor
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn game_hashes_accessor_returns_empty_for_new_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;
    assert!(game.hashes().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn game_hashes_accessor_roundtrips_through_set_hashes() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;

    let original: Vec<u64> = vec![0, 1, u64::MAX, 0xDEAD_BEEF_CAFE_BABE];
    let db_hashes: Vec<Option<i64>> = original.iter().map(|h| Some(*h as i64)).collect();
    Game::set_hashes(game.id, db_hashes, &mut conn)
        .await
        .unwrap();

    let reloaded: Game = games::table.find(game.id).first(&mut conn).await.unwrap();
    assert_eq!(reloaded.hashes(), original);
}

// ---------------------------------------------------------------------------
// Integration tests — backfill query methods
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn backfill_query_finds_finished_games_with_empty_hashes() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let (game, _, _) = setup_game_with_history("f1", "f2", "wQ;bQ /wQ;", &mut conn).await;

    let empty = Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap();
    assert!(empty.is_empty());

    diesel::update(games::table.find(game.id))
        .set(games::finished.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();

    let needing = Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap();
    assert_eq!(needing.len(), 1);
    assert_eq!(needing[0].id, game.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn backfill_query_skips_games_with_populated_hashes() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let (game, _, _) = setup_game_with_history("g1", "g2", "wQ;bQ /wQ;", &mut conn).await;
    diesel::update(games::table.find(game.id))
        .set(games::finished.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();

    Game::set_hashes(game.id, vec![Some(1)], &mut conn)
        .await
        .unwrap();

    let needing = Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap();
    assert!(needing.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn backfill_query_skips_games_with_empty_history() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let (game, _, _) = setup_game_with_history("h1", "h2", "", &mut conn).await;
    diesel::update(games::table.find(game.id))
        .set(games::finished.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();

    let needing = Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap();
    assert!(needing.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn backfill_cursor_pagination_works() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let (g1, _, _) = setup_game_with_history("p1", "p2", "wQ;", &mut conn).await;
    let (g2, _, _) = setup_game_with_history("p3", "p4", "wQ;", &mut conn).await;
    for gid in [g1.id, g2.id] {
        diesel::update(games::table.find(gid))
            .set(games::finished.eq(true))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    let first_batch = Game::find_needing_hash_backfill(None, 1, &mut conn)
        .await
        .unwrap();
    assert_eq!(first_batch.len(), 1);

    let second_batch = Game::find_needing_hash_backfill(Some(first_batch[0].id), 1, &mut conn)
        .await
        .unwrap();
    assert_eq!(second_batch.len(), 1);
    assert_ne!(first_batch[0].id, second_batch[0].id);

    let third_batch = Game::find_needing_hash_backfill(Some(second_batch[0].id), 1, &mut conn)
        .await
        .unwrap();
    assert!(third_batch.is_empty());
}

/// Without the delete, the `on_conflict_do_nothing` insert would keep stale hashes at existing
/// turns and report success - a hash-algorithm change would silently survive its own backfill.
#[tokio::test(flavor = "multi_thread")]
async fn insert_for_game_replaces_stale_rows_and_trims_extinct_turns() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (game, _, _) = setup_game(&mut conn).await;

    // Three stale rows, as an old hash algorithm would have left them.
    let ctx = test_ctx(Some(1500.0), Some(1600.0));
    let stale = GameHash::from_engine_hashes(game.id, &[111, 222, 333], &[], &ctx);
    GameHash::insert_batch(&stale, &mut conn).await.unwrap();

    GameHash::insert_for_game(game.id, &[901, 902], &[], &ctx, &mut conn)
        .await
        .unwrap();

    let mut rows: Vec<(i32, i64)> = gh_schema::table
        .filter(gh_schema::game_id.eq(game.id))
        .select((gh_schema::turn, gh_schema::hash))
        .load(&mut conn)
        .await
        .unwrap();
    rows.sort();
    assert_eq!(
        rows,
        vec![(0, 901), (1, 902)],
        "old hashes replaced at turns 0 and 1, stale turn 2 removed"
    );
}

// ---------------------------------------------------------------------------
// Integration test — full backfill flow (replay + set_hashes + game_hashes)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn backfill_replays_history_and_populates_both_tables() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    let history = "wQ;bQ /wQ;";
    let (game, _, _) = setup_game_with_history("r1", "r2", history, &mut conn).await;
    diesel::update(games::table.find(game.id))
        .set(games::finished.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();

    let state = State::new_from_str(history, &GameType::MLP.to_string()).unwrap();
    assert!(!state.hashes.is_empty());

    let db_hashes: Vec<Option<i64>> = state.hashes.iter().map(|h| Some(*h as i64)).collect();
    Game::set_hashes(game.id, db_hashes, &mut conn)
        .await
        .unwrap();

    let ctx = test_ctx(None, None);
    let hash_entries = GameHash::from_engine_hashes(game.id, &state.hashes, &[], &ctx);
    GameHash::insert_batch(&hash_entries, &mut conn)
        .await
        .unwrap();

    let reloaded: Game = games::table.find(game.id).first(&mut conn).await.unwrap();
    assert_eq!(reloaded.hashes(), state.hashes);

    let count: i64 = gh_schema::table
        .filter(gh_schema::game_id.eq(game.id))
        .count()
        .get_result(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, state.hashes.len() as i64);
}

// ---------------------------------------------------------------------------
// Integration tests — next_moves / aggregate_one (opening explorer)
// ---------------------------------------------------------------------------

fn nm_filters() -> shared_types::ExplorerFilters {
    shared_types::ExplorerFilters {
        game_type: GameType::MLP,
        speeds: Vec::new(),
        rated: None,
        min_game_length: None,
    }
}

async fn seed_game(
    w: &str,
    b: &str,
    hashes: &[u64],
    moves: &[(&str, &str)],
    result: &str,
    conn: &mut db_lib::DbConn<'_>,
) -> Game {
    let (game, _, _) = setup_game_named(w, b, conn).await;
    let owned: Vec<(String, String)> = moves
        .iter()
        .map(|(p, q)| (p.to_string(), q.to_string()))
        .collect();
    let mut ctx = test_ctx(Some(1500.0), Some(1500.0));
    ctx.result = result.to_string();
    let entries = GameHash::from_engine_hashes(game.id, hashes, &owned, &ctx);
    GameHash::insert_batch(&entries, conn).await.unwrap();
    game
}

#[tokio::test(flavor = "multi_thread")]
async fn next_moves_returns_continuations_with_results() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    // Position 100 -> (move bA1) -> 200 -> (move wQ) -> 300, white wins.
    seed_game(
        "nm1",
        "nm2",
        &[100, 200, 300],
        &[("wA1", ""), ("bA1", "wA1-"), ("wQ", "\\wA1")],
        "Finished(1-0)",
        &mut conn,
    )
    .await;

    let moves = GameHash::next_moves(100, &nm_filters(), None, &mut conn)
        .await
        .unwrap();
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].next_hash, 200);
    assert_eq!(moves[0].piece, "bA1");
    assert_eq!(moves[0].position, "wA1-");
    assert_eq!(moves[0].total, 1);
    assert_eq!(moves[0].white_wins, 1);
    assert_eq!(moves[0].black_wins, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn next_moves_merges_transpositions_by_next_hash() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    // Two games reach the same resulting hash 999 from 100, but the move is spelled
    // differently (rotational frame). Grouping by next hash must merge them.
    seed_game(
        "ta1",
        "ta2",
        &[100, 999],
        &[("wA1", ""), ("bA1", "wA1-")],
        "Finished(1-0)",
        &mut conn,
    )
    .await;
    seed_game(
        "tb1",
        "tb2",
        &[100, 999],
        &[("wA1", ""), ("bA1", "-wA1")],
        "Finished(0-1)",
        &mut conn,
    )
    .await;

    let moves = GameHash::next_moves(100, &nm_filters(), None, &mut conn)
        .await
        .unwrap();
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].next_hash, 999);
    assert_eq!(moves[0].total, 2);
    assert_eq!(moves[0].white_wins, 1);
    assert_eq!(moves[0].black_wins, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn next_moves_counts_game_once_per_continuation() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    // One game revisits position 100 (turns 0 and 2) and both times plays into 200.
    // Distinct-game counting must yield total = 1, not 2.
    seed_game(
        "rep1",
        "rep2",
        &[100, 200, 100, 200],
        &[("wA1", ""), ("bA1", "wA1-"), ("wB1", "x"), ("bA1", "wA1-")],
        "Finished(1-0)",
        &mut conn,
    )
    .await;

    let moves = GameHash::next_moves(100, &nm_filters(), None, &mut conn)
        .await
        .unwrap();
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].next_hash, 200);
    assert_eq!(moves[0].total, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn aggregate_one_counts_results_for_a_position() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    seed_game(
        "ag1",
        "ag2",
        &[100, 555],
        &[("wA1", ""), ("bA1", "wA1-")],
        "Finished(1-0)",
        &mut conn,
    )
    .await;
    seed_game(
        "ag3",
        "ag4",
        &[100, 556],
        &[("wA1", ""), ("bA1", "-wA1")],
        "Finished(½-½)",
        &mut conn,
    )
    .await;

    let stats = GameHash::aggregate_one(100, &nm_filters(), &mut conn)
        .await
        .unwrap();
    assert_eq!(stats.next_hash, 100);
    assert_eq!(stats.total, 2);
    assert_eq!(stats.white_wins, 1);
    assert_eq!(stats.draws, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn next_moves_filters_out_short_games() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    // game_length = 2; min_game_length = Some(8) must exclude it.
    seed_game(
        "sh1",
        "sh2",
        &[100, 200],
        &[("wA1", ""), ("bA1", "wA1-")],
        "Finished(1-0)",
        &mut conn,
    )
    .await;

    let mut filters = nm_filters();
    filters.min_game_length = Some(8);
    let moves = GameHash::next_moves(100, &filters, None, &mut conn)
        .await
        .unwrap();
    assert!(moves.is_empty());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup_game(conn: &mut db_lib::DbConn<'_>) -> (Game, User, User) {
    setup_game_with_history("alice", "bob", "", conn).await
}

async fn setup_game_named(w: &str, b: &str, conn: &mut db_lib::DbConn<'_>) -> (Game, User, User) {
    setup_game_with_history(w, b, "", conn).await
}

async fn setup_game_with_history(
    white_name: &str,
    black_name: &str,
    history: &str,
    conn: &mut db_lib::DbConn<'_>,
) -> (Game, User, User) {
    let white = User::create(
        NewUser::new(white_name, "password", &format!("{white_name}@test.com")).unwrap(),
        conn,
    )
    .await
    .unwrap();
    let black = User::create(
        NewUser::new(black_name, "password", &format!("{black_name}@test.com")).unwrap(),
        conn,
    )
    .await
    .unwrap();

    let now = Utc::now();
    let time_left = Some(60_000_000_000_i64);
    let turn = history.split_terminator(';').count() as i32;
    let timeout_at = if turn > 0 {
        time_left.map(|nanos| now + chrono::Duration::nanoseconds(nanos))
    } else {
        None
    };

    let game = Game::create(
        NewGame {
            nanoid: nanoid::nanoid!(12),
            current_player_id: if turn % 2 == 0 { white.id } else { black.id },
            black_id: black.id,
            finished: false,
            game_status: if turn > 0 {
                GameStatus::InProgress.to_string()
            } else {
                GameStatus::NotStarted.to_string()
            },
            game_type: GameType::MLP.to_string(),
            history: history.to_string(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: false,
            turn,
            white_id: white.id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: if turn > 0 { Some(now) } else { None },
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at,
        },
        conn,
    )
    .await
    .unwrap();

    (game, white, black)
}

// ---------------------------------------------------------------------------
// Integration tests — the backfill pass as the boot job runs it
// ---------------------------------------------------------------------------

/// Mirrors `apis/src/jobs/hash_backfill.rs`: cursor batches, one txn per game, failures skipped.
async fn run_backfill_pass(pool: &db_lib::DbPool, batch_size: i64) -> u64 {
    use diesel_async::AsyncConnection;
    let mut conn = get_conn(pool).await.unwrap();
    let mut last_id = None;
    let mut total = 0u64;
    loop {
        let batch = Game::find_needing_hash_backfill(last_id, batch_size, &mut conn)
            .await
            .unwrap();
        if batch.is_empty() {
            break;
        }
        for game in &batch {
            last_id = Some(game.id);
            let Ok(state) = State::new_from_str(&game.history, &game.game_type) else {
                continue;
            };
            let new_hashes: Vec<Option<i64>> =
                state.hashes.iter().map(|h| Some(*h as i64)).collect();
            let ctx = GameFinishContext::from_finished_game(game);
            conn.transaction(async |conn| {
                Game::set_hashes(game.id, new_hashes, conn).await?;
                GameHash::insert_for_game(game.id, &state.hashes, &state.history.moves, &ctx, conn)
                    .await?;
                Ok::<_, db_lib::db_error::DbError>(())
            })
            .await
            .unwrap();
            total += 1;
        }
    }
    total
}

async fn finish(game: &Game, conn: &mut db_lib::DbConn<'_>) {
    diesel::update(games::table.find(game.id))
        .set(games::finished.eq(true))
        .execute(conn)
        .await
        .unwrap();
}

async fn all_hash_rows(conn: &mut db_lib::DbConn<'_>) -> Vec<(uuid::Uuid, i32, i64)> {
    let mut rows: Vec<(uuid::Uuid, i32, i64)> = gh_schema::table
        .select((gh_schema::game_id, gh_schema::turn, gh_schema::hash))
        .load(conn)
        .await
        .unwrap();
    rows.sort();
    rows
}

#[tokio::test(flavor = "multi_thread")]
async fn backfill_pass_is_idempotent() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    for (w, b) in [("ip1", "ip2"), ("ip3", "ip4"), ("ip5", "ip6")] {
        let (game, _, _) = setup_game_with_history(w, b, "wQ;bQ /wQ;wA1 wQ-;", &mut conn).await;
        finish(&game, &mut conn).await;
    }
    assert_eq!(run_backfill_pass(&db.pool, 2).await, 3);
    let after_first = all_hash_rows(&mut conn).await;
    assert!(!after_first.is_empty());

    assert_eq!(
        run_backfill_pass(&db.pool, 2).await,
        0,
        "nothing left to select"
    );
    assert_eq!(
        all_hash_rows(&mut conn).await,
        after_first,
        "rows unchanged"
    );
}

/// "Empty hashes" is the queue and the cursor is never persisted, so a fresh pass just resumes.
#[tokio::test(flavor = "multi_thread")]
async fn backfill_resumes_after_an_interrupted_pass() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    for (w, b) in [
        ("rp1", "rp2"),
        ("rp3", "rp4"),
        ("rp5", "rp6"),
        ("rp7", "rp8"),
    ] {
        let (game, _, _) = setup_game_with_history(w, b, "wQ;bQ /wQ;", &mut conn).await;
        finish(&game, &mut conn).await;
    }
    // The "crash": process exactly one batch of two, then stop.
    let batch = Game::find_needing_hash_backfill(None, 2, &mut conn)
        .await
        .unwrap();
    assert_eq!(batch.len(), 2);
    for game in &batch {
        let state = State::new_from_str(&game.history, &game.game_type).unwrap();
        let new_hashes: Vec<Option<i64>> = state.hashes.iter().map(|h| Some(*h as i64)).collect();
        Game::set_hashes(game.id, new_hashes, &mut conn)
            .await
            .unwrap();
    }
    let outstanding = Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap();
    assert_eq!(outstanding.len(), 2, "half the queue survives the crash");

    assert_eq!(
        run_backfill_pass(&db.pool, 2).await,
        2,
        "the next boot drains the rest"
    );
    assert!(Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap()
        .is_empty());
}

/// A record that continued past a threefold (grandfathered under the old rule) must still
/// backfill - replay may not re-referee it.
#[tokio::test(flavor = "multi_thread")]
async fn backfill_handles_games_recorded_past_a_repetition() {
    use hive_lib::{Direction, Piece};
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();

    // Two out-and-home cycles return to the ply-4 position twice (threefold at ply 12),
    // then one move past it.
    let mut record = State::new(GameType::MLP, false);
    record.set_replaying(true);
    for (piece, position) in [("wQ", ""), ("bQ", "-wQ"), ("wA1", "wQ-"), ("bA1", "-bQ")] {
        record.play_turn_from_history(piece, position).unwrap();
    }
    let wa: Piece = "wA1".parse().unwrap();
    let ba: Piece = "bA1".parse().unwrap();
    let wa_home = record.board.position_of_piece(wa).unwrap();
    let ba_home = record.board.position_of_piece(ba).unwrap();
    let wa_away = record
        .board
        .position_of_piece("wQ".parse().unwrap())
        .unwrap()
        .to(Direction::NE);
    let ba_away = record
        .board
        .position_of_piece("bQ".parse().unwrap())
        .unwrap()
        .to(Direction::SW);
    for _ in 0..2 {
        for (piece, target) in [(wa, wa_away), (ba, ba_away), (wa, wa_home), (ba, ba_home)] {
            record.play_turn_from_position(piece, target).unwrap();
        }
    }
    record.play_turn_from_position(wa, wa_away).unwrap();
    assert!(
        !record.repeating_moves.is_empty(),
        "the fixture really repeats"
    );

    let history = record
        .history
        .moves
        .iter()
        .map(|(piece, position)| {
            if position.is_empty() {
                piece.clone()
            } else {
                format!("{piece} {position}")
            }
        })
        .collect::<Vec<_>>()
        .join(";")
        + ";";
    let (game, _, _) = setup_game_with_history("gp1", "gp2", &history, &mut conn).await;
    finish(&game, &mut conn).await;

    assert_eq!(run_backfill_pass(&db.pool, 10).await, 1);
    let reloaded: Game = games::table.find(game.id).first(&mut conn).await.unwrap();
    assert_eq!(
        reloaded.hashes(),
        record.hashes,
        "hashes equal the record's own"
    );
}

/// An unreplayable game stays in the queue; the final recount is what stops "done" from lying.
#[tokio::test(flavor = "multi_thread")]
async fn backfill_skips_unreplayable_games_and_leaves_them_outstanding() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let (good, _, _) = setup_game_with_history("sk1", "sk2", "wQ;bQ /wQ;", &mut conn).await;
    finish(&good, &mut conn).await;
    let (bad, _, _) = setup_game_with_history("sk3", "sk4", "wQ;INVALID;", &mut conn).await;
    finish(&bad, &mut conn).await;

    assert_eq!(
        run_backfill_pass(&db.pool, 10).await,
        1,
        "only the good game processes"
    );
    let outstanding = Game::find_needing_hash_backfill(None, 100, &mut conn)
        .await
        .unwrap();
    assert_eq!(outstanding.len(), 1);
    assert_eq!(outstanding[0].id, bad.id, "the bad game stays in the queue");
    let bad_rows: i64 = gh_schema::table
        .filter(gh_schema::game_id.eq(bad.id))
        .count()
        .get_result(&mut conn)
        .await
        .unwrap();
    assert_eq!(bad_rows, 0, "no partial rows for the skipped game");
}
