use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::sync::OnceCell;

static MIGRATION_POOL: OnceCell<PgPool> = OnceCell::const_new();

async fn get_migration_pool() -> &'static PgPool {
    MIGRATION_POOL
        .get_or_init(|| async {
            let database_url = std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set for tests (e.g. postgresql://localhost:5432/schoolnify_test)");

            let pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&database_url)
                .await
                .expect("Failed to connect to test database for migrations");

            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("Failed to run migrations on test database");

            pool
        })
        .await
}

/// Create a fresh pool for each test. Migrations run once (via a separate pool).
pub async fn setup_test_db() -> PgPool {
    // Ensure migrations are run
    let _ = get_migration_pool().await;

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for tests");

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Truncate all tables. Use in tests that need a clean slate.
#[allow(dead_code)]
pub async fn truncate_all(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE refresh_tokens, users, organizations RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("Failed to truncate tables");
}
