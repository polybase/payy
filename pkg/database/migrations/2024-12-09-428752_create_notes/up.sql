CREATE TABLE "notes"(
	"id" UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    "address" BPCHAR(64) NOT NULL,
    "private_key" BPCHAR(64) NOT NULL,
    "psi" BPCHAR(64) NOT NULL,
    "value" NUMERIC NOT NULL,
    "source" BPCHAR(64) NOT NULL,
    "token" TEXT NOT NULL,
    "owner_id" UUID NOT NULL,

    "status" TEXT NOT NULL,
    "parent_id" UUID,
    "claim_reason" TEXT,
    "claim_id" UUID UNIQUE,
    "claimed_at" TIMESTAMP WITH TIME ZONE,

    "added_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);
