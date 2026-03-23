CREATE TABLE "blocklist_ip"(
	"ip" BPCHAR(45) NOT NULL PRIMARY KEY,
	"data" JSONB NOT NULL DEFAULT '{}'::jsonb,
	"block" BOOL NOT NULL,
	"request_count" INTEGER NOT NULL DEFAULT 0,
	"request_count_reset_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"request_count_total" INTEGER NOT NULL DEFAULT 0,
	"added_at" timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TABLE "blocklist_mobile"(
	"mobile" BPCHAR(20) NOT NULL PRIMARY KEY,
	"reason" TEXT,
	"ip" BPCHAR(45),
	"block" BOOL NOT NULL,
	"added_at" timestamp with time zone NOT NULL DEFAULT now()
);

ALTER TABLE faucets DROP COLUMN phone_hash;


