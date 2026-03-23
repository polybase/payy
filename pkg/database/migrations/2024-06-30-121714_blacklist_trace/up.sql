-- Your SQL goes here

ALTER TABLE "blocklist_mobile" ADD COLUMN "country_code" BPCHAR(2);

CREATE TABLE "blocklist_trace"(
	id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
	"path" TEXT NOT NULL,
	"mobile" BPCHAR(20) NOT NULL,
	"ip" BPCHAR(45),
	"added_at" timestamp with time zone NOT NULL DEFAULT now()
);

