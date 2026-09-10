use super::model::{
    LegacyGameRow,
    LegacyInvitationRow,
    LegacyMembershipRow,
    LegacyOrganizerRow,
    LegacyScheduleRow,
    LegacySeriesOrganizerRow,
    LegacySeriesRow,
    LegacySource,
    LegacySourceMetadata,
    LegacyTournamentRow,
};
use anyhow::{bail, Context, Result};
use db_lib::{helpers::run_read_only_repeatable_read, DbConn};
use diesel::{
    sql_query,
    sql_types::{Bool, Integer, Jsonb, Text},
};
use diesel_async::RunQueryDsl;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Clone, Copy)]
struct ExpectedTable {
    name: &'static str,
    columns: &'static [ExpectedColumn],
}

#[derive(Clone, Copy)]
struct ExpectedColumn {
    name: &'static str,
    data_type: &'static str,
    udt_name: &'static str,
    nullable: bool,
}

const fn column(
    name: &'static str,
    data_type: &'static str,
    udt_name: &'static str,
    nullable: bool,
) -> ExpectedColumn {
    ExpectedColumn {
        name,
        data_type,
        udt_name,
        nullable,
    }
}

const SOURCE_TABLES: &[ExpectedTable] = &[
    ExpectedTable {
        name: "games",
        columns: &[
            column("id", "uuid", "uuid", false),
            column("nanoid", "text", "text", false),
            column("current_player_id", "uuid", "uuid", false),
            column("black_id", "uuid", "uuid", false),
            column("finished", "boolean", "bool", false),
            column("game_status", "text", "text", false),
            column("history", "text", "text", false),
            column("game_control_history", "text", "text", false),
            column("turn", "integer", "int4", false),
            column("white_id", "uuid", "uuid", false),
            column(
                "created_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
            column(
                "updated_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
            column("time_mode", "text", "text", false),
            column("time_base", "integer", "int4", true),
            column("time_increment", "integer", "int4", true),
            column("hashes", "ARRAY", "_int8", false),
            column("conclusion", "text", "text", false),
            column("tournament_id", "uuid", "uuid", true),
            column("tournament_game_result", "text", "text", false),
            column("game_start", "text", "text", false),
            column("move_times", "ARRAY", "_int8", false),
        ],
    },
    ExpectedTable {
        name: "schedules",
        columns: &[
            column("id", "uuid", "uuid", false),
            column("game_id", "uuid", "uuid", false),
            column("tournament_id", "uuid", "uuid", false),
            column("proposer_id", "uuid", "uuid", false),
            column("opponent_id", "uuid", "uuid", false),
            column("start_t", "timestamp with time zone", "timestamptz", false),
            column("agreed", "boolean", "bool", false),
            column("notified", "boolean", "bool", false),
        ],
    },
    ExpectedTable {
        name: "tournament_series",
        columns: &[
            column("id", "uuid", "uuid", false),
            column("nanoid", "text", "text", false),
            column("name", "text", "text", false),
            column("description", "text", "text", false),
            column(
                "created_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
            column(
                "updated_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
        ],
    },
    ExpectedTable {
        name: "tournament_series_organizers",
        columns: &[
            column("tournament_series_id", "uuid", "uuid", false),
            column("organizer_id", "uuid", "uuid", false),
        ],
    },
    ExpectedTable {
        name: "tournaments",
        columns: &[
            column("id", "uuid", "uuid", false),
            column("nanoid", "text", "text", false),
            column("name", "text", "text", false),
            column("description", "text", "text", false),
            column("scoring", "text", "text", false),
            column("tiebreaker", "ARRAY", "_text", false),
            column("seats", "integer", "int4", false),
            column("min_seats", "integer", "int4", false),
            column("rounds", "integer", "int4", false),
            column("invite_only", "boolean", "bool", false),
            column("mode", "text", "text", false),
            column("time_mode", "text", "text", false),
            column("time_base", "integer", "int4", true),
            column("time_increment", "integer", "int4", true),
            column("band_upper", "integer", "int4", true),
            column("band_lower", "integer", "int4", true),
            column("start_mode", "text", "text", false),
            column("starts_at", "timestamp with time zone", "timestamptz", true),
            column("ends_at", "timestamp with time zone", "timestamptz", true),
            column(
                "started_at",
                "timestamp with time zone",
                "timestamptz",
                true,
            ),
            column("round_duration", "integer", "int4", true),
            column("status", "text", "text", false),
            column(
                "created_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
            column(
                "updated_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
            column("series", "uuid", "uuid", true),
        ],
    },
    ExpectedTable {
        name: "tournaments_invitations",
        columns: &[
            column("tournament_id", "uuid", "uuid", false),
            column("invitee_id", "uuid", "uuid", false),
            column(
                "created_at",
                "timestamp with time zone",
                "timestamptz",
                false,
            ),
        ],
    },
    ExpectedTable {
        name: "tournaments_organizers",
        columns: &[
            column("tournament_id", "uuid", "uuid", false),
            column("organizer_id", "uuid", "uuid", false),
        ],
    },
    ExpectedTable {
        name: "tournaments_users",
        columns: &[
            column("tournament_id", "uuid", "uuid", false),
            column("user_id", "uuid", "uuid", false),
        ],
    },
];

#[derive(diesel::QueryableByName)]
struct DatabaseNameQuery {
    #[diesel(sql_type = Text)]
    database_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, diesel::QueryableByName)]
struct InformationSchemaTableQuery {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = Text)]
    table_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, diesel::QueryableByName)]
struct InformationSchemaColumnQuery {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = Text)]
    column_name: String,
    #[diesel(sql_type = Text)]
    data_type: String,
    #[diesel(sql_type = Text)]
    udt_name: String,
    #[diesel(sql_type = Bool)]
    is_nullable: bool,
    #[diesel(sql_type = Integer)]
    ordinal_position: i32,
}

#[derive(diesel::QueryableByName)]
struct JsonRow {
    #[diesel(sql_type = Jsonb)]
    row: Value,
}

/// Loads every legacy tournament fact from one coherent, read-only snapshot.
///
/// The current Diesel schema describes the post-cutover database and must never
/// be used here. All source reads below are deliberately raw SQL, and schema
/// validation runs before any query names a legacy-only column.
pub async fn load(conn: &mut DbConn<'_>) -> Result<LegacySource> {
    run_read_only_repeatable_read(conn, |conn| {
        Box::pin(async move { load_snapshot(conn).await })
    })
    .await
}

async fn load_snapshot(conn: &mut DbConn<'_>) -> Result<LegacySource> {
    let database_name = sql_query("SELECT current_database()::text AS database_name")
        .get_result::<DatabaseNameQuery>(conn)
        .await
        .context("could not identify the legacy source database")?
        .database_name;

    let information_schema_tables = sql_query(
        "SELECT table_name::text AS table_name,
                table_type::text AS table_type
         FROM information_schema.tables
         WHERE table_schema = 'public'
         ORDER BY table_name COLLATE \"C\"",
    )
    .load::<InformationSchemaTableQuery>(conn)
    .await
    .context("could not inspect the legacy source tables")?;
    validate_tables(&information_schema_tables)?;

    let information_schema_columns = sql_query(
        "SELECT table_name::text AS table_name,
                column_name::text AS column_name,
                data_type::text AS data_type,
                udt_name::text AS udt_name,
                (is_nullable = 'YES') AS is_nullable,
                ordinal_position::integer AS ordinal_position
         FROM information_schema.columns
         WHERE table_schema = 'public'
         ORDER BY table_name COLLATE \"C\", ordinal_position",
    )
    .load::<InformationSchemaColumnQuery>(conn)
    .await
    .context("could not inspect the legacy source schema")?;
    validate_schema(&information_schema_columns)?;

    let metadata = LegacySourceMetadata { database_name };

    let tournaments = load_json_rows::<LegacyTournamentRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (
             SELECT id, nanoid, name, description, scoring, tiebreaker,
                    seats, min_seats, rounds, invite_only, mode, time_mode,
                    time_base, time_increment, band_upper, band_lower,
                    start_mode, starts_at, ends_at, started_at, round_duration,
                    status, created_at, updated_at, series
             FROM public.tournaments
         ) AS source_row
         ORDER BY source_row.id",
        "tournaments",
    )
    .await?;
    let memberships = load_json_rows::<LegacyMembershipRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (SELECT tournament_id, user_id FROM public.tournaments_users) AS source_row
         ORDER BY source_row.tournament_id, source_row.user_id",
        "tournament memberships",
    )
    .await?;
    let organizers = load_json_rows::<LegacyOrganizerRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (SELECT tournament_id, organizer_id FROM public.tournaments_organizers) AS source_row
         ORDER BY source_row.tournament_id, source_row.organizer_id",
        "tournament organizers",
    )
    .await?;
    let invitations = load_json_rows::<LegacyInvitationRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (
             SELECT tournament_id, invitee_id, created_at,
                    NULL::timestamptz AS declined_at
             FROM public.tournaments_invitations
         ) AS source_row
         ORDER BY source_row.tournament_id, source_row.invitee_id",
        "tournament invitations",
    )
    .await?;
    let games = load_json_rows::<LegacyGameRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (
             SELECT id, nanoid, current_player_id, black_id, finished,
                    game_status, history, game_control_history,
                    turn, white_id,
                    created_at, updated_at, time_mode, time_base,
                    time_increment, white_rating, black_rating, hashes,
                    conclusion, tournament_id,
                    tournament_game_result, game_start, move_times
             FROM public.games
             WHERE tournament_id IS NOT NULL
         ) AS source_row
         ORDER BY source_row.tournament_id, source_row.created_at, source_row.id",
        "tournament games",
    )
    .await?;
    let schedules = load_json_rows::<LegacyScheduleRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (
             SELECT id, game_id, tournament_id, proposer_id, opponent_id,
                    start_t AS starts_at, agreed, notified
             FROM public.schedules
         ) AS source_row
         ORDER BY source_row.tournament_id, source_row.id",
        "tournament schedules",
    )
    .await?;
    let series = load_json_rows::<LegacySeriesRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (
             SELECT id, nanoid, name, description, created_at, updated_at
             FROM public.tournament_series
             WHERE id IN (
                 SELECT tournament.series
                 FROM public.tournaments AS tournament
                 WHERE tournament.series IS NOT NULL
             )
         ) AS source_row
         ORDER BY source_row.id",
        "referenced tournament series",
    )
    .await?;
    let series_organizers = load_json_rows::<LegacySeriesOrganizerRow>(
        conn,
        "SELECT to_jsonb(source_row) AS row
         FROM (
             SELECT series_organizer.tournament_series_id AS series_id,
                    series_organizer.organizer_id
             FROM public.tournament_series_organizers AS series_organizer
             WHERE EXISTS (
                 SELECT 1
                 FROM public.tournaments AS tournament
                 WHERE tournament.series = series_organizer.tournament_series_id
             )
         ) AS source_row
         ORDER BY source_row.series_id, source_row.organizer_id",
        "referenced tournament series organizers",
    )
    .await?;
    Ok(LegacySource {
        metadata,
        tournaments,
        memberships,
        organizers,
        invitations,
        games,
        schedules,
        series,
        series_organizers,
    })
}

async fn load_json_rows<T>(
    conn: &mut DbConn<'_>,
    query: &'static str,
    description: &'static str,
) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    sql_query(query)
        .load::<JsonRow>(conn)
        .await
        .with_context(|| format!("could not read legacy {description}"))?
        .into_iter()
        .map(|row| {
            serde_json::from_value(row.row)
                .with_context(|| format!("legacy {description} contained an invalid row"))
        })
        .collect()
}

fn validate_schema(actual: &[InformationSchemaColumnQuery]) -> Result<()> {
    for expected_table in SOURCE_TABLES {
        let actual_columns = actual
            .iter()
            .filter(|column| column.table_name == expected_table.name)
            .collect::<Vec<_>>();
        for expected in expected_table.columns {
            let found = actual_columns
                .iter()
                .find(|found| found.column_name == expected.name)
                .with_context(|| {
                    format!(
                        "legacy source is missing required column {}.{}",
                        expected_table.name, expected.name
                    )
                })?;
            if found.data_type != expected.data_type
                || found.udt_name != expected.udt_name
                || found.is_nullable != expected.nullable
            {
                bail!(
                    "legacy source schema mismatch at {}.{}: expected type {}/{} nullable={}, found type {}/{} nullable={}",
                    expected_table.name,
                    expected.name,
                    expected.data_type,
                    expected.udt_name,
                    expected.nullable,
                    found.data_type,
                    found.udt_name,
                    found.is_nullable,
                );
            }
        }
    }

    Ok(())
}

fn validate_tables(actual: &[InformationSchemaTableQuery]) -> Result<()> {
    for expected in SOURCE_TABLES {
        let found = actual
            .iter()
            .find(|found| found.table_name == expected.name)
            .with_context(|| {
                format!("legacy source is missing required table {}", expected.name)
            })?;
        if found.table_type != "BASE TABLE" {
            bail!(
                "legacy source public relation mismatch: expected base table {}, found {} ({})",
                expected.name,
                found.table_name,
                found.table_type,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_tables() -> Vec<InformationSchemaTableQuery> {
        SOURCE_TABLES
            .iter()
            .map(|table| InformationSchemaTableQuery {
                table_name: table.name.to_string(),
                table_type: "BASE TABLE".to_string(),
            })
            .collect()
    }

    fn expected_schema() -> Vec<InformationSchemaColumnQuery> {
        SOURCE_TABLES
            .iter()
            .flat_map(|table| {
                table.columns.iter().enumerate().map(|(index, column)| {
                    InformationSchemaColumnQuery {
                        table_name: table.name.to_string(),
                        column_name: column.name.to_string(),
                        data_type: column.data_type.to_string(),
                        udt_name: column.udt_name.to_string(),
                        is_nullable: column.nullable,
                        ordinal_position: i32::try_from(index + 1).unwrap(),
                    }
                })
            })
            .collect()
    }

    #[test]
    fn source_shape_accepts_extensions_and_rejects_required_column_drift() {
        let mut tables = expected_tables();
        tables.push(InformationSchemaTableQuery {
            table_name: "mystery_tournament_facts".to_string(),
            table_type: "BASE TABLE".to_string(),
        });
        tables.sort_by(|left, right| left.table_name.cmp(&right.table_name));
        assert!(validate_tables(&tables).is_ok());

        let mut schema = expected_schema();
        schema.push(InformationSchemaColumnQuery {
            table_name: "mystery_tournament_facts".to_string(),
            column_name: "tournament_id".to_string(),
            data_type: "uuid".to_string(),
            udt_name: "uuid".to_string(),
            is_nullable: false,
            ordinal_position: 1,
        });
        schema.sort_by(|left, right| {
            (&left.table_name, left.ordinal_position)
                .cmp(&(&right.table_name, right.ordinal_position))
        });
        assert!(validate_schema(&schema).is_ok());

        let mut schema = expected_schema();
        let tournament_mode = schema
            .iter_mut()
            .find(|column| column.table_name == "tournaments" && column.column_name == "mode")
            .unwrap();
        tournament_mode.column_name = "format".to_string();

        let error = validate_schema(&schema).unwrap_err().to_string();
        assert!(error.contains("tournaments.mode"), "{error}");
        assert!(error.contains("missing required column"), "{error}");

        let mut schema = expected_schema();
        let tournament_mode = schema
            .iter_mut()
            .find(|column| column.table_name == "tournaments" && column.column_name == "mode")
            .unwrap();
        tournament_mode.data_type = "integer".to_string();
        tournament_mode.udt_name = "int4".to_string();

        let error = validate_schema(&schema).unwrap_err().to_string();
        assert!(error.contains("tournaments.mode"), "{error}");
        assert!(error.contains("expected type text/text"), "{error}");
        assert!(error.contains("found type integer/int4"), "{error}");
    }
}
