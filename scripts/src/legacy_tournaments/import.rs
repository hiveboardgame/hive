use super::{
    bundle::{self, BundleManifest, BundleRecord},
    export::manifest_path,
};
use anyhow::{ensure, Context, Error, Result};
use db_lib::DbConn;
use diesel::{
    sql_query,
    sql_types::{
        Array,
        BigInt,
        Bool,
        Integer,
        Jsonb,
        Nullable,
        Text,
        Timestamptz,
        Uuid as SqlUuid,
    },
};
use diesel_async::{AsyncConnection, RunQueryDsl, SimpleAsyncConnection};
use serde_json::Value;
use shared_types::tournament::SlotKey;
use std::{collections::HashMap, fs, path::Path};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ApplyReport {
    pub imported_records: u64,
    pub skipped_records: u64,
    pub tournaments: u64,
    pub slots: u64,
    pub swiss_rounds: u64,
    pub games: u64,
    pub scheduled_slots: u64,
    pub schedule_offers: u64,
    pub schedule_candidates: u64,
    pub final_outcomes: u64,
}

#[derive(diesel::QueryableByName)]
struct DatabaseName {
    #[diesel(sql_type = Text)]
    name: String,
}

#[derive(diesel::QueryableByName)]
struct TargetState {
    #[diesel(sql_type = Bool)]
    occupied: bool,
}

#[derive(diesel::QueryableByName)]
struct ImportedSlotRow {
    #[diesel(sql_type = SqlUuid)]
    id: Uuid,
    #[diesel(sql_type = Jsonb)]
    native_key: Value,
}

pub async fn apply(
    conn: &mut DbConn<'_>,
    input: &Path,
    confirmed_source: &str,
    confirmed_target: &str,
    confirmed_checksum: &str,
) -> Result<ApplyReport> {
    ensure!(
        !confirmed_source.is_empty(),
        "the confirmed source database name cannot be empty"
    );
    ensure!(
        !confirmed_target.is_empty(),
        "the confirmed target database name cannot be empty"
    );
    ensure!(
        !confirmed_checksum.is_empty(),
        "the confirmed bundle checksum cannot be empty"
    );

    let jsonl = fs::read(input).with_context(|| format!("could not read {}", input.display()))?;
    let manifest: BundleManifest = serde_json::from_slice(
        &fs::read(manifest_path(input)).context("could not read canonical bundle manifest")?,
    )
    .context("invalid canonical bundle manifest")?;
    ensure!(
        manifest.source_identifier == confirmed_source,
        "bundle source database mismatch: manifest names {:?}, confirmed {:?}",
        manifest.source_identifier,
        confirmed_source
    );
    ensure!(
        manifest.jsonl_sha256 == confirmed_checksum,
        "bundle checksum confirmation mismatch"
    );
    let records = bundle::validate(&jsonl, &manifest)?;
    let skipped_records = manifest.skipped_record_counts.values().sum();

    let actual_target = sql_query("select current_database() as name")
        .get_result::<DatabaseName>(conn)
        .await?
        .name;
    ensure!(
        actual_target == confirmed_target,
        "target database confirmation mismatch: connected to {:?}, confirmed {:?}",
        actual_target,
        confirmed_target
    );

    conn.transaction::<_, Error, _>(async move |tc| {
        tc.batch_execute(
            "lock table tournaments, tournaments_organizers, tournaments_users, \
             tournaments_invitations, tournament_slots, tournament_swiss_rounds, \
             tournament_final_outcomes, schedule_offers, games \
             in access exclusive mode",
        )
        .await?;
        ensure_target_is_empty(tc).await?;
        let mut report = apply_records(&records, tc).await?;
        report.skipped_records = skipped_records;
        Ok(report)
    })
    .await
}

async fn ensure_target_is_empty(conn: &mut DbConn<'_>) -> Result<()> {
    let occupied = sql_query(
        "select (\
             exists(select 1 from tournaments) or \
             exists(select 1 from tournaments_organizers) or \
             exists(select 1 from tournaments_users) or \
             exists(select 1 from tournaments_invitations) or \
             exists(select 1 from tournament_slots) or \
             exists(select 1 from tournament_swiss_rounds) or \
             exists(select 1 from tournament_final_outcomes) or \
             exists(select 1 from schedule_offers) or \
             exists(select 1 from games where tournament_id is not null \
                                      or tournament_slot_id is not null \
                                      or arena_ordinal is not null \
                                      )\
         ) as occupied",
    )
    .get_result::<TargetState>(conn)
    .await?
    .occupied;
    ensure!(
        !occupied,
        "canonical Tournament target is not empty; refusing to continue a partial import"
    );
    Ok(())
}

async fn apply_records(records: &[BundleRecord], conn: &mut DbConn<'_>) -> Result<ApplyReport> {
    let mut report = ApplyReport::default();
    let mut tournaments = Vec::new();
    let mut organizers = Vec::new();
    let mut memberships = Vec::new();
    let mut invitations = Vec::new();
    let mut swiss_rounds = Vec::new();
    let mut slots = Vec::new();
    let mut games = Vec::new();
    let mut schedule_offers = Vec::new();
    let mut final_outcomes = Vec::new();
    for record in records {
        match record {
            BundleRecord::Tournament { tournament } => tournaments.push(tournament.as_ref()),
            BundleRecord::Organizer { organizer } => organizers.push(organizer),
            BundleRecord::Membership { membership } => memberships.push(membership),
            BundleRecord::Invitation { invitation } => invitations.push(invitation),
            BundleRecord::SwissRound { round } => swiss_rounds.push(round),
            BundleRecord::Slot { slot } => slots.push(slot),
            BundleRecord::Game { game } => games.push(game),
            BundleRecord::ScheduleOffer { offer } => schedule_offers.push(offer),
            BundleRecord::FinalOutcome { outcome } => final_outcomes.push(outcome),
        }
    }

    for tournament in tournaments {
        let configuration = serde_json::to_value(&tournament.configuration)?;
        sql_query(
            "insert into tournaments \
                 (id,nanoid,name,description,seats,min_seats,invite_only,band_upper,band_lower,\
                  starts_at,started_at,created_at,updated_at,series,configuration,finished_at) \
                 values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
        )
        .bind::<SqlUuid, _>(tournament.id)
        .bind::<Text, _>(&tournament.nanoid)
        .bind::<Text, _>(&tournament.name)
        .bind::<Nullable<Text>, _>(tournament.description.as_deref())
        .bind::<Integer, _>(tournament.seats)
        .bind::<Integer, _>(tournament.min_seats)
        .bind::<Bool, _>(tournament.invite_only)
        .bind::<Nullable<Integer>, _>(tournament.band_upper)
        .bind::<Nullable<Integer>, _>(tournament.band_lower)
        .bind::<Nullable<Timestamptz>, _>(tournament.starts_at)
        .bind::<Nullable<Timestamptz>, _>(tournament.started_at)
        .bind::<Timestamptz, _>(tournament.created_at)
        .bind::<Timestamptz, _>(tournament.updated_at)
        .bind::<Nullable<SqlUuid>, _>(tournament.series)
        .bind::<Jsonb, _>(configuration)
        .bind::<Nullable<Timestamptz>, _>(tournament.finished_at)
        .execute(conn)
        .await?;
        report.imported_records += 1;
        report.tournaments += 1;
    }
    for organizer in organizers {
        sql_query("insert into tournaments_organizers (tournament_id,organizer_id) values ($1,$2)")
            .bind::<SqlUuid, _>(organizer.tournament_id)
            .bind::<SqlUuid, _>(organizer.organizer_id)
            .execute(conn)
            .await?;
        report.imported_records += 1;
    }
    for membership in memberships {
        sql_query(
            "insert into tournaments_users \
                 (tournament_id,user_id,accepted_at,pairing_number) values ($1,$2,$3,$4)",
        )
        .bind::<SqlUuid, _>(membership.tournament_id)
        .bind::<SqlUuid, _>(membership.user_id)
        .bind::<Timestamptz, _>(membership.accepted_at)
        .bind::<Nullable<Integer>, _>(membership.pairing_number)
        .execute(conn)
        .await?;
        report.imported_records += 1;
    }
    for invitation in invitations {
        sql_query(
            "insert into tournaments_invitations \
                 (tournament_id,invitee_id,created_at,declined_at) values ($1,$2,$3,$4)",
        )
        .bind::<SqlUuid, _>(invitation.tournament_id)
        .bind::<SqlUuid, _>(invitation.invitee_id)
        .bind::<Timestamptz, _>(invitation.created_at)
        .bind::<Nullable<Timestamptz>, _>(invitation.declined_at)
        .execute(conn)
        .await?;
        report.imported_records += 1;
    }
    for round in swiss_rounds {
        let mut pairings = serde_json::to_value(&round.pairings)?;
        pairings
            .as_object_mut()
            .context("canonical Swiss pairings did not serialize as an object")?
            .insert(
                String::from("accepted_ratings"),
                serde_json::to_value(&round.accepted_ratings)?,
            );
        sql_query(
            "insert into tournament_swiss_rounds \
                     (tournament_id,round_id,pairings,accepted_at) values ($1,$2,$3,$4)",
        )
        .bind::<SqlUuid, _>(round.tournament_id)
        .bind::<BigInt, _>(round.round_id)
        .bind::<Jsonb, _>(pairings)
        .bind::<Timestamptz, _>(round.accepted_at)
        .execute(conn)
        .await?;
        report.imported_records += 1;
        report.swiss_rounds += 1;
    }
    let mut imported_slot_ids = HashMap::with_capacity(slots.len());
    for slot in slots {
        let ImportedSlotRow { id, native_key } = sql_query(
            "insert into tournament_slots \
                 (tournament_id,native_key,white_id,black_id,clock,resolution,resolved_at,\
                  scheduled_at,deadline_at) \
                 values ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
                 returning id,native_key",
        )
        .bind::<SqlUuid, _>(slot.tournament_id)
        .bind::<Jsonb, _>(serde_json::to_value(slot.key)?)
        .bind::<SqlUuid, _>(slot.white_id)
        .bind::<SqlUuid, _>(slot.black_id)
        .bind::<Jsonb, _>(serde_json::to_value(slot.clock)?)
        .bind::<Nullable<Jsonb>, _>(slot.resolution.map(serde_json::to_value).transpose()?)
        .bind::<Nullable<Timestamptz>, _>(slot.resolved_at)
        .bind::<Nullable<Timestamptz>, _>(slot.scheduled_at)
        .bind::<Nullable<Timestamptz>, _>(slot.deadline_at)
        .get_result(conn)
        .await?;
        let key = serde_json::from_value::<SlotKey>(native_key)
            .context("database returned an invalid canonical Slot key")?;
        ensure!(
            imported_slot_ids
                .insert((slot.tournament_id, key), id)
                .is_none(),
            "canonical Slot key {:?} occurs more than once in tournament {}",
            key,
            slot.tournament_id
        );
        report.imported_records += 1;
        report.slots += 1;
        if slot.scheduled_at.is_some() {
            report.scheduled_slots += 1;
        }
    }
    for game in games {
        let tournament_slot_id =
            imported_slot_id(&imported_slot_ids, game.tournament_id, game.key)?;
        let changed = sql_query(
                "update games set tournament_id=$2,tournament_slot_id=$3,tournament_game_result=$4,finished_at=$5 \
                 where id=$1 and tournament_id is null and tournament_slot_id is null",
            )
            .bind::<SqlUuid, _>(game.id)
            .bind::<SqlUuid, _>(game.tournament_id)
            .bind::<SqlUuid, _>(tournament_slot_id)
            .bind::<Text, _>(game.tournament_game_result.to_string())
            .bind::<Nullable<Timestamptz>, _>(game.finished_at)
            .execute(conn)
            .await?;
        ensure!(
            changed == 1,
            "canonical Hive Game {} is missing or already owned",
            game.id
        );
        report.imported_records += 1;
        report.games += 1;
    }
    for offer in schedule_offers {
        let tournament_slot_id =
            imported_slot_id(&imported_slot_ids, offer.tournament_id, offer.key)?;
        sql_query(
            "insert into schedule_offers \
                 (tournament_id,tournament_slot_id,proposer_id,candidate_times,created_at) \
                 values ($1,$2,$3,$4,$5)",
        )
        .bind::<SqlUuid, _>(offer.tournament_id)
        .bind::<SqlUuid, _>(tournament_slot_id)
        .bind::<SqlUuid, _>(offer.proposer_id)
        .bind::<Array<Timestamptz>, _>(offer.candidate_times.clone())
        .bind::<Timestamptz, _>(offer.created_at)
        .execute(conn)
        .await?;
        report.imported_records += 1;
        report.schedule_offers += 1;
        report.schedule_candidates += u64::try_from(offer.candidate_times.len())
            .context("canonical schedule candidate count does not fit the import report")?;
    }
    for outcome in final_outcomes {
        sql_query(
            "insert into tournament_final_outcomes \
                 (tournament_id,standings) values ($1,$2)",
        )
        .bind::<SqlUuid, _>(outcome.tournament_id)
        .bind::<Jsonb, _>(serde_json::to_value(&outcome.standings)?)
        .execute(conn)
        .await?;
        report.imported_records += 1;
        report.final_outcomes += 1;
    }

    Ok(report)
}

fn imported_slot_id(
    imported_slot_ids: &HashMap<(Uuid, SlotKey), Uuid>,
    tournament_id: Uuid,
    key: SlotKey,
) -> Result<Uuid> {
    imported_slot_ids
        .get(&(tournament_id, key))
        .copied()
        .with_context(|| {
            format!(
                "canonical record references missing Slot key {key:?} in tournament {tournament_id}"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_tournaments::{
        bundle::{CanonicalGame, CanonicalOrganizer, CanonicalScheduleOffer},
        model::{
            CanonicalAcceptedRating,
            CanonicalMembership,
            CanonicalSlot,
            CanonicalSwissRound,
            CanonicalTournament,
        },
    };
    use anyhow::{anyhow, ensure};
    use chrono::{DateTime, Duration, Utc};
    use db_lib::{
        get_conn,
        models::{Game, NewGame, TournamentSlot},
        schema::{
            games,
            schedule_offers,
            tournament_final_outcomes,
            tournament_slots,
            tournament_swiss_rounds,
            tournaments,
        },
        tournaments::fixed_field,
    };
    use diesel::{ExpressionMethods, QueryDsl};
    use diesel_async::{AsyncConnection, RunQueryDsl};
    use shared_types::{
        tournament::{
            round_robin::Config as RoundRobinConfig,
            swiss::Config as SwissConfig,
            BotAdmission,
            Clock,
            Config,
            FormatConfig,
            GameOutcome,
            PlayedGameOutcome,
            RealtimeClock,
            Resolution,
            RoundRobinGameId,
            SlotKey,
        },
        TournamentGameResult,
        TournamentStatus,
    };
    use std::num::NonZeroU32;
    use tournamint::{
        swiss::{DoubleSwissPrimaryScore, RoundPairings},
        Pairing,
        PlayerId,
    };
    use uuid::Uuid;

    mod test_database {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../db/tests/common/test_db_support.rs"
        ));
    }

    fn clock() -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 3,
        })
    }

    fn configuration() -> Config {
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::RoundRobin(RoundRobinConfig::standard(
                NonZeroU32::new(2).unwrap(),
                clock(),
            )),
        }
    }

    fn canonical_tournament(
        id: Uuid,
        nanoid: &str,
        starts_at: Option<DateTime<Utc>>,
        lifecycle: TournamentStatus,
    ) -> BundleRecord {
        canonical_tournament_with_configuration(id, nanoid, starts_at, lifecycle, configuration())
    }

    fn canonical_tournament_with_configuration(
        id: Uuid,
        nanoid: &str,
        starts_at: Option<DateTime<Utc>>,
        lifecycle: TournamentStatus,
        configuration: Config,
    ) -> BundleRecord {
        let now = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).unwrap();
        BundleRecord::Tournament {
            tournament: Box::new(CanonicalTournament {
                id,
                nanoid: nanoid.to_owned(),
                name: format!("Importer starts_at {nanoid}"),
                description: Some(String::from(
                    "An importer integration fixture long enough for tournament validation.",
                )),
                seats: 2,
                min_seats: 2,
                invite_only: false,
                band_upper: None,
                band_lower: None,
                starts_at,
                started_at: (lifecycle != TournamentStatus::NotStarted).then_some(now),
                created_at: now,
                updated_at: now,
                series: None,
                configuration,
                lifecycle,
                finished_at: (lifecycle == TournamentStatus::Finished).then_some(now),
            }),
        }
    }

    fn new_game(
        nanoid: &str,
        white_id: Uuid,
        black_id: Uuid,
        finished: bool,
        now: DateTime<Utc>,
    ) -> NewGame {
        NewGame {
            nanoid: nanoid.to_owned(),
            current_player_id: white_id,
            black_id,
            finished,
            game_status: if finished {
                String::from("Finished(1-0)")
            } else {
                String::from("NotStarted")
            },
            game_type: String::from("MLP"),
            history: if finished {
                String::from("wA1;")
            } else {
                String::new()
            },
            game_control_history: if finished {
                String::from("legacy-control-history")
            } else {
                String::new()
            },
            rated: !finished,
            tournament_queen_rule: true,
            turn: if finished { 1 } else { 0 },
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: String::from("Real Time"),
            time_base: Some(300),
            time_increment: Some(3),
            last_interaction: finished.then_some(now),
            black_time_left: Some(300_000_000_000),
            white_time_left: Some(300_000_000_000),
            speed: String::from("Blitz"),
            hashes: Vec::new(),
            conclusion: if finished {
                String::from("Board")
            } else {
                String::from("Unknown")
            },
            tournament_game_result: if finished {
                String::from("1-0")
            } else {
                String::from("Unknown")
            },
            game_start: String::from("Ready"),
            move_times: Vec::new(),
            timeout_at: None,
            tournament_id: None,
            white_berserked: false,
            black_berserked: false,
            arena_move_due_at: None,
            tournament_slot_id: None,
            arena_ordinal: None,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "requires the dedicated TEST_DATABASE_URL"]
    async fn fresh_schema_import_preserves_fixed_field_records_and_swiss_ratings() {
        let db = test_database::test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");

        let now = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).unwrap();
        let starts_at = now - Duration::hours(1);
        let tournament_id = Uuid::from_u128(1);
        let peer_tournament_id = Uuid::from_u128(2);
        let white_id = Uuid::from_u128(3);
        let black_id = Uuid::from_u128(4);
        let swiss_tournament_id = Uuid::from_u128(5);
        let rollback = conn
            .transaction::<(), Error, _>(async move |tc| {
                for (user_id, username) in [(white_id, "import-white"), (black_id, "import-black")]
                {
                    sql_query(
                        "insert into users \
                             (id,username,password,email,created_at,updated_at,normalized_username) \
                             values ($1,$2,$3,$4,$5,$6,$7)",
                    )
                    .bind::<SqlUuid, _>(user_id)
                    .bind::<Text, _>(username)
                    .bind::<Text, _>("unused-password")
                    .bind::<Text, _>(format!("{username}@example.invalid"))
                    .bind::<Timestamptz, _>(now)
                    .bind::<Timestamptz, _>(now)
                    .bind::<Text, _>(username)
                    .execute(tc)
                    .await?;
                }

                let terminal_game = Game::create(
                    new_game("import-terminal", white_id, black_id, true, now),
                    tc,
                )
                .await?;
                let open_game = Game::create(
                    new_game("import-open", black_id, white_id, false, now),
                    tc,
                )
                .await?;
                let terminal_key = SlotKey::RoundRobin {
                    slot: RoundRobinGameId::new(0),
                };
                let open_key = SlotKey::RoundRobin {
                    slot: RoundRobinGameId::new(1),
                };
                let terminal_outcome = GameOutcome::Played(PlayedGameOutcome::WhiteWin);
                let records = vec![
                    canonical_tournament(
                        tournament_id,
                        "in-progress",
                        Some(starts_at),
                        TournamentStatus::InProgress,
                    ),
                    canonical_tournament(
                        peer_tournament_id,
                        "peer",
                        None,
                        TournamentStatus::NotStarted,
                    ),
                    canonical_tournament_with_configuration(
                        swiss_tournament_id,
                        "legacy-double-swiss",
                        None,
                        TournamentStatus::InProgress,
                        Config {
                            bot_admission: BotAdmission::HumansAndBots,
                            format: FormatConfig::Swiss(
                                SwissConfig::automatic_double_swiss(
                                    0,
                                    clock(),
                                    DoubleSwissPrimaryScore::GamePoints,
                                ),
                            ),
                        },
                    ),
                    BundleRecord::Organizer {
                        organizer: CanonicalOrganizer {
                            tournament_id,
                            organizer_id: white_id,
                        },
                    },
                    BundleRecord::Membership {
                        membership: CanonicalMembership {
                            tournament_id,
                            user_id: white_id,
                            accepted_at: now,
                            pairing_number: Some(0),
                        },
                    },
                    BundleRecord::Membership {
                        membership: CanonicalMembership {
                            tournament_id: swiss_tournament_id,
                            user_id: white_id,
                            accepted_at: now,
                            pairing_number: Some(0),
                        },
                    },
                    BundleRecord::Membership {
                        membership: CanonicalMembership {
                            tournament_id: swiss_tournament_id,
                            user_id: black_id,
                            accepted_at: now,
                            pairing_number: Some(1),
                        },
                    },
                    BundleRecord::SwissRound {
                        round: CanonicalSwissRound {
                            tournament_id: swiss_tournament_id,
                            round_id: 0,
                            pairings: RoundPairings {
                                games: vec![Pairing::new(PlayerId::new(0), PlayerId::new(1))],
                                byes: Vec::new(),
                            },
                            accepted_ratings: vec![
                                CanonicalAcceptedRating {
                                    player: PlayerId::new(0),
                                    rating: 1_800,
                                },
                                CanonicalAcceptedRating {
                                    player: PlayerId::new(1),
                                    rating: 1_900,
                                },
                            ],
                            accepted_at: now,
                        },
                    },
                    BundleRecord::Membership {
                        membership: CanonicalMembership {
                            tournament_id,
                            user_id: black_id,
                            accepted_at: now,
                            pairing_number: Some(1),
                        },
                    },
                    BundleRecord::Slot {
                        slot: CanonicalSlot {
                            tournament_id,
                            key: terminal_key,
                            white_id,
                            black_id,
                            clock: clock(),
                            resolution: Some(Resolution::Result(terminal_outcome)),
                            resolved_at: Some(now),
                            scheduled_at: None,
                            deadline_at: None,
                        },
                    },
                    BundleRecord::Slot {
                        slot: CanonicalSlot {
                            tournament_id,
                            key: open_key,
                            white_id: black_id,
                            black_id: white_id,
                            clock: clock(),
                            resolution: None,
                            resolved_at: None,
                            scheduled_at: None,
                            deadline_at: None,
                        },
                    },
                    BundleRecord::Game {
                        game: CanonicalGame {
                            id: terminal_game.id,
                            tournament_id,
                            key: terminal_key,
                            tournament_game_result: "1-0".parse()?,
                            finished_at: Some(now),
                        },
                    },
                    BundleRecord::Game {
                        game: CanonicalGame {
                            id: open_game.id,
                            tournament_id,
                            key: open_key,
                            tournament_game_result: TournamentGameResult::Unknown,
                            finished_at: None,
                        },
                    },
                    BundleRecord::ScheduleOffer {
                        offer: CanonicalScheduleOffer {
                            tournament_id,
                            key: open_key,
                            proposer_id: black_id,
                            candidate_times: vec![now + Duration::hours(1)],
                            created_at: now,
                        },
                    },
                ];
                let report = apply_records(&records, tc).await?;
                ensure!(report.tournaments == 3, "all tournaments must import");
                ensure!(report.swiss_rounds == 1, "the legacy Swiss round must import");
                ensure!(report.slots == 2, "both native Slots must import");
                ensure!(report.games == 2, "both Hive Games must bind");
                ensure!(report.schedule_offers == 1, "the pending offer must import");
                ensure!(report.final_outcomes == 0, "an in-progress import cannot be frozen");
                let swiss_pairings = tournament_swiss_rounds::table
                    .find((swiss_tournament_id, 0_i64))
                    .select(tournament_swiss_rounds::pairings)
                    .first::<Value>(tc)
                    .await?;
                let accepted_ratings = serde_json::from_value::<Vec<CanonicalAcceptedRating>>(
                    swiss_pairings["accepted_ratings"].clone(),
                )?;
                ensure!(
                    accepted_ratings
                        == vec![
                            CanonicalAcceptedRating {
                                player: PlayerId::new(0),
                                rating: 1_800,
                            },
                            CanonicalAcceptedRating {
                                player: PlayerId::new(1),
                                rating: 1_900,
                            },
                        ],
                    "legacy accepted ratings were not persisted with the Swiss round"
                );
                let rows = tournaments::table
                    .select((tournaments::id, tournaments::starts_at))
                    .order(tournaments::id)
                    .load::<(Uuid, Option<DateTime<Utc>>)>(tc)
                    .await?;
                ensure!(
                    rows
                        == vec![
                            (tournament_id, Some(starts_at)),
                            (peer_tournament_id, None),
                            (swiss_tournament_id, None),
                        ],
                    "imported starts_at values differ from the canonical peer fields"
                );

                let persisted_slots = tournament_slots::table
                    .filter(tournament_slots::tournament_id.eq(tournament_id))
                    .select((
                        tournament_slots::id,
                        tournament_slots::native_key,
                        tournament_slots::resolution,
                    ))
                    .load::<(Uuid, Value, Option<Value>)>(tc)
                    .await?;
                let persisted_slots = persisted_slots
                    .into_iter()
                    .map(|(id, key, resolution)| {
                        Ok((serde_json::from_value::<SlotKey>(key)?, (id, resolution)))
                    })
                    .collect::<Result<HashMap<_, _>>>()?;
                ensure!(persisted_slots.len() == 2, "the importer must create two Slot UUIDs");
                let (terminal_slot_id, terminal_resolution) = persisted_slots[&terminal_key].clone();
                let (open_slot_id, open_resolution) = persisted_slots[&open_key].clone();
                ensure!(terminal_slot_id != Uuid::nil(), "the terminal Slot UUID is nil");
                ensure!(open_slot_id != Uuid::nil(), "the open Slot UUID is nil");
                ensure!(terminal_slot_id != open_slot_id, "Slot UUIDs are not unique");
                ensure!(terminal_resolution.is_some(), "the effective result was not preserved");
                ensure!(open_resolution.is_none(), "the open Slot became terminal during import");

                let persisted_games = games::table
                    .filter(games::id.eq_any([terminal_game.id, open_game.id]))
                    .select((
                        games::id,
                        games::tournament_id,
                        games::tournament_slot_id,
                        games::history,
                        games::game_control_history,
                    ))
                    .load::<(Uuid, Option<Uuid>, Option<Uuid>, String, String)>(tc)
                    .await?
                    .into_iter()
                    .map(|row| (row.0, row))
                    .collect::<HashMap<_, _>>();
                ensure!(
                    persisted_games[&terminal_game.id].1 == Some(tournament_id)
                        && persisted_games[&terminal_game.id].2 == Some(terminal_slot_id),
                    "the terminal Game was not bound through its native key"
                );
                ensure!(
                    persisted_games[&terminal_game.id].3 == "wA1;"
                        && persisted_games[&terminal_game.id].4 == "legacy-control-history",
                    "the imported terminal Game history changed"
                );
                ensure!(
                    persisted_games[&open_game.id].1 == Some(tournament_id)
                        && persisted_games[&open_game.id].2 == Some(open_slot_id),
                    "the open Game was not bound through its native key"
                );
                let offer_slot_ids = schedule_offers::table
                    .filter(schedule_offers::tournament_id.eq(tournament_id))
                    .select(schedule_offers::tournament_slot_id)
                    .load::<Uuid>(tc)
                    .await?;
                ensure!(
                    offer_slot_ids == vec![open_slot_id],
                    "the schedule offer did not resolve the same native key"
                );
                let frozen_before = tournament_final_outcomes::table
                    .filter(tournament_final_outcomes::tournament_id.eq(tournament_id))
                    .count()
                    .get_result::<i64>(tc)
                    .await?;
                ensure!(frozen_before == 0, "the in-progress import created a final outcome");

                let expected_resolution = TournamentSlot::find(
                    tournament_id,
                    open_slot_id,
                    tc,
                )
                .await?
                .resolution;
                let adjudication = fixed_field::adjudicate_slot_atomic(
                    tournament_id,
                    open_slot_id,
                    "1-0".parse()?,
                    expected_resolution,
                    white_id,
                    tc,
                )
                .await?;
                ensure!(adjudication.newly_terminal, "the remaining Slot did not seal normally");
                let finish_effects = adjudication
                    .commit
                    .as_ref()
                    .context("the imported tournament finish produced no tournament effects")?;
                ensure!(
                    finish_effects.finished_now,
                    "the imported tournament did not finish normally"
                );
                let frozen_after = tournament_final_outcomes::table
                    .filter(tournament_final_outcomes::tournament_id.eq(tournament_id))
                    .count()
                    .get_result::<i64>(tc)
                    .await?;
                ensure!(frozen_after == 1, "normal finish did not freeze the final outcome");
                Err(anyhow!("rollback importer persistence fixture"))
            })
            .await
            .expect_err("the fixture transaction intentionally rolls back");
        assert_eq!(
            rollback.to_string(),
            "rollback importer persistence fixture"
        );
    }
}
