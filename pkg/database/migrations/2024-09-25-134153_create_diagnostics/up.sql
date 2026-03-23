CREATE TABLE "diagnostics"(
	"id" UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
	"address" BPCHAR(64) NOT NULL,
	"backup_diffs" JSONB NOT NULL,
	"state" JSONB NOT NULL,
	"mnemonic" TEXT NOT NULL,
	"device_info" JSONB NOT NULL,
	"message" TEXT,
	"added_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

