use super::UserResponse;
use chrono::{DateTime, Utc};
use hive_lib::Color;
use serde::{Deserialize, Serialize};
use shared_types::{
    tournament::{standings::Snapshot, BotAdmission, Config, Format, FormatConfig},
    tournament_view::{
        ArenaGameResponse,
        ArenaPlayerStatsResponse,
        PlayerStatsResponse,
        SlotResponse,
        TournamentFormatResponse,
    },
    Clock,
    GameId,
    SwissRoundSummary,
    TournamentId,
    TournamentStartSetup,
    TournamentStatus,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const TOURNAMENT_BROWSE_PAGE_SIZE: usize = 20;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum TournamentCategory {
    Upcoming,
    InProgress,
    Finished,
    Joined,
    Organizing,
    Invitations,
    History,
}

impl TournamentCategory {
    pub const fn requires_viewer(self) -> bool {
        matches!(
            self,
            Self::Joined | Self::Organizing | Self::Invitations | Self::History
        )
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum TournamentAccessState {
    Open,
    PendingInvitation,
    Joined,
    AuthPending,
    LoginRequired,
    InvitationRequired,
    RatingBelow(i32),
    RatingAbove(i32),
    HumansOnly,
    BotsOnly,
    MissingClock,
    Full,
    EntryClosed,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum TournamentCardProgress {
    Entrants {
        current: u32,
        minimum: u32,
        capacity: Option<u32>,
    },
    Arena {
        elapsed_seconds: u32,
        duration_seconds: u32,
    },
    Slots {
        resolved: u32,
        total: u32,
    },
    Swiss {
        completed_rounds: u32,
        total_rounds: u32,
        current_round: Option<SwissRoundSummary>,
    },
    Elimination {
        decided_nodes: u32,
        total_nodes: u32,
    },
    Complete,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum TournamentCardDate {
    Starts(DateTime<Utc>),
    Started(DateTime<Utc>),
    Finished(DateTime<Utc>),
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct AdmissionRestrictions {
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ViewerRelationship {
    pub joined: bool,
    pub invited: bool,
    pub organizing: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct TournamentCardSummary {
    pub format: Format,
    pub primary_clock: Option<Clock>,
    pub access: Option<TournamentAccessState>,
    pub progress: TournamentCardProgress,
    pub relevant_date: Option<TournamentCardDate>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentCardResponse {
    pub tournament_id: TournamentId,
    pub name: String,
    pub players: u32,
    pub seats: Option<i32>,
    pub invited: bool,
    pub organizer_invited: bool,
    pub summary: TournamentCardSummary,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentBrowsePage {
    pub tournaments: Vec<TournamentCardResponse>,
    pub total: u32,
    pub page: usize,
}

#[cfg(feature = "ssr")]
fn viewer_relationships(
    tournament_id: Uuid,
    joined: &HashSet<Uuid>,
    invited: &HashSet<Uuid>,
    organized: &HashSet<Uuid>,
) -> (bool, bool, bool) {
    (
        joined.contains(&tournament_id),
        invited.contains(&tournament_id),
        organized.contains(&tournament_id),
    )
}

#[derive(Clone, Copy, Debug)]
pub enum TournamentAdmissionViewer {
    Pending,
    Guest,
    User { bot: bool, rating: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct TournamentAdmission {
    pub entry_open: bool,
    pub full: bool,
    pub restrictions: AdmissionRestrictions,
    pub relationship: ViewerRelationship,
    pub clock: Option<Clock>,
    pub bot_admission: BotAdmission,
}

impl TournamentAdmission {
    /// Presentation of the server admission policy; mutations still revalidate in the database.
    pub fn decision(self, viewer: TournamentAdmissionViewer) -> TournamentAccessState {
        use TournamentAccessState as Access;
        if matches!(viewer, TournamentAdmissionViewer::Pending) {
            return Access::AuthPending;
        }
        if self.relationship.joined {
            return Access::Joined;
        }
        if !self.entry_open {
            return Access::EntryClosed;
        }
        let TournamentAdmissionViewer::User { bot, rating } = viewer else {
            return Access::LoginRequired;
        };
        if self.full {
            return Access::Full;
        }
        if self.restrictions.invite_only
            && !self.relationship.invited
            && !self.relationship.organizing
        {
            return Access::InvitationRequired;
        }
        if self.clock.is_none() {
            return Access::MissingClock;
        }
        match self.bot_admission {
            BotAdmission::HumansOnly if bot => return Access::HumansOnly,
            BotAdmission::BotsOnly if !bot => return Access::BotsOnly,
            _ => (),
        }
        if let Some(lower) = self
            .restrictions
            .band_lower
            .filter(|lower| rating < f64::from(*lower))
        {
            return Access::RatingBelow(lower);
        }
        if let Some(upper) = self
            .restrictions
            .band_upper
            .filter(|upper| rating > f64::from(*upper))
        {
            return Access::RatingAbove(upper);
        }
        if self.relationship.invited {
            Access::PendingInvitation
        } else {
            Access::Open
        }
    }
}

impl TournamentAccessState {
    pub const fn can_enter(self) -> bool {
        matches!(self, Self::Open | Self::PendingInvitation)
    }

    // TODO: i18n once copy is approved.
    pub fn label(self) -> String {
        match self {
            Self::Open => String::from("Open"),
            Self::PendingInvitation => String::from("Invited"),
            Self::Joined => String::from("Joined"),
            Self::AuthPending => String::from("Checking your account…"),
            Self::LoginRequired => String::from("Log in to join"),
            Self::InvitationRequired => String::from("Invitation required"),
            Self::RatingBelow(lower) => format!("Rating must be at least {lower}"),
            Self::RatingAbove(upper) => format!("Rating must be at most {upper}"),
            Self::HumansOnly => String::from("Human accounts only"),
            Self::BotsOnly => String::from("Bot accounts only"),
            Self::MissingClock => String::from("Tournament clock is missing"),
            Self::Full => String::from("Full"),
            Self::EntryClosed => String::from("Entry closed"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TournamentAbstractResponse {
    pub tournament_id: TournamentId,
    pub name: String,
    pub players: u32,
    pub joined: bool,
    pub invited: bool,
    pub organizing: bool,
    pub seats: Option<i32>,
    pub invite_only: bool,
    pub configuration: Config,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TournamentResponse {
    pub tournament_id: TournamentId,
    pub name: String,
    pub description: Option<String>,
    pub invitees: Vec<UserResponse>,
    pub declined_invitees: Vec<UserResponse>,
    pub players: HashMap<Uuid, UserResponse>,
    pub pairing_numbers: HashMap<Uuid, usize>,
    pub organizers: Vec<UserResponse>,
    pub organizer_invitees: Vec<UserResponse>,
    pub withdrawn: HashSet<Uuid>,
    pub status: TournamentStatus,
    pub bot_admission: BotAdmission,
    pub format: TournamentFormatResponse,
    pub player_stats: Vec<PlayerStatsResponse>,
    pub standings: Option<Snapshot>,
    pub seats: Option<i32>,
    pub min_seats: i32,
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub start_setup: Option<TournamentStartSetup>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TournamentLifecycleDetails {
    pub tournament_id: TournamentId,
    pub name: String,
    pub description: Option<String>,
    pub status: TournamentStatus,
    pub seats: Option<i32>,
    pub min_seats: i32,
    pub invite_only: bool,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub start_setup: Option<TournamentStartSetup>,
}

impl TournamentLifecycleDetails {
    pub fn from_response(response: &TournamentResponse) -> Self {
        Self {
            tournament_id: response.tournament_id.clone(),
            name: response.name.clone(),
            description: response.description.clone(),
            status: response.status,
            seats: response.seats,
            min_seats: response.min_seats,
            invite_only: response.invite_only,
            band_upper: response.band_upper,
            band_lower: response.band_lower,
            starts_at: response.starts_at,
            started_at: response.started_at,
            finished_at: response.finished_at,
            created_at: response.created_at,
            start_setup: response.start_setup.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TournamentMemberships {
    pub invitees: Vec<UserResponse>,
    pub declined_invitees: Vec<UserResponse>,
    pub players: HashMap<Uuid, UserResponse>,
    pub pairing_numbers: HashMap<Uuid, usize>,
    pub organizers: Vec<UserResponse>,
    pub organizer_invitees: Vec<UserResponse>,
    pub withdrawn: HashSet<Uuid>,
}

impl TournamentMemberships {
    pub fn from_response(response: &TournamentResponse) -> Self {
        Self {
            invitees: response.invitees.clone(),
            declined_invitees: response.declined_invitees.clone(),
            players: response.players.clone(),
            pairing_numbers: response.pairing_numbers.clone(),
            organizers: response.organizers.clone(),
            organizer_invitees: response.organizer_invitees.clone(),
            withdrawn: response.withdrawn.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TournamentStandings {
    pub player_stats: Vec<PlayerStatsResponse>,
    pub snapshot: Option<Snapshot>,
}

impl TournamentStandings {
    pub fn from_response(response: &TournamentResponse) -> Self {
        Self {
            player_stats: response.player_stats.clone(),
            snapshot: response.standings.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TournamentPatch {
    ArenaBerserked {
        game_id: GameId,
        color: Color,
    },
    ArenaGameUpsert(ArenaGameResponse),
    ArenaPlayerStatsUpsert(ArenaPlayerStatsResponse),
    ArenaFeaturedGameChanged(Option<GameId>),
    SlotUpsert(SlotResponse),
    MembershipsReplace(TournamentMemberships),
    StandingsReplace(TournamentStandings),
    RoundRobinAvailabilityReplace {
        withdrawable_entrants: HashSet<Uuid>,
        closeout_eligible_slots: u32,
    },
    SwissAvailabilityReplace {
        withdrawable_entrants: HashSet<Uuid>,
        closeout_eligible_slots: u32,
    },
    EliminationAvailabilityReplace {
        withdrawable_entrants: HashSet<Uuid>,
    },
    FormatReplace(TournamentFormatResponse),
    LifecycleDetailsReplace(TournamentLifecycleDetails),
    DescriptionChanged(Option<String>),
}

fn summary_clock(configuration: &Config) -> Option<Clock> {
    match &configuration.format {
        FormatConfig::RoundRobin(configuration) => Some(configuration.clock),
        FormatConfig::Swiss(configuration) => Some(configuration.clock),
        FormatConfig::Elimination(_) => None,
        FormatConfig::Arena(configuration) => Some(Clock::Realtime(configuration.game_clock)),
    }
}

pub(crate) fn rating_clock(configuration: &FormatConfig) -> Option<Clock> {
    match configuration {
        FormatConfig::RoundRobin(configuration) => Some(configuration.clock),
        FormatConfig::Swiss(configuration) => Some(configuration.clock),
        FormatConfig::Elimination(configuration) => configuration
            .default_plan
            .phases
            .first()
            .map(|phase| phase.clock),
        FormatConfig::Arena(configuration) => Some(Clock::Realtime(configuration.game_clock)),
    }
}

fn arena_duration_seconds(configuration: &Config) -> Option<u32> {
    match &configuration.format {
        FormatConfig::Arena(configuration) => Some(configuration.duration_seconds.get()),
        _ => None,
    }
}

impl TournamentAbstractResponse {
    pub const fn format(&self) -> Format {
        self.configuration.format()
    }

    pub fn admission_clock(&self) -> Option<Clock> {
        rating_clock(&self.configuration.format)
    }

    pub fn summary_clock(&self) -> Option<Clock> {
        summary_clock(&self.configuration)
    }

    pub fn arena_duration_seconds(&self) -> Option<u32> {
        arena_duration_seconds(&self.configuration)
    }

    pub const fn scheduled_starts_at(&self) -> Option<DateTime<Utc>> {
        self.starts_at
    }
}

impl TournamentCardResponse {
    pub const fn format(&self) -> Format {
        self.summary.format
    }
}

impl TournamentResponse {
    pub fn fixed_slots(&self) -> Vec<&SlotResponse> {
        match &self.format {
            TournamentFormatResponse::RoundRobin { rounds, .. } => rounds
                .iter()
                .flat_map(|round| round.slots.iter().map(|slot| &slot.slot))
                .collect(),
            TournamentFormatResponse::Swiss { rounds, .. } => rounds
                .iter()
                .flat_map(|round| &round.encounters)
                .flat_map(|encounter| &encounter.slots)
                .collect(),
            TournamentFormatResponse::Elimination { nodes, .. } => nodes
                .iter()
                .filter_map(|node| node.series.as_ref())
                .flat_map(|series| &series.sets)
                .flat_map(|set| &set.slots)
                .map(|slot| &slot.slot)
                .collect(),
            TournamentFormatResponse::Arena { .. } => Vec::new(),
        }
    }

    pub fn arena_games(&self) -> &[ArenaGameResponse] {
        match &self.format {
            TournamentFormatResponse::Arena { games, .. } => games,
            _ => &[],
        }
    }
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use anyhow::{Error, Result};
use shared_types::GameSpeed;
use db_lib::models::User;
#[cfg(feature = "ssr")]
use db_lib::{
    models::{Tournament, TournamentEliminationNode},
    tournaments::{
        public::{
            fixed_card_progress_for_tournaments,
            FixedTournamentCardProgress,
            TournamentSnapshot,
        },
    },
    DbConn,
};
use diesel::pg::Pg;
use db_lib::schema::tournaments::BoxedQuery;
fn pairing_number_projection(
    memberships: impl IntoIterator<Item = (Uuid, Option<i32>)>,
) -> Result<HashMap<Uuid, usize>> {
    memberships
        .into_iter()
        .filter_map(|(user_id, pairing_number)| {
            pairing_number.map(|pairing_number| {
                usize::try_from(pairing_number)
                    .map(|pairing_number| (user_id, pairing_number))
                    .map_err(Error::from)
            })
        })
        .collect()
}


fn tournament_list_query(
    category: TournamentCategory,
    viewer_id: Option<Uuid>,
    search_pattern: Option<String>,
) -> BoxedQuery<'static, Pg> {
    use db_lib::schema::{
        tournaments,
        tournaments_invitations,
        tournaments_organizer_invitations,
        tournaments_organizers,
        tournaments_users,
    };
    use diesel::{
        dsl::exists,
        BoolExpressionMethods,
        ExpressionMethods,
        PgTextExpressionMethods,
        QueryDsl,
    };

    let mut query = tournaments::table.into_boxed();
    query = match category {
        TournamentCategory::Upcoming => query
            .filter(tournaments::started_at.is_null())
            .filter(tournaments::finished_at.is_null()),
        TournamentCategory::InProgress => query
            .filter(tournaments::started_at.is_not_null())
            .filter(tournaments::finished_at.is_null()),
        TournamentCategory::Finished => query.filter(tournaments::finished_at.is_not_null()),
        TournamentCategory::Joined => {
            let viewer_id = viewer_id.expect("joined tournaments require a viewer");
            query
                .filter(tournaments::finished_at.is_null())
                .filter(exists(
                    tournaments_users::table
                        .filter(tournaments_users::user_id.eq(viewer_id))
                        .filter(tournaments_users::tournament_id.eq(tournaments::id)),
                ))
        }
        TournamentCategory::Organizing => {
            let viewer_id = viewer_id.expect("organized tournaments require a viewer");
            query
                .filter(tournaments::finished_at.is_null())
                .filter(exists(
                    tournaments_organizers::table
                        .filter(tournaments_organizers::organizer_id.eq(viewer_id))
                        .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
                ))
        }
        TournamentCategory::Invitations => {
            let viewer_id = viewer_id.expect("tournament invitations require a viewer");
            query
                .filter(tournaments::finished_at.is_null())
                .filter(exists(
                    tournaments_invitations::table
                        .filter(tournaments_invitations::invitee_id.eq(viewer_id))
                        .filter(tournaments_invitations::declined_at.is_null())
                        .filter(tournaments_invitations::tournament_id.eq(tournaments::id)),
                ).or(exists(
                    tournaments_organizer_invitations::table
                        .filter(tournaments_organizer_invitations::invitee_id.eq(viewer_id))
                        .filter(tournaments_organizer_invitations::tournament_id.eq(tournaments::id)),
                )))
        }
        TournamentCategory::History => {
            let viewer_id = viewer_id.expect("tournament history requires a viewer");
            query.filter(tournaments::finished_at.is_not_null()).filter(
                exists(
                    tournaments_users::table
                        .filter(tournaments_users::user_id.eq(viewer_id))
                        .filter(tournaments_users::tournament_id.eq(tournaments::id)),
                )
                .or(exists(
                    tournaments_organizers::table
                        .filter(tournaments_organizers::organizer_id.eq(viewer_id))
                        .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
                )),
            )
        }
    };
    if let Some(search_pattern) = search_pattern {
        query = query.filter(tournaments::name.ilike(search_pattern));
    }
    query
}

impl TournamentBrowsePage {
    pub async fn load(
        category: TournamentCategory,
        viewer_id: Option<Uuid>,
        search: &str,
        requested_page: usize,
        conn: &mut DbConn<'_>,
    ) -> Result<Self> {
        use db_lib::schema::tournaments;
        use diesel::{
            ExpressionMethods,
            PgSortExpressionMethods,
            QueryDsl,
        };
        use diesel_async::RunQueryDsl;

        let query_viewer_id = if category.requires_viewer() {
            Some(viewer_id.ok_or_else(|| anyhow::anyhow!("authentication required"))?)
        } else {
            None
        };

        let search = search.trim();
        let search_pattern = (!search.is_empty()).then(|| format!("%{search}%"));
        let count_query =
            tournament_list_query(category, query_viewer_id, search_pattern.clone());
        let mut page_query = tournament_list_query(category, query_viewer_id, search_pattern);

        let total = usize::try_from(count_query.count().get_result::<i64>(conn).await?)?;
        let total_pages = total.max(1).div_ceil(TOURNAMENT_BROWSE_PAGE_SIZE);
        let page = requested_page.max(1).min(total_pages);
        let offset = i64::try_from((page - 1) * TOURNAMENT_BROWSE_PAGE_SIZE)?;
        let limit = i64::try_from(TOURNAMENT_BROWSE_PAGE_SIZE)?;
        page_query = match category {
            TournamentCategory::Upcoming => page_query.order((
                tournaments::starts_at.asc().nulls_last(),
                tournaments::created_at.desc(),
            )),
            TournamentCategory::InProgress => page_query.order((
                tournaments::started_at.desc(),
                tournaments::created_at.desc(),
            )),
            TournamentCategory::Joined | TournamentCategory::Organizing => page_query.order((
                tournaments::started_at.desc().nulls_last(),
                tournaments::starts_at.asc().nulls_last(),
                tournaments::created_at.desc(),
            )),
            TournamentCategory::Invitations => page_query.order((
                tournaments::starts_at.asc().nulls_last(),
                tournaments::created_at.desc(),
            )),
            TournamentCategory::History => page_query.order((
                tournaments::finished_at.desc(),
                tournaments::created_at.desc(),
            )),
            TournamentCategory::Finished => page_query.order((
                tournaments::finished_at.desc(),
                tournaments::created_at.desc(),
            )),
        };
        let models = page_query.limit(limit).offset(offset).load::<Tournament>(conn).await?;
        let tournaments = TournamentCardResponse::from_models(&models, viewer_id, conn).await?;

        Ok(Self {
            tournaments,
            total: u32::try_from(total)?,
            page,
        })
    }
}

struct TournamentRelations {
    player_counts: HashMap<Uuid, i64>,
    organized_tournaments: HashSet<Uuid>,
    joined_tournaments: HashSet<Uuid>,
    invited_tournaments: HashSet<Uuid>,
}

async fn tournament_relations(tournament_ids: &[Uuid], viewer_id: Option<Uuid>, conn: &mut DbConn<'_>) -> Result<TournamentRelations> {
    use db_lib::schema::{tournaments_invitations, tournaments_organizers, tournaments_users};
    use diesel::{dsl::count_star, ExpressionMethods, QueryDsl};
    use diesel_async::RunQueryDsl;
        let player_counts = tournaments_users::table
            .filter(tournaments_users::tournament_id.eq_any(tournament_ids))
            .group_by(tournaments_users::tournament_id)
            .select((tournaments_users::tournament_id, count_star()))
            .load::<(Uuid, i64)>(conn)
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();

        let organized_tournaments = match viewer_id {
            Some(viewer_id) => tournaments_organizers::table
                .filter(tournaments_organizers::tournament_id.eq_any(tournament_ids))
                .filter(tournaments_organizers::organizer_id.eq(viewer_id))
                .select(tournaments_organizers::tournament_id)
                .load::<Uuid>(conn)
                .await?
                .into_iter()
                .collect::<HashSet<_>>(),
            None => HashSet::new(),
        };
        let joined_tournaments = match viewer_id {
            Some(viewer_id) => tournaments_users::table
                .filter(tournaments_users::tournament_id.eq_any(tournament_ids))
                .filter(tournaments_users::user_id.eq(viewer_id))
                .select(tournaments_users::tournament_id)
                .load::<Uuid>(conn)
                .await?
                .into_iter()
                .collect::<HashSet<_>>(),
            None => HashSet::new(),
        };
        let invited_tournaments = match viewer_id {
            Some(viewer_id) => tournaments_invitations::table
                .filter(tournaments_invitations::tournament_id.eq_any(tournament_ids))
                .filter(tournaments_invitations::invitee_id.eq(viewer_id))
                .filter(tournaments_invitations::declined_at.is_null())
                .select(tournaments_invitations::tournament_id)
                .load::<Uuid>(conn)
                .await?
                .into_iter()
                .collect::<HashSet<_>>(),
            None => HashSet::new(),
        };
    Ok(TournamentRelations { player_counts, organized_tournaments, joined_tournaments, invited_tournaments })
}

impl TournamentCardResponse {
    async fn from_models(
        tournaments: &[Tournament],
        viewer_id: Option<Uuid>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>> {
        use db_lib::schema::{
            ratings,
            tournaments_organizer_invitations,
        };
        use diesel::{ExpressionMethods, QueryDsl};
        use diesel_async::RunQueryDsl;

        if tournaments.is_empty() {
            return Ok(Vec::new());
        }

        let tournament_ids = tournaments
            .iter()
            .map(|tournament| tournament.id)
            .collect::<Vec<_>>();
        let TournamentRelations { player_counts, organized_tournaments, joined_tournaments, invited_tournaments } = tournament_relations(&tournament_ids, viewer_id, conn).await?;
        let organizer_invited_tournaments = match viewer_id {
            Some(viewer_id) => tournaments_organizer_invitations::table
                .filter(tournaments_organizer_invitations::tournament_id.eq_any(&tournament_ids))
                .filter(tournaments_organizer_invitations::invitee_id.eq(viewer_id))
                .select(tournaments_organizer_invitations::tournament_id)
                .load::<Uuid>(conn)
                .await?
                .into_iter()
                .collect::<HashSet<_>>(),
            None => HashSet::new(),
        };
        let viewer_ratings = match viewer_id {
            Some(viewer_id) => ratings::table
                .filter(ratings::user_uid.eq(viewer_id))
                .select((ratings::speed, ratings::rating))
                .load::<(String, f64)>(conn)
                .await?
                .into_iter()
                .collect::<HashMap<_, _>>(),
            None => HashMap::new(),
        };

        let viewer_bot = match viewer_id {
            Some(viewer_id) => Some(User::find_active_by_uuid(&viewer_id, conn).await?.bot),
            None => None,
        };

        let fixed_progress = fixed_card_progress_for_tournaments(tournaments, conn).await?;
        let elimination_nodes =
            TournamentEliminationNode::progress_for_tournaments(&tournament_ids, conn).await?;

        let now = Utc::now();
        tournaments
            .iter()
            .map(|tournament| {
                let players = u32::try_from(
                    player_counts.get(&tournament.id).copied().unwrap_or_default(),
                )?;
                let configuration = tournament.configuration();
                let progress = tournament_card_progress(
                    tournament,
                    configuration,
                    players,
                    fixed_progress.get(&tournament.id),
                    elimination_nodes
                        .get(&tournament.id)
                        .map(|progress| (progress.decided_nodes, progress.total_nodes))
                        .unwrap_or_default(),
                    now,
                )?;
                let (joined, invited, organizing) = viewer_relationships(
                    tournament.id,
                    &joined_tournaments,
                    &invited_tournaments,
                    &organized_tournaments,
                );
                let primary_clock = rating_clock(&configuration.format);
                let viewer_rating = primary_clock
                    .map(GameSpeed::from)
                    .and_then(|speed| viewer_ratings.get(&speed.to_string()).copied());
                let joining_relevant = !tournament.start_setup().is_some_and(|setup| setup.active_at(now))
                    && tournament.status() == TournamentStatus::NotStarted
                    && tournament.starts_at.is_none_or(|starts_at| starts_at > now);
                let full = tournament.seats.is_some_and(|capacity| {
                    players
                        >= u32::try_from(capacity)
                            .expect("validated tournament capacity is nonnegative")
                });
                let restrictions = AdmissionRestrictions {
                    invite_only: tournament.invite_only,
                    band_upper: tournament.band_upper,
                    band_lower: tournament.band_lower,
                };
                let viewer = ViewerRelationship {
                    joined,
                    invited,
                    organizing,
                };
                let admission_viewer = match viewer_bot {
                    Some(bot) => TournamentAdmissionViewer::User {
                        bot,
                        rating: viewer_rating.unwrap_or_default(),
                    },
                    None => TournamentAdmissionViewer::Guest,
                };
                let access = (joining_relevant || invited || joined).then(|| TournamentAdmission {
                    entry_open: joining_relevant,
                    full: full && configuration.format() != Format::Arena,
                    restrictions,
                    relationship: viewer,
                    clock: primary_clock,
                    bot_admission: configuration.bot_admission,
                }.decision(admission_viewer));
                let relevant_date = tournament.finished_at.map(TournamentCardDate::Finished)
                    .or_else(|| tournament.started_at.map(TournamentCardDate::Started))
                    .or_else(|| tournament.starts_at.map(TournamentCardDate::Starts));
                Ok(Self {
                    tournament_id: TournamentId(tournament.nanoid.clone()),
                    name: tournament.name.clone(),
                    players,
                    seats: tournament.seats,
                    invited,
                    organizer_invited: organizer_invited_tournaments.contains(&tournament.id),
                    summary: TournamentCardSummary {
                        format: configuration.format(),
                        primary_clock,
                        access,
                        progress,
                        relevant_date,
                    },
                })
            })
            .collect()
    }
}

fn tournament_card_progress(
    tournament: &Tournament,
    configuration: &Config,
    players: u32,
    fixed_progress: Option<&FixedTournamentCardProgress>,
    elimination_nodes: (usize, usize),
    now: DateTime<Utc>,
) -> Result<TournamentCardProgress> {
    if tournament.status() == TournamentStatus::Finished {
        return Ok(TournamentCardProgress::Complete);
    }
    if tournament.status() == TournamentStatus::NotStarted {
        return Ok(TournamentCardProgress::Entrants {
            current: players,
            minimum: u32::try_from(tournament.min_seats)
                .expect("validated tournament minimum seats are nonnegative"),
            capacity: tournament.seats.map(|seats| {
                u32::try_from(seats).expect("validated tournament capacity is nonnegative")
            }),
        });
    }

    Ok(match &configuration.format {
        FormatConfig::Arena(configuration) => {
            let duration_seconds = configuration.duration_seconds.get();
            let elapsed_seconds = tournament
                .started_at
                .map(|started_at| {
                    (now - started_at)
                        .num_seconds()
                        .clamp(0, i64::from(duration_seconds)) as u32
                })
                .unwrap_or_default();
            TournamentCardProgress::Arena {
                elapsed_seconds,
                duration_seconds,
            }
        }
        FormatConfig::RoundRobin(_) => {
            let Some(FixedTournamentCardProgress::RoundRobin { resolved, total }) = fixed_progress
            else {
                return Err(Error::msg("Round Robin card progress is unavailable"));
            };
            TournamentCardProgress::Slots {
                resolved: *resolved,
                total: *total,
            }
        }
        FormatConfig::Swiss(_) => {
            let Some(FixedTournamentCardProgress::Swiss {
                completed_rounds,
                total_rounds,
                current_round,
            }) = fixed_progress
            else {
                return Err(Error::msg("Swiss card progress is unavailable"));
            };
            TournamentCardProgress::Swiss {
                completed_rounds: *completed_rounds,
                total_rounds: *total_rounds,
                current_round: *current_round,
            }
        }
        FormatConfig::Elimination(_) => TournamentCardProgress::Elimination {
            decided_nodes: u32::try_from(elimination_nodes.0)?,
            total_nodes: u32::try_from(elimination_nodes.1)?,
        },
    })
}

impl TournamentAbstractResponse {
    pub async fn from_models(
        tournaments: &[Tournament],
        viewer_id: Option<Uuid>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>> {

        if tournaments.is_empty() {
            return Ok(Vec::new());
        }

        let tournament_ids = tournaments
            .iter()
            .map(|tournament| tournament.id)
            .collect::<Vec<_>>();
        let TournamentRelations { player_counts, organized_tournaments, joined_tournaments, invited_tournaments } = tournament_relations(&tournament_ids, viewer_id, conn).await?;

        tournaments
            .iter()
            .map(|tournament| {
                let players = player_counts.get(&tournament.id).copied().unwrap_or_default();
                let (joined, invited, organizing) = viewer_relationships(
                    tournament.id,
                    &joined_tournaments,
                    &invited_tournaments,
                    &organized_tournaments,
                );
                Ok(Self {
                    tournament_id: TournamentId(tournament.nanoid.clone()),
                    name: tournament.name.clone(),
                    players: u32::try_from(players)?,
                    joined,
                    invited,
                    organizing,
                    seats: tournament.seats,
                    invite_only: tournament.invite_only,
                    configuration: tournament.configuration().clone(),
                    band_upper: tournament.band_upper,
                    band_lower: tournament.band_lower,
                    starts_at: tournament.starts_at,
                    started_at: tournament.started_at,
                    finished_at: tournament.finished_at,
                })
            })
            .collect()
    }
}

impl TournamentResponse {
    pub fn from_snapshot(snapshot: TournamentSnapshot) -> Result<Box<Self>> {
        let bot_admission = snapshot.tournament.configuration().bot_admission;
        let users = UserResponse::from_models_with_ratings(&snapshot.users, &snapshot.ratings)?;
        let response_for = |user_id| {
            users.get(&user_id).cloned().ok_or_else(|| {
                anyhow::anyhow!("tournament member {user_id} has no user response")
            })
        };
        let invitees = snapshot
            .invitations
            .iter()
            .filter(|row| row.declined_at.is_none())
            .map(|row| response_for(row.invitee_id))
            .collect::<Result<_>>()?;
        let declined_invitees = snapshot
            .invitations
            .iter()
            .filter(|row| row.declined_at.is_some())
            .map(|row| response_for(row.invitee_id))
            .collect::<Result<_>>()?;
        let organizer_invitees = snapshot.organizer_invitee_ids.iter()
            .map(|id| response_for(*id))
            .collect::<Result<_>>()?;
        let organizers = snapshot
            .organizer_ids
            .iter()
            .map(|id| response_for(*id))
            .collect::<Result<_>>()?;
        let players = snapshot
            .memberships
            .iter()
            .map(|row| Ok((row.user_id, response_for(row.user_id)?)))
            .collect::<Result<HashMap<_, _>>>()?;
        let pairing_numbers = pairing_number_projection(
            snapshot
                .memberships
                .iter()
                .map(|row| (row.user_id, row.pairing_number)),
        )?;
        let withdrawn = snapshot
            .memberships
            .iter()
            .filter(|row| row.withdrawn_at.is_some())
            .map(|row| row.user_id)
            .collect();
        let status = snapshot.tournament.status();
        let start_setup = snapshot.tournament.start_setup();
        Ok(Box::new(Self {
            tournament_id: TournamentId(snapshot.tournament.nanoid),
            name: snapshot.tournament.name,
            description: snapshot.tournament.description,
            invitees,
            declined_invitees,
            players,
            pairing_numbers,
            organizers,
            organizer_invitees,
            withdrawn,
            status,
            bot_admission,
            format: snapshot.format,
            player_stats: snapshot.player_stats,
            standings: snapshot.standings,
            seats: snapshot.tournament.seats,
            min_seats: snapshot.tournament.min_seats,
            invite_only: snapshot.tournament.invite_only,
            band_upper: snapshot.tournament.band_upper,
            band_lower: snapshot.tournament.band_lower,
            starts_at: snapshot.tournament.starts_at,
            started_at: snapshot.tournament.started_at,
            finished_at: snapshot.tournament.finished_at,
            created_at: snapshot.tournament.created_at,
            start_setup,
        }))
    }
}

}}
