-- This file should undo anything in `up.sql`
ALTER TABLE "ramps_events"
DROP COLUMN IF EXISTS success;
