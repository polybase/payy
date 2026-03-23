ALTER TABLE "notes"
ADD COLUMN "source" TEXT;

ALTER TABLE "notes"
ADD COLUMN "token" TEXT;

ALTER TABLE "notes"
DROP COLUMN "version";

ALTER TABLE "notes"
ALTER COLUMN "received_ref_id" TYPE UUID;

ALTER TABLE "notes"
ALTER COLUMN "spend_ref_id" TYPE UUID;
