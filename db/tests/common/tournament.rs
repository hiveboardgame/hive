use chrono::{DateTime, Duration, TimeZone, Utc};
use db_lib::{
    models::{NewTournament, NewUser, Tournament, TournamentUser, User},
    schema::{tournaments, tournaments_users},
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::{
    tournament::{
        round_robin::Config as RoundRobinConfig,
        standings::{
            Group as StandingGroup,
            Placement as StandingPlacement,
            Row as StandingRow,
            Snapshot,
            Value as StandingValue,
        },
        BotAdmission,
        Clock,
        Config,
        FormatConfig,
        RealtimeClock,
        ReleasePolicy,
        Score,
    },
    TournamentDetails,
    TournamentStatus,
};
use std::num::NonZeroU32;
use uuid::Uuid;

pub fn round_robin_configuration() -> Config {
    round_robin_configuration_with(
        NonZeroU32::new(1).unwrap(),
        ReleasePolicy::SequentialPerMatchup,
    )
}

pub fn round_robin_configuration_with(
    repeats: NonZeroU32,
    release_policy: ReleasePolicy,
) -> Config {
    let mut round_robin = RoundRobinConfig::standard(repeats, realtime_clock());
    round_robin.release_policy = release_policy;
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::RoundRobin(round_robin),
    }
}

pub fn realtime_clock() -> Clock {
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(300).unwrap(),
        increment_seconds: 3,
    })
}

pub async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    User::create(
        NewUser::new(username, "password", &format!("{username}@example.test"))
            .expect("build user fixture"),
        conn,
    )
    .await
    .expect("insert user fixture")
}

pub struct RoundRobinScenario {
    organizer_id: Uuid,
    pub player_ids: [Uuid; 2],
}

impl RoundRobinScenario {
    pub async fn create(
        organizer_prefix: &str,
        player_prefix: &str,
        conn: &mut DbConn<'_>,
    ) -> Self {
        let organizer = create_user(&format!("{organizer_prefix}_organizer"), conn).await;
        let first = create_user(&format!("{player_prefix}_first"), conn).await;
        let second = create_user(&format!("{player_prefix}_second"), conn).await;
        Self {
            organizer_id: organizer.id,
            player_ids: [first.id, second.id],
        }
    }

    pub async fn tournament(
        &self,
        name: &str,
        status: TournamentStatus,
        conn: &mut DbConn<'_>,
    ) -> Tournament {
        let tournament = create_rr_tournament(self.organizer_id, name, status, conn).await;
        insert_dense_memberships(tournament.id, &self.player_ids, conn).await;
        tournament
    }
}

pub async fn create_rr_tournament(
    organizer_id: Uuid,
    name: &str,
    status: TournamentStatus,
    conn: &mut DbConn<'_>,
) -> Tournament {
    create_rr_tournament_with_configuration(
        organizer_id,
        name,
        status,
        None,
        round_robin_configuration(),
        conn,
    )
    .await
}

pub async fn create_rr_tournament_with_configuration(
    organizer_id: Uuid,
    name: &str,
    status: TournamentStatus,
    starts_at: Option<DateTime<Utc>>,
    configuration: Config,
    conn: &mut DbConn<'_>,
) -> Tournament {
    let min_seats = match &configuration.format {
        FormatConfig::Arena(_) => 0,
        FormatConfig::Swiss(_) => 5,
        _ => 2,
    };
    let new_tournament = NewTournament::new(TournamentDetails {
        name: name.to_string(),
        description: Some(String::from(
            "A deterministic tournament fixture used for database integration behavior tests.",
        )),
        seats: match &configuration.format {
            FormatConfig::Arena(_) => None,
            FormatConfig::Swiss(_) => Some(8),
            _ => Some(4),
        },
        min_seats,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at,
        configuration,
    })
    .expect("build Round Robin tournament fixture");
    let tournament = Tournament::create(organizer_id, &new_tournament, conn)
        .await
        .expect("insert Round Robin tournament fixture");
    apply_lifecycle(tournament, status, conn).await
}

pub async fn apply_lifecycle(
    tournament: Tournament,
    status: TournamentStatus,
    conn: &mut DbConn<'_>,
) -> Tournament {
    if status == TournamentStatus::NotStarted {
        return tournament;
    }
    let now = fixed_instant();
    let (started_at, finished_at) = match status {
        TournamentStatus::NotStarted => unreachable!("handled above"),
        TournamentStatus::InProgress => (now, None),
        TournamentStatus::Finished => (now - Duration::hours(1), Some(now)),
    };
    diesel::update(tournaments::table.find(tournament.id))
        .set((
            tournaments::started_at.eq(Some(started_at)),
            tournaments::finished_at.eq(finished_at),
            tournaments::updated_at.eq(now),
        ))
        .get_result(conn)
        .await
        .expect("apply persisted tournament lifecycle fixture")
}

pub async fn insert_unfrozen_memberships(
    tournament_id: Uuid,
    user_ids: &[Uuid],
    accepted_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) {
    for (index, user_id) in user_ids.iter().copied().enumerate() {
        let membership = TournamentUser::accepted_at(
            tournament_id,
            user_id,
            accepted_at + Duration::seconds(index as i64),
        );
        diesel::insert_into(tournaments_users::table)
            .values(&membership)
            .execute(conn)
            .await
            .expect("insert unfrozen tournament membership");
    }
}

pub async fn insert_dense_memberships(
    tournament_id: Uuid,
    user_ids: &[Uuid],
    conn: &mut DbConn<'_>,
) {
    for (index, user_id) in user_ids.iter().copied().enumerate() {
        let mut membership = TournamentUser::accepted_at(
            tournament_id,
            user_id,
            fixed_instant() + Duration::seconds(index as i64),
        );
        membership.pairing_number = Some(i32::try_from(index).unwrap());
        diesel::insert_into(tournaments_users::table)
            .values(&membership)
            .execute(conn)
            .await
            .expect("insert dense tournament membership");
    }
}

pub fn round_robin_snapshot(_tournament_id: Uuid, users: [Uuid; 2]) -> Snapshot {
    let rows = users
        .into_iter()
        .enumerate()
        .map(|(index, user_id)| {
            let score = Score::new(if index == 0 { 2 } else { 0 });
            StandingRow {
                user_id,
                primary_score: StandingValue::Score(score),
                games_played: 1,
                matches_played: None,
                wins: u32::from(index == 0),
                draws: 0,
                losses: u32::from(index == 1),
                counts: vec![0],
                values: vec![StandingValue::Score(score)],
            }
        })
        .collect::<Vec<_>>();
    Snapshot {
        groups: rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| StandingGroup {
                placement: StandingPlacement::CompetitionRank(u32::try_from(index + 1).unwrap()),
                rows: vec![row],
                separated_by: None,
            })
            .collect(),
    }
}

pub fn fixed_instant() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 11, 12, 0, 0)
        .single()
        .unwrap()
}
