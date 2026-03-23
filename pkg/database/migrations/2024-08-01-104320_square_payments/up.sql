-- Your SQL goes here

CREATE TABLE nfts (
    "id" uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    "url" text NOT NULL,
	"price" INTEGER NOT NULL,
	"payment_id" uuid,
    "claimed_by" char(64),
    "claimed_at" timestamp with time zone,
    "added_at" timestamp with time zone NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX nfts_unique_payment_id
	ON nfts(payment_id)
	WHERE payment_id IS NOT NULL;

CREATE UNIQUE INDEX nfts_url_key ON nfts(url text_ops);

CREATE TABLE "payments" (
    "id" uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    "product" TEXT NOT NULL, 
    "provider" TEXT NOT NULL,
    "external_id" TEXT,
    "data" JSONB NOT NULL DEFAULT '{}'::jsonb, -- extra data from vendor
    "amount" INTEGER NOT NULL,
    "currency" TEXT NOT NULL,
    "status" TEXT NOT NULL, -- PENDING, PAID, COMPLETE
    "payment_by" char(64),
    "added_at" timestamp with time zone NOT NULL DEFAULT now(),
    
    CONSTRAINT unique_provider_external_id UNIQUE("provider", "external_id")
);