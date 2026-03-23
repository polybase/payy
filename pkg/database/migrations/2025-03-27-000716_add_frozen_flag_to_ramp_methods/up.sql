-- Your SQL goes here
ALTER TABLE "ramps_methods"
ADD COLUMN frozen BOOLEAN NOT NULL DEFAULT FALSE;
