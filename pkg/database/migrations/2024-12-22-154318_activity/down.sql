ALTER TABLE "ramps_accounts" DROP COLUMN "updated_at";

ALTER TABLE "ramps_transactions" DROP COLUMN "updated_at";

ALTER TABLE "rewards" DROP COLUMN "updated_at";

DROP TRIGGER IF EXISTS "set_updated_at_for_ramps_accounts" ON "ramps_accounts";

DROP TRIGGER IF EXISTS "set_updated_at_for_ramps_transactions" ON "ramps_transactions";

DROP TRIGGER IF EXISTS "set_updated_at_for_rewards" ON "rewards";
