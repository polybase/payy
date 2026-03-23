ALTER TABLE "notes" ADD COLUMN "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE "notes" ADD COLUMN "commitment" BPCHAR(64) NOT NULL;
CREATE UNIQUE INDEX notes_commitment_idx ON notes (commitment);


ALTER TABLE "ramps_accounts" ADD COLUMN "kyc_delegated_id" UUID REFERENCES ramps_accounts(id);
ALTER TABLE "ramps_transactions" ADD COLUMN "icon" TEXT;


ALTER TABLE "wallets" ALTER COLUMN "expo_push_token" DROP NOT NULL;
ALTER TABLE "wallets" ALTER COLUMN "deposit_address" DROP NOT NULL;
CREATE UNIQUE INDEX wallets_address_idx ON wallets (address);


CREATE TRIGGER set_updated_at_for_notes
BEFORE UPDATE ON notes
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE UNIQUE INDEX ramps_transactions_provider_external_id_idx ON ramps_transactions (provider, external_id) WHERE external_id IS NOT NULL;
