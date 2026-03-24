mod postgres_fixture;
mod schema;

use serial_test::serial;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use postgres_fixture::{docker_available, DockerPostgres, FixtureError};
use schema::setup_schema;

type TestResult<T> = std::result::Result<T, FixtureError>;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn replit_column_permissions_enforced() -> TestResult<()> {
    if !docker_available() {
        eprintln!(
            "skipping replit_column_permissions_enforced: docker binary not available on PATH"
        );
        return Ok(());
    }

    let fixture = match DockerPostgres::start().await {
        Ok(fixture) => fixture,
        Err(err) => {
            if err.is_missing_docker() {
                eprintln!(
                    "skipping replit_column_permissions_enforced: docker binary not available ({err})"
                );
                return Ok(());
            }

            return Err(err);
        }
    };
    let db_name = format!("replit_perm_{}", Uuid::new_v4().simple());
    fixture.recreate_database(&db_name).await?;
    setup_schema(&fixture, &db_name).await?;

    let replit_client = fixture.connect(&db_name, "replit", "replit").await?;
    let rows = replit_client
        .query("SELECT private_key FROM notes", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    let rows = replit_client
        .query("SELECT private_key FROM ramps_transactions", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    drop(replit_client);

    fixture
        .apply_sql(
            &db_name,
            include_str!(
                "../../migrations/2025-09-30-155759_restrict_replit_sensitive_columns/up.sql"
            ),
        )
        .await?;

    let replit_client = fixture.connect(&db_name, "replit", "replit").await?;

    let err = replit_client
        .query("SELECT private_key FROM notes", &[])
        .await
        .expect_err("replit should not read notes.private_key after migration");
    assert_eq!(err.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE));

    let rows = replit_client
        .query("SELECT private_key FROM ramps_transactions", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    let rows = replit_client
        .query("SELECT id, value FROM notes", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    let rows = replit_client
        .query("SELECT id, status FROM ramps_transactions", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    drop(replit_client);

    fixture
        .apply_sql(
            &db_name,
            include_str!(
                "../../migrations/2025-09-30-155759_restrict_replit_sensitive_columns/down.sql"
            ),
        )
        .await?;

    let replit_client = fixture.connect(&db_name, "replit", "replit").await?;
    let rows = replit_client
        .query("SELECT private_key FROM notes", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    let rows = replit_client
        .query("SELECT private_key FROM ramps_transactions", &[])
        .await?;
    assert_eq!(rows.len(), 1);

    Ok(())
}
