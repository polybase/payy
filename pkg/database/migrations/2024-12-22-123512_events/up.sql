CREATE TABLE "ramps_events"(
	"id" UUID NOT NULL PRIMARY KEY,
	"provider" TEXT NOT NULL,
	"data" JSONB NOT NULL,
	"source" TEXT NOT NULL,
	"added_at" TIMESTAMPTZ NOT NULL
);
