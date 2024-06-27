use super::{Game, NewGame, TournamentInvitation};
use crate::{
    db_error::DbError,
    models::{
        tournament_organizer::TournamentOrganizer, tournament_user::TournamentUser, user::User,
    },
    schema::{
        games::{self, tournament_id as tournament_id_column},
        tournaments::{
            self, nanoid as nanoid_field, series as series_column, status as status_column,
            updated_at,
        },
        tournaments_organizers, users,
    },
    DbConn,
};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use itertools::Itertools;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::{TimeMode, TournamentDetails, TournamentId, TournamentStatus};
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tournaments)]
pub struct NewTournament {
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub rounds: i32,
    pub joinable: bool,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

impl NewTournament {
    pub fn new(details: TournamentDetails) -> Result<Self, DbError> {
        if matches!(details.time_mode, TimeMode::Untimed) {
            return Err(DbError::InvalidInput {
                info: String::from("How did you trigger this?"),
                error: String::from("Cannot create untimed tournament."),
            });
        }

        // TOOD: @leex add some validations

        Ok(Self {
            nanoid: nanoid!(11),
            name: details.name,
            description: details.description,
            scoring: details.scoring,
            tiebreaker: details.tiebreaker,
            seats: details.seats,
            rounds: details.rounds,
            joinable: details.joinable,
            invite_only: details.invite_only,
            mode: details.mode,
            time_mode: details.time_mode.to_string(),
            time_base: details.time_base,
            time_increment: details.time_increment,
            band_upper: details.band_upper,
            band_lower: details.band_lower,
            start_at: details.start_at,
            status: TournamentStatus::NotStarted.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: details.series,
        })
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = tournaments)]
pub struct Tournament {
    pub id: Uuid,
    pub nanoid: String,
    pub name: String,
    pub description: String,
    pub scoring: String,
    pub tiebreaker: Vec<Option<String>>,
    pub seats: i32,
    pub rounds: i32,
    pub joinable: bool,
    pub invite_only: bool,
    pub mode: String,
    pub time_mode: String,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
    pub start_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub series: Option<Uuid>,
}

impl Tournament {
    pub async fn create(
        user_id: Uuid,
        new_tournament: &NewTournament,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        // TODO: create only works when user's rating is RANKABLE
        let tournament: Tournament = diesel::insert_into(tournaments::table)
            .values(new_tournament)
            .get_result(conn)
            .await?;
        let tournament_organizer = TournamentOrganizer::new(tournament.id, user_id);
        diesel::insert_into(tournaments_organizers::table)
            .values(tournament_organizer)
            .execute(conn)
            .await?;
        Ok(tournament)
    }

    pub async fn delete(&mut self, user_id: Uuid, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(&user_id, conn).await?;
        diesel::delete(tournaments::table.find(self.id))
            .execute(conn)
            .await?;
        Ok(())
    }

    fn ensure_not_started(&self) -> Result<(), DbError> {
        if self.status != TournamentStatus::NotStarted.to_string() {
            return Err(DbError::InvalidInput {
                info: format!("Tournament status is {}", self.status),
                error: String::from("Cannot start tournament a second time"),
            });
        }
        Ok(())
    }

    async fn ensure_user_is_organizer(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let organizers = self.organizers(conn).await?;
        if organizers.iter().any(|o| o.id == *user_id) {
            return Ok(());
        }
        Err(DbError::Unauthorized)
    }

    pub async fn create_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(user_id, conn).await?;
        if TournamentInvitation::exists(&self.id, invitee, conn).await? {
            return Ok(self.clone());
        }
        let invitation = TournamentInvitation::new(self.id, *invitee);
        invitation.insert(conn).await?;
        // Ok(diesel::update(tournaments::table.find(self.id))
        //     .set(invitees_column.eq(invited))
        //     .get_result(conn)
        //     .await?)
        //     TODO: maybe change updated_at
        Ok(self.clone())
    }

    pub async fn retract_invitation(
        &self,
        user_id: &Uuid,
        invitee: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(user_id, conn).await?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, invitee, conn).await {
            invitation.delete(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn decline_invitation(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn accept_invitation(
        &self,
        user_id: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
            let tournament_user = TournamentUser::new(self.id, *user_id);
            tournament_user.insert(conn).await?;
            Ok(self.clone())
        } else {
            Err(DbError::NotFound {
                reason: String::from("No invitation found"),
            })
        }
    }

    pub async fn add_to_series(
        &self,
        series_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(Some(series_id)))
            .get_result(conn)
            .await?)
    }

    pub async fn remove_from_series(&self, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(series_column.eq(None::<Uuid>))
            .get_result(conn)
            .await?)
    }

    pub async fn join(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        self.ensure_not_started()?;
        let players = self.players(conn).await?;
        if players.len() as i32 == self.seats {
            return Ok(self.clone());
        }
        if players.iter().any(|player| player.id == *user_id) {
            return Ok(self.clone());
        }
        if let Ok(invitation) = TournamentInvitation::find_by_ids(&self.id, user_id, conn).await {
            invitation.delete(conn).await?;
        }
        let tournament_user = TournamentUser::new(self.id, *user_id);
        tournament_user.insert(conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn leave(&self, user_id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        self.ensure_not_started()?;
        TournamentUser::delete(self.id, *user_id, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn kick(
        &self,
        organizer: &Uuid,
        player: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(organizer, conn).await?;
        TournamentUser::delete(self.id, *player, conn).await?;
        Ok(diesel::update(tournaments::table.find(self.id))
            .set(updated_at.eq(Utc::now()))
            .get_result(conn)
            .await?)
    }

    pub async fn from_uuid(uuid: &Uuid, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table.find(uuid).first(conn).await?)
    }

    pub async fn from_nanoid(nano: &str, conn: &mut DbConn<'_>) -> Result<Tournament, DbError> {
        Ok(tournaments::table
            .filter(nanoid_field.eq(nano))
            .first(conn)
            .await?)
    }

    pub async fn invitees(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentInvitation::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn players(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentUser::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn organizers(&self, conn: &mut DbConn<'_>) -> Result<Vec<User>, DbError> {
        Ok(TournamentOrganizer::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn games(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        Ok(games::table
            .filter(tournament_id_column.eq(Some(self.id)))
            .get_results(conn)
            .await?)
    }

    pub async fn start(
        &self,
        organizer: &Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(Tournament, Vec<Game>), DbError> {
        self.ensure_not_started()?;
        self.ensure_user_is_organizer(organizer, conn).await?;
        // Make sure all the conditions have been met
        // and then call different starts for different tournament types
        let games = self.round_robin_start(conn).await?;
        let tournament = diesel::update(self)
            .set((
                updated_at.eq(Utc::now()),
                status_column.eq(TournamentStatus::InProgress.to_string()),
            ))
            .get_result(conn)
            .await?;
        Ok((tournament, games))
    }

    pub async fn round_robin_start(&self, conn: &mut DbConn<'_>) -> Result<Vec<Game>, DbError> {
        let mut games = Vec::new();
        let players = self.players(conn).await?;
        let combinations: Vec<Vec<User>> = players.into_iter().combinations(2).collect();
        for combination in combinations {
            let white = combination[0].id;
            let black = combination[1].id;
            let new_game = NewGame::new_from_tournament(white, black, self);
            let game = Game::create(new_game, conn).await?;
            games.push(game);
            let new_game = NewGame::new_from_tournament(black, white, self);
            let game = Game::create(new_game, conn).await?;
            games.push(game);
        }
        Ok(games)
    }

    pub async fn get_all(conn: &mut DbConn<'_>) -> Result<Vec<Tournament>, DbError> {
        Ok(tournaments::table.get_results(conn).await?)
    }

    pub async fn find_by_tournament_id(
        tournament_id: &TournamentId,
        conn: &mut DbConn<'_>,
    ) -> Result<Tournament, DbError> {
        let TournamentId(id) = tournament_id;
        Ok(tournaments::table
            .filter(nanoid_field.eq(id))
            .first(conn)
            .await?)
    }
}
