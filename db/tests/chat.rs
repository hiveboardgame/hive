mod common;

use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    helpers::{
        chat_inbox_unread_states,
        get_dm_conversations_for_user,
        get_game_channels_for_user,
        get_tournament_channels_for_user,
        get_tournament_thread_data,
        insert_chat_message,
        insert_chat_message_and_mark_sender_read,
        is_tournament_chat_muted,
        load_chat_history,
        mark_chat_read,
        resolve_chat_target,
        set_tournament_chat_muted,
        unread_chat_count_for_channel,
    },
    models::{Game, NewGame, NewTournament, NewUser, Tournament, User},
    schema::{chat_channels, users},
    DbConn,
};
use diesel::prelude::*;
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use hive_lib::{GameStatus, GameType};
use shared_types::{
    Conclusion,
    ConversationKey,
    GameId,
    GameSpeed,
    GameStart,
    GameThread,
    ScoringMode,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentGameResult,
    TournamentId,
    TournamentMode,
    TournamentStatus,
};
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn tournament_chat_boundaries_authorize_members_and_reject_outsiders() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("chat_boundary_org", &mut conn).await;
    let participant = create_user("chat_boundary_part", &mut conn).await;
    let admin = create_user("chat_boundary_admin", &mut conn).await;
    let outsider = create_user("chat_boundary_out", &mut conn).await;
    diesel::update(users::table.find(admin.id))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .expect("mark site admin");
    let tournament = create_tournament(organizer.id, "Chat boundaries", &mut conn).await;
    tournament
        .join(&participant.id, &mut conn)
        .await
        .expect("join participant");
    let tournament_id = TournamentId(tournament.nanoid.clone());
    let key = ConversationKey::tournament(&tournament_id);

    for user_id in [organizer.id, participant.id, admin.id] {
        resolve_chat_target(&mut conn, Some(user_id), &key)
            .await
            .expect("authorized user resolves tournament chat");
        assert_eq!(
            get_tournament_thread_data(&mut conn, user_id, &tournament.nanoid)
                .await
                .expect("authorized user resolves route"),
            tournament.name,
        );
    }

    let participant_target = resolve_chat_target(&mut conn, Some(participant.id), &key)
        .await
        .expect("resolve participant tournament target");
    insert_chat_message(
        &mut conn,
        participant.id,
        Uuid::new_v4(),
        &participant_target,
        "authorized tournament send",
        None,
    )
    .await
    .expect("participant sends tournament chat");
    set_tournament_chat_muted(&mut conn, participant.id, &tournament.nanoid, true)
        .await
        .expect("participant mutes tournament chat");
    assert!(
        is_tournament_chat_muted(&mut conn, participant.id, tournament.id,)
            .await
            .expect("load participant mute state")
    );
    set_tournament_chat_muted(&mut conn, participant.id, &tournament.nanoid, false)
        .await
        .expect("participant unmutes tournament chat");

    assert!(get_tournament_channels_for_user(&mut conn, admin.id)
        .await
        .expect("load admin hub tournaments")
        .is_empty());
    assert!(chat_inbox_unread_states(&mut conn, admin.id)
        .await
        .expect("load admin inbox")
        .is_empty());

    assert!(matches!(
        resolve_chat_target(&mut conn, Some(outsider.id), &key).await,
        Err(DbError::Unauthorized)
    ));
    assert!(matches!(
        get_tournament_thread_data(&mut conn, outsider.id, &tournament.nanoid).await,
        Err(DbError::Unauthorized)
    ));
    assert!(matches!(
        set_tournament_chat_muted(&mut conn, outsider.id, &tournament.nanoid, true).await,
        Err(DbError::Unauthorized)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_first_sends_create_one_channel() {
    let db = common::db::test_db().await;
    let mut setup_conn = get_conn(&db.pool).await.expect("get setup connection");
    let first = create_user("first_chan_one", &mut setup_conn).await;
    let second = create_user("first_chan_two", &mut setup_conn).await;
    let first_target =
        resolve_chat_target(&mut setup_conn, Some(first.id), &ConversationKey::Global)
            .await
            .expect("resolve first target");
    let second_target =
        resolve_chat_target(&mut setup_conn, Some(second.id), &ConversationKey::Global)
            .await
            .expect("resolve second target");
    drop(setup_conn);

    let mut first_conn = get_conn(&db.pool).await.expect("get first connection");
    let mut second_conn = get_conn(&db.pool).await.expect("get second connection");
    let (first_result, second_result) = tokio::join!(
        insert_chat_message(
            &mut first_conn,
            first.id,
            Uuid::new_v4(),
            &first_target,
            "first concurrent message",
            None,
        ),
        insert_chat_message(
            &mut second_conn,
            second.id,
            Uuid::new_v4(),
            &second_target,
            "second concurrent message",
            None,
        ),
    );
    first_result.expect("persist first concurrent message");
    second_result.expect("persist second concurrent message");

    let count = chat_channels::table
        .filter(chat_channels::kind.eq("global"))
        .count()
        .get_result::<i64>(&mut first_conn)
        .await
        .expect("count global channels");
    assert_eq!(count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn player_threads_are_created_lazily_and_spectators_are_excluded_from_messages_hub() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let white = create_user("spectator_hub_white", &mut conn).await;
    let black = create_user("spectator_hub_black", &mut conn).await;
    let spectator = create_user("spectator_hub_reader", &mut conn).await;
    let game = create_game(white.id, black.id, &mut conn).await;
    let game_id = GameId(game.nanoid.clone());
    let player_key = ConversationKey::game(&game_id, GameThread::Players);
    let player_target = resolve_chat_target(&mut conn, Some(white.id), &player_key)
        .await
        .expect("resolve player target without creating channel");
    assert_eq!(player_target.channel_id(), None);
    let (player_message, _) = insert_chat_message_and_mark_sender_read(
        &mut conn,
        white.id,
        Uuid::new_v4(),
        &player_target,
        "player message",
        None,
    )
    .await
    .expect("persist player message and sender receipt");
    let player_target = resolve_chat_target(&mut conn, Some(white.id), &player_key)
        .await
        .expect("re-resolve created player target");
    assert_eq!(player_target.channel_id(), Some(player_message.channel_id));

    let key = ConversationKey::game(&game_id, GameThread::Spectators);

    let target = resolve_chat_target(&mut conn, Some(spectator.id), &key)
        .await
        .expect("resolve spectator target");
    resolve_chat_target(&mut conn, None, &key)
        .await
        .expect("resolve anonymous spectator target");
    insert_chat_message(
        &mut conn,
        spectator.id,
        Uuid::new_v4(),
        &target,
        "spectator message",
        None,
    )
    .await
    .expect("persist spectator message");

    assert!(get_game_channels_for_user(&mut conn, spectator.id)
        .await
        .expect("load spectator hub games")
        .is_empty());
    assert!(chat_inbox_unread_states(&mut conn, spectator.id)
        .await
        .expect("load spectator inbox")
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn dm_catalog_keeps_recent_fifty_plus_older_unread_until_read() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let viewer = create_user("cat_window_view", &mut conn).await;
    let mut conversations = Vec::new();

    for index in 0..51 {
        let peer = create_user(&format!("cat_window_peer_{index}"), &mut conn).await;
        let peer_target = resolve_chat_target(
            &mut conn,
            Some(peer.id),
            &ConversationKey::direct(viewer.id),
        )
        .await
        .expect("resolve peer direct target");
        let (message, _) = insert_chat_message(
            &mut conn,
            peer.id,
            Uuid::new_v4(),
            &peer_target,
            &format!("catalog message {index}"),
            None,
        )
        .await
        .expect("persist catalog message");
        let viewer_target = resolve_chat_target(
            &mut conn,
            Some(viewer.id),
            &ConversationKey::direct(peer.id),
        )
        .await
        .expect("resolve viewer direct target");
        assert_eq!(viewer_target.channel_id(), Some(message.channel_id));
        conversations.push((peer, viewer_target, message.id));
    }

    for (_, target, message_id) in conversations.iter().skip(1) {
        mark_chat_read(&mut conn, viewer.id, target, *message_id)
            .await
            .expect("mark recent conversation read");
    }
    diesel::update(users::table.find(conversations[0].0.id))
        .set(users::deleted.eq(true))
        .execute(&mut conn)
        .await
        .expect("soft-delete oldest peer");
    resolve_chat_target(
        &mut conn,
        Some(viewer.id),
        &ConversationKey::direct(conversations[0].0.id),
    )
    .await
    .expect("resolve existing conversation with deleted peer");

    let catalog = get_dm_conversations_for_user(&mut conn, viewer.id)
        .await
        .expect("load catalog with unread overflow");
    assert_eq!(catalog.len(), 51);
    assert!(catalog
        .windows(2)
        .all(|rows| rows[0].last_message_id > rows[1].last_message_id));
    assert!(
        catalog
            .iter()
            .find(|row| row.other_user_id == conversations[0].0.id)
            .expect("old unread row remains")
            .peer_deleted
    );

    mark_chat_read(
        &mut conn,
        viewer.id,
        &conversations[0].1,
        conversations[0].2,
    )
    .await
    .expect("mark overflow conversation read");
    let catalog = get_dm_conversations_for_user(&mut conn, viewer.id)
        .await
        .expect("reload bounded catalog");
    assert_eq!(catalog.len(), 50);
    assert!(catalog
        .iter()
        .all(|row| row.other_user_id != conversations[0].0.id));
}

#[tokio::test(flavor = "multi_thread")]
async fn muted_tournament_is_kept_when_recent_but_excluded_from_unread_overflow() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("cat_mute_org", &mut conn).await;
    let participant = create_user("cat_mute_part", &mut conn).await;
    let mut tournaments = Vec::new();

    for index in 0..51 {
        let tournament = create_tournament(
            organizer.id,
            &format!("Catalog mute tournament {index}"),
            &mut conn,
        )
        .await;
        tournament
            .join(&participant.id, &mut conn)
            .await
            .expect("join catalog tournament");
        let key = ConversationKey::tournament(&TournamentId(tournament.nanoid.clone()));
        let target = resolve_chat_target(&mut conn, Some(organizer.id), &key)
            .await
            .expect("resolve tournament target");
        insert_chat_message(
            &mut conn,
            organizer.id,
            Uuid::new_v4(),
            &target,
            &format!("catalog tournament message {index}"),
            None,
        )
        .await
        .expect("persist tournament catalog message");
        tournaments.push(tournament);
    }

    set_tournament_chat_muted(&mut conn, participant.id, &tournaments[0].nanoid, true)
        .await
        .expect("mute oldest tournament");
    let catalog = get_tournament_channels_for_user(&mut conn, participant.id)
        .await
        .expect("load catalog without muted overflow");
    assert_eq!(catalog.len(), 50);
    assert!(catalog
        .iter()
        .all(|row| row.tournament_id.0 != tournaments[0].nanoid));

    set_tournament_chat_muted(&mut conn, participant.id, &tournaments[50].nanoid, true)
        .await
        .expect("mute newest tournament");
    let catalog = get_tournament_channels_for_user(&mut conn, participant.id)
        .await
        .expect("load catalog with recent muted tournament");
    assert!(catalog
        .iter()
        .any(|row| row.tournament_id.0 == tournaments[50].nanoid));
    let muted_key = ConversationKey::tournament(&TournamentId(tournaments[50].nanoid.clone()));
    let unread = chat_inbox_unread_states(&mut conn, participant.id)
        .await
        .expect("load unread states with recent muted tournament");
    assert!(unread.iter().all(|state| state.key != muted_key));
}

#[tokio::test(flavor = "multi_thread")]
async fn limited_history_returns_the_newest_messages_in_ascending_order() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let sender = create_user("history_sender", &mut conn).await;
    let peer = create_user("history_peer", &mut conn).await;
    let key = ConversationKey::direct(peer.id);
    let initial_target = resolve_chat_target(&mut conn, Some(sender.id), &key)
        .await
        .expect("resolve direct target");
    insert_chat_message(
        &mut conn,
        sender.id,
        Uuid::new_v4(),
        &initial_target,
        "message 0",
        None,
    )
    .await
    .expect("persist initial message");
    let target = resolve_chat_target(&mut conn, Some(sender.id), &key)
        .await
        .expect("resolve existing direct target");
    assert!(target.channel_id().is_some());
    for index in 1..60 {
        insert_chat_message(
            &mut conn,
            sender.id,
            Uuid::new_v4(),
            &target,
            &format!("message {index}"),
            None,
        )
        .await
        .expect("persist history message");
    }

    let newest_page = load_chat_history(&mut conn, &target, None, 50)
        .await
        .expect("load limited history");

    assert_eq!(newest_page.messages.len(), 50);
    assert_eq!(
        newest_page.next_before_message_id,
        newest_page
            .messages
            .as_slice()
            .first()
            .map(|message| message.id)
    );
    assert_eq!(
        newest_page
            .messages
            .as_slice()
            .first()
            .map(|message| message.message.as_str()),
        Some("message 10")
    );
    assert_eq!(
        newest_page
            .messages
            .as_slice()
            .last()
            .map(|message| message.message.as_str()),
        Some("message 59")
    );
    assert!(newest_page
        .messages
        .windows(2)
        .all(|window| window[0].id < window[1].id));

    let older_page = load_chat_history(&mut conn, &target, newest_page.next_before_message_id, 50)
        .await
        .expect("load older history");

    assert_eq!(older_page.messages.len(), 10);
    assert_eq!(older_page.next_before_message_id, None);
    assert!(older_page
        .messages
        .windows(2)
        .all(|window| window[0].id < window[1].id));
    assert!(
        older_page
            .messages
            .as_slice()
            .last()
            .map(|message| message.id)
            < newest_page
                .messages
                .as_slice()
                .first()
                .map(|message| message.id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn identical_client_id_retry_returns_the_existing_message() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let sender = create_user("idempotent_sender", &mut conn).await;
    let peer = create_user("idempotent_peer", &mut conn).await;
    let key = ConversationKey::direct(peer.id);
    let target = resolve_chat_target(&mut conn, Some(sender.id), &key)
        .await
        .expect("resolve direct target");
    let client_id = Uuid::new_v4();

    let (first, first_inserted) = insert_chat_message(
        &mut conn,
        sender.id,
        client_id,
        &target,
        "idempotent message",
        Some(7),
    )
    .await
    .expect("persist initial message");
    let (retry, retry_inserted) = insert_chat_message(
        &mut conn,
        sender.id,
        client_id,
        &target,
        "idempotent message",
        Some(7),
    )
    .await
    .expect("return existing message for identical retry");

    assert_eq!(retry.id, first.id);
    assert_eq!(retry.created_at, first.created_at);
    assert_eq!(retry.client_id, client_id);
    assert!(first_inserted);
    assert!(!retry_inserted);
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_same_client_id_retry_inserts_one_message() {
    let db = common::db::test_db().await;
    let mut setup_conn = get_conn(&db.pool).await.expect("get setup connection");
    let sender = create_user("concurrent_sender", &mut setup_conn).await;
    let initial_target =
        resolve_chat_target(&mut setup_conn, Some(sender.id), &ConversationKey::Global)
            .await
            .expect("resolve global target");
    insert_chat_message(
        &mut setup_conn,
        sender.id,
        Uuid::new_v4(),
        &initial_target,
        "channel seed",
        None,
    )
    .await
    .expect("seed the global channel");
    let target = resolve_chat_target(&mut setup_conn, Some(sender.id), &ConversationKey::Global)
        .await
        .expect("resolve seeded global target");
    assert!(target.channel_id().is_some());
    let mut first_conn = get_conn(&db.pool).await.expect("get first connection");
    let mut retry_conn = get_conn(&db.pool).await.expect("get retry connection");
    let client_id = Uuid::new_v4();
    let first_target = target.clone();
    let retry_target = target.clone();

    let (first, retry) = tokio::join!(
        insert_chat_message(
            &mut first_conn,
            sender.id,
            client_id,
            &first_target,
            "concurrent idempotent message",
            None,
        ),
        insert_chat_message(
            &mut retry_conn,
            sender.id,
            client_id,
            &retry_target,
            "concurrent idempotent message",
            None,
        ),
    );
    let (first, first_inserted) = first.expect("persist first concurrent attempt");
    let (retry, retry_inserted) = retry.expect("persist concurrent retry");

    assert_eq!(first.id, retry.id);
    assert_ne!(first_inserted, retry_inserted);
}

#[tokio::test(flavor = "multi_thread")]
async fn non_receipt_message_insert_does_not_wait_for_channel_row_lock() {
    let db = common::db::test_db().await;
    let mut setup_conn = get_conn(&db.pool).await.expect("get setup connection");
    let sender = create_user("unlocked_global_send", &mut setup_conn).await;
    let empty_target =
        resolve_chat_target(&mut setup_conn, Some(sender.id), &ConversationKey::Global)
            .await
            .expect("resolve empty global target");
    insert_chat_message(
        &mut setup_conn,
        sender.id,
        Uuid::new_v4(),
        &empty_target,
        "seed global channel",
        None,
    )
    .await
    .expect("seed global channel");
    let target = resolve_chat_target(&mut setup_conn, Some(sender.id), &ConversationKey::Global)
        .await
        .expect("resolve global channel");
    let channel_id = target.channel_id().expect("global channel exists");
    drop(setup_conn);

    let mut locking_conn = get_conn(&db.pool).await.expect("get locking connection");
    let mut inserting_conn = get_conn(&db.pool).await.expect("get inserting connection");
    locking_conn
        .transaction::<_, DbError, _>(move |conn| {
            async move {
                chat_channels::table
                    .find(channel_id)
                    .for_no_key_update()
                    .select(chat_channels::id)
                    .first::<i64>(conn)
                    .await
                    .map_err(DbError::from)?;
                inserting_conn
                    .transaction::<_, DbError, _>(|conn| {
                        async move {
                            diesel::sql_query("SET LOCAL lock_timeout = '100ms'")
                                .execute(conn)
                                .await
                                .map_err(DbError::from)?;
                            insert_chat_message(
                                conn,
                                sender.id,
                                Uuid::new_v4(),
                                &target,
                                "lock-free global message",
                                None,
                            )
                            .await?;
                            Ok(())
                        }
                        .scope_boxed()
                    })
                    .await?;
                Ok(())
            }
            .scope_boxed()
        })
        .await
        .expect("hold channel row lock while inserting message");
}

#[tokio::test(flavor = "multi_thread")]
async fn reused_client_id_with_different_message_fields_conflicts() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let sender = create_user("conflict_sender", &mut conn).await;
    let first_peer = create_user("conflict_first_peer", &mut conn).await;
    let second_peer = create_user("conflict_second_peer", &mut conn).await;
    let first_target = resolve_chat_target(
        &mut conn,
        Some(sender.id),
        &ConversationKey::direct(first_peer.id),
    )
    .await
    .expect("resolve first direct target");
    let second_target = resolve_chat_target(
        &mut conn,
        Some(sender.id),
        &ConversationKey::direct(second_peer.id),
    )
    .await
    .expect("resolve second direct target");
    let client_id = Uuid::new_v4();

    insert_chat_message(
        &mut conn,
        sender.id,
        client_id,
        &first_target,
        "original message",
        Some(4),
    )
    .await
    .expect("persist original message");

    for (target, body, turn) in [
        (&first_target, "different body", Some(4)),
        (&first_target, "original message", Some(5)),
        (&second_target, "original message", Some(4)),
    ] {
        let result = insert_chat_message(&mut conn, sender.id, client_id, target, body, turn).await;
        assert!(matches!(result, Err(DbError::ChatClientIdConflict)));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn persisted_sender_message_advances_the_read_receipt_atomically() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let sender = create_user("receipt_sender", &mut conn).await;
    let peer = create_user("receipt_peer", &mut conn).await;
    let peer_key = ConversationKey::direct(sender.id);
    let peer_target = resolve_chat_target(&mut conn, Some(peer.id), &peer_key)
        .await
        .expect("resolve peer target");
    insert_chat_message(
        &mut conn,
        peer.id,
        Uuid::new_v4(),
        &peer_target,
        "unread message",
        None,
    )
    .await
    .expect("persist unread message");
    let sender_key = ConversationKey::direct(peer.id);
    let sender_target = resolve_chat_target(&mut conn, Some(sender.id), &sender_key)
        .await
        .expect("resolve sender target");
    let channel_id = sender_target.channel_id().expect("existing direct channel");
    assert_eq!(
        unread_chat_count_for_channel(&mut conn, sender.id, channel_id)
            .await
            .expect("count unread before send"),
        1
    );

    let sender_client_id = Uuid::new_v4();
    let (sender_message, sender_inserted) = insert_chat_message_and_mark_sender_read(
        &mut conn,
        sender.id,
        sender_client_id,
        &sender_target,
        "sender reply",
        None,
    )
    .await
    .expect("persist sender reply");
    assert!(sender_inserted);

    assert_eq!(
        unread_chat_count_for_channel(&mut conn, sender.id, channel_id)
            .await
            .expect("count unread after send"),
        0
    );

    insert_chat_message(
        &mut conn,
        peer.id,
        Uuid::new_v4(),
        &peer_target,
        "newer unread message",
        None,
    )
    .await
    .expect("persist newer unread message");
    assert_eq!(
        unread_chat_count_for_channel(&mut conn, sender.id, channel_id)
            .await
            .expect("count unread before idempotent retry"),
        1
    );

    let (retry, retry_inserted) = insert_chat_message_and_mark_sender_read(
        &mut conn,
        sender.id,
        sender_client_id,
        &sender_target,
        "sender reply",
        None,
    )
    .await
    .expect("retry sender reply");
    assert_eq!(retry.id, sender_message.id);
    assert!(!retry_inserted);
    assert_eq!(
        unread_chat_count_for_channel(&mut conn, sender.id, channel_id)
            .await
            .expect("count unread after idempotent retry"),
        1
    );

    let conflict = insert_chat_message_and_mark_sender_read(
        &mut conn,
        sender.id,
        sender_client_id,
        &sender_target,
        "conflicting sender reply",
        None,
    )
    .await;
    assert!(matches!(conflict, Err(DbError::ChatClientIdConflict)));
    assert_eq!(
        unread_chat_count_for_channel(&mut conn, sender.id, channel_id)
            .await
            .expect("count unread after conflicting retry"),
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn deleted_senders_cannot_persist_chat_messages() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let sender = create_user("deleted_chat_sender", &mut conn).await;
    let peer = create_user("deleted_chat_peer", &mut conn).await;
    let target = resolve_chat_target(
        &mut conn,
        Some(sender.id),
        &ConversationKey::direct(peer.id),
    )
    .await
    .expect("resolve direct target before deletion");
    diesel::update(users::table.find(sender.id))
        .set(users::deleted.eq(true))
        .execute(&mut conn)
        .await
        .expect("mark sender deleted");

    assert!(insert_chat_message(
        &mut conn,
        sender.id,
        Uuid::new_v4(),
        &target,
        "untracked send after deletion",
        None,
    )
    .await
    .is_err());
    assert!(insert_chat_message_and_mark_sender_read(
        &mut conn,
        sender.id,
        Uuid::new_v4(),
        &target,
        "tracked send after deletion",
        None,
    )
    .await
    .is_err());
}

async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    User::create(
        NewUser::new(username, "password", &format!("{username}@example.test"))
            .expect("build user"),
        conn,
    )
    .await
    .expect("insert user")
}

async fn create_tournament(organizer_id: Uuid, name: &str, conn: &mut DbConn<'_>) -> Tournament {
    Tournament::create(
        organizer_id,
        &NewTournament {
            nanoid: nanoid::nanoid!(11),
            name: name.to_string(),
            description: String::new(),
            scoring: ScoringMode::Game.to_string(),
            tiebreaker: vec![Some(Tiebreaker::RawPoints.to_string())],
            seats: 4,
            min_seats: 2,
            rounds: 1,
            invite_only: false,
            mode: TournamentMode::DoubleRoundRobin.to_string(),
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual.to_string(),
            starts_at: None,
            ends_at: None,
            started_at: None,
            round_duration: None,
            status: TournamentStatus::NotStarted.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: None,
        },
        conn,
    )
    .await
    .expect("insert tournament")
}

async fn create_game(white_id: Uuid, black_id: Uuid, conn: &mut DbConn<'_>) -> Game {
    let now = Utc::now();
    let time_left = Some(60_000_000_000_i64);
    Game::create(
        NewGame {
            nanoid: nanoid::nanoid!(12),
            current_player_id: white_id,
            black_id,
            finished: false,
            game_status: GameStatus::NotStarted.to_string(),
            game_type: GameType::MLP.to_string(),
            history: String::new(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: false,
            turn: 0,
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: None,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at: None,
        },
        conn,
    )
    .await
    .expect("insert game")
}
