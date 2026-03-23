-- Revert column renames
ALTER TABLE "notes" RENAME COLUMN "received_ref_kind" TO "claim_reason";
ALTER TABLE "notes" RENAME COLUMN "received_ref_id" TO "claim_id";
ALTER TABLE "notes" RENAME COLUMN "spent_at" TO "claimed_at";
ALTER TABLE "notes" RENAME COLUMN "parent_1_id" TO "parent_id";

-- Remove newly added columns
ALTER TABLE "notes" DROP COLUMN "spend_ref_kind";
ALTER TABLE "notes" DROP COLUMN "spend_ref_id";
ALTER TABLE "notes" DROP COLUMN "parent_2_id";
