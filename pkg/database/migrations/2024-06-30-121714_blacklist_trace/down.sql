-- This file should undo anything in `up.sql`

ALTER TABLE "blocklist_mobile" DROP COLUMN "country_code";

DROP TABLE IF EXISTS "blocklist_trace";
