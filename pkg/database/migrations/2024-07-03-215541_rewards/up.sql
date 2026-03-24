CREATE TABLE "rewards"(
	"address" BPCHAR(64) NOT NULL PRIMARY KEY,
	"code" BPCHAR(6) NOT NULL,
	"points" INTEGER NOT NULL DEFAULT 0,
	"invites" INTEGER NOT NULL DEFAULT 0,
	"claims" JSONB NOT NULL DEFAULT '{}'::jsonb,
	"prize" TEXT,
	"added_at" timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TABLE "rewards_points"(
	"id" UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
	"address" BPCHAR(64) NOT NULL,
	"reason" TEXT NOT NULL,
	"points" INTEGER NOT NULL,
	"added_at" timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TABLE "rewards_invites"(
	"from_address" BPCHAR(64) NOT NULL,
	"to_address" BPCHAR(64) NOT NULL UNIQUE,
	"added_at" timestamp with time zone NOT NULL DEFAULT now(),
	PRIMARY KEY("from_address", "to_address")
);

