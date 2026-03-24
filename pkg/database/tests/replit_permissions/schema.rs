use contextful::ResultContextExt;
use tokio_postgres::Client;
use uuid::Uuid;

use crate::postgres_fixture::{DockerPostgres, FixtureError};

type Result<T> = std::result::Result<T, FixtureError>;

const CREATE_TABLES_SQL: &str = include_str!("sql/create_tables.sql");

pub async fn setup_schema(fixture: &DockerPostgres, db: &str) -> Result<()> {
    let client = fixture
        .connect_superuser(db)
        .await
        .context("connect as superuser to set up schema")?;

    client
        .batch_execute(CREATE_TABLES_SQL)
        .await
        .context("apply initial tables for replit permissions test")?;

    seed_data(&client).await?;
    ensure_replit_role(&client, db).await?;

    Ok(())
}

async fn seed_data(client: &Client) -> Result<()> {
    let note_id = Uuid::new_v4();
    let txn_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let account_id = Uuid::new_v4();
    let wallet_id = Uuid::new_v4();
    let address_value = "A".repeat(64);
    let note_private_key = "B".repeat(64);
    let note_psi = "C".repeat(64);
    let commitment = "D".repeat(64);

    client
        .execute(
            "INSERT INTO notes (id, address, private_key, psi, value, owner_id, status, commitment)
             VALUES ($1, $2, $3, $4, 42, $5, 'UNSPENT', $6)",
            &[
                &note_id,
                &address_value,
                &note_private_key,
                &note_psi,
                &owner_id,
                &commitment,
            ],
        )
        .await
        .context("seed initial test note for replit permissions test")?;

    client
        .execute(
            "INSERT INTO ramps_transactions (
                id, provider, account_id, status, from_currency, from_amount, from_network,
                to_currency, to_amount, to_network, category, added_at, updated_at,
                wallet_id, private_key, funding_kind, from_note_kind, to_note_kind
            ) VALUES (
                $1, 'MOCK', $2, 'PENDING', 'USD', 100, 'ACH', 'USDC', 100, 'ETH', 'PAYIN', NOW(), NOW(),
                $3, 'rampsprivkey', 'CRYPTO', NULL, NULL
            )",
            &[&txn_id, &account_id, &wallet_id],
        )
        .await
        .context("seed initial ramps transaction for replit permissions test")?;

    Ok(())
}

async fn ensure_replit_role(client: &Client, db: &str) -> Result<()> {
    client
        .batch_execute(&format!(
            "DO $$BEGIN
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'replit') THEN
                    CREATE ROLE replit LOGIN PASSWORD 'replit';
                ELSE
                    ALTER ROLE replit WITH LOGIN PASSWORD 'replit';
                END IF;
            END$$;
            GRANT CONNECT ON DATABASE {db} TO replit;
            GRANT USAGE ON SCHEMA public TO replit;
            GRANT SELECT ON TABLE notes TO replit;
            GRANT SELECT ON TABLE ramps_transactions TO replit;"
        ))
        .await
        .context("ensure replit role has baseline privileges")?;

    Ok(())
}
