CREATE TABLE support_canned_responses (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "name" TEXT UNIQUE NOT NULL,
    "display_name" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE support_canned_response_tags (
    "support_canned_response_id" UUID NOT NULL,
    "support_tag_id" UUID NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("support_canned_response_id", "support_tag_id"),
    CONSTRAINT fk_canned_response FOREIGN KEY ("support_canned_response_id")
        REFERENCES support_canned_responses("id") ON DELETE CASCADE,
    CONSTRAINT fk_support_tag FOREIGN KEY ("support_tag_id")
        REFERENCES support_tags("id") ON DELETE CASCADE
);

CREATE INDEX idx_canned_responses_is_active ON support_canned_responses (is_active);
CREATE INDEX idx_canned_response_tags_tag_id ON support_canned_response_tags (support_tag_id);

CREATE TRIGGER update_support_canned_responses_updated_at
    BEFORE UPDATE ON support_canned_responses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
