ALTER TABLE "ramps_accounts" ADD COLUMN "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE "ramps_transactions" ADD COLUMN "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE "rewards" ADD COLUMN "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW();


CREATE TRIGGER set_updated_at_for_ramps_accounts
BEFORE UPDATE ON ramps_accounts
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER set_updated_at_for_ramps_transactions
BEFORE UPDATE ON ramps_transactions
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER set_updated_at_for_rewards
BEFORE UPDATE ON rewards
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

