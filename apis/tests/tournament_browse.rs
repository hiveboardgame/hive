#![cfg(feature = "ssr")]

mod common;

use apis::responses::{
    TournamentAccessState,
    TournamentBrowsePage,
    TournamentCardResponse,
    TournamentCategory,
    TournamentFamily,
    TournamentTimeFilter,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use db_lib::{
    get_conn,
    models::{NewTournament, Tournament},
    schema::tournaments,
    DbConn,
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        elimination::{Config as EliminationConfig, SeriesPlan, Topology},
        round_robin::Config as RoundRobinConfig,
        swiss::{Config as SwissConfig, PrimaryScore as DoubleSwissPrimaryScore},
        BotAdmission,
        Config,
        Format,
        FormatConfig,
    },
    Clock,
    CorrespondenceClock,
    RealtimeClock,
    TournamentDetails,
    TournamentId,
};
use std::{collections::HashSet, num::NonZeroU32};

fn instant() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2099, 5, 10, 12, 0, 0)
        .single()
        .unwrap()
}

fn live_clock() -> Clock {
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(60).unwrap(),
        increment_seconds: 0,
    })
}

fn configuration(format: Format, clock: Clock) -> Config {
    Config {
        bot_admission: BotAdmission::HumansOnly,
        format: match format {
            Format::RoundRobin => FormatConfig::RoundRobin(RoundRobinConfig::standard(
                NonZeroU32::new(1).unwrap(),
                clock,
            )),
            Format::Swiss => FormatConfig::Swiss(SwissConfig::automatic_swiss(0, clock)),
            Format::DoubleSwiss => FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
                0,
                clock,
                DoubleSwissPrimaryScore::GamePoints,
            )),
            Format::SingleElimination | Format::DoubleElimination => {
                FormatConfig::Elimination(EliminationConfig {
                    topology: if format == Format::SingleElimination {
                        Topology::Single { bronze: false }
                    } else {
                        Topology::Double
                    },
                    default_plan: SeriesPlan::balanced(clock),
                    stage_overrides: Vec::new(),
                })
            }
            Format::Arena => {
                let Clock::Realtime(clock) = clock else {
                    panic!("Arena fixture requires a live clock");
                };
                FormatConfig::Arena(ArenaConfig::new(NonZeroU32::new(3600).unwrap(), clock))
            }
        },
    }
}

async fn insert_tournament(
    name: &str,
    configuration: Config,
    starts_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    conn: &mut DbConn<'_>,
) -> Tournament {
    let format = configuration.format();
    let mut new = NewTournament::new(TournamentDetails {
        name: name.to_owned(),
        description: None,
        seats: (format != Format::Arena).then_some(16),
        min_seats: match format {
            Format::Arena => 0,
            Format::Swiss | Format::DoubleSwiss => 5,
            _ => 2,
        },
        invite_only: false,
        band_lower: Some(1300),
        band_upper: Some(1800),
        starts_at,
        configuration,
    })
    .expect("validate tournament query fixture");
    new.created_at = instant();
    new.started_at = started_at;
    new.finished_at = finished_at;
    diesel::insert_into(tournaments::table)
        .values(new)
        .get_result(conn)
        .await
        .expect("insert tournament query fixture")
}

#[tokio::test(flavor = "multi_thread")]
async fn family_and_mode_filters_include_every_supported_variant() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let clocks = [
        live_clock(),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: NonZeroU32::new(86400).unwrap(),
        }),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach {
            seconds_each: NonZeroU32::new(86400).unwrap(),
        }),
    ];
    for format in [
        Format::RoundRobin,
        Format::Swiss,
        Format::DoubleSwiss,
        Format::SingleElimination,
        Format::DoubleElimination,
    ] {
        for (index, clock) in clocks.into_iter().enumerate() {
            insert_tournament(
                &format!("Variant {} {index}", format.as_str()),
                configuration(format, clock),
                Some(instant()),
                None,
                None,
                &mut conn,
            )
            .await;
        }
    }
    insert_tournament(
        "Variant Arena",
        configuration(Format::Arena, live_clock()),
        Some(instant()),
        None,
        None,
        &mut conn,
    )
    .await;

    for (family, mode, expected) in [
        (None, TournamentTimeFilter::Both, 16),
        (None, TournamentTimeFilter::Live, 6),
        (None, TournamentTimeFilter::Correspondence, 10),
        (Some(TournamentFamily::Swiss), TournamentTimeFilter::Both, 6),
        (Some(TournamentFamily::Swiss), TournamentTimeFilter::Live, 2),
        (
            Some(TournamentFamily::Swiss),
            TournamentTimeFilter::Correspondence,
            4,
        ),
        (
            Some(TournamentFamily::RoundRobin),
            TournamentTimeFilter::Both,
            3,
        ),
        (
            Some(TournamentFamily::RoundRobin),
            TournamentTimeFilter::Live,
            1,
        ),
        (
            Some(TournamentFamily::RoundRobin),
            TournamentTimeFilter::Correspondence,
            2,
        ),
        (
            Some(TournamentFamily::Elimination),
            TournamentTimeFilter::Both,
            6,
        ),
        (
            Some(TournamentFamily::Elimination),
            TournamentTimeFilter::Live,
            2,
        ),
        (
            Some(TournamentFamily::Elimination),
            TournamentTimeFilter::Correspondence,
            4,
        ),
        (
            Some(TournamentFamily::Arena),
            TournamentTimeFilter::Correspondence,
            1,
        ),
    ] {
        let page = TournamentBrowsePage::load(
            TournamentCategory::Upcoming,
            None,
            "",
            1,
            family,
            mode,
            &mut conn,
        )
        .await
        .unwrap();
        assert_eq!(page.total, expected, "{family:?} {mode:?}");
        assert_eq!(page.tournaments.len(), expected as usize);
        for card in page.tournaments {
            if let Some(family) = family {
                assert_eq!(card.summary.format.family_tag(), family.as_str());
            }
            if family != Some(TournamentFamily::Arena) && mode != TournamentTimeFilter::Both {
                let clock = card.summary.primary_clock.unwrap();
                assert_eq!(
                    matches!(clock, Clock::Realtime(_)),
                    mode == TournamentTimeFilter::Live
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn search_family_and_mode_are_applied_before_counting_and_pagination() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let mut expected_ids = HashSet::new();
    for index in 0..25 {
        let tournament = insert_tournament(
            &format!("Matching Swiss {index:02}"),
            configuration(
                if index % 2 == 0 {
                    Format::Swiss
                } else {
                    Format::DoubleSwiss
                },
                live_clock(),
            ),
            Some(instant() + Duration::minutes(index)),
            None,
            None,
            &mut conn,
        )
        .await;
        expected_ids.insert(TournamentId(tournament.nanoid));
    }
    for (name, format, clock, finished) in [
        (
            "Matching wrong family",
            Format::RoundRobin,
            live_clock(),
            false,
        ),
        (
            "Matching wrong mode",
            Format::Swiss,
            Clock::Correspondence(CorrespondenceClock::DaysPerMove {
                seconds_per_move: NonZeroU32::new(86400).unwrap(),
            }),
            false,
        ),
        ("Different name", Format::Swiss, live_clock(), false),
        (
            "Matching already finished",
            Format::Swiss,
            live_clock(),
            true,
        ),
    ] {
        insert_tournament(
            name,
            configuration(format, clock),
            Some(instant()),
            finished.then_some(instant()),
            finished.then_some(instant()),
            &mut conn,
        )
        .await;
    }
    let first = TournamentBrowsePage::load(
        TournamentCategory::Upcoming,
        None,
        "mAtChInG",
        1,
        Some(TournamentFamily::Swiss),
        TournamentTimeFilter::Live,
        &mut conn,
    )
    .await
    .unwrap();
    let last = TournamentBrowsePage::load(
        TournamentCategory::Upcoming,
        None,
        "mAtChInG",
        usize::MAX,
        Some(TournamentFamily::Swiss),
        TournamentTimeFilter::Live,
        &mut conn,
    )
    .await
    .unwrap();
    assert_eq!(
        (first.total, first.page, first.tournaments.len()),
        (25, 1, 20)
    );
    assert_eq!((last.total, last.page, last.tournaments.len()), (25, 2, 5));
    let loaded_ids = first
        .tournaments
        .into_iter()
        .chain(last.tournaments)
        .map(|card| card.tournament_id)
        .collect::<HashSet<_>>();
    assert_eq!(loaded_ids, expected_ids);
}

#[tokio::test(flavor = "multi_thread")]
async fn displayed_card_lookup_is_bounded_and_keeps_public_rules_separate_from_viewer_access() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let tournament = insert_tournament(
        "Restricted upcoming event",
        configuration(Format::Swiss, live_clock()),
        Some(instant()),
        None,
        None,
        &mut conn,
    )
    .await;
    diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::invite_only.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();
    let ids = HashSet::from([
        TournamentId(tournament.nanoid.clone()),
        TournamentId("missing".to_owned()),
    ]);
    let cards = TournamentCardResponse::load_by_ids(&ids, None, &mut conn)
        .await
        .unwrap();
    assert_eq!(cards.len(), 1);
    let summary = &cards[0].summary;
    assert_eq!(summary.access, Some(TournamentAccessState::LoginRequired));
    assert!(summary.entry_open);
    assert!(summary.restrictions.invite_only);
    assert_eq!(summary.restrictions.band_lower, Some(1300));
    assert_eq!(summary.restrictions.band_upper, Some(1800));
    assert_eq!(summary.bot_admission, BotAdmission::HumansOnly);

    diesel::update(tournaments::table.find(tournament.id))
        .set((
            tournaments::started_at.eq(Some(instant())),
            tournaments::finished_at.eq(Some(instant())),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    let cards = TournamentCardResponse::load_by_ids(&ids, None, &mut conn)
        .await
        .unwrap();
    assert_eq!(cards.len(), 1);
    assert!(!cards[0].summary.entry_open);

    let mut excessive_ids = (0..20)
        .map(|index| TournamentId(format!("missing{index}")))
        .collect::<HashSet<_>>();
    assert!(
        TournamentCardResponse::load_by_ids(&excessive_ids, None, &mut conn)
            .await
            .unwrap()
            .is_empty()
    );
    excessive_ids.insert(TournamentId("one-too-many".to_owned()));
    assert!(
        TournamentCardResponse::load_by_ids(&excessive_ids, None, &mut conn)
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn homepage_selection_includes_window_boundaries_and_all_ongoing_arenas() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let now = instant();
    let mut expected = Vec::new();
    for (name, starts_at, started_at) in [
        (
            "Ongoing recent",
            now - Duration::minutes(10),
            Some(now - Duration::minutes(9)),
        ),
        (
            "Ongoing old",
            now - Duration::days(1),
            Some(now - Duration::days(1)),
        ),
        ("Upcoming now", now, None),
        ("Upcoming limit", now + Duration::minutes(15), None),
    ] {
        let tournament = insert_tournament(
            name,
            configuration(Format::Arena, live_clock()),
            Some(starts_at),
            started_at,
            None,
            &mut conn,
        )
        .await;
        expected.push(tournament.id);
        if name == "Ongoing recent" {
            let cards = TournamentCardResponse::load_by_ids(
                &HashSet::from([TournamentId(tournament.nanoid)]),
                None,
                &mut conn,
            )
            .await
            .unwrap();
            assert_eq!(
                cards[0].summary.arena_ends_at,
                Some(starts_at + Duration::hours(1))
            );
        }
    }
    for (name, starts_at, started_at, finished_at, format) in [
        (
            "Too early",
            now + Duration::minutes(15) + Duration::seconds(1),
            None,
            None,
            Format::Arena,
        ),
        (
            "Overdue unstarted",
            now - Duration::seconds(1),
            None,
            None,
            Format::Arena,
        ),
        (
            "Finished arena",
            now - Duration::hours(2),
            Some(now - Duration::hours(2)),
            Some(now),
            Format::Arena,
        ),
        ("Upcoming Swiss", now, None, None, Format::Swiss),
    ] {
        insert_tournament(
            name,
            configuration(format, live_clock()),
            Some(starts_at),
            started_at,
            finished_at,
            &mut conn,
        )
        .await;
    }
    let selected = Tournament::get_live_arenas(now, &mut conn).await.unwrap();
    assert_eq!(
        selected
            .into_iter()
            .map(|tournament| tournament.id)
            .collect::<Vec<_>>(),
        expected
    );
}
