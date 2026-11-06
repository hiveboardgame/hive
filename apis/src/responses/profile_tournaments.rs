use super::TournamentCardResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ProfileTournamentRole {
    pub playing: bool,
    pub organizing: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ProfileTournamentResult {
    Ranked { rank: u32, total: u32 },
    Withdrawn,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProfileTournamentCard {
    pub card: TournamentCardResponse,
    pub role: ProfileTournamentRole,
    pub result: Option<ProfileTournamentResult>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProfileTournamentsPage {
    pub active: Vec<ProfileTournamentCard>,
    pub finished: Vec<ProfileTournamentCard>,
    pub finished_total: u32,
    pub page: usize,
    pub invitations: Option<Vec<String>>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct MyTournamentCounts {
    pub organizing: u32,
    pub invitations: u32,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use super::{
    tournament::{tournament_list_query, tournament_relations, TournamentRelations},
    TournamentCategory,
    TournamentTimeFilter,
    TOURNAMENT_BROWSE_PAGE_SIZE,
};
use crate::common::display_standing_rows;
use anyhow::Result;
use db_lib::{
    models::{Tournament, TournamentFinalOutcome},
    schema::{tournaments, tournaments_organizers, tournaments_users},
    DbConn,
};
use diesel::{
    dsl::exists,
    BoolExpressionMethods,
    ExpressionMethods,
    PgSortExpressionMethods,
    QueryDsl,
};
use diesel_async::RunQueryDsl;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn member_tournaments(user_id: Uuid) -> tournaments::BoxedQuery<'static, diesel::pg::Pg> {
    tournaments::table
        .filter(
            exists(
                tournaments_users::table
                    .filter(tournaments_users::user_id.eq(user_id))
                    .filter(tournaments_users::tournament_id.eq(tournaments::id)),
            )
            .or(exists(
                tournaments_organizers::table
                    .filter(tournaments_organizers::organizer_id.eq(user_id))
                    .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
            )),
        )
        .into_boxed()
}

async fn withdrawn_by_tournament(
    tournament_ids: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<HashMap<Uuid, HashSet<Uuid>>> {
    let mut withdrawn: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    for (tournament_id, user_id) in tournaments_users::table
        .filter(tournaments_users::tournament_id.eq_any(tournament_ids))
        .filter(tournaments_users::withdrawn_at.is_not_null())
        .select((tournaments_users::tournament_id, tournaments_users::user_id))
        .load::<(Uuid, Uuid)>(conn)
        .await?
    {
        withdrawn.entry(tournament_id).or_default().insert(user_id);
    }
    Ok(withdrawn)
}

fn profile_result(
    outcome: &TournamentFinalOutcome,
    withdrawn: &HashSet<Uuid>,
    user_id: Uuid,
) -> Option<ProfileTournamentResult> {
    let rows = display_standing_rows(&outcome.standings, withdrawn);
    let total = u32::try_from(rows.iter().filter(|row| !row.withdrawn).count()).ok()?;
    let standing = rows.iter().find(|row| row.row.user_id == user_id)?;
    if standing.withdrawn {
        return Some(ProfileTournamentResult::Withdrawn);
    }
    standing
        .rank
        .map(|rank| ProfileTournamentResult::Ranked { rank, total })
}

impl ProfileTournamentsPage {
    pub async fn load(
        user_id: Uuid,
        viewer_id: Option<Uuid>,
        requested_page: usize,
        conn: &mut DbConn<'_>,
    ) -> Result<Self> {
        let active_models = member_tournaments(user_id)
            .filter(tournaments::finished_at.is_null())
            .order((
                tournaments::started_at.desc().nulls_last(),
                tournaments::starts_at.asc().nulls_last(),
                tournaments::created_at.desc(),
                tournaments::id.asc(),
            ))
            .load::<Tournament>(conn)
            .await?;

        let finished_total = usize::try_from(
            member_tournaments(user_id)
                .filter(tournaments::finished_at.is_not_null())
                .count()
                .get_result::<i64>(conn)
                .await?,
        )?;
        let total_pages = finished_total.max(1).div_ceil(TOURNAMENT_BROWSE_PAGE_SIZE);
        let page = requested_page.max(1).min(total_pages);
        let finished_models = member_tournaments(user_id)
            .filter(tournaments::finished_at.is_not_null())
            .order((
                tournaments::finished_at.desc(),
                tournaments::created_at.desc(),
                tournaments::id.asc(),
            ))
            .limit(i64::try_from(TOURNAMENT_BROWSE_PAGE_SIZE)?)
            .offset(i64::try_from((page - 1) * TOURNAMENT_BROWSE_PAGE_SIZE)?)
            .load::<Tournament>(conn)
            .await?;

        let mut models = active_models;
        let active_count = models.len();
        models.extend(finished_models);
        let model_ids = models.iter().map(|tournament| tournament.id).collect::<Vec<_>>();
        let finished_ids = model_ids[active_count..].to_vec();

        let cards = TournamentCardResponse::from_models(&models, viewer_id, conn).await?;
        let TournamentRelations {
            joined_tournaments,
            organized_tournaments,
            ..
        } = tournament_relations(&model_ids, Some(user_id), conn).await?;
        let outcomes = TournamentFinalOutcome::load_many(&finished_ids, conn)
            .await?
            .into_iter()
            .map(|outcome| (outcome.tournament_id, outcome))
            .collect::<HashMap<_, _>>();
        let withdrawn = withdrawn_by_tournament(&finished_ids, conn).await?;
        let nobody_withdrawn = HashSet::new();

        let mut profile_cards = models
            .iter()
            .zip(cards)
            .map(|(tournament, card)| {
                let role = ProfileTournamentRole {
                    playing: joined_tournaments.contains(&tournament.id),
                    organizing: organized_tournaments.contains(&tournament.id),
                };
                let result = role
                    .playing
                    .then(|| outcomes.get(&tournament.id))
                    .flatten()
                    .and_then(|outcome| {
                        profile_result(
                            outcome,
                            withdrawn.get(&tournament.id).unwrap_or(&nobody_withdrawn),
                            user_id,
                        )
                    });
                ProfileTournamentCard { card, role, result }
            })
            .collect::<Vec<_>>();
        let finished = profile_cards.split_off(active_count);

        let invitations = match viewer_id {
            Some(viewer_id) if viewer_id == user_id => Some(
                tournament_list_query(
                    TournamentCategory::Invitations,
                    Some(viewer_id),
                    None,
                    None,
                    TournamentTimeFilter::Both,
                )
                .order(tournaments::created_at.desc())
                .select(tournaments::name)
                .load::<String>(conn)
                .await?,
            ),
            _ => None,
        };

        Ok(Self {
            active: profile_cards,
            finished,
            finished_total: u32::try_from(finished_total)?,
            page,
            invitations,
        })
    }
}

async fn category_count(
    category: TournamentCategory,
    viewer_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<u32> {
    Ok(u32::try_from(
        tournament_list_query(category, Some(viewer_id), None, None, TournamentTimeFilter::Both)
            .count()
            .get_result::<i64>(conn)
            .await?,
    )?)
}

impl MyTournamentCounts {
    pub async fn load(viewer_id: Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
        Ok(Self {
            organizing: category_count(TournamentCategory::Organizing, viewer_id, conn).await?,
            invitations: category_count(TournamentCategory::Invitations, viewer_id, conn).await?,
        })
    }
}
}}
