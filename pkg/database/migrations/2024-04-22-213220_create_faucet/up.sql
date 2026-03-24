-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE faucets (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    url text NOT NULL,
    claimed_by char(64),
    claimed_at timestamp with time zone,
    added_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX faucets_url_key ON faucets(url text_ops);