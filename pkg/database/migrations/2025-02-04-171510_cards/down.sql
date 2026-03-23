DROP INDEX IF EXISTS ramps_transactions_provider_external_id_idx;

DROP TRIGGER IF EXISTS set_updated_at_for_notes ON notes;

DROP INDEX IF EXISTS wallets_address_idx;

ALTER TABLE "wallets" ALTER COLUMN "deposit_address" SET NOT NULL;
ALTER TABLE "wallets" ALTER COLUMN "expo_push_token" SET NOT NULL;

ALTER TABLE "ramps_transactions" DROP COLUMN "icon";

ALTER TABLE "ramps_accounts" DROP COLUMN "kyc_delegated_id";

DROP INDEX IF EXISTS notes_commitment_idx;
ALTER TABLE "notes" DROP COLUMN "commitment";
ALTER TABLE "notes" DROP COLUMN "updated_at";
