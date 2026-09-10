mod common;

use common::tournament::{create_rr_tournament, create_user};
use db_lib::{
    db_error::DbError,
    get_conn,
    helpers::run_serializable,
    models::{Tournament, TournamentOrganizerInvitation, TournamentUser},
};
use shared_types::{TournamentId, TournamentStatus};
use std::sync::Arc;
use tokio::sync::Barrier;

#[tokio::test(flavor = "multi_thread")]
async fn organizer_invitations_require_acceptance_and_leave_player_membership_independent() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let first = create_user("org_first", &mut conn).await;
    let second = create_user("org_second", &mut conn).await;
    let third = create_user("org_third", &mut conn).await;
    let tournament = create_rr_tournament(
        first.id,
        "Organizers",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    let id = TournamentId(tournament.nanoid.clone());
    run_serializable(&mut conn, move |tc| {
        let id = id.clone();
        Box::pin(async move {
            let tournament = Tournament::find_by_tournament_id_for_update(&id, tc).await?;
            tournament.join(&second.id, tc).await?;
            assert!(tournament.invite_organizer(first.id, second.id, tc).await?);
            assert!(!tournament.invite_organizer(first.id, second.id, tc).await?);
            assert!(matches!(
                tournament
                    .ensure_user_is_organizer_or_admin(&second.id, tc)
                    .await,
                Err(DbError::Unauthorized)
            ));
            assert!(matches!(
                tournament.invite_organizer(second.id, third.id, tc).await,
                Err(DbError::Unauthorized)
            ));
            assert!(tournament
                .accept_organizer_invitation(third.id, tc)
                .await
                .is_err());
            tournament
                .accept_organizer_invitation(second.id, tc)
                .await?;
            tournament
                .ensure_user_is_organizer_or_admin(&second.id, tc)
                .await?;
            assert!(TournamentUser::contains(tournament.id, second.id, tc).await?);
            assert!(!TournamentUser::contains(tournament.id, first.id, tc).await?);
            tournament.invite_organizer(first.id, third.id, tc).await?;
            tournament.leave_organizers(first.id, tc).await?;
            // The pending invitation belongs to the tournament, not its sender.
            tournament.accept_organizer_invitation(third.id, tc).await?;
            tournament.leave_organizers(second.id, tc).await?;
            assert!(TournamentUser::contains(tournament.id, second.id, tc).await?);
            assert!(!TournamentUser::contains(tournament.id, third.id, tc).await?);
            assert!(tournament.leave_organizers(third.id, tc).await.is_err());
            tournament.invite_organizer(third.id, first.id, tc).await?;
            tournament
                .decline_organizer_invitation(first.id, tc)
                .await?;
            assert!(tournament
                .accept_organizer_invitation(first.id, tc)
                .await
                .is_err());
            tournament.invite_organizer(third.id, first.id, tc).await?;
            tournament
                .retract_organizer_invitation(third.id, first.id, tc)
                .await?;
            assert!(tournament
                .accept_organizer_invitation(first.id, tc)
                .await
                .is_err());
            Ok(())
        })
    })
    .await
    .unwrap();
    assert!(
        TournamentOrganizerInvitation::find_by_user(second.id, &mut conn)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_organizer_departures_preserve_one_active_organizer() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let first = create_user("race_org_first", &mut conn).await;
    let second = create_user("race_org_second", &mut conn).await;
    let tournament = create_rr_tournament(
        first.id,
        "Organizer race",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    let id = TournamentId(tournament.nanoid.clone());
    run_serializable(&mut conn, |tc| {
        let id = id.clone();
        Box::pin(async move {
            let tournament = Tournament::find_by_tournament_id_for_update(&id, tc).await?;
            tournament.invite_organizer(first.id, second.id, tc).await?;
            tournament.accept_organizer_invitation(second.id, tc).await
        })
    })
    .await
    .unwrap();
    drop(conn);
    let barrier = Arc::new(Barrier::new(2));
    let mut tasks = Vec::new();
    for actor in [first.id, second.id] {
        let pool = db.pool.clone();
        let barrier = Arc::clone(&barrier);
        let id = id.clone();
        tasks.push(tokio::spawn(async move {
            let mut conn = get_conn(&pool).await.unwrap();
            barrier.wait().await;
            run_serializable(&mut conn, |tc| {
                let id = id.clone();
                Box::pin(async move {
                    let tournament = Tournament::find_by_tournament_id_for_update(&id, tc).await?;
                    tournament.leave_organizers(actor, tc).await
                })
            })
            .await
        }));
    }
    let mut succeeded = 0;
    for task in tasks {
        if task.await.unwrap().is_ok() {
            succeeded += 1;
        }
    }
    assert_eq!(succeeded, 1);
    let mut conn = get_conn(&db.pool).await.unwrap();
    assert_eq!(tournament.organizers(&mut conn).await.unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn finishing_consumes_pending_organizer_invitations_and_closes_role_changes() {
    use db_lib::{schema::tournaments_organizer_invitations, tournaments::fixed_field};
    use diesel::{dsl::exists, prelude::*};
    use diesel_async::RunQueryDsl;

    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let first = create_user("finish_org_first", &mut conn).await;
    let second = create_user("finish_org_second", &mut conn).await;
    let pending = create_user("finish_org_pending", &mut conn).await;
    let tournament = create_rr_tournament(
        first.id,
        "Organizer close",
        TournamentStatus::NotStarted,
        &mut conn,
    )
    .await;
    let id = TournamentId(tournament.nanoid.clone());
    run_serializable(&mut conn, |tc| {
        let id = id.clone();
        Box::pin(async move {
            let tournament = Tournament::find_by_tournament_id_for_update(&id, tc).await?;
            tournament.join(&first.id, tc).await?;
            tournament.join(&second.id, tc).await?;
            tournament.invite_organizer(first.id, second.id, tc).await?;
            tournament
                .accept_organizer_invitation(second.id, tc)
                .await?;
            tournament
                .invite_organizer(first.id, pending.id, tc)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();
    let started = fixed_field::start_by_organizer(
        tournament.id,
        first.id,
        vec![first.id, second.id],
        &mut conn,
    )
    .await
    .unwrap();
    assert_eq!(
        TournamentOrganizerInvitation::find_by_user(pending.id, &mut conn)
            .await
            .unwrap()
            .len(),
        1
    );
    fixed_field::close_unstarted_slots(
        tournament.id,
        first.id,
        started
            .games
            .iter()
            .map(|game| game.tournament_slot_id.unwrap())
            .collect(),
        &mut conn,
    )
    .await
    .unwrap();
    let finished = Tournament::find(tournament.id, &mut conn).await.unwrap();
    assert_eq!(finished.status(), TournamentStatus::Finished);
    assert!(!diesel::select(exists(
        tournaments_organizer_invitations::table.find((tournament.id, pending.id))
    ))
    .get_result::<bool>(&mut conn)
    .await
    .unwrap());
    assert!(finished
        .leave_organizers(first.id, &mut conn)
        .await
        .is_err());
    assert!(finished
        .invite_organizer(first.id, pending.id, &mut conn)
        .await
        .is_err());
}
