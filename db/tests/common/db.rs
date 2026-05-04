use db_lib::{get_conn, get_pool, DbPool};
use diesel::{sql_types::Text, QueryableByName};
use diesel_async::{AsyncConnection, AsyncMigrationHarness, AsyncPgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::sync::OnceLock;
use tokio::sync::{Mutex, MutexGuard};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

static DB_LOCK: Mutex<()> = Mutex::const_new(());
static MIGRATED: OnceLock<()> = OnceLock::new();

#[derive(QueryableByName)]
struct CurrentDatabase {
    #[diesel(sql_type = Text)]
    name: String,
}

pub struct TestDb {
    pub pool: DbPool,
    _lock: MutexGuard<'static, ()>,
}

pub async fn test_db() -> TestDb {
    let lock = DB_LOCK.lock().await;
    let database_url = test_database_url();

    if MIGRATED.get().is_none() {
        let mut conn = AsyncPgConnection::establish(&database_url)
            .await
            .expect("connect to test database for migration");
        assert_test_database(&mut conn).await;
        let mut harness = AsyncMigrationHarness::new(conn);
        harness
            .run_pending_migrations(MIGRATIONS)
            .expect("run test database migrations");
        MIGRATED.set(()).ok();
    }

    let pool = get_pool(&database_url)
        .await
        .expect("create test database pool");
    truncate(&pool).await;

    TestDb { pool, _lock: lock }
}

fn test_database_url() -> String {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set")
        .replace("@localhost:/", "@localhost/");
    database_url
}

pub async fn truncate(pool: &DbPool) {
    let mut conn = get_conn(pool)
        .await
        .expect("get test database connection for truncate");
    assert_test_database(&mut conn).await;
    diesel::sql_query(
        r#"
        DO $$
        DECLARE
            stmt text;
        BEGIN
            SELECT 'TRUNCATE TABLE ' ||
                string_agg(format('%I.%I', schemaname, tablename), ', ') ||
                ' RESTART IDENTITY CASCADE'
            INTO stmt
            FROM pg_tables
            WHERE schemaname = 'public'
                AND tablename <> '__diesel_schema_migrations';

            IF stmt IS NOT NULL THEN
                EXECUTE stmt;
            END IF;
        END $$;
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("truncate test database");
}

async fn assert_test_database(conn: &mut AsyncPgConnection) {
    let CurrentDatabase { name } = diesel::sql_query("SELECT current_database() AS name")
        .get_result(conn)
        .await
        .expect("query current database before destructive test operation");

    assert!(
        is_test_database_name(&name),
        "refusing destructive DB test operation against non-test database: {name}"
    );
}

fn is_test_database_name(name: &str) -> bool {
    name.split(|c| c == '_' || c == '-')
        .any(|part| part == "test")
}

#[cfg(test)]
mod tests {
    use super::is_test_database_name;

    #[test]
    fn test_database_names_must_identify_the_database_itself_as_test() {
        assert!(is_test_database_name("test"));
        assert!(is_test_database_name("test_hive"));
        assert!(is_test_database_name("hive-test"));
        assert!(is_test_database_name("hive_test_db"));

        assert!(!is_test_database_name("hive"));
        assert!(!is_test_database_name("contest"));
        assert!(!is_test_database_name("latest"));
    }
}
